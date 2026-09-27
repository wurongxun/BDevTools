//! # 日志索引与筛选（供前端 logs 插件使用）
//!
//! 本模块是**纯业务层**：不出现 `#[tauri::command]`、不依赖 `tauri` crate，
//! 命令包装统一放在 `commands.rs`（见该文件头部的架构约定）。
//!
//! ## 为什么需要服务端索引
//! 前端分块懒加载时只持有部分行，无法回答"整个文件里有多少 ERROR"这类问题。
//! 级别/时间筛选若只作用于已加载的分块，结果会随滚动不断变化，无法接受。
//! 因此筛选以服务端全量扫描结果为准（前端只按返回的行段去取内容渲染）。
//!
//! ## 解析口径必须与前端保持一致
//! 前端实现见 `src/plugins/logs/core/parser.ts` 与 `core/patterns.ts`：
//! - 级别词表与别名相同（FATAL/CRITICAL/CRIT/SEVERE/PANIC → FATAL ……）
//! - 时间戳支持同样的三种格式
//! - **级别继承**：`thrown` / `Caused by` 首行为 ERROR，其后的堆栈帧继承同一级别，
//!   异常块结束后回退到最近一条普通行的级别——否则筛选 ERROR 时会丢掉整个堆栈
//! - **时间戳继承**：无时间戳行（堆栈帧、续行）继承上一条有效时间戳，
//!   否则时间范围筛选会把异常块从中间切断

use crate::plugins::logs::search_log_parallel::{read_line_lossy, resolve_encoding};
use crate::response::resp_data::ResData;
use chrono::{Local, NaiveDate, TimeZone};
use encoding_rs::Encoding;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use rayon::prelude::*;
use regex::{Regex, RegexBuilder};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

// ==================== 级别编码 ====================
// 与前端 LogLevel 一一对应，按"严重程度"升序，便于用 >= 做阈值比较

pub const LV_UNKNOWN: u8 = 0;
pub const LV_TRACE: u8 = 1;
pub const LV_DEBUG: u8 = 2;
pub const LV_INFO: u8 = 3;
pub const LV_WARN: u8 = 4;
pub const LV_ERROR: u8 = 5;
pub const LV_FATAL: u8 = 6;

/// 级别码个数（前端 levels 数组长度）
const LV_COUNT: usize = 7;

/** 无时间戳的行使用的时间戳值（时间筛选激活时这些行会被排除） */
const NO_TS: i64 = -1;

/** 单文件参与索引的最大行数：超出部分仍计入总行数，但不参与筛选与时间轴统计 */
const MAX_INDEX_LINES: u64 = 8_000_000;

/** 时间轴桶数量（后续批次的时间轴视图直接消费） */
const BUCKET_COUNT: usize = 240;

/**
 * 索引缓存的**字节预算**（所有条目估算内存之和的上限）。
 *
 * 为什么按字节而不是按条目数：单条目内存方差极大（小文件几 MB、10GB 文件约 100MB），
 * 固定条目数会让"3 个小文件"与"3 个 10GB 文件"被同等对待——前者远未触顶却把
 * 仍在使用的条目清掉，后者一旦超限又是整表清空（§7.8）。
 */
const INDEX_CACHE_BUDGET: usize = 512 * 1024 * 1024;

/** 单个查询最多返回多少段，避免稀疏命中时传输超大数组 */
const DEFAULT_MAX_SEGMENTS: usize = 2000;

/// 逻辑访问序号：LRU 排序依据（单调递增，无锁）
static CACHE_CLOCK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_cache_clock() -> u64 {
    CACHE_CLOCK.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

// ==================== 正则（与前端 patterns.ts 一致） ====================

lazy_static! {
    static ref TS_RE: Regex = Regex::new(
        r"^\s*\[?(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:[.,]\d{1,9})?(?:Z|[+-]\d{2}:?\d{2})?|\d{4}/\d{1,2}/\d{1,2}[T ]\d{2}:\d{2}:\d{2}(?:[.,]\d{1,3})?|\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:[.,]\d{1,3})?)\]?"
    )
    .unwrap();
    static ref LEVEL_RE: Regex = Regex::new(
        r"(?i)\b(FATAL|CRITICAL|CRIT|SEVERE|PANIC|ERROR|ERR|EXCEPTION|WARNING|WARN|INFORMATION|INFO|NOTICE|DEBUG|TRACE|VERBOSE|FINEST|FINER|FINE)\b"
    )
    .unwrap();
    static ref JAVA_FRAME_RE: Regex =
        Regex::new(r"^\s*at\s+((?:[\w$]+\.)+)([\w$<>]+)\(([^)]*)\)\s*$").unwrap();
    static ref PYTHON_FRAME_RE: Regex =
        Regex::new(r#"^\s*File\s+"([^"]+)",\s*line\s+(\d+)(?:,\s*in\s+(.+))?\s*$"#).unwrap();
    static ref JAVA_CAUSE_RE: Regex =
        Regex::new(r"^\s*Caused by:\s*((?:[\w$]+\.)+[\w$]+)(?::\s?(.*))?$").unwrap();
    static ref JAVA_THROWN_RE: Regex =
        Regex::new(r"^\s*((?:[\w$]+\.)*[A-Z][\w$]*(?:Exception|Error|Throwable))(?::\s?(.*))?$")
            .unwrap();
    static ref PYTHON_TRACEBACK_RE: Regex =
        Regex::new(r"^\s*Traceback \(most recent call last\):\s*$").unwrap();
    static ref STACK_MORE_RE: Regex =
        Regex::new(r"^\s*\.\.\.\s+(\d+)\s+(?:more|common frames omitted)\s*$").unwrap();
    /// 索引缓存：key = 文件路径 + 编码
    ///
    /// 值为 `IndexEntry`，持有一个**不可变快照** `Arc<IndexSnapshot>`。
    /// 构建分两个阶段推进（先 L2 偏移、后 L1 块级聚合），每阶段结束整体替换快照，
    /// 读侧克隆 `Arc` 即可无锁访问——这样"构建中"也能读到已完成的部分（§2.2.1）。
    static ref INDEX_CACHE: Mutex<HashMap<String, IndexEntry>> = Mutex::new(HashMap::new());
    /// 每个缓存条目一把异步锁：并发请求串行化构建，避免同一文件被重复扫描
    static ref BUILD_LOCKS: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>> =
        Mutex::new(HashMap::new());
}

// ==================== 行分析 ====================

/** 级别词 → 级别码（与前端 LEVEL_ALIASES 完全对应） */
fn level_from_word(word: &str) -> Option<u8> {
    match word {
        "FATAL" | "CRITICAL" | "CRIT" | "SEVERE" | "PANIC" => Some(LV_FATAL),
        "ERROR" | "ERR" | "EXCEPTION" => Some(LV_ERROR),
        "WARNING" | "WARN" => Some(LV_WARN),
        "INFORMATION" | "INFO" | "NOTICE" => Some(LV_INFO),
        "DEBUG" | "FINE" | "FINER" => Some(LV_DEBUG),
        "TRACE" | "VERBOSE" | "FINEST" => Some(LV_TRACE),
        _ => None,
    }
}

/**
 * 判定堆栈语义。
 * - `None`：普通日志行
 * - `Some(None)`：堆栈帧/续行，级别需要继承
 * - `Some(Some(lv))`：异常首行，自带级别
 */
fn stack_kind(line: &str) -> Option<Option<u8>> {
    if JAVA_FRAME_RE.is_match(line) {
        return Some(None);
    }
    if PYTHON_FRAME_RE.is_match(line) {
        return Some(None);
    }
    if JAVA_CAUSE_RE.is_match(line) {
        return Some(Some(LV_ERROR));
    }
    if JAVA_THROWN_RE.is_match(line) {
        return Some(Some(LV_ERROR));
    }
    if PYTHON_TRACEBACK_RE.is_match(line) {
        return Some(None);
    }
    if STACK_MORE_RE.is_match(line) {
        return Some(None);
    }
    None
}

/** 安全切片：从字节偏移 from 起取最多 max_chars 个字符，避免切断 UTF-8 边界 */
fn safe_window(line: &str, from: usize, max_chars: usize) -> &str {
    let mut start = from.min(line.len());
    while start < line.len() && !line.is_char_boundary(start) {
        start += 1;
    }
    let rest = &line[start..];
    match rest.char_indices().nth(max_chars) {
        Some((i, _)) => &rest[..i],
        None => rest,
    }
}

/** 级别词扫描窗口（时间戳之后的字符上限）：覆盖含长 logger / MDC 前缀、级别排在较后的格式 */
const LEVEL_SCAN_WINDOW: usize = 256;

/**
 * 在一段文本里找"最像日志级别"的词并返回级别码（无则 None）。
 *
 * 同一行里可能同时出现多个级别词（如正文里写了 "error"），按结构化程度打分取最优：
 * - 3 分：被括号包裹 `[ERROR]` / `(ERROR)` / `<ERROR>`
 * - 2 分：大写形式 `ERROR`（日志级别惯例为大写）
 * - 1 分：小写形式 `error`
 * 同分时取首次出现，避免把正文里的 "fatal/error" 误判为行级别。
 */
fn best_level_in(text: &str) -> Option<u8> {
    let bytes = text.as_bytes();
    let mut best: Option<(u8, u8)> = None; // (score, code)
    for c in LEVEL_RE.captures_iter(text) {
        let m = match c.get(1) {
            Some(m) => m,
            None => break,
        };
        let word = m.as_str();
        let code = level_from_word(&word.to_uppercase())?;
        let upper = word.chars().any(|ch| ch.is_ascii_uppercase());
        let prev = m.start().checked_sub(1).and_then(|i| bytes.get(i).copied());
        let next = bytes.get(m.end());
        let bracketed = matches!(prev, Some(b'[' | b'(' | b'<')) && matches!(next, Some(b']' | b')' | b'>'));
        let score: u8 = if bracketed { 3 } else if upper { 2 } else { 1 };
        if best.map_or(true, |b| score > b.0) {
            best = Some((score, code));
        }
    }
    best.map(|b| b.1)
}

/**
 * 解析单行的级别。
 * 优先在时间戳之后的窗口内识别（覆盖含长 logger / MDC 前缀、级别排在较后的格式）；
 * 未命中再回退到整行识别（覆盖无标准时间戳、或级别排在行尾的格式）。
 */
fn detect_level(line: &str, ts_end: usize) -> u8 {
    if ts_end < line.len() {
        let window = safe_window(line, ts_end, LEVEL_SCAN_WINDOW);
        if let Some(lv) = best_level_in(window) {
            return lv;
        }
    }
    best_level_in(line).unwrap_or(LV_UNKNOWN)
}

/** 解析一段纯数字（定长），任一字符非数字则返回 None */
fn num(b: &[u8]) -> Option<u32> {
    if b.is_empty() {
        return None;
    }
    let mut v: u32 = 0;
    for &c in b {
        if !c.is_ascii_digit() {
            return None;
        }
        v = v * 10 + (c - b'0') as u32;
    }
    Some(v)
}

/**
 * 时间戳文本 → UTC 毫秒。
 * 无年份的 `MM-dd HH:mm:ss` 一律返回 None（与前端 `Date.parse` 行为一致，
 * 避免用当前年份硬套导致的错误时间轴）。
 */
fn ts_text_to_millis(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() < 19 {
        return None;
    }
    if !(b[0].is_ascii_digit() && b[1].is_ascii_digit() && b[2].is_ascii_digit() && b[3].is_ascii_digit())
    {
        // MM-dd HH:mm:ss：缺年份，不解析
        return None;
    }
    let year = num(&b[0..4])? as i32;
    let sep = b[4];
    if sep != b'-' && sep != b'/' {
        return None;
    }

    // 月、日：各 1~2 位，用 sep 分隔（兼容 yyyy-MM-dd 与 yyyy/M/d）
    let mut idx = 5;
    let m_start = idx;
    while idx < b.len() && b[idx].is_ascii_digit() {
        idx += 1;
    }
    let month = num(&b[m_start..idx])?;
    if idx >= b.len() || b[idx] != sep {
        return None;
    }
    idx += 1;
    let d_start = idx;
    while idx < b.len() && b[idx].is_ascii_digit() {
        idx += 1;
    }
    let day = num(&b[d_start..idx])?;

    // 日期与时间之间的分隔符：空格或 T（历史 bug 把这里误当成日期分隔符 sep，导致全部返回 None）
    if idx >= b.len() || (b[idx] != b' ' && b[idx] != b'T' && b[idx] != b't') {
        return None;
    }
    idx += 1;

    // 时:分:秒：各 2 位
    if idx + 2 > b.len() {
        return None;
    }
    let hour = num(&b[idx..idx + 2])?;
    idx += 2;
    if idx >= b.len() || b[idx] != b':' {
        return None;
    }
    idx += 1;
    if idx + 2 > b.len() {
        return None;
    }
    let minute = num(&b[idx..idx + 2])?;
    idx += 2;
    if idx >= b.len() || b[idx] != b':' {
        return None;
    }
    idx += 1;
    if idx + 2 > b.len() {
        return None;
    }
    let second = num(&b[idx..idx + 2])?;
    idx += 2;

    let mut millis: u32 = 0;
    if idx < b.len() && (b[idx] == b'.' || b[idx] == b',') {
        idx += 1;
        let start = idx;
        while idx < b.len() && b[idx].is_ascii_digit() {
            idx += 1;
        }
        let digits = &b[start..idx];
        if digits.is_empty() {
            return None;
        }
        let v = num(digits)?;
        millis = match digits.len() {
            1 => v * 100,
            2 => v * 10,
            3 => v,
            n => v / 10u32.pow(n as u32 - 3), // 超过 3 位（如微秒）截断为毫秒
        };
    }

    // 时区偏移：Z 视为 0；+HH:mm / +HHmm / +HH 支持
    let mut offset_secs: Option<i32> = None;
    if idx < b.len() {
        match b[idx] {
            b'Z' | b'z' => {
                offset_secs = Some(0);
            }
            b'+' | b'-' => {
                let sign = if b[idx] == b'-' { -1 } else { 1 };
                idx += 1;
                if idx + 2 > b.len() {
                    return None;
                }
                let hh = num(&b[idx..idx + 2])? as i32;
                idx += 2;
                let mut mm = 0i32;
                if idx < b.len() && b[idx] == b':' {
                    idx += 1;
                }
                if idx + 2 <= b.len() && b[idx].is_ascii_digit() && b[idx + 1].is_ascii_digit() {
                    mm = num(&b[idx..idx + 2])? as i32;
                }
                offset_secs = Some(sign * (hh * 3600 + mm * 60));
            }
            _ => {}
        }
    }

    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    let naive = date.and_hms_milli_opt(hour, minute, second, millis)?;
    match offset_secs {
        // 带时区：naive 视为该时区本地时间，减去偏移得到 UTC 毫秒
        Some(off) => Some(naive.and_utc().timestamp_millis() - off as i64 * 1000),
        // 不带时区：按本机时区解释（日志文件通常在当前机器产生）
        None => Local
            .from_local_datetime(&naive)
            .single()
            .map(|dt| dt.timestamp_millis()),
    }
}

/** 解析行首时间戳，返回 (UTC 毫秒, 时间戳文本结束的字节偏移) */
fn parse_ts(line: &str) -> Option<(i64, usize)> {
    let m = TS_RE.captures(line)?;
    let g = m.get(1)?;
    let millis = ts_text_to_millis(g.as_str())?;
    Some((millis, g.end()))
}

/// 单行分析结果
struct LineInfo {
    ts: i64,
    level: u8,
    /** 本行是否自带时间戳（用于统计"有多少行可参与时间筛选"） */
    ts_own: bool,
}

/**
 * 解析一行，应用级别/时间戳继承规则。
 *
 * `last_ts` / `last_level`：最近一条普通日志行的时间戳与级别。
 * `stack_level`：当前异常块的首行级别，跨行可变。
 */
/**
 * `need_ts = false` 时**跳过时间戳解析**：`parse_ts` 要依次试多种格式，是单行解析里
 * 最贵的一步，而"只按级别筛选"根本用不到它。谓词下推的第一步就是别做用不到的工作。
 * 此时 `ts_own` 恒为 false（时间戳走继承），调用方不得再据此做时间判定。
 */
fn analyze_line(
    line: &str,
    last_ts: i64,
    last_level: u8,
    stack_level: &mut Option<u8>,
    need_ts: bool,
) -> LineInfo {
    let ts_info = if need_ts { parse_ts(line) } else { None };
    let (ts_own, ts_val) = match &ts_info {
        Some((ms, _)) => (true, *ms),
        None => (false, NO_TS),
    };

    match stack_kind(line) {
        Some(forced) => {
            // 堆栈行：级别取"异常首行自带" → "当前异常块" → "最近普通行"
            let level = forced.unwrap_or_else(|| stack_level.unwrap_or(last_level));
            *stack_level = Some(level);
            LineInfo {
                ts: if ts_own { ts_val } else { last_ts },
                level,
                ts_own,
            }
        }
        None => {
            *stack_level = None;
            // 级别优先在时间戳之后的窗口内识别，未命中回退整行
            let start = ts_info.map(|(_, end)| end).unwrap_or(0);
            let level = detect_level(line, start);
            LineInfo {
                ts: if ts_own { ts_val } else { last_ts },
                level,
                ts_own,
            }
        }
    }
}

// ==================== 索引结构 ====================

#[derive(Serialize, Clone)]
pub struct TimeBucket {
    /** 桶起始时间（含，UTC 毫秒） */
    pub start: i64,
    /** 桶结束时间（不含，UTC 毫秒） */
    pub end: i64,
    pub total: u64,
    pub warn: u64,
    /** ERROR 及以上（含 FATAL） */
    pub error: u64,
    /** 该桶内第一条带时间戳日志的行号（1-based）；空桶为 None */
    pub first_line: Option<u64>,
}

// ==================== L1 块级聚合（剪枝层） ====================

/** 每块聚合多少行 */
const BLOCK_LINES: u64 = 1024;

/**
 * 单个块的聚合统计（约 80 字节/块 → 10GB 约 4MB）。
 *
 * 用途严格限定为**整块排除**：块级统计只能证明"这块里没有符合条件的行"，
 * **不能**证明某行命中——任何"命中"结论都必须回读原文逐行复核（§2.3）。
 *
 * `*_inherit_prev` 三项是**复核入口状态**，不可省：候选块往往不连续（中间块被剪掉），
 * 而级别 / 时间戳 / 堆栈上下文是沿行序继承的。若记不住块首行该从哪个状态接上，
 * 复核时块首若干行会继承到错误的级别或时间戳，症状是"筛选结果莫名少几行"。
 */
#[derive(Clone, Copy)]
struct BlockStat {
    /** 块首行的字节偏移（复核时 seek 的起点） */
    byte_start: u64,
    /** 块首行行号（1-based） */
    line_start: u64,
    line_count: u32,
    /** 块内各级别行数，下标与级别码一致；记的是**继承解析后**的级别 */
    level_counts: [u32; LV_COUNT],
    /** 块内**自带时间戳**行的最小 / 最大值；无自带时间戳时为 NO_TS */
    ts_min: i64,
    ts_max: i64,
    /** 块首行继承的时间戳（来自上一块末尾）；NO_TS 表示无从继承 */
    ts_inherit_prev: i64,
    /** 块内**自带**时间戳的行数（时间轴分桶沿用旧口径：只统计带时间戳的行） */
    ts_own_count: u32,
    /** 块首行继承的"最近级别"（堆栈帧继承用） */
    level_inherit_prev: u8,
    /** 块首行继承的堆栈级别；-1 表示块首不在堆栈中 */
    stack_inherit_prev: i8,
}

impl BlockStat {
    /**
     * 整块排除判定：`true` 表示块内**不可能**有满足条件的行，可整块跳过。
     *
     * 保守优先——无法证明可排除时一律返回 `false`（不排除）。
     * `invert` 模式不做块级剪枝（取反后"排除"的语义不成立），由调用方保证。
     */
    fn can_skip(&self, mask: u16, from: Option<i64>, to: Option<i64>) -> bool {
        // 级别：块内被选中的级别计数全为 0 ⇒ 确实没有这些级别的行
        if mask != 0 {
            let mut any = 0u32;
            for c in 0..LV_COUNT {
                if mask & (1u16 << c) != 0 {
                    any += self.level_counts[c];
                }
            }
            if any == 0 {
                return true;
            }
        }
        // 时间：块内自有 ts 区间，并入"块首继承值"（继承值可能落在区间外，必须并入）
        if from.is_some() || to.is_some() {
            let mut lo = self.ts_min;
            let mut hi = self.ts_max;
            if self.ts_inherit_prev != NO_TS {
                if lo == NO_TS || self.ts_inherit_prev < lo {
                    lo = self.ts_inherit_prev;
                }
                if hi == NO_TS || self.ts_inherit_prev > hi {
                    hi = self.ts_inherit_prev;
                }
            }
            // 块内没有任何可用时间戳 ⇒ 开启时间条件时这些行必然不命中
            if lo == NO_TS || hi == NO_TS {
                return true;
            }
            if let Some(f) = from {
                if hi < f {
                    return true;
                }
            }
            if let Some(t) = to {
                if lo > t {
                    return true;
                }
            }
        }
        false
    }
}

// ==================== L1.5 每行级别位图（纯内存筛选层） ====================

/**
 * 每行级别，4 bit/行（2 行 1 字节）。
 *
 * ## 为什么需要它——压测得出的结论
 * 只按级别筛选时，**块级剪枝对均匀分布的常见级别无效**：300MB 样本里 ERROR 占 7%，
 * 每个 1024 行块都含 ERROR ⇒ 剪枝 0 块 ⇒ 退化成"全文件逐行回读 + 重新解析"，
 * 实测 4.4s/300MB，反而远慢于旧实现（旧实现持有逐行级别数组，纯内存毫秒级）。
 *
 * 折中：级别只要 3 bit 就够（7 种），按 4 bit 存 → **0.5 字节/行**。
 * 对比被删掉的 `ts: Vec<i64>` + `levels: Vec<u8>`（9 字节/行）省 18 倍，
 * 10GB（约 5000 万行）也只占 25MB，仍在 500MB 预算内。
 *
 * ## 适用条件
 * 仅当"**无时间条件 + 无关键词**"时可绕过 IO：时间戳没有等价的紧凑表示
 * （8 字节/行太贵），关键词则必须读行内容。
 */
struct PackedLevels {
    data: Vec<u8>,
    count: u64,
}

impl PackedLevels {
    fn with_capacity(lines: u64) -> PackedLevels {
        PackedLevels { data: Vec::with_capacity((lines / 2 + 1) as usize), count: 0 }
    }
    /** 追加一行的级别（必须按行序调用） */
    fn push(&mut self, level: u8) {
        if self.count % 2 == 0 {
            self.data.push(level & 0x0f);
        } else {
            let last = self.data.len() - 1;
            self.data[last] |= (level & 0x0f) << 4;
        }
        self.count += 1;
    }
    /** 取第 `line_no` 行（1-based）的级别；越界或未建到该行返回 None */
    fn get(&self, line_no: u64) -> Option<u8> {
        if line_no == 0 || line_no > self.count {
            return None;
        }
        let i = (line_no - 1) as usize;
        let b = *self.data.get(i / 2)?;
        Some(if i % 2 == 0 { b & 0x0f } else { b >> 4 })
    }
    fn approx_bytes(&self) -> usize {
        self.data.capacity()
    }
}

// ==================== L2 每行压缩偏移（定位层） ====================

/**
 * 每 128 行一块：块头存该块首行的**绝对偏移**，块内其余行存**相邻行差**
 * （即上一行的字节长度）。
 *
 * 存"相邻差"而非"相对块首"有两个理由：差值更小（行长 ≪ 到块首的距离）因而更省；
 * 且差值本身就是行长，`line_len` 可直接复用。
 *
 * **块内编码按块自适应**：要么 varint（变宽，逐值适应），要么定宽 1/2/4/8 字节
 * （取块内最大差值所需的宽度）。定宽模式下定位是纯下标运算，**无需块内解码循环**。
 *
 * 为什么不是 StreamVByte：它是"每 4 个值一组"的定宽方案，介于两者之间。
 * 实测（见 `docs/design/log-filter-search-unification.md` §7.5）显示它在常见分布上
 * **从未优于** `min(varint, 定宽)`——短行被 varint 支配、均匀长行被定宽支配，
 * 而含超长堆栈行的偏斜块两者都不如 varint。故不引入该依赖，只保留两种极端形态并逐块择优。
 *
 * 注意口径：块内解码必须**累加**相邻差。若存成"相对块首"却累加，偏移会从第 2 行起
 * 逐步放大——症状是跳转落到错误位置，属于极难排查的静默错位。
 */
const OFFSET_BLOCK_LINES: u64 = 128;

/** varint 写入（LEB128，无符号） */
fn write_varint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

/** varint 读取，返回 (值, 下一个位置) */
fn read_varint(buf: &[u8], mut pos: usize) -> Option<(u64, usize)> {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        let b = *buf.get(pos)?;
        pos += 1;
        result |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 {
            return Some((result, pos));
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }
}

/** 块内编码模式：0 = varint 变宽；1 / 2 / 4 / 8 = 定宽字节数 */
const MODE_VARINT: u8 = 0;

/** 差值所需的最小定宽字节数 */
fn width_of(v: u64) -> u8 {
    if v < 1 << 8 {
        1
    } else if v < 1 << 16 {
        2
    } else if v < 1 << 32 {
        4
    } else {
        8
    }
}

/** 定宽写入（小端） */
fn push_fixed(out: &mut Vec<u8>, v: u64, w: u8) {
    for i in 0..w {
        out.push((v >> (8 * i)) as u8);
    }
}

/** 定宽读取（小端） */
fn read_fixed(buf: &[u8], off: usize, w: usize) -> Option<u64> {
    let mut v = 0u64;
    for i in 0..w {
        v |= (*buf.get(off + i)? as u64) << (8 * i);
    }
    Some(v)
}

/** varint 占用字节数（封块择优时比较用） */
fn varint_len(mut v: u64) -> usize {
    let mut n = 1;
    while v >= 0x80 {
        n += 1;
        v >>= 7;
    }
    n
}

/**
 * 每行的**起始字节偏移**，按块压缩存储（定位层）。
 *
 * 内存口径：自适应后实测约 **1.0–2.0 字节/行** —— 行长 <128B 的块 1.0；
 * 128–255B 的块从 varint 的 2.0 降到定宽的 1.0（最常见的日志行区间，**省约 50%**）；
 * 含超长堆栈行的偏斜块仍走 varint，不被块内最大值拖累。
 *
 * 附加能力：**行长 = `offset[line+1] - offset[line]`**——
 * 不必读整行即可判断该行是否超长（跳过超长堆栈行的前提）。
 */
#[derive(Default, Clone)]
struct LineOffsets {
    /** 每块首行的绝对偏移 */
    block_base: Vec<u64>,
    /**
     * 每块在 `deltas` 中的起始下标，以及块内编码模式。
     *
     * 两者**只在封块时成对追加**，长度恒等于"已封块数"；与 `block_base` 的长度差
     * 就是当前未封块——查询未封块时走 `pending`（原始值，无需解码）。
     */
    block_pos: Vec<u32>,
    block_mode: Vec<u8>,
    /** 已封块的编码数据 */
    deltas: Vec<u8>,
    /** 当前未封块的原始相邻差；封块后清空 */
    pending: Vec<u64>,
    /** 已记录的行数（**含** `pending` 中的行） */
    count: u64,
    /**
     * 上一行的起始偏移（构建游标）。
     *
     * 块首行时它等于块首绝对偏移，块内其余行时等于前一行偏移——
     * 两种情形下 `start_offset - prev_offset` 都恰好是"上一行的字节长度"。
     */
    prev_offset: u64,
}

impl LineOffsets {
    /** 追加一行的起始字节偏移（必须按行序调用） */
    fn push(&mut self, start_offset: u64) {
        let idx = self.count;
        if idx % OFFSET_BLOCK_LINES == 0 {
            // 块首行只记绝对偏移，不产生 delta
            self.block_base.push(start_offset);
        } else {
            self.pending.push(start_offset - self.prev_offset);
            if self.pending.len() as u64 + 1 == OFFSET_BLOCK_LINES {
                self.flush_block();
            }
        }
        self.prev_offset = start_offset;
        self.count += 1;
    }

    /**
     * 封块：在 varint 与定宽之间择优写入 `deltas`。
     *
     * 比较口径只算**数据区**：两种模式都要为每块存 1 字节模式标记，是常量开销。
     */
    fn flush_block(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let mut max_v = 0u64;
        let mut varint_total = 0usize;
        for &v in &self.pending {
            if v > max_v {
                max_v = v;
            }
            varint_total += varint_len(v);
        }
        let w = width_of(max_v) as usize;

        let pos = self.deltas.len() as u32;
        let mode = if varint_total <= w * self.pending.len() {
            for &v in &self.pending {
                write_varint(&mut self.deltas, v);
            }
            MODE_VARINT
        } else {
            for &v in &self.pending {
                push_fixed(&mut self.deltas, v, w as u8);
            }
            w as u8
        };
        self.block_pos.push(pos);
        self.block_mode.push(mode);
        self.pending.clear();
    }

    /** 收尾：把末尾不足一块的行也封上（`build_offsets` 结束后必须调用） */
    fn finish(&mut self) {
        self.flush_block();
    }

    /**
     * 取第 `line_no` 行（1-based）的起始字节偏移。
     *
     * 定宽块是**纯下标运算**（无循环累加相邻的 varint）；变宽块才需顺序解码。
     * 两者上界都是 `OFFSET_BLOCK_LINES - 1` 次，与"顺序读到目标行"有数量级差异。
     * 行号越界或未建到该行返回 `None`。
     */
    fn offset_of(&self, line_no: u64) -> Option<u64> {
        if line_no == 0 || line_no > self.count {
            return None;
        }
        let idx = line_no - 1;
        let block = (idx / OFFSET_BLOCK_LINES) as usize;
        let within = (idx % OFFSET_BLOCK_LINES) as usize;
        let base = *self.block_base.get(block)?;
        if within == 0 {
            return Some(base);
        }
        // 未封块（构建中的末块）：直接累加原始值
        if block >= self.block_mode.len() {
            let k = within - 1;
            if k >= self.pending.len() {
                return None;
            }
            let mut acc = 0u64;
            for i in 0..=k {
                acc += self.pending[i];
            }
            return Some(base + acc);
        }
        let mode = self.block_mode[block] as usize;
        let pos = self.block_pos[block] as usize;
        let mut acc = 0u64;
        if mode == 0 {
            let mut p = pos;
            for _ in 0..within {
                let (d, next) = read_varint(&self.deltas, p)?;
                acc += d;
                p = next;
            }
        } else {
            for i in 0..within {
                acc += read_fixed(&self.deltas, pos + i * mode, mode)?;
            }
        }
        Some(base + acc)
    }

    /** 估算占用字节数（缓存按预算淘汰用；取 capacity 而非 len，避免低估 Vec 冗余容量） */
    fn approx_bytes(&self) -> usize {
        self.block_base.capacity() * 8
            + self.block_pos.capacity() * 4
            + self.block_mode.capacity()
            + self.deltas.capacity()
            + self.pending.capacity() * 8
    }

    /**
     * 第 `line_no` 行的字节长度（含行尾换行符）。
     *
     * 行长是 L2 的附加能力——不必读整行就能判断该行是否超长，
     * 是"跳过超长堆栈行"的前提（P6）。末行或未建到该行时返回 `None`
     * （末行长度需文件总大小，调用方可另行取）。
     */
    #[allow(dead_code)]
    fn line_len(&self, line_no: u64) -> Option<u64> {
        let a = self.offset_of(line_no)?;
        let b = self.offset_of(line_no + 1)?;
        Some(b - a)
    }
}

/**
 * 索引快照（**不可变**）。
 *
 * 构建分两阶段推进，每阶段结束整体替换缓存里的 `Arc<IndexSnapshot>`：
 * - 阶段一 `offsets_ready`：L2 全量偏移就绪 ⇒ 可 O(1) 定位、可自由滚动跳转；
 * - 阶段二 `blocks_ready`：L1 块级聚合就绪 ⇒ 可做级别 / 时间筛选。
 *
 * 之所以拆两遍扫描：阶段一只在字节流里找换行（不解码、不跑正则），速度接近磁盘带宽；
 * 阶段二要解码并跑多条正则，慢一个量级。拆开后**用户不必等慢的那遍就能滚动与跳转**
 * ——这正是「打开 10GB 不卡」的关键。代价是多一遍顺序读（第二遍通常命中 OS 页缓存）。
 */
struct IndexSnapshot {
    // ---- L1 剪枝层 ----
    blocks: Vec<BlockStat>,
    // ---- L1.5 纯内存筛选层（每行级别，4bit/行） ----
    levels: PackedLevels,
    // ---- L2 定位层（与阶段二快照共享同一份，避免重复占用） ----
    offsets: Arc<LineOffsets>,
    // ---- 文件级统计 ----
    total_lines: u64,
    indexed_lines: u64,
    truncated: bool,
    level_counts: [u64; LV_COUNT],
    ts_min: Option<i64>,
    ts_max: Option<i64>,
    ts_count: u64,
    buckets: Vec<TimeBucket>,
    // ---- 完成度 ----
    offsets_ready: bool,
    blocks_ready: bool,
    /** 建立索引时的文件指纹，用于判断是否已过期 */
    mtime_ms: u64,
    size: u64,
}

/** 缓存条目：持有当前快照的 `Arc`，读侧克隆后即可无锁访问 */
struct IndexEntry {
    snap: Arc<IndexSnapshot>,
    mtime_ms: u64,
    size: u64,
    /** 估算占用字节数（按预算淘汰的依据，见 `INDEX_CACHE_BUDGET`） */
    bytes: usize,
    /** 最近一次被访问的逻辑序号（LRU 排序用，不依赖 HashMap 的任意迭代顺序） */
    last_used: u64,
}

impl IndexSnapshot {
    /**
     * 估算占用字节数。
     *
     * 取 `capacity` 而非 `len`：Vec 的冗余容量也是实打实占着的内存，
     * 用 `len` 会系统性低估，预算就守不住。
     */
    fn approx_bytes(&self) -> usize {
        self.blocks.capacity() * std::mem::size_of::<BlockStat>()
            + self.buckets.capacity() * std::mem::size_of::<TimeBucket>()
            + self.levels.approx_bytes()
            + self.offsets.approx_bytes()
    }
}

#[derive(Serialize)]
pub struct LogIndexMeta {
    pub total_lines: u64,
    /** 实际参与索引的行数（超过 MAX_INDEX_LINES 时会小于 total_lines） */
    pub indexed_lines: u64,
    pub truncated: bool,
    /** 各级别行数，下标与级别码一致：[UNKNOWN, TRACE, DEBUG, INFO, WARN, ERROR, FATAL] */
    pub levels: Vec<u64>,
    pub ts_min: Option<i64>,
    pub ts_max: Option<i64>,
    /** 自带时间戳的行数（其余行的时间戳是继承来的） */
    pub ts_count: u64,
    pub buckets: Vec<TimeBucket>,
    pub elapsed_ms: u64,
    /** 是否命中缓存（命中时 elapsed_ms 接近 0） */
    pub cached: bool,
    /**
     * L1 块级聚合是否已就绪。
     *
     * `false` 表示阶段二尚未完成——此时只能定位与滚动（L2 已就绪），
     * **不能**做级别 / 时间筛选。前端据此提示"索引构建中"而不是给出空白结果。
     */
    pub blocks_ready: bool,
}

#[derive(Serialize, Clone, Copy)]
pub struct LineSegment {
    /** 段起始行号（1-based，含） */
    pub start: u64,
    /** 段结束行号（1-based，含） */
    pub end: u64,
}

#[derive(Serialize)]
pub struct LogQueryResult {
    /** 命中行段（按行号升序，已合并相邻命中行）。渲染按段取内容 */
    pub segments: Vec<LineSegment>,
    /** 命中行号（1-based，升序）。**导航用**，是段之外的第二视图 */
    pub lines: Vec<u64>,
    /** 最终命中行数真值（不受任何截断影响） */
    pub matched: u64,
    /** 仅筛选条件的命中行数（不含关键词），UI 显示「筛选 M 行 · 匹配 K 处」 */
    pub filter_matched: u64,
    /** 段数是否因超过 max_segments 被截断 */
    pub truncated: bool,
    /** `lines` 是否因超过 max_matches 被截断（`matched` 仍是真值） */
    pub matches_truncated: bool,
    /** 实际回读原文的行数（衡量剪枝收益） */
    pub scanned_lines: u64,
    /** 被块级剪枝整块跳过的块数（衡量剪枝收益） */
    pub pruned_blocks: u64,
    /** 因超出索引上限而未参与本次查询的行数（>0 时 UI 必须提示，见 §4） */
    pub unindexed_lines: u64,
    pub elapsed_ms: u64,
}

/** 全文件搜索默认最多返回多少条命中行号（防止超大文件把数组撑爆） */
const DEFAULT_MAX_MATCHES: u64 = 50_000;

#[derive(Serialize)]
pub struct LogExportResult {
    pub path: String,
    pub lines: u64,
    pub bytes: u64,
    pub elapsed_ms: u64,
}

// ==================== 索引构建 ====================

fn cache_key(path: &str, encoding: &str) -> String {
    format!("{}|{}", path, encoding)
}

/** 文件指纹：(修改时间毫秒, 字节大小) */
fn file_stamp(path: &str) -> Result<(u64, u64), String> {
    let md = std::fs::metadata(path).map_err(|e| format!("获取文件信息失败: {}", e))?;
    let mtime = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    Ok((mtime, md.len()))
}

/**
 * 由**块级聚合**近似生成时间轴分桶。
 *
 * 口径变化（删除逐行 `ts[]` / `levels[]` 的直接后果）：不再逐行归桶，改为整块归到
 * "块内最小自带时间戳"所在的桶，`first_line` 取块首行号，**误差上界 = BLOCK_LINES**。
 * `total` 仍只累计自带时间戳的行（`ts_own_count`），与旧口径一致，时间轴总数不变。
 *
 * 若要恢复逐行精度，需在扫描时顺带累加桶计数（等于在索引里常驻一份桶数据），
 * 属后续可选优化——先接受近似，§7.2 已记录。
 */
fn compute_buckets(blocks: &[BlockStat], ts_min: i64, ts_max: i64) -> Vec<TimeBucket> {
    let span = (ts_max - ts_min).max(1);
    let width = ((span as f64) / BUCKET_COUNT as f64).ceil().max(1.0) as i64;
    let count = ((span / width) + 1).min(BUCKET_COUNT as i64) as usize;

    let mut buckets: Vec<TimeBucket> = (0..count)
        .map(|i| TimeBucket {
            start: ts_min + i as i64 * width,
            end: ts_min + (i + 1) as i64 * width,
            total: 0,
            warn: 0,
            error: 0,
            first_line: None,
        })
        .collect();

    for b in blocks {
        if b.ts_min == NO_TS {
            continue;
        }
        let idx = ((b.ts_min - ts_min) / width).max(0) as usize;
        if idx >= buckets.len() {
            continue;
        }
        let t = &mut buckets[idx];
        t.total += b.ts_own_count as u64;
        t.warn += b.level_counts[LV_WARN as usize] as u64;
        t.error +=
            (b.level_counts[LV_ERROR as usize] + b.level_counts[LV_FATAL as usize]) as u64;
        if t.first_line.is_none() {
            t.first_line = Some(b.line_start);
        }
    }
    buckets
}

/**
 * 阶段一：只在字节流里找换行，建立 L2 全量行偏移（同步阻塞）。
 *
 * **不解码、不跑正则**，速度接近磁盘带宽；阶段二要解码并跑多条正则，慢一个量级。
 * 拆成两遍是为了让"可定位"尽早可用——用户不必等慢的那遍就能滚动与跳转。
 */
fn build_offsets(path: &str) -> Result<(LineOffsets, u64), String> {
    let file = File::open(path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut buf: Vec<u8> = Vec::with_capacity(64 * 1024);
    let mut offsets = LineOffsets::default();
    let mut pos: u64 = 0;
    let mut total: u64 = 0;
    loop {
        buf.clear();
        let n = reader
            .read_until(b'\n', &mut buf)
            .map_err(|e| format!("读取行失败: {}", e))?;
        if n == 0 {
            break;
        }
        offsets.push(pos);
        pos += n as u64;
        total += 1;
    }
    // 末尾不足一块的行必须封块，否则它们只存在于 pending、无法走定宽快路径
    offsets.finish();
    Ok((offsets, total))
}

/** 阶段二的产出：L1 块级聚合 + 文件级统计 */
struct BlockBuild {
    blocks: Vec<BlockStat>,
    /** 每行级别（4bit/行），供"无时间条件 + 无关键词"的纯内存筛选使用 */
    levels: PackedLevels,
    level_counts: [u64; LV_COUNT],
    ts_min: Option<i64>,
    ts_max: Option<i64>,
    ts_count: u64,
    indexed_lines: u64,
}

/**
 * 阶段二：解码 + 正则解析，建立 L1 块级聚合（同步阻塞，慢）。
 *
 * `offsets` 仅用于取每块的 `byte_start`——**偏移的唯一事实源是阶段一**，
 * 杜绝两遍扫描各算一套偏移导致 seek 错位。
 */
fn build_blocks(
    path: &str,
    encoding: &str,
    offsets: &Arc<LineOffsets>,
    total_lines: u64,
) -> Result<BlockBuild, String> {
    let enc = resolve_encoding(encoding);
    let file = File::open(path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut reader = BufReader::with_capacity(256 * 1024, file);
    let mut buf: Vec<u8> = Vec::with_capacity(1024);

    let cap = (total_lines.min(MAX_INDEX_LINES) / BLOCK_LINES + 1) as usize;
    let mut blocks: Vec<BlockStat> = Vec::with_capacity(cap.min(1 << 20));
    let mut level_counts = [0u64; LV_COUNT];
    let mut ts_min: i64 = i64::MAX;
    let mut ts_max: i64 = i64::MIN;
    let mut ts_count: u64 = 0;
    let mut indexed_lines: u64 = 0;
    let mut levels = PackedLevels::with_capacity(total_lines.min(MAX_INDEX_LINES));

    let mut last_ts: i64 = NO_TS;
    let mut last_level: u8 = LV_UNKNOWN;
    let mut stack_level: Option<u8> = None;
    let mut line_start: u64 = 0;
    let mut cur: Option<BlockStat> = None;

    while let Some((line, consumed)) = read_line_lossy(&mut reader, &mut buf, Some(enc))
        .map_err(|e| format!("读取行失败: {}", e))?
    {
        let line_no = indexed_lines + 1;
        if line_no > MAX_INDEX_LINES {
            break;
        }
        if cur.is_none() {
            cur = Some(BlockStat {
                byte_start: offsets.offset_of(line_no).unwrap_or(line_start),
                line_start: line_no,
                line_count: 0,
                level_counts: [0u32; LV_COUNT],
                ts_min: NO_TS,
                ts_max: NO_TS,
                ts_inherit_prev: last_ts,
                ts_own_count: 0,
                level_inherit_prev: last_level,
                stack_inherit_prev: stack_level.map(|v| v as i8).unwrap_or(-1),
            });
        }
        let b = cur.as_mut().unwrap();

        let info = analyze_line(&line, last_ts, last_level, &mut stack_level, true);
        if info.ts_own {
            last_ts = info.ts;
            ts_count += 1;
            b.ts_own_count += 1;
            if info.ts < ts_min {
                ts_min = info.ts;
            }
            if info.ts > ts_max {
                ts_max = info.ts;
            }
            if b.ts_min == NO_TS || info.ts < b.ts_min {
                b.ts_min = info.ts;
            }
            if b.ts_max == NO_TS || info.ts > b.ts_max {
                b.ts_max = info.ts;
            }
        }
        if !matches!(stack_kind(&line), Some(_)) {
            // 普通行：更新"最近级别"，供后续堆栈帧兜底继承
            if info.level != LV_UNKNOWN {
                last_level = info.level;
            }
        }
        level_counts[info.level as usize] += 1;
        levels.push(info.level);
        b.level_counts[info.level as usize] += 1;
        b.line_count += 1;
        indexed_lines += 1;
        line_start += consumed as u64;

        if b.line_count as u64 >= BLOCK_LINES {
            blocks.push(cur.take().unwrap());
        }
    }
    if let Some(b) = cur {
        if b.line_count > 0 {
            blocks.push(b);
        }
    }

    let (ts_min_opt, ts_max_opt) = if ts_count > 0 {
        (Some(ts_min), Some(ts_max))
    } else {
        (None, None)
    };
    Ok(BlockBuild {
        blocks,
        levels,
        level_counts,
        ts_min: ts_min_opt,
        ts_max: ts_max_opt,
        ts_count,
        indexed_lines,
    })
}

/**
 * 按字节预算淘汰最久未用者，返回被淘汰的 key（供调用方回收构建锁）。
 *
 * 抽成纯函数是为了可测：不依赖全局 `INDEX_CACHE` 与预算常量，单测可直接喂一个
 * 构造好的 `HashMap`——否则要凑出 512MB 的真实内存才能验证淘汰逻辑。
 *
 * 三条硬约束：
 * - `keep` 永不被淘汰（它可能正被读侧持有，也不该被自己挤掉）；
 * - 不用 `clear()`——那会连带清掉正在构建的条目（§7.8 记录的现状缺陷）；
 * - 单个条目就超预算时（如 10GB 文件），淘汰完其它条目后仍超限是可接受的：
 *   至少保留当前正在用的这份。
 */
fn evict_to_budget(
    cache: &mut HashMap<String, IndexEntry>,
    keep: &str,
    budget: usize,
) -> Vec<String> {
    let mut total: usize = cache.values().map(|e| e.bytes).sum();
    if total <= budget {
        return Vec::new();
    }
    let mut candidates: Vec<(u64, String)> = cache
        .iter()
        .filter(|(k, _)| k.as_str() != keep)
        .map(|(k, e)| (e.last_used, k.clone()))
        .collect();
    candidates.sort_unstable_by_key(|(t, _)| *t);

    let mut evicted = Vec::new();
    for (_, k) in candidates {
        if total <= budget {
            break;
        }
        if let Some(e) = cache.remove(&k) {
            total -= e.bytes;
            evicted.push(k);
        }
    }
    evicted
}

/** 取缓存中满足条件的快照；文件指纹不匹配一律视为失效。命中即刷新 LRU 序号 */
fn cached_snapshot(
    key: &str,
    stamp: (u64, u64),
    ready: fn(&IndexSnapshot) -> bool,
) -> Option<Arc<IndexSnapshot>> {
    let mut cache = INDEX_CACHE.lock();
    let e = cache.get_mut(key)?;
    if e.mtime_ms != stamp.0 || e.size != stamp.1 || !ready(&e.snap) {
        return None;
    }
    e.last_used = next_cache_clock();
    Some(e.snap.clone())
}

/**
 * 发布快照（构建线程调用）。
 *
 * 超出字节预算时按 **LRU 淘汰其它条目**：
 * - 绝不淘汰刚发布的 `key`——它可能正被读侧持有，也不该被自己挤掉；
 * - 绝不 `clear()`——那会连带清掉正在构建的条目（§7.8 记录的现状缺陷）；
 * - 单个条目就超预算时（如 10GB 文件），淘汰完其它条目后仍超限是可接受的：
 *   至少保留当前正在用的这份，而不是什么都不留。
 */
fn publish(key: &str, snap: Arc<IndexSnapshot>) {
    let bytes = snap.approx_bytes();
    let evicted: Vec<String>;
    {
        let mut cache = INDEX_CACHE.lock();
        cache.insert(
            key.to_string(),
            IndexEntry {
                mtime_ms: snap.mtime_ms,
                size: snap.size,
                bytes,
                last_used: next_cache_clock(),
                snap,
            },
        );
        evicted = evict_to_budget(&mut cache, key, INDEX_CACHE_BUDGET);
    }


    // 顺带回收构建锁；仅在无人持有时移除，避免打断正在进行的构建
    if !evicted.is_empty() {
        let mut locks = BUILD_LOCKS.lock();
        for k in evicted {
            if let Some(l) = locks.get(&k) {
                if Arc::strong_count(l) == 1 {
                    locks.remove(&k);
                }
            }
        }
    }
}

/** 每个 key 一把异步锁：并发请求串行化，避免同一文件被重复扫描 */
fn build_lock(key: &str) -> Arc<tokio::sync::Mutex<()>> {
    let mut locks = BUILD_LOCKS.lock();
    locks
        .entry(key.to_string())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

/** 阶段一完成后发布的快照（L2 就绪、L1 未就绪） */
fn offset_only_snapshot(offsets: LineOffsets, total_lines: u64, stamp: (u64, u64)) -> Arc<IndexSnapshot> {
    Arc::new(IndexSnapshot {
        blocks: Vec::new(),
        levels: PackedLevels::with_capacity(0),
        offsets: Arc::new(offsets),
        total_lines,
        indexed_lines: 0,
        truncated: total_lines > MAX_INDEX_LINES,
        level_counts: [0; LV_COUNT],
        ts_min: None,
        ts_max: None,
        ts_count: 0,
        buckets: Vec::new(),
        offsets_ready: true,
        blocks_ready: false,
        mtime_ms: stamp.0,
        size: stamp.1,
    })
}

/** 阶段一：构建并发布 offset-only 快照（**调用方须已持有该 key 的构建锁**） */
async fn build_and_publish_offsets(
    path: &str,
    key: &str,
    stamp: (u64, u64),
) -> Result<Arc<IndexSnapshot>, String> {
    let p = path.to_string();
    let (offsets, total_lines) = tokio::task::spawn_blocking(move || build_offsets(&p))
        .await
        .map_err(|e| format!("索引任务失败: {}", e))??;
    let snap = offset_only_snapshot(offsets, total_lines, stamp);
    publish(key, snap.clone());
    Ok(snap)
}

/**
 * 确保 L2 行偏移就绪（阶段一）。完成后即具备 O(1) 定位能力，**不必等待阶段二**。
 *
 * 渐进可用的公开入口：只读场景（滚动 / 跳转）调用它即可，不必承担阶段二的代价。
 * 当前 `ensure_blocks` 走的是同一段逻辑（见 `build_and_publish_offsets`）。
 */
#[allow(dead_code)]
async fn ensure_offsets(path: &str, encoding: &str) -> Result<Arc<IndexSnapshot>, String> {
    let key = cache_key(path, encoding);
    let stamp = file_stamp(path)?;
    if let Some(s) = cached_snapshot(&key, stamp, |s| s.offsets_ready) {
        return Ok(s);
    }
    let lock = build_lock(&key);
    let _g = lock.lock().await;
    if let Some(s) = cached_snapshot(&key, stamp, |s| s.offsets_ready) {
        return Ok(s);
    }
    build_and_publish_offsets(path, &key, stamp).await
}

/**
 * 确保 L1 块级聚合就绪（阶段二，慢）。
 *
 * 依赖阶段一的偏移；阶段一未就绪时**就地补做**而非递归调用 `ensure_offsets`——
 * 后者会再次请求同一把锁，`tokio::sync::Mutex` 不可重入，会直接死锁。
 */
async fn ensure_blocks(path: &str, encoding: &str) -> Result<Arc<IndexSnapshot>, String> {
    let key = cache_key(path, encoding);
    let stamp = file_stamp(path)?;
    if let Some(s) = cached_snapshot(&key, stamp, |s| s.blocks_ready) {
        return Ok(s);
    }
    let lock = build_lock(&key);
    let _g = lock.lock().await;
    if let Some(s) = cached_snapshot(&key, stamp, |s| s.blocks_ready) {
        return Ok(s);
    }

    let base = match cached_snapshot(&key, stamp, |s| s.offsets_ready) {
        Some(s) => s,
        None => build_and_publish_offsets(path, &key, stamp).await?,
    };

    let p = path.to_string();
    let enc = encoding.to_string();
    let offsets = base.offsets.clone();
    let total_lines = base.total_lines;
    let built = tokio::task::spawn_blocking(move || build_blocks(&p, &enc, &offsets, total_lines))
        .await
        .map_err(|e| format!("索引任务失败: {}", e))??;

    // 指纹复核：构建期间文件被改写则本次结果作废，不发布（交给下次重建）
    if file_stamp(path)? != stamp {
        return Err("文件在索引构建期间被修改，请重试".to_string());
    }

    let buckets = match (built.ts_min, built.ts_max) {
        (Some(a), Some(b)) => compute_buckets(&built.blocks, a, b),
        _ => Vec::new(),
    };
    let snap = Arc::new(IndexSnapshot {
        blocks: built.blocks,
        levels: built.levels,
        offsets: base.offsets.clone(),
        total_lines: base.total_lines,
        indexed_lines: built.indexed_lines,
        truncated: base.total_lines > MAX_INDEX_LINES,
        level_counts: built.level_counts,
        ts_min: built.ts_min,
        ts_max: built.ts_max,
        ts_count: built.ts_count,
        buckets,
        offsets_ready: true,
        blocks_ready: true,
        mtime_ms: stamp.0,
        size: stamp.1,
    });
    publish(&key, snap.clone());
    Ok(snap)
}

fn meta_from_index(idx: &IndexSnapshot, elapsed_ms: u64, cached: bool) -> LogIndexMeta {
    LogIndexMeta {
        total_lines: idx.total_lines,
        indexed_lines: idx.indexed_lines,
        truncated: idx.truncated,
        levels: idx.level_counts.to_vec(),
        ts_min: idx.ts_min,
        ts_max: idx.ts_max,
        ts_count: idx.ts_count,
        buckets: idx.buckets.clone(),
        elapsed_ms,
        cached,
        blocks_ready: idx.blocks_ready,
    }
}

/**
 * 查询指定行的起始字节偏移（供 `read_file_lines` 做 O(1) 定位）。
 *
 * **不触发构建**——只查已发布的快照。无索引、指纹已过期、或该行尚未建到时返回 `None`，
 * 调用方**必须**保留顺序扫描的回退路径：这既是无索引文件的正确性问题，
 * 也是索引构建期间的降级路径（§2.4）。
 */
pub fn lookup_line_offset(path: &str, encoding: &str, line_no: u64) -> Option<u64> {
    let key = cache_key(path, encoding);
    let snap = {
        let mut cache = INDEX_CACHE.lock();
        let e = cache.get_mut(&key)?;
        let snap = e.snap.clone();
        if file_stamp(path).ok()? != (snap.mtime_ms, snap.size) {
            return None;
        }
        // 定位是真实使用：必须刷新 LRU，否则"只读不发布"的索引会被误判为冷数据淘汰
        e.last_used = next_cache_clock();
        Some(snap)
    }?;
    snap.offsets.offset_of(line_no)
}

// ==================== 对外业务函数 ====================

/**
 * 建立（或复用）日志索引，返回文件级统计信息。
 *
 * 前端应在打开日志后后台调用一次：结果里的级别计数、时间范围与时间桶
 * 直接驱动筛选栏与时间轴视图，无需前端全量加载文件。
 */
pub async fn log_index_build(
    file_path: String,
    encoding: Option<String>,
) -> Result<ResData<LogIndexMeta>, String> {
    if !Path::new(&file_path).exists() {
        return Err("文件不存在".to_string());
    }
    let enc = encoding.unwrap_or_else(|| "UTF-8".to_string());
    let start = std::time::Instant::now();

    let key = cache_key(&file_path, &enc);
    let stamp = file_stamp(&file_path)?;
    // 只有 L1 就绪才算命中缓存——否则本次仍要等阶段二，不能报 cached
    let cached = {
        let cache = INDEX_CACHE.lock();
        matches!(cache.get(&key), Some(e) if e.mtime_ms == stamp.0 && e.size == stamp.1 && e.snap.blocks_ready)
    };

    let idx = ensure_blocks(&file_path, &enc).await?;

    let elapsed = start.elapsed().as_millis() as u64;
    Ok(ResData {
        code: 200,
        msg: if cached {
            "索引已就绪（命中缓存）".to_string()
        } else {
            format!("索引完成，共 {} 行", idx.total_lines)
        },
        data: meta_from_index(&idx, elapsed, cached),
    })
}

/** 前端级别名数组 → 位掩码；空数组表示"不限级别" */
fn levels_to_mask(levels: &[String]) -> u16 {
    let mut mask: u16 = 0;
    for s in levels {
        if let Some(code) = level_from_word(&s.to_uppercase()) {
            mask |= 1u16 << code;
        }
    }
    mask
}

#[inline]
fn level_match(level: u8, mask: u16) -> bool {
    mask == 0 || (mask & (1u16 << level)) != 0
}

#[inline]
fn ts_match(ts: i64, from: Option<i64>, to: Option<i64>) -> bool {
    if ts == NO_TS {
        // 无时间戳且无从继承的行：只要开启了时间条件就排除
        return from.is_none() && to.is_none();
    }
    if let Some(f) = from {
        if ts < f {
            return false;
        }
    }
    if let Some(t) = to {
        if ts > t {
            return false;
        }
    }
    true
}

/**
 * 合并各并行分片产出的段（分片内已各自升序合并）。
 *
 * `matched` 的**真值**由各分片直接计数累加，不依赖这里的段合并结果——
 * 段可能因 `max_seg` 截断，但命中总数必须始终是真值。
 */
fn merge_segments(parts: &[PartOut], gap: u64, max_seg: usize) -> (Vec<LineSegment>, bool) {
    let mut out: Vec<LineSegment> = Vec::new();
    let mut truncated = false;
    for p in parts {
        for s in &p.segs {
            match out.last_mut() {
                Some(last) if s.start <= last.end + gap + 1 => {
                    if s.end > last.end {
                        last.end = s.end;
                    }
                }
                _ => {
                    if out.len() >= max_seg {
                        // 一旦超限就停止合并：继续延长末段会让段范围失真
                        truncated = true;
                        return (out, truncated);
                    }
                    out.push(*s);
                }
            }
        }
    }
    (out, truncated)
}

// ==================== 关键词匹配器（P6） ====================

/**
 * 关键词匹配器。
 *
 * ## 为什么不做"手写 SIMD"
 * Rust `regex` crate **已内建** memchr → Teddy → Aho-Corasick 的 SIMD 链路，且已是依赖。
 * 需要补的只有**单文件内的并行**（ripgrep 的并行是跨文件级的，单文件内不切分）。
 *
 * ## 原始字节快路径
 * 文本模式且**大小写敏感**时，把关键词按目标编码编码一次，之后直接在行字节上做
 * `memmem`（SIMD）匹配——**不必逐行解码**。GBK 等非 UTF-8 文件收益最大。
 * 忽略大小写时不能用这条路：ASCII 折叠在多字节编码下会漏匹配，故退回归一化后的
 * 解码比较（`to_lowercase()` 口径与前端 `indexOf` 一致）。
 *
 * ## 正则双引擎降级（借鉴 klogg 的 Hyperscan → Qt 正则）
 * `regex` 不支持前瞻 / 后顾 / 反向引用。编译失败时自动换 `fancy-regex`（纯 Rust，
 * 支持这些语法），保证"用户写的正则总能跑出来"；**不退回前端 JS 全量扫**——
 * 文件一大就卡。两引擎都不支持时才返回错误。
 */
enum Matcher {
    None,
    Text {
        needle: String,
        needle_lower: String,
        /** 按目标编码编码后的字节，供原始字节快路径使用 */
        needle_bytes: Vec<u8>,
        ignore_case: bool,
    },
    Regex(Box<Regex>),
    #[allow(dead_code)]
    Fancy(Box<fancy_regex::Regex>),
}

impl Matcher {
    fn build(keyword: &str, mode: &str, ignore_case: bool, enc: &'static encoding_rs::Encoding) -> Result<Matcher, String> {
        if keyword.is_empty() {
            return Ok(Matcher::None);
        }
        if mode.eq_ignore_ascii_case("regex") {
            match RegexBuilder::new(keyword).case_insensitive(ignore_case).build() {
                Ok(re) => return Ok(Matcher::Regex(Box::new(re))),
                Err(e) => {
                    // 双引擎降级：regex 不支持的语法换 fancy-regex 再试
                    match fancy_regex::Regex::new(keyword) {
                        Ok(fr) => return Ok(Matcher::Fancy(Box::new(fr))),
                        Err(_) => {
                            return Err(format!(
                                "正则表达式不被支持（已尝试备用引擎）: {}",
                                e
                            ))
                        }
                    }
                }
            }
        }
        let (bytes, _, _) = enc.encode(keyword);
        Ok(Matcher::Text {
            needle: keyword.to_string(),
            needle_lower: keyword.to_lowercase(),
            needle_bytes: bytes.into_owned(),
            ignore_case,
        })
    }

    /**
     * 可在字节上直接做 `memmem` 匹配时返回关键词字节（文本模式 + 大小写敏感）。
     *
     * - `needle_utf8`：解码后行（`line.as_bytes()`）。UTF-8 自同步，字节级子串与 `str::contains` 等价。
     * - `needle_enc` ：原始字节行。关键词已按目标编码编码一次，故非 UTF-8 文件也能免解码匹配。
     */
    fn needle_utf8(&self) -> Option<&[u8]> {
        match self {
            Matcher::Text { needle, ignore_case, .. } if !*ignore_case => Some(needle.as_bytes()),
            _ => None,
        }
    }

    fn needle_enc(&self) -> Option<&[u8]> {
        match self {
            Matcher::Text { needle_bytes, ignore_case, .. } if !*ignore_case => {
                Some(needle_bytes.as_slice())
            }
            _ => None,
        }
    }

    fn hit_str(&self, line: &str) -> bool {
        match self {
            Matcher::None => true,
            Matcher::Text { needle, needle_lower, ignore_case, .. } => {
                if *ignore_case {
                    line.to_lowercase().contains(needle_lower.as_str())
                } else {
                    line.contains(needle.as_str())
                }
            }
            Matcher::Regex(re) => re.is_match(line),
            Matcher::Fancy(re) => {
                re.is_match(line).unwrap_or_else(|_| line.is_empty())
            }
        }
    }
}

/** 单个并行分片的产出 */
struct PartOut {
    /** 分片序号，用于合并时恢复行序 */
    order: usize,
    /** 分片内已按行号升序合并的命中段 */
    segs: Vec<LineSegment>,
    /** 命中行号（仅在有关键词时收集；无关键词时为空） */
    lines: Vec<u64>,
    matched: u64,
    filter_matched: u64,
    scanned: u64,
}

/** 分片内累积器（产出一个 `PartOut`） */
struct PartAcc {
    order: usize,
    segs: Vec<LineSegment>,
    lines: Vec<u64>,
    matched: u64,
    filter_matched: u64,
    scanned: u64,
    cur_start: Option<u64>,
    cur_end: u64,
    gap: u64,
    max_matches: u64,
    collect_lines: bool,
}

impl PartAcc {
    fn hit(&mut self, line_no: u64) {
        self.matched += 1;
        if self.collect_lines && (self.lines.len() as u64) < self.max_matches {
            self.lines.push(line_no);
        }
        match self.cur_start {
            None => {
                self.cur_start = Some(line_no);
                self.cur_end = line_no;
            }
            Some(_) if line_no <= self.cur_end + self.gap + 1 => {
                self.cur_end = line_no;
            }
            Some(s) => {
                self.segs.push(LineSegment { start: s, end: self.cur_end });
                self.cur_start = Some(line_no);
                self.cur_end = line_no;
            }
        }
    }

    fn finish(mut self) -> PartOut {
        if let Some(s) = self.cur_start {
            self.segs.push(LineSegment { start: s, end: self.cur_end });
        }
        PartOut {
            order: self.order,
            segs: self.segs,
            lines: self.lines,
            matched: self.matched,
            filter_matched: self.filter_matched,
            scanned: self.scanned,
        }
    }
}

/** 筛选条件（级别 + 时间）；`None` 表示无筛选 */
struct FilterSpec {
    mask: u16,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    invert: bool,
}

/** 一个连续的字节区间 + 其起始行号（用于无筛选的全文件扫描分片） */
struct RangeItem {
    byte_start: u64,
    byte_end: u64,
    line_start: u64,
    line_end: u64,
}

/** 把 [1, total] 行区间切成 `n` 段（供并行扫 `PackedLevels` 用） */
fn split_lines(total: u64, n: usize) -> Vec<(u64, u64)> {
    if total == 0 {
        return Vec::new();
    }
    let n = n.max(1).min(total as usize);
    let base = total / n as u64;
    let rem = total % n as u64;
    let mut out = Vec::with_capacity(n);
    let mut start = 1u64;
    for k in 0..n as u64 {
        let take = base + if k < rem { 1 } else { 0 };
        let end = start + take - 1;
        out.push((start, end));
        start = end + 1;
    }
    out
}

/**
 * 纯内存级别筛选（无时间条件 + 无关键词）：直接扫 `PackedLevels`，**完全不读文件**。
 *
 * 块级剪枝在这类查询上收益为零——常见级别（如 ERROR 占 7%）几乎每块都有，
 * 于是退化成"全文件回读 + 重新解析"，实测反而比旧实现慢。
 * 级别在建索引时已经解析过一次，把它以 4bit/行 记下来再扫，是这类查询的正解。
 */
fn scan_packed_levels(
    lv: &PackedLevels,
    f: &FilterSpec,
    gap: u64,
    max_matches: u64,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<PartOut>, String> {
    let groups = split_lines(lv.count, worker_count() * 4);
    let results: Vec<Result<PartOut, String>> = groups
        .par_iter()
        .enumerate()
        .map(|(order, &(start, end))| {
            if let Some(c) = cancel {
                if c.load(Ordering::Relaxed) {
                    return Err("已取消".to_string());
                }
            }
            let mut acc = PartAcc {
                order,
                segs: Vec::new(),
                lines: Vec::new(),
                matched: 0,
                filter_matched: 0,
                scanned: 0,
                cur_start: None,
                cur_end: 0,
                gap,
                max_matches,
                collect_lines: false,
            };
            for line_no in start..=end {
                let level = lv.get(line_no).unwrap_or(LV_UNKNOWN);
                let hit = level_match(level, f.mask);
                if f.invert != hit {
                    acc.filter_matched += 1;
                    acc.hit(line_no);
                }
            }
            Ok(acc.finish())
        })
        .collect();
    let mut outs = Vec::with_capacity(results.len());
    for r in results {
        outs.push(r?);
    }
    Ok(outs)
}

fn worker_count() -> usize {
    rayon::current_num_threads().max(1)
}

/** 把候选块下标切成 `n` 组（**连续**切分以保文件局部性，降低 seek） */
fn split_groups(cand: &[usize], n: usize) -> Vec<Vec<usize>> {
    if cand.is_empty() {
        return Vec::new();
    }
    let n = n.max(1).min(cand.len());
    let base = cand.len() / n;
    let rem = cand.len() % n;
    let mut out: Vec<Vec<usize>> = Vec::with_capacity(n);
    let mut i = 0usize;
    for k in 0..n {
        let take = base + if k < rem { 1 } else { 0 };
        out.push(cand[i..i + take].to_vec());
        i += take;
    }
    out
}

/**
 * 把"全文件"按 L1 块边界切成若干字节区间。
 *
 * 块边界同时给出字节偏移与行号，因此**每个分片都能独立知道自己从第几行开始**——
 * 这是无筛选全文件搜索能并行的前提。最后一个分片延伸到 EOF，
 * 因此索引截断（`MAX_INDEX_LINES`）之外的尾部行也一并覆盖（§2.6 组合 3）。
 */
fn split_ranges(blocks: &[BlockStat], total_lines: u64, file_len: u64) -> Vec<RangeItem> {
    if blocks.is_empty() {
        return vec![RangeItem { byte_start: 0, byte_end: file_len, line_start: 1, line_end: total_lines }];
    }
    let n = worker_count().min(blocks.len()) * 4;
    let n = n.max(1).min(blocks.len());
    let base = blocks.len() / n;
    let rem = blocks.len() % n;
    let mut out: Vec<RangeItem> = Vec::with_capacity(n);
    let mut i = 0usize;
    for k in 0..n {
        let take = base + if k < rem { 1 } else { 0 };
        let first = &blocks[i];
        let (byte_end, line_end) = if i + take >= blocks.len() {
            (file_len, total_lines)
        } else {
            (blocks[i + take].byte_start, blocks[i + take].line_start - 1)
        };
        out.push(RangeItem {
            byte_start: first.byte_start,
            byte_end,
            line_start: first.line_start,
            line_end,
        });
        i += take;
    }
    out
}

/**
 * 候选块复核（同步阻塞；调用方负责放到阻塞线程池）。
 *
 * ## 为什么可以逐块并行
 * 每块的 `BlockStat` 都记了**复核入口状态**（`ts_inherit_prev` / `level_inherit_prev` /
 * `stack_inherit_prev`），块与块之间没有顺序依赖。旧实现把连续候选块合并成一次顺序读
 * 是 IO 优化，但那样就无法并行。改为逐块 seek 后，每块 1024 行（约 200KB）的顺序读
 * 远大于一次 seek 的开销；相邻块则直接续读、不 seek（`pos` 游标判定）。
 */
fn run_block_group(
    path: &str,
    enc: &'static Encoding,
    blocks: &[BlockStat],
    idxs: &[usize],
    f: &FilterSpec,
    m: &Matcher,
    gap: u64,
    max_matches: u64,
    cancel: Option<&AtomicBool>,
    // 无时间条件时传入：直接查位图拿级别，省掉每行的级别/时间戳解析
    packed: Option<&PackedLevels>,
    order: usize,
) -> Result<PartOut, String> {
    let file = File::open(path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut reader = BufReader::with_capacity(256 * 1024, file);
    let mut buf: Vec<u8> = Vec::with_capacity(1024);
    let collect = !matches!(m, Matcher::None);
    let fast = m.needle_utf8();
    // 只按级别筛选时不必解析时间戳（§2.8：谓词下推——先做便宜的判定）
    let need_ts = f.ts_from.is_some() || f.ts_to.is_some();
    debug_assert!(
        need_ts || packed.is_some(),
        "无时间条件时必须能用位图旁路逐行解析"
    );
    let mut acc = PartAcc {
        order,
        segs: Vec::new(),
        lines: Vec::new(),
        matched: 0,
        filter_matched: 0,
        scanned: 0,
        cur_start: None,
        cur_end: 0,
        gap,
        max_matches,
        collect_lines: collect,
    };
    let mut pos: u64 = u64::MAX;

    for &bi in idxs {
        if let Some(c) = cancel {
            if c.load(Ordering::Relaxed) {
                return Err("已取消".to_string());
            }
        }
        let b = &blocks[bi];
        if pos != b.byte_start {
            reader
                .seek(SeekFrom::Start(b.byte_start))
                .map_err(|e| format!("定位失败: {}", e))?;
        }
        pos = b.byte_start;

        let mut last_ts = b.ts_inherit_prev;
        let mut last_level = b.level_inherit_prev;
        let mut stack_level = if b.stack_inherit_prev < 0 {
            None
        } else {
            Some(b.stack_inherit_prev as u8)
        };

        let mut line_no = b.line_start;
        let end_line = b.line_start + b.line_count as u64 - 1;
        while line_no <= end_line {
            let (line, consumed) = match read_line_lossy(&mut reader, &mut buf, Some(enc))
                .map_err(|e| format!("读取行失败: {}", e))?
            {
                Some(v) => v,
                None => break,
            };
            pos += consumed as u64;
            acc.scanned += 1;

            // 位图可用时直接取级别：建索引时已解析过一次且已含继承结果，不必再算一遍
            let (level, ts_ok) = match packed {
                Some(lv) => (lv.get(line_no).unwrap_or(LV_UNKNOWN), true),
                None => {
                    let info = analyze_line(&line, last_ts, last_level, &mut stack_level, true);
                    if info.ts_own {
                        last_ts = info.ts;
                    }
                    if !matches!(stack_kind(&line), Some(_)) && info.level != LV_UNKNOWN {
                        last_level = info.level;
                    }
                    (info.level, ts_match(info.ts, f.ts_from, f.ts_to))
                }
            };
            let hit = level_match(level, f.mask) && ts_ok;
            let hit = if f.invert { !hit } else { hit };
            if hit {
                acc.filter_matched += 1;
                let khit = match fast {
                    Some(nd) => memchr::memmem::find(line.as_bytes(), nd).is_some(),
                    None => m.hit_str(&line),
                };
                if khit {
                    acc.hit(line_no);
                }
            }
            line_no += 1;
        }
    }
    Ok(acc.finish())
}

/**
 * 无筛选时按字节区间扫关键词（同步阻塞）。
 *
 * 文本模式且大小写敏感时**完全不解码**：直接 `read_until(b'\n')` 取原始行字节，
 * 用按目标编码编码好的关键词做 `memmem`（SIMD）。GBK 文件收益最大（§2.8）。
 */
fn run_range(
    path: &str,
    enc: &'static Encoding,
    it: &RangeItem,
    m: &Matcher,
    gap: u64,
    max_matches: u64,
    cancel: Option<&AtomicBool>,
    order: usize,
) -> Result<PartOut, String> {
    let file = File::open(path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut reader = BufReader::with_capacity(256 * 1024, file);
    let mut buf: Vec<u8> = Vec::with_capacity(64 * 1024);
    let collect = !matches!(m, Matcher::None);
    let fast = m.needle_enc();
    let mut acc = PartAcc {
        order,
        segs: Vec::new(),
        lines: Vec::new(),
        matched: 0,
        filter_matched: 0,
        scanned: 0,
        cur_start: None,
        cur_end: 0,
        gap,
        max_matches,
        collect_lines: collect,
    };

    reader
        .seek(SeekFrom::Start(it.byte_start))
        .map_err(|e| format!("定位失败: {}", e))?;
    let mut line_no = it.line_start;
    let mut pos = it.byte_start;
    loop {
        // 双保险：行号越界或已越过本分片字节上界即停（避免读到下一分片去）
        if line_no > it.line_end || pos >= it.byte_end {
            break;
        }
        if let Some(c) = cancel {
            if c.load(Ordering::Relaxed) {
                return Err("已取消".to_string());
            }
        }
        buf.clear();
        let n = reader
            .read_until(b'\n', &mut buf)
            .map_err(|e| format!("读取行失败: {}", e))?;
        if n == 0 {
            break;
        }
        // 去掉行尾换行符（\r\n / \n），否则关键词含 \r 时会误匹配
        let mut raw: &[u8] = &buf;
        if raw.last() == Some(&b'\n') {
            raw = &raw[..raw.len() - 1];
        }
        if raw.last() == Some(&b'\r') {
            raw = &raw[..raw.len() - 1];
        }
        acc.scanned += 1;
        // 本路径仅在「无筛选 + 有关键词」下使用：此时筛选条件恒真，
        // 故 `filter_matched` = 扫描过的全部行（UI 的「筛选 M 行 · 匹配 K 处」口径）
        acc.filter_matched += 1;
        pos += n as u64;
        let khit = match fast {
            Some(nd) => memchr::memmem::find(raw, nd).is_some(),
            None => {
                let (cow, _, _) = enc.decode(raw);
                m.hit_str(cow.as_ref())
            }
        };
        if khit {
            acc.hit(line_no);
        }
        line_no += 1;
    }
    Ok(acc.finish())
}

struct QuerySpec {
    path: String,
    encoding: String,
    snap: Option<Arc<IndexSnapshot>>,
    file_len: u64,
    total_lines: u64,
    f: Option<FilterSpec>,
    keyword: String,
    mode: String,
    ignore_case: bool,
    gap: u64,
    max_seg: usize,
    max_matches: u64,
}

struct QueryOutcome {
    segments: Vec<LineSegment>,
    lines: Vec<u64>,
    matched: u64,
    filter_matched: u64,
    scanned_lines: u64,
    pruned_blocks: u64,
    truncated: bool,
    matches_truncated: bool,
    unindexed_lines: u64,
}

/**
 * 合并查询主流程（同步阻塞）：**筛选与关键词在一次扫描内完成**。
 *
 * 这是本次改造的核心——旧实现里筛选走索引（`log_index_query`）、关键词走另一遍全文件重扫
 * （`log_search_lines`），两条链路互不知情，所以筛选后的命中行号可能根本不在筛选结果里，
 * 跳转必然断裂。合成一次后，`lines ⊆ ∪segments` 成为**同一次查询的两个视图**间的天然不变量。
 *
 * 四种组合见 §2.6；本函数严格按该表分支。
 */
fn query_lines_blocking(q: &QuerySpec, cancel: Option<&AtomicBool>) -> Result<QueryOutcome, String> {
    let enc = resolve_encoding(&q.encoding);
    let m = Matcher::build(&q.keyword, &q.mode, q.ignore_case, enc)?;
    let has_kw = !matches!(m, Matcher::None);
    let total_lines = q.total_lines;

    // 组合 1：无筛选 + 无关键词 → 全量视图，无需读文件
    if q.f.is_none() && !has_kw {
        let segments = if total_lines == 0 {
            Vec::new()
        } else {
            vec![LineSegment { start: 1, end: total_lines }]
        };
        return Ok(QueryOutcome {
            segments,
            lines: Vec::new(),
            matched: total_lines,
            filter_matched: total_lines,
            scanned_lines: 0,
            pruned_blocks: 0,
            truncated: false,
            matches_truncated: false,
            unindexed_lines: 0,
        });
    }

    // 快速路径：**仅级别筛选**（有时间条件或关键词时不成立）→ 纯内存扫 PackedLevels。
    // 这是压测倒逼出来的一层：块级剪枝对均匀分布的常见级别毫无收益，
    // 回读复核反而把"建索引时已解析过的结果"重算一遍。
    if let (Some(fs), false) = (&q.f, has_kw) {
        if fs.ts_from.is_none() && fs.ts_to.is_none() {
            if let Some(snap) = &q.snap {
                let parts = scan_packed_levels(&snap.levels, fs, q.gap, q.max_matches, cancel)?;
                let (segments, truncated) = merge_segments(&parts, q.gap, q.max_seg);
                return Ok(QueryOutcome {
                    segments,
                    lines: Vec::new(),
                    matched: parts.iter().map(|p| p.matched).sum(),
                    filter_matched: parts.iter().map(|p| p.filter_matched).sum(),
                    scanned_lines: 0,
                    pruned_blocks: 0,
                    truncated,
                    matches_truncated: false,
                    unindexed_lines: total_lines.saturating_sub(snap.indexed_lines),
                });
            }
        }
    }

    let blocks: &[BlockStat] = match &q.snap {
        Some(s) => &s.blocks,
        None => &[],
    };

    // 无时间条件 → 级别直接查位图（已含继承结果），跳过每行的级别/时间戳解析
    let packed: Option<&PackedLevels> = match &q.f {
        Some(fs) if fs.ts_from.is_none() && fs.ts_to.is_none() => {
            q.snap.as_ref().map(|sn| &sn.levels)
        }
        _ => None,
    };

    let mut pruned: u64 = 0;
    let mut parts: Vec<PartOut> = match &q.f {
        Some(fs) => {
            // 组合 2 / 4：L1 剪枝 → 逐块复核（有关键词时同一次读内完成匹配）
            let cand: Vec<usize> = (0..blocks.len())
                .filter(|&i| fs.invert || !blocks[i].can_skip(fs.mask, fs.ts_from, fs.ts_to))
                .collect();
            pruned = blocks.len() as u64 - cand.len() as u64;
            let groups = split_groups(&cand, worker_count());
            let results: Vec<Result<PartOut, String>> = groups
                .par_iter()
                .enumerate()
                .map(|(order, g)| {
                    run_block_group(
                        &q.path, enc, blocks, g, fs, &m, q.gap, q.max_matches, cancel, packed,
                        order,
                    )
                })
                .collect();
            // 任一分片失败即整体失败（含"已取消"）
            let mut outs = Vec::with_capacity(results.len());
            for r in results {
                outs.push(r?);
            }
            outs
        }
        None => {
            // 组合 3：无筛选 + 有关键词 → **全文件扫**。
            // 不能用 L1 的行数代替：索引在 MAX_INDEX_LINES 处截断，尾部行仍可能命中。
            let items = split_ranges(blocks, total_lines, q.file_len);
            let results: Vec<Result<PartOut, String>> = items
                .par_iter()
                .enumerate()
                .map(|(order, it)| {
                    run_range(&q.path, enc, it, &m, q.gap, q.max_matches, cancel, order)
                })
                .collect();
            let mut outs = Vec::with_capacity(results.len());
            for r in results {
                outs.push(r?);
            }
            outs
        }
    };
    parts.sort_by_key(|p| p.order);

    let (segments, truncated) = merge_segments(&parts, q.gap, q.max_seg);
    let matched: u64 = parts.iter().map(|p| p.matched).sum();
    let filter_matched: u64 = parts.iter().map(|p| p.filter_matched).sum();
    let scanned: u64 = parts.iter().map(|p| p.scanned).sum();
    let mut lines: Vec<u64> = Vec::new();
    for p in &parts {
        lines.extend(p.lines.iter().copied());
    }
    // 分片各自截断后合并仍可能超限，统一再截一次
    if (lines.len() as u64) > q.max_matches {
        lines.truncate(q.max_matches as usize);
    }

    let unindexed_lines = if q.f.is_some() {
        match &q.snap {
            Some(s) => total_lines.saturating_sub(s.indexed_lines),
            None => total_lines,
        }
    } else {
        0
    };

    Ok(QueryOutcome {
        segments,
        matched,
        filter_matched,
        scanned_lines: scanned,
        pruned_blocks: pruned,
        truncated,
        matches_truncated: (lines.len() as u64) < matched,
        unindexed_lines,
        lines,
    })
}

/**
 * 合并查询：**筛选 + 关键词一次完成**（取代 `log_index_query` 与 `log_search_lines`）。
 *
 * 返回两个视图：
 * - `segments`——渲染用（前端按段取内容）；
 * - `lines`——导航用（命中行号全集）。
 *
 * 二者出自同一次扫描，因此满足核心不变量 `lines ⊆ ∪segments`，跳转不再断裂（§1.1）。
 *
 * `task_id` 存在时注册取消令牌，可用 `logs_cancel` 中止；分片循环每块检查一次。
 */
#[allow(clippy::too_many_arguments)]
pub async fn log_query_lines(
    file_path: String,
    encoding: Option<String>,
    levels: Option<Vec<String>>,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    invert: Option<bool>,
    merge_gap: Option<u64>,
    max_segments: Option<u64>,
    keyword: Option<String>,
    mode: Option<String>,
    ignore_case: Option<bool>,
    max_matches: Option<u64>,
    task_id: Option<String>,
) -> Result<ResData<LogQueryResult>, String> {
    if !Path::new(&file_path).exists() {
        return Err("文件不存在".to_string());
    }
    let enc = encoding.unwrap_or_else(|| "UTF-8".to_string());
    let levels = levels.unwrap_or_default();
    let mask = levels_to_mask(&levels);
    let invert = invert.unwrap_or(false);
    let has_filter = !levels.is_empty() || ts_from.is_some() || ts_to.is_some() || invert;
    let gap = merge_gap.unwrap_or(0);
    let max_seg = (max_segments.unwrap_or(DEFAULT_MAX_SEGMENTS as u64) as usize).max(1);
    let kw = keyword.unwrap_or_default();
    let mode = mode.unwrap_or_else(|| "text".to_string());
    let ignore_case = ignore_case.unwrap_or(false);
    let max_matches = max_matches.unwrap_or(DEFAULT_MAX_MATCHES).max(1);

    let start = std::time::Instant::now();

    // 关键词为空且无筛选 → 纯全量视图，连文件都不必读
    // 有筛选必须有索引（剪枝依赖 L1）；仅有关键词时索引缺失也能扫，故不强求
    let snap = ensure_blocks(&file_path, &enc).await.ok();
    if has_filter && snap.is_none() {
        return Err("索引构建失败，无法筛选".to_string());
    }
    let (file_len, total_lines) = match &snap {
        Some(s) => (s.size, s.total_lines),
        None => (
            std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0),
            0,
        ),
    };
    // 无索引且无筛选（仅关键词）：行数未知，用 u64::MAX 让扫描读到 EOF 为止
    let total_lines = if total_lines == 0 {
        count_lines_quick(&file_path).unwrap_or(0)
    } else {
        total_lines
    };

    let handle = register_task(&task_id);
    let spec = QuerySpec {
        path: file_path,
        encoding: enc,
        snap,
        file_len,
        total_lines,
        f: if has_filter {
            Some(FilterSpec { mask, ts_from, ts_to, invert })
        } else {
            None
        },
        keyword: kw,
        mode,
        ignore_case,
        gap,
        max_seg,
        max_matches,
    };

    let flag = handle.as_ref().map(|(_, f)| f.clone());
    let res = tokio::task::spawn_blocking(move || {
        query_lines_blocking(&spec, flag.as_deref())
    })
    .await
    .map_err(|e| format!("查询任务失败: {}", e))?;

    if let Some((id, _)) = &handle {
        CANCEL_TOKENS.lock().remove(id);
    }
    let out = res?;

    let elapsed = start.elapsed().as_millis() as u64;
    Ok(ResData {
        code: 200,
        msg: format!(
            "命中 {} 行，{} 段；扫描 {} 行，剪枝 {} 块，耗时 {}ms",
            out.matched,
            out.segments.len(),
            out.scanned_lines,
            out.pruned_blocks,
            elapsed
        ),
        data: LogQueryResult {
            segments: out.segments,
            lines: out.lines,
            matched: out.matched,
            filter_matched: out.filter_matched,
            truncated: out.truncated,
            matches_truncated: out.matches_truncated,
            scanned_lines: out.scanned_lines,
            pruned_blocks: out.pruned_blocks,
            unindexed_lines: out.unindexed_lines,
            elapsed_ms: elapsed,
        },
    })
}

/** 快速数行（仅在无索引可用时兜底，避免为拿总行数再建一次索引） */
fn count_lines_quick(path: &str) -> Result<u64, String> {
    let file = File::open(path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut buf: Vec<u8> = Vec::with_capacity(64 * 1024);
    let mut total: u64 = 0;
    loop {
        buf.clear();
        let n = reader
            .read_until(b'\n', &mut buf)
            .map_err(|e| format!("读取行失败: {}", e))?;
        if n == 0 {
            break;
        }
        total += 1;
    }
    Ok(total)
}

// ==================== 任务取消（P6） ====================

// 取消令牌表。与 glogg 的 `interruptSearch()` + worker 线程同构：
// 长任务在分片循环里检查标志，前端点取消后结果被静默丢弃。
lazy_static! {
    static ref CANCEL_TOKENS: Mutex<HashMap<String, Arc<AtomicBool>>> =
        Mutex::new(HashMap::new());
}

/** 注册任务；`task_id` 为空时不注册（该任务不可取消） */
fn register_task(task_id: &Option<String>) -> Option<(String, Arc<AtomicBool>)> {
    let id = task_id.as_ref()?.clone();
    if id.is_empty() {
        return None;
    }
    let flag = Arc::new(AtomicBool::new(false));
    let mut m = CANCEL_TOKENS.lock();
    // 防止前端泄漏 task_id 导致表无限增长
    if m.len() > 64 {
        m.retain(|_, v| !v.load(Ordering::Relaxed));
    }
    m.insert(id.clone(), flag.clone());
    Some((id, flag))
}

/** 请求取消：返回是否命中一个在跑的任务 */
pub fn cancel_query(task_id: String) -> bool {
    let m = CANCEL_TOKENS.lock();
    match m.get(&task_id) {
        Some(f) => {
            f.store(true, Ordering::Relaxed);
            true
        }
        None => false,
    }
}

/**
 * 按筛选条件导出到目标文件（流式写入，不依赖索引，内存占用 O(1)）。
 *
 * 与查询分离的原因：导出是一次性全量写出，走流式只读一遍原文即可，
 * 不必为了拿到行号再回读一次文件。
 */
pub async fn log_index_export(
    file_path: String,
    encoding: Option<String>,
    dest_path: String,
    levels: Option<Vec<String>>,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    invert: Option<bool>,
    with_line_number: Option<bool>,
    with_bom: Option<bool>,
) -> Result<ResData<LogExportResult>, String> {
    if !Path::new(&file_path).exists() {
        return Err("文件不存在".to_string());
    }
    let enc_name = encoding.unwrap_or_else(|| "UTF-8".to_string());
    let mask = levels_to_mask(&levels.unwrap_or_default());
    let invert = invert.unwrap_or(false);
    let with_no = with_line_number.unwrap_or(false);
    let bom = with_bom.unwrap_or(false);

    let start = std::time::Instant::now();
    let result = tokio::task::spawn_blocking(move || -> Result<LogExportResult, String> {
        let enc = resolve_encoding(&enc_name);
        let src = File::open(&file_path).map_err(|e| format!("打开源文件失败: {}", e))?;
        let mut reader = BufReader::with_capacity(256 * 1024, src);
        let mut buf: Vec<u8> = Vec::with_capacity(1024);

        if let Some(parent) = Path::new(&dest_path).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("创建目标目录失败: {}", e))?;
            }
        }
        let dst = File::create(&dest_path).map_err(|e| format!("创建目标文件失败: {}", e))?;
        let mut writer = BufWriter::with_capacity(256 * 1024, dst);
        if bom {
            writer
                .write_all(&[0xEF, 0xBB, 0xBF])
                .map_err(|e| format!("写入 BOM 失败: {}", e))?;
        }

        let mut last_ts: i64 = NO_TS;
        let mut last_level: u8 = LV_UNKNOWN;
        let mut stack_level: Option<u8> = None;
        let mut line_no: u64 = 0;
        let mut written: u64 = 0;
        let mut bytes: u64 = 0;

        while let Some((line, _)) = read_line_lossy(&mut reader, &mut buf, Some(enc))
            .map_err(|e| format!("读取行失败: {}", e))?
        {
            line_no += 1;
            let info = analyze_line(&line, last_ts, last_level, &mut stack_level, true);
            if info.ts_own {
                last_ts = info.ts;
            }
            if stack_kind(&line).is_none() && info.level != LV_UNKNOWN {
                last_level = info.level;
            }

            let hit = level_match(info.level, mask) && ts_match(info.ts, ts_from, ts_to);
            let hit = if invert { !hit } else { hit };
            if !hit {
                continue;
            }

            let payload = if with_no {
                format!("{}: {}\n", line_no, line)
            } else {
                format!("{}\n", line)
            };
            bytes += payload.len() as u64;
            writer
                .write_all(payload.as_bytes())
                .map_err(|e| format!("写入失败: {}", e))?;
            written += 1;
        }
        writer.flush().map_err(|e| format!("刷新缓冲区失败: {}", e))?;

        Ok(LogExportResult {
            path: dest_path,
            lines: written,
            bytes,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    })
    .await
    .map_err(|e| format!("导出任务失败: {}", e))??;

    Ok(ResData {
        code: 200,
        msg: format!("已导出 {} 行", result.lines),
        data: result,
    })
}

/** 释放指定文件的索引缓存（关闭日志源时调用，及时回收内存） */
pub async fn log_index_drop(file_path: String) -> Result<ResData<bool>, String> {
    let mut cache = INDEX_CACHE.lock();
    let before = cache.len();
    let prefix = format!("{}|", file_path);
    cache.retain(|k, _| !k.starts_with(prefix.as_str()));
    Ok(ResData {
        code: 200,
        msg: "success".to_string(),
        data: cache.len() != before,
    })
}

#[cfg(test)]
mod level_detect_tests {
    use super::*;

    fn detect(line: &str) -> u8 {
        let mut stack: Option<u8> = None;
        analyze_line(line, NO_TS, LV_UNKNOWN, &mut stack, true).level
    }

    fn name(code: u8) -> &'static str {
        match code {
            LV_UNKNOWN => "UNKNOWN",
            LV_TRACE => "TRACE",
            LV_DEBUG => "DEBUG",
            LV_INFO => "INFO",
            LV_WARN => "WARN",
            LV_ERROR => "ERROR",
            LV_FATAL => "FATAL",
            _ => "?",
        }
    }

    #[test]
    fn detect_common_levels() {
        let cases = [
            ("2024-01-15 10:30:00,123 [main] INFO com.foo - started", LV_INFO),
            ("2024-01-15 10:30:01,000 [main] ERROR com.foo - failed to connect", LV_ERROR),
            ("2024-01-15 10:30:02,000 [main] WARN com.foo - low memory", LV_WARN),
            ("2024-01-15 10:30:03,000 [main] DEBUG com.foo - tick", LV_DEBUG),
            ("2024-01-15 10:30:04,000 [main] FATAL com.foo - boom", LV_FATAL),
            ("2024-01-15 10:30:05,000 ERROR something bad happened", LV_ERROR),
            ("15:30:00 ERROR no date prefix", LV_ERROR),
            ("2024-01-15 10:30:06,000 [main] c.f.Bar - ok", LV_UNKNOWN),
            ("java.lang.NullPointerException: x", LV_ERROR),
            ("Caused by: java.lang.IllegalStateError: y", LV_ERROR),
            // 级别词排在 120 字符之后（长 logger / MDC 前缀）—— 旧逻辑会漏识别
            (
                "2024-01-15 10:30:07,000 [http-nio-8080-exec-9] c.o.m.s.MyServiceImplementationClassWithLongName - processing request id=1234567890 status=ok payload={\"a\":1,\"b\":2,\"c\":3,\"d\":4,\"e\":5} result=ERROR unexpected failure",
                LV_ERROR,
            ),
            // 级别在行尾（远超窗口）—— 依赖整行兜底扫描
            (
                "2024-01-15 10:30:08,000 [main] c.f.Bar - operation finished with outcome that indicates a fatal termination ERROR",
                LV_ERROR,
            ),
        ];
        for (line, expect) in cases {
            let got = detect(line);
            eprintln!("[detect] {:?} => {} (expect {})", line, name(got), name(expect));
            assert_eq!(got, expect, "line: {line}");
        }
    }

    #[test]
    fn parse_ts_common_formats() {
        // (行, 期望的时间戳文本结束偏移)
        let cases: &[(&str, Option<usize>)] = &[
            ("2026-06-10 09:34:56,653 INFO started", Some(23)),
            ("2024-01-15 10:30:00,123 [main] INFO - started", Some(23)),
            ("2024-01-15 10:30:00.123 INFO - started", Some(23)),
            ("2024-01-15 10:30:00 INFO - started", Some(19)),
            ("2024/1/5 10:30:00,123 INFO - started", Some(21)),
            ("2024-01-15T10:30:00,123 INFO - started", Some(23)),
            ("2024-01-15T10:30:00.123Z INFO - started", Some(24)),
            ("2024-01-15 10:30:00,123+08:00 INFO - started", Some(29)),
            ("10:30:00 INFO no date", None),
            ("2024-01-15 INFO no time", None),
        ];
        for (line, expect_end) in cases {
            let got = parse_ts(line).map(|(_, end)| end);
            eprintln!("[ts] {:?} => {:?} (expect {:?})", line, got, expect_end);
            assert_eq!(got, *expect_end, "line: {line}");
        }
    }
}

#[cfg(test)]
mod index_structure_tests {
    use super::*;

    fn temp_log(name: &str, content: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("bdevtools_idx_{}", name));
        std::fs::write(&p, content.as_bytes()).unwrap();
        p
    }

    /** 独立算出的"每行起始偏移"（走字符串切分，与 `build_offsets` 的实现路径无关） */
    fn expected_offsets(content: &str) -> Vec<u64> {
        let mut v = Vec::new();
        let mut off = 0u64;
        for seg in content.split_inclusive('\n') {
            v.push(off);
            off += seg.len() as u64;
        }
        v
    }

    #[test]
    fn offsets_match_independent_byte_scan() {
        let mut content = String::new();
        for i in 0..3000 {
            // 行长刻意变化：覆盖 varint 的单字节与多字节两种编码
            content.push_str(&format!(
                "2026-01-01 00:00:00 INFO message {} {}\n",
                i,
                "x".repeat(i % 200)
            ));
        }
        content.push_str("tail without newline");

        let p = temp_log("offsets.txt", &content);
        let path = p.to_str().unwrap();
        let (offsets, total) = build_offsets(path).unwrap();
        let expect = expected_offsets(&content);

        assert_eq!(total, expect.len() as u64, "总行数应与独立字节扫描一致");
        for (i, &e) in expect.iter().enumerate() {
            assert_eq!(
                offsets.offset_of(i as u64 + 1),
                Some(e),
                "第 {} 行偏移不符",
                i + 1
            );
        }
        assert_eq!(offsets.offset_of(total + 1), None, "越界行号必须返回 None");
    }

    #[test]
    fn byte_line_count_matches_decoded_scan() {
        // CRLF / 空行 / 中文 / 堆栈行 / 末行无换行：
        // 阶段一（字节级）与阶段二（解码级）的行数口径一旦不一致，行号会整体错位
        let content =
            "a\r\nb\n\n中文行\nERROR boom\n    at Foo.bar(Foo.java:1)\n末尾无换行";
        let p = temp_log("count.txt", content);
        let path = p.to_str().unwrap();

        let (_, total) = build_offsets(path).unwrap();
        let enc = resolve_encoding("UTF-8");
        let mut reader = BufReader::new(File::open(path).unwrap());
        let mut buf = Vec::new();
        let mut decoded = 0u64;
        while read_line_lossy(&mut reader, &mut buf, Some(enc))
            .unwrap()
            .is_some()
        {
            decoded += 1;
        }
        assert_eq!(total, decoded, "字节级数行与解码级行数必须一致（否则行号错位）");
    }

    /**
     * 自适应编码（P1b）：块内择优 varint / 定宽。
     *
     * 两条断言缺一不可——只测"偏移正确"会漏掉"根本没走自适应"的实现，
     * 只测"两种模式都出现"会漏掉"选错了模式导致偏移错位"。
     */
    #[test]
    fn adaptive_block_encoding_is_correct_and_beats_plain_varint() {
        let mut content = String::new();
        // 前半：行长 ~66B（<128）→ varint 1B/行 最优
        // 后半：行长 ~206B（128–255）→ 定宽 1B/行 最优，varint 要 2B/行
        for i in 0..5000 {
            if i < 2500 {
                content.push_str(&format!("{:04} {}\n", i, "s".repeat(60)));
            } else {
                content.push_str(&format!("{:04} {}\n", i, "L".repeat(200)));
            }
        }
        let p = temp_log("adaptive.txt", &content);
        let (offsets, total) = build_offsets(p.to_str().unwrap()).unwrap();
        let expect = expected_offsets(&content);

        // ① 偏移必须仍然逐行精确（自适应最容易写错的就是定宽块的累加口径）
        assert_eq!(total, expect.len() as u64);
        for (i, &e) in expect.iter().enumerate() {
            assert_eq!(
                offsets.offset_of(i as u64 + 1),
                Some(e),
                "自适应编码下第 {} 行偏移不符",
                i + 1
            );
        }

        // ② 两种模式都必须被选中，否则说明择优逻辑事实上没生效
        let varint_blocks = offsets.block_mode.iter().filter(|&&m| m == MODE_VARINT).count();
        let fixed_blocks = offsets.block_mode.iter().filter(|&&m| m != MODE_VARINT).count();
        assert!(
            varint_blocks > 0 && fixed_blocks > 0,
            "两种编码都应被选中：varint={varint_blocks} fixed={fixed_blocks}"
        );

        // ③ 严格优于纯 varint（择优按块取 min，故必然 ≤；出现定宽块即严格 <）
        let plain_varint: usize = expect.windows(2).map(|w| varint_len(w[1] - w[0])).sum();
        let used = offsets.deltas.len() + offsets.block_mode.len();
        assert!(
            used < plain_varint,
            "自适应必须严格优于纯 varint：实际 {} < 纯 varint {}",
            used,
            plain_varint
        );
    }

    /** 末块不足一块时必须也能封块（否则这些行只能走 pending 慢路径） */
    #[test]
    fn trailing_partial_block_is_flushed() {
        let content = "a\n".repeat(300); // 300 = 2*128 + 44，末块不满
        let p = temp_log("partial.txt", &content);
        let (offsets, total) = build_offsets(p.to_str().unwrap()).unwrap();
        assert_eq!(total, 300);
        assert_eq!(
            offsets.block_mode.len(),
            offsets.block_base.len(),
            "finish() 后所有块都应已封块"
        );
        assert!(offsets.pending.is_empty());
        assert_eq!(offsets.offset_of(300), Some(299 * 2), "末行偏移");
        assert_eq!(offsets.offset_of(301), None, "越界必须返回 None");
    }

    fn dummy_entry(bytes: usize, last_used: u64) -> IndexEntry {
        IndexEntry {
            snap: Arc::new(IndexSnapshot {
                blocks: Vec::new(),
                levels: PackedLevels::with_capacity(0),
                offsets: Arc::new(LineOffsets::default()),
                total_lines: 0,
                indexed_lines: 0,
                truncated: false,
                level_counts: [0; LV_COUNT],
                ts_min: None,
                ts_max: None,
                ts_count: 0,
                buckets: Vec::new(),
                offsets_ready: false,
                blocks_ready: false,
                mtime_ms: 0,
                size: 0,
            }),
            mtime_ms: 0,
            size: 0,
            bytes,
            last_used,
        }
    }

    /** 按字节预算 + LRU 淘汰（§7.8：原实现是「超条目数就 clear()」） */
    #[test]
    fn cache_evicts_lru_within_byte_budget() {
        let mut cache = HashMap::new();
        cache.insert("fresh".to_string(), dummy_entry(400, 99));
        cache.insert("old".to_string(), dummy_entry(300, 1));
        cache.insert("mid".to_string(), dummy_entry(300, 50));

        // 预算 500、总量 1000 → 按 LRU 先淘汰 old，再淘汰 mid，恰好回到 400
        let evicted = evict_to_budget(&mut cache, "fresh", 500);
        assert_eq!(evicted, vec!["old".to_string(), "mid".to_string()]);
        assert!(cache.contains_key("fresh"), "刚发布的条目绝不能被自己挤掉");
        assert!(cache.values().map(|e| e.bytes).sum::<usize>() <= 500);
    }

    /** 单条目就超预算时也必须保留它——否则会什么都不留，下一次打开又是全量重建 */
    #[test]
    fn oversized_single_entry_survives_eviction() {
        let mut cache = HashMap::new();
        cache.insert("big".to_string(), dummy_entry(1000, 5));
        cache.insert("small".to_string(), dummy_entry(10, 1));

        let evicted = evict_to_budget(&mut cache, "big", 500);
        assert_eq!(evicted, vec!["small".to_string()]);
        assert!(cache.contains_key("big"), "超预算的单条目必须保留");
    }

    /** 全量逐行判定（不剪枝、不依赖索引）——对拍基准 */
    fn full_scan_hits(
        path: &str,
        mask: u16,
        from: Option<i64>,
        to: Option<i64>,
        invert: bool,
    ) -> Vec<u64> {
        let enc = resolve_encoding("UTF-8");
        let mut reader = BufReader::new(File::open(path).unwrap());
        let mut buf = Vec::new();
        let mut out = Vec::new();
        let mut last_ts = NO_TS;
        let mut last_level = LV_UNKNOWN;
        let mut stack_level = None;
        let mut n = 0u64;
        while let Some((line, _)) = read_line_lossy(&mut reader, &mut buf, Some(enc)).unwrap() {
            n += 1;
            let info = analyze_line(&line, last_ts, last_level, &mut stack_level, true);
            if info.ts_own {
                last_ts = info.ts;
            }
            if !matches!(stack_kind(&line), Some(_)) && info.level != LV_UNKNOWN {
                last_level = info.level;
            }
            let hit = level_match(info.level, mask) && ts_match(info.ts, from, to);
            if invert != hit {
                out.push(n);
            }
        }
        out
    }

    /** 对拍基准：全量逐行判定（可选关键词），不剪枝、不依赖索引 */
    fn full_scan_query(
        path: &str,
        mask: u16,
        from: Option<i64>,
        to: Option<i64>,
        invert: bool,
        keyword: &str,
    ) -> (Vec<u64>, Vec<u64>) {
        let enc = resolve_encoding("UTF-8");
        let mut reader = BufReader::new(File::open(path).unwrap());
        let mut buf = Vec::new();
        let mut filter_hits = Vec::new();
        let mut hits = Vec::new();
        let mut last_ts = NO_TS;
        let mut last_level = LV_UNKNOWN;
        let mut stack_level = None;
        let mut n = 0u64;
        while let Some((line, _)) = read_line_lossy(&mut reader, &mut buf, Some(enc)).unwrap() {
            n += 1;
            let info = analyze_line(&line, last_ts, last_level, &mut stack_level, true);
            if info.ts_own {
                last_ts = info.ts;
            }
            if !matches!(stack_kind(&line), Some(_)) && info.level != LV_UNKNOWN {
                last_level = info.level;
            }
            let hit = level_match(info.level, mask) && ts_match(info.ts, from, to);
            if invert != hit {
                filter_hits.push(n);
                if keyword.is_empty() || line.contains(keyword) {
                    hits.push(n);
                }
            }
        }
        (filter_hits, hits)
    }

    /** 构造样本：约 6000 行、跨多个块，含跨块 ERROR 堆栈继承与可检索关键词 */
    fn sample_content() -> String {
        let mut content = String::new();
        for i in 0..5000u64 {
            let secs = i * 60;
            let day = 1 + secs / 86400;
            let sod = secs % 86400;
            let ts = format!(
                "2026-01-{:02} {:02}:{:02}:{:02}",
                day,
                sod / 3600,
                (sod / 60) % 60,
                sod % 60
            );
            match i % 10 {
                0..=5 => content.push_str(&format!("{} INFO normal {} NEEDLE
", ts, i)),
                6..=7 => content.push_str(&format!("{} WARN careful {}
", ts, i)),
                8 => {
                    content.push_str(&format!("{} ERROR boom {} NEEDLE
", ts, i));
                    content.push_str("    at com.example.Foo.bar(Foo.java:12)
");
                    content.push_str("    at com.example.Baz.qux(Baz.java:34)
");
                }
                _ => content.push_str(&format!("{} DEBUG detail {}
", ts, i)),
            }
        }
        content
    }

    fn run_query(path: &str, snap: IndexSnapshot, f: Option<FilterSpec>, keyword: &str) -> QueryOutcome {
        let total_lines = snap.total_lines;
        let spec = QuerySpec {
            path: path.to_string(),
            encoding: "UTF-8".to_string(),
            snap: Some(Arc::new(snap)),
            file_len: std::fs::metadata(path).unwrap().len(),
            total_lines,
            f,
            keyword: keyword.to_string(),
            mode: "text".to_string(),
            ignore_case: false,
            gap: 0,
            max_seg: 100_000,
            max_matches: 100_000,
        };
        query_lines_blocking(&spec, None).unwrap()
    }

    fn expand_segments(segs: &[LineSegment]) -> Vec<u64> {
        let mut v = Vec::new();
        for s in segs {
            for l in s.start..=s.end {
                v.push(l);
            }
        }
        v
    }

    fn build_snapshot(path: &str) -> IndexSnapshot {
        let (offsets, total) = build_offsets(path).unwrap();
        let offsets = Arc::new(offsets);
        let built = build_blocks(path, "UTF-8", &offsets, total).unwrap();
        IndexSnapshot {
            blocks: built.blocks,
            levels: built.levels,
            offsets,
            total_lines: total,
            indexed_lines: built.indexed_lines,
            truncated: total > MAX_INDEX_LINES,
            level_counts: built.level_counts,
            ts_min: built.ts_min,
            ts_max: built.ts_max,
            ts_count: built.ts_count,
            buckets: Vec::new(),
            offsets_ready: true,
            blocks_ready: true,
            mtime_ms: 0,
            size: 0,
        }
    }

    /**
     * 块级剪枝 + 回读复核，必须与"全量逐行判定"逐行一致。
     *
     * 这是本阶段最重要的回归防线：剪枝写错（漏剪或误剪）症状是
     * **筛选结果悄悄少几行**，肉眼极难发现。
     */
    #[test]
    fn query_matches_full_scan_for_all_filters() {
        let mut content = String::new();
        // 5000 轮 → 约 6000 行，跨 6 个 BLOCK_LINES 块：
        // 保证存在 ERROR 的堆栈继承**跨块**的情况（最易出错的分支）
        for i in 0..5000u64 {
            let secs = i * 60;
            let day = 1 + secs / 86400;
            let sod = secs % 86400;
            let ts = format!(
                "2026-01-{:02} {:02}:{:02}:{:02}",
                day,
                sod / 3600,
                (sod / 60) % 60,
                sod % 60
            );
            match i % 10 {
                0..=5 => content.push_str(&format!("{} INFO normal {}\n", ts, i)),
                6..=7 => content.push_str(&format!("{} WARN careful {}\n", ts, i)),
                8 => {
                    content.push_str(&format!("{} ERROR boom {}\n", ts, i));
                    content.push_str("    at com.example.Foo.bar(Foo.java:12)\n");
                    content.push_str("    at com.example.Baz.qux(Baz.java:34)\n");
                }
                _ => content.push_str(&format!("{} DEBUG detail {}\n", ts, i)),
            }
        }

        let p = temp_log("query.txt", &content);
        let path = p.to_str().unwrap();
        let snap = build_snapshot(path);
        assert!(snap.blocks.len() >= 5, "测试数据应跨多个块，否则剪枝未被覆盖");

        let err_mask = 1u16 << LV_ERROR;
        let warn_mask = 1u16 << LV_WARN;
        let (tmin, tmax) = (snap.ts_min.unwrap(), snap.ts_max.unwrap());
        let from = tmin + (tmax - tmin) / 4;
        let to = tmin + (tmax - tmin) / 2;

        let cases: Vec<(&str, u16, Option<i64>, Option<i64>, bool)> = vec![
            ("仅 ERROR", err_mask, None, None, false),
            ("仅 WARN", warn_mask, None, None, false),
            ("不限级别+时间窗", 0, Some(from), Some(to), false),
            ("ERROR+时间窗", err_mask, Some(from), Some(to), false),
            ("反向：非 ERROR", err_mask, None, None, true),
            ("不限任何条件", 0, None, None, false),
        ];

        for (name, mask, from, to, invert) in cases {
            let snap = build_snapshot(path);
            let (filter_expect, expect) = full_scan_query(path, mask, from, to, invert, "");
            let out = run_query(
                path,
                snap,
                Some(FilterSpec { mask, ts_from: from, ts_to: to, invert }),
                "",
            );
            assert_eq!(
                expand_segments(&out.segments),
                expect,
                "场景「{}」的段展开结果与全量逐行不一致",
                name
            );
            assert_eq!(out.matched, expect.len() as u64, "场景「{}」的命中计数不一致", name);
            assert_eq!(
                out.filter_matched,
                filter_expect.len() as u64,
                "场景「{}」的筛选计数不一致",
                name
            );
        }
    }

    /**
     * 合并查询（P3）：**四种组合**都必须与全量逐行判定一致（§2.6）。
     *
     * 这是本阶段最关键的回归防线——筛选与搜索合成一次后，
     * 任一组合的候选集选错（剪枝误杀 / 关键词漏扫）症状都是"悄悄少几行"。
     */
    #[test]
    fn query_lines_matches_full_scan_for_all_combinations() {
        let content = sample_content();
        let p = temp_log("combos.txt", &content);
        let path = p.to_str().unwrap();

        let err = FilterSpec { mask: 1u16 << LV_ERROR, ts_from: None, ts_to: None, invert: false };
        let cases: Vec<(&str, Option<(u16, Option<i64>, Option<i64>, bool)>, &str)> = vec![
            ("无筛选+无关键词", None, ""),
            ("有筛选+无关键词", Some((err.mask, None, None, false)), ""),
            ("无筛选+有关键词", None, "NEEDLE"),
            ("有筛选+有关键词", Some((err.mask, None, None, false)), "NEEDLE"),
        ];

        for (name, fspec, kw) in cases {
            let (mask, from, to, invert) = fspec.unwrap_or((0, None, None, false));
            let snap = build_snapshot(path);
            let total_lines = snap.total_lines;
            let out = run_query(
                path,
                snap,
                fspec.map(|(m, f, t, iv)| FilterSpec { mask: m, ts_from: f, ts_to: t, invert: iv }),
                kw,
            );
            let (filter_expect, expect) = full_scan_query(path, mask, from, to, invert, kw);

            assert_eq!(
                expand_segments(&out.segments),
                expect,
                "组合「{}」段展开结果与全量逐行不一致",
                name
            );
            assert_eq!(out.matched, expect.len() as u64, "组合「{}」命中计数不一致", name);
            assert_eq!(
                out.filter_matched,
                filter_expect.len() as u64,
                "组合「{}」筛选计数不一致",
                name
            );
            if !kw.is_empty() {
                // 核心不变量：导航行号 == 段展开（lines ⊆ ∪segments 的最强形式）
                assert_eq!(out.lines, expect, "组合「{}」导航行号不一致", name);
            }
            if name == "无筛选+无关键词" {
                assert_eq!(out.matched, total_lines, "全量视图应命中所有行");
            }
        }
    }

    /** 有筛选时必须报告被剪枝的块数（否则无法判断剪枝是否真的生效） */
    #[test]
    fn query_reports_pruned_blocks() {
        let mut content = String::new();
        for i in 0..3000 {
            content.push_str(&format!("2026-01-01 00:00:00 INFO only-info {}
", i));
        }
        for i in 0..3000 {
            content.push_str(&format!("2026-01-02 00:00:00 ERROR boom {}
", i));
        }
        let p = temp_log("pruned.txt", &content);
        let path = p.to_str().unwrap();
        let snap = build_snapshot(path);

        // ① 仅级别筛选（无时间、无关键词）→ 走 PackedLevels 纯内存路径，一次 IO 都不做
        let fast = run_query(
            path,
            build_snapshot(path),
            Some(FilterSpec { mask: 1u16 << LV_ERROR, ts_from: None, ts_to: None, invert: false }),
            "",
        );
        assert_eq!(fast.matched, 3000, "应有 3000 行 ERROR");
        assert_eq!(fast.scanned_lines, 0, "纯内存路径不应回读原文");
        assert_eq!(fast.pruned_blocks, 0, "纯内存路径不经过剪枝");

        // ② 叠加时间条件 → 退回"剪枝 + 回读复核"，此时前段（纯 INFO 且时间不符）应整块排除
        let ts_max = snap.ts_max.unwrap();
        let out = run_query(
            path,
            snap,
            Some(FilterSpec { mask: 1u16 << LV_ERROR, ts_from: Some(ts_max - 1000), ts_to: None, invert: false }),
            "",
        );
        assert!(out.pruned_blocks > 0, "时间窗之外的块应被整块排除");
        assert!(
            out.scanned_lines < 6000,
            "剪枝后扫描行数应少于全文件：{}",
            out.scanned_lines
        );
    }

    /** 取消令牌（P6）：置位后必须尽快返回 Err，而不是跑完整文件 */
    #[test]
    fn query_can_be_cancelled() {
        let content = sample_content();
        let p = temp_log("cancel.txt", &content);
        let path = p.to_str().unwrap();
        let snap = build_snapshot(path);
        let spec = QuerySpec {
            path: path.to_string(),
            encoding: "UTF-8".to_string(),
            snap: Some(Arc::new(snap)),
            file_len: std::fs::metadata(path).unwrap().len(),
            total_lines: std::fs::metadata(path).unwrap().len(),
            f: None,
            keyword: "NEEDLE".to_string(),
            mode: "text".to_string(),
            ignore_case: false,
            gap: 0,
            max_seg: 1000,
            max_matches: 1000,
        };
        let flag = AtomicBool::new(true);
        let err = match query_lines_blocking(&spec, Some(&flag)) {
            Ok(_) => panic!("取消后不应返回结果"),
            Err(e) => e,
        };
        assert!(err.contains("已取消"), "错误信息应标明已取消，实际：{err}");
    }

    /** 正则双引擎降级（§2.8）：regex 不支持前瞻 → 必须落到 fancy-regex 而不是报错 */
    #[test]
    fn regex_falls_back_to_fancy_engine() {
        let m = Matcher::build(r"\d+(?=ms)", "regex", false, resolve_encoding("UTF-8")).unwrap();
        assert!(matches!(m, Matcher::Fancy(_)), "前瞻应走备用引擎");
        assert!(m.hit_str("took 120ms"), "前瞻必须能匹配");
        assert!(!m.hit_str("took 120s"), "前瞻不应匹配");
    }

    /**
     * 压测（P6）：默认不跑——设 `BDEV_STRESS_FILE=<大日志文件>` 才启用。
     *
     * 只报告耗时，不做断言：阈值得在目标机型上实测后才定，
     * 写死数值只会让 CI 在慢机器上假红。
     */
    #[test]
    fn stress_large_file_reports_timings() {
        let Ok(path) = std::env::var("BDEV_STRESS_FILE") else { return };
        let t = std::time::Instant::now();
        let (offsets, total) = build_offsets(&path).unwrap();
        let t_off = t.elapsed().as_millis();
        let offsets = Arc::new(offsets);

        let t = std::time::Instant::now();
        let built = build_blocks(&path, "UTF-8", &offsets, total).unwrap();
        let t_blocks = t.elapsed().as_millis();

        let file_len = std::fs::metadata(&path).unwrap().len();
        let snap = Arc::new(IndexSnapshot {
            blocks: built.blocks,
            levels: built.levels,
            offsets: offsets.clone(),
            total_lines: total,
            indexed_lines: built.indexed_lines,
            truncated: false,
            level_counts: built.level_counts,
            ts_min: built.ts_min,
            ts_max: built.ts_max,
            ts_count: built.ts_count,
            buckets: Vec::new(),
            offsets_ready: true,
            blocks_ready: true,
            mtime_ms: 0,
            size: file_len,
        });

        let mb = file_len as f64 / 1024.0 / 1024.0;
        let mk = |f: Option<FilterSpec>, kw: &str, max_matches: u64| QuerySpec {
            path: path.clone(),
            encoding: "UTF-8".to_string(),
            snap: Some(snap.clone()),
            file_len,
            total_lines: total,
            f,
            keyword: kw.to_string(),
            mode: "text".to_string(),
            ignore_case: false,
            gap: 0,
            max_seg: 2000,
            max_matches,
        };

        let t = std::time::Instant::now();
        let kw_out = query_lines_blocking(&mk(None, "payload", 50_000), None).unwrap();
        let t_kw = t.elapsed().as_millis();

        let t = std::time::Instant::now();
        let err_out = query_lines_blocking(
            &mk(
                Some(FilterSpec { mask: 1u16 << LV_ERROR, ts_from: None, ts_to: None, invert: false }),
                "",
                50_000,
            ),
            None,
        )
        .unwrap();
        let t_err = t.elapsed().as_millis();

        let t = std::time::Instant::now();
        let both_out = query_lines_blocking(
            &mk(
                Some(FilterSpec { mask: 1u16 << LV_ERROR, ts_from: None, ts_to: None, invert: false }),
                "payload",
                50_000,
            ),
            None,
        )
        .unwrap();
        let t_both = t.elapsed().as_millis();

        eprintln!(
            "[stress] {:.0}MB / {} 行 | L2 偏移 {}ms ({:.1}ms/100MB) | L1 块 {}ms | \
关键词全扫 {}ms (命中 {}, 扫描 {}) | ERROR 筛选 {}ms (命中 {}, 剪枝 {} 块, 扫描 {}) | 筛选+关键词 {}ms (命中 {}) | \
L2 估算 {}MB",
            mb, total, t_off, t_off as f64 / mb * 100.0, t_blocks,
            t_kw, kw_out.matched, kw_out.scanned_lines,
            t_err, err_out.matched, err_out.pruned_blocks, err_out.scanned_lines,
            t_both, both_out.matched,
            offsets.approx_bytes() as f64 / 1024.0 / 1024.0,
        );
    }

    /** 文本模式：字节快路径必须与解码后的 contains 等价 */
    #[test]
    fn text_matcher_raw_and_decoded_agree() {
        let m = Matcher::build("NEEDLE", "text", false, resolve_encoding("UTF-8")).unwrap();
        assert!(m.needle_utf8().is_some(), "大小写敏感的文本应走字节快路径");
        assert!(m.hit_str("a NEEDLE here"));
        assert!(!m.hit_str("a needle here"), "大小写敏感时不该命中");
        // 忽略大小写：不能用字节快路径（多字节编码下 ASCII 折叠会漏匹配）
        let ci = Matcher::build("NEEDLE", "text", true, resolve_encoding("UTF-8")).unwrap();
        assert!(ci.needle_utf8().is_none());
        assert!(ci.hit_str("a needle here"));
    }

    #[test]
    fn block_prune_skips_blocks_without_target_level() {
        // 前 3000 行只有 INFO，后 3000 行有 ERROR：筛 ERROR 时前段应整块排除
        let mut content = String::new();
        for i in 0..3000 {
            content.push_str(&format!("2026-01-01 00:00:00 INFO only-info {}\n", i));
        }
        for i in 0..3000 {
            content.push_str(&format!("2026-01-02 00:00:00 ERROR boom {}\n", i));
        }
        let p = temp_log("prune.txt", &content);
        let path = p.to_str().unwrap();
        let snap = build_snapshot(path);

        let err_mask = 1u16 << LV_ERROR;
        let skipped = snap.blocks.iter().filter(|b| b.can_skip(err_mask, None, None)).count();
        assert!(skipped > 0, "纯 INFO 的块应被整块排除（剪枝未生效）");
        assert!(
            skipped < snap.blocks.len(),
            "含 ERROR 的块不得被排除，否则剪枝误杀"
        );

        // 剪枝生效后结果仍须与全量一致
        let out = run_query(
            path,
            snap,
            Some(FilterSpec { mask: err_mask, ts_from: None, ts_to: None, invert: false }),
            "",
        );
        assert_eq!(expand_segments(&out.segments), full_scan_hits(path, err_mask, None, None, false));
        assert_eq!(out.matched, 3000, "应有 3000 行 ERROR");
    }
}
