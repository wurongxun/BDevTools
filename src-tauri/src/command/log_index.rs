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

use crate::command::search_log_parallel::{read_line_lossy, resolve_encoding};
use crate::response::resp_data::ResData;
use chrono::{Local, NaiveDate, TimeZone};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
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

/** 最多缓存多少个文件的索引，超出后整体清空，避免长时间占用内存 */
const MAX_CACHE_ENTRIES: usize = 3;

/** 单个查询最多返回多少段，避免稀疏命中时传输超大数组 */
const DEFAULT_MAX_SEGMENTS: usize = 2000;

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
    static ref INDEX_CACHE: Mutex<HashMap<String, LogIndex>> = Mutex::new(HashMap::new());
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
fn analyze_line(line: &str, last_ts: i64, last_level: u8, stack_level: &mut Option<u8>) -> LineInfo {
    let ts_info = parse_ts(line);
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

/** 内存中的日志索引（按行存时间戳与级别） */
struct LogIndex {
    ts: Vec<i64>,
    levels: Vec<u8>,
    level_counts: [u64; LV_COUNT],
    total_lines: u64,
    indexed_lines: u64,
    truncated: bool,
    ts_min: Option<i64>,
    ts_max: Option<i64>,
    ts_count: u64,
    buckets: Vec<TimeBucket>,
    /** 建立索引时的文件指纹，用于判断是否已过期 */
    mtime_ms: u64,
    size: u64,
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
}

#[derive(Serialize)]
pub struct LineSegment {
    /** 段起始行号（1-based，含） */
    pub start: u64,
    /** 段结束行号（1-based，含） */
    pub end: u64,
}

#[derive(Serialize)]
pub struct LogQueryResult {
    /** 命中行段（按行号升序，已合并相邻命中行） */
    pub segments: Vec<LineSegment>,
    /** 命中总行数 */
    pub matched: u64,
    /** 段数是否因超过 max_segments 被截断 */
    pub truncated: bool,
    pub elapsed_ms: u64,
}

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

/** 按 ts_min..ts_max 均匀分桶，统计每桶总行数与警告/错误数 */
fn compute_buckets(ts: &[i64], levels: &[u8], ts_min: i64, ts_max: i64) -> Vec<TimeBucket> {
    let mut buckets: Vec<TimeBucket> = Vec::with_capacity(BUCKET_COUNT.min(ts.len().max(1)));
    let span = (ts_max - ts_min).max(1);
    let width = ((span as f64) / BUCKET_COUNT as f64).ceil().max(1.0) as i64;
    let count = ((span / width) + 1).min(BUCKET_COUNT as i64) as usize;

    for i in 0..count {
        buckets.push(TimeBucket {
            start: ts_min + i as i64 * width,
            end: ts_min + (i + 1) as i64 * width,
            total: 0,
            warn: 0,
            error: 0,
            first_line: None,
        });
    }

    for (i, &t) in ts.iter().enumerate() {
        if t == NO_TS {
            continue;
        }
        let idx = ((t - ts_min) / width).max(0) as usize;
        if idx >= buckets.len() {
            continue;
        }
        let b = &mut buckets[idx];
        b.total += 1;
        if b.first_line.is_none() {
            b.first_line = Some((i as u64) + 1);
        }
        match levels[i] {
            LV_WARN => b.warn += 1,
            l if l >= LV_ERROR => b.error += 1,
            _ => {}
        }
    }
    buckets
}

/** 全量扫描文件建立索引（同步阻塞，调用方负责放到阻塞线程池） */
fn build_index(path: &str, encoding: &str) -> Result<LogIndex, String> {
    let enc = resolve_encoding(encoding);
    let file = File::open(path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut reader = BufReader::with_capacity(256 * 1024, file);
    let mut buf: Vec<u8> = Vec::with_capacity(1024);

    // 按"平均每行 40 字节"预估容量，避免向量扩容抖动
    let cap = (std::fs::metadata(path).map(|m| m.len()).unwrap_or(0) / 40)
        .clamp(1024, MAX_INDEX_LINES) as usize;
    let mut ts: Vec<i64> = Vec::with_capacity(cap);
    let mut levels: Vec<u8> = Vec::with_capacity(cap);

    let mut level_counts = [0u64; LV_COUNT];
    let mut last_ts: i64 = NO_TS;
    let mut last_level: u8 = LV_UNKNOWN;
    let mut stack_level: Option<u8> = None;
    let mut total_lines: u64 = 0;
    let mut indexed_lines: u64 = 0;
    let mut ts_min: i64 = i64::MAX;
    let mut ts_max: i64 = i64::MIN;
    let mut ts_count: u64 = 0;

    while let Some((line, _)) = read_line_lossy(&mut reader, &mut buf, Some(enc))
        .map_err(|e| format!("读取行失败: {}", e))?
    {
        total_lines += 1;
        if total_lines > MAX_INDEX_LINES {
            continue;
        }
        let info = analyze_line(&line, last_ts, last_level, &mut stack_level);
        if info.ts_own {
            last_ts = info.ts;
            ts_count += 1;
            if info.ts < ts_min {
                ts_min = info.ts;
            }
            if info.ts > ts_max {
                ts_max = info.ts;
            }
        }
        if !matches!(stack_kind(&line), Some(_)) {
            // 普通行：更新"最近级别"，供后续堆栈帧兜底继承
            if info.level != LV_UNKNOWN {
                last_level = info.level;
            }
        }
        level_counts[info.level as usize] += 1;
        ts.push(info.ts);
        levels.push(info.level);
        indexed_lines += 1;
    }

    let (ts_min_opt, ts_max_opt) = if ts_count > 0 {
        (Some(ts_min), Some(ts_max))
    } else {
        (None, None)
    };
    let buckets = match (ts_min_opt, ts_max_opt) {
        (Some(a), Some(b)) => compute_buckets(&ts, &levels, a, b),
        _ => Vec::new(),
    };

    let (mtime_ms, size) = file_stamp(path)?;
    Ok(LogIndex {
        ts,
        levels,
        level_counts,
        total_lines,
        indexed_lines,
        truncated: total_lines > MAX_INDEX_LINES,
        ts_min: ts_min_opt,
        ts_max: ts_max_opt,
        ts_count,
        buckets,
        mtime_ms,
        size,
    })
}

/** 确保索引可用（过期或缺失时重建）；构建过程放到阻塞线程池，避免卡住异步运行时 */
async fn ensure_index(path: &str, encoding: &str) -> Result<(), String> {
    let key = cache_key(path, encoding);
    let stamp = file_stamp(path)?;
    {
        let cache = INDEX_CACHE.lock();
        if let Some(e) = cache.get(&key) {
            if e.mtime_ms == stamp.0 && e.size == stamp.1 {
                return Ok(());
            }
        }
    }

    let p = path.to_string();
    let enc = encoding.to_string();
    let built = tokio::task::spawn_blocking(move || build_index(&p, &enc))
        .await
        .map_err(|e| format!("索引任务失败: {}", e))??;

    let mut cache = INDEX_CACHE.lock();
    if cache.len() >= MAX_CACHE_ENTRIES {
        cache.clear();
    }
    cache.insert(key, built);
    Ok(())
}

fn meta_from_index(idx: &LogIndex, elapsed_ms: u64, cached: bool) -> LogIndexMeta {
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
    }
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
    let cached = {
        let cache = INDEX_CACHE.lock();
        matches!(cache.get(&key), Some(e) if e.mtime_ms == stamp.0 && e.size == stamp.1)
    };

    ensure_index(&file_path, &enc).await?;

    let elapsed = start.elapsed().as_millis() as u64;
    let guard = INDEX_CACHE.lock();
    let idx = guard.get(&key).ok_or("索引已失效，请重试")?;
    Ok(ResData {
        code: 200,
        msg: if cached {
            "索引已就绪（命中缓存）".to_string()
        } else {
            format!("索引完成，共 {} 行", idx.total_lines)
        },
        data: meta_from_index(idx, elapsed, cached),
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
 * 按级别与时间范围筛选，返回命中行的**连续段**。
 *
 * 返回段而非行号数组：命中往往成片（如一个时间窗内连续几千行），
 * 用段表示既能把传输量压到 KB 级，前端也能直接用 `read_file_lines` 按段取内容。
 */
pub async fn log_index_query(
    file_path: String,
    encoding: Option<String>,
    levels: Option<Vec<String>>,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    invert: Option<bool>,
    merge_gap: Option<u64>,
    max_segments: Option<u64>,
) -> Result<ResData<LogQueryResult>, String> {
    if !Path::new(&file_path).exists() {
        return Err("文件不存在".to_string());
    }
    let enc = encoding.unwrap_or_else(|| "UTF-8".to_string());
    let mask = levels_to_mask(&levels.unwrap_or_default());
    let invert = invert.unwrap_or(false);
    let gap = merge_gap.unwrap_or(0);
    let max_seg = (max_segments.unwrap_or(DEFAULT_MAX_SEGMENTS as u64) as usize).max(1);

    let start = std::time::Instant::now();
    ensure_index(&file_path, &enc).await?;
    let key = cache_key(&file_path, &enc);

    // 索引遍历是纯内存操作，持锁时间可控（百万行级约数毫秒）
    let (segments, matched, truncated) = {
        let cache = INDEX_CACHE.lock();
        let idx = cache.get(&key).ok_or("索引已失效，请重试")?;
        let mut segments: Vec<LineSegment> = Vec::new();
        let mut matched: u64 = 0;
        let mut truncated = false;
        let mut cur_start: Option<u64> = None;
        let mut cur_end: u64 = 0;

        for (i, &level) in idx.levels.iter().enumerate() {
            let ts = idx.ts[i];
            let hit = level_match(level, mask) && ts_match(ts, ts_from, ts_to);
            let hit = if invert { !hit } else { hit };
            if !hit {
                continue;
            }
            matched += 1;
            let line_no = i as u64 + 1;
            match cur_start {
                None => {
                    cur_start = Some(line_no);
                    cur_end = line_no;
                }
                Some(s) => {
                    if line_no <= cur_end + gap + 1 && segments.len() < max_seg {
                        cur_end = line_no;
                    } else {
                        segments.push(LineSegment { start: s, end: cur_end });
                        if segments.len() >= max_seg {
                            truncated = true;
                            cur_start = None;
                            // 继续统计 matched，但不新增段
                        } else {
                            cur_start = Some(line_no);
                            cur_end = line_no;
                        }
                    }
                }
            }
        }
        if let Some(s) = cur_start {
            if segments.len() < max_seg {
                segments.push(LineSegment { start: s, end: cur_end });
            } else {
                truncated = true;
            }
        }
        (segments, matched, truncated)
    };

    Ok(ResData {
        code: 200,
        msg: format!("命中 {} 行，{} 段", matched, segments.len()),
        data: LogQueryResult {
            segments,
            matched,
            truncated,
            elapsed_ms: start.elapsed().as_millis() as u64,
        },
    })
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
            let info = analyze_line(&line, last_ts, last_level, &mut stack_level);
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
        analyze_line(line, NO_TS, LV_UNKNOWN, &mut stack).level
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
