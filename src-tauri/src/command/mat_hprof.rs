//! # 内存分析（MAT）—— Java 堆转储解析（供前端 mat 插件使用）
//!
//! 本模块是**纯业务层**：不出现 `#[tauri::command]`、不依赖 `tauri` crate，
//! 命令包装统一放在 `commands.rs`（见该文件头部的架构约定）。
//!
//! ## 能力范围（当前批次）
//! - `mat_hprof_header`：识别堆转储文件格式并返回 dump 元信息
//!   （HPROF「JAVA PROFILE 1.0.2」/ PHD / 未知；id 宽度、dump 时间、文件大小）。
//! - `mat_hprof_histogram`：真直方图 —— 按类统计对象数量与**浅堆**（Shallow Heap）。
//!   默认**不限制行数**（返回全部类，配合前端虚拟滚动表格）；显式传 limit 才截断前 N 行。
//!
//! 浅堆口径与 Eclipse MAT 一致：
//! - 普通对象：取所在类的 `instance_size`（HPROF ClassDump 给出，含对象头与全部继承字段）；
//! - 对象数组：数组头（约 16B，64 位 id）+ 元素数 × id 宽度；
//! - 基本类型数组：数组头 + 元素数 × 元素宽度。
//!
//! ## 解析引擎
//! 基于 `jvm-hprof`（零拷贝 mmap 解析，官方 benchmark ~2GB/s，面向数亿对象大 dump）。
//! 遍历两遍：第一遍收集 Utf8 名表 / LoadClass 类名表 / ClassDump 实例大小，
//! 第二遍统计 Instance / 对象数组 / 基本类型数组 —— 避免实例记录先于类记录时缺数据。
//!
//! Android Studio 的 .hprof 同样以「JAVA PROFILE」开头（ART 变体），解析器遇到未知
//! 子记录会 panic（上游实现 `panic!("Unexpected sub-record type")`），因此外层用
//! `catch_unwind` 兜底并转成可读错误，避免拖垮整个后端进程。

use crate::response::resp_data::ResData;
use chrono::{Local, TimeZone};
use memmap2::Mmap;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

// ==================== 返回结构（与前端 mat api 字段一一对应，snake_case） ====================

#[derive(Debug, Serialize)]
pub struct MatHeaderInfo {
    /// 格式名：JAVA PROFILE 1.0.2 / PHD / 其它可打印标识
    pub format: String,
    /// id 宽度（4/8 字节），仅 HPROF 有意义
    pub id_size: u8,
    /// dump 时间（epoch 毫秒，仅 HPROF）
    pub timestamp_ms: i64,
    /// dump 时间（本地时间文本，仅 HPROF）
    pub timestamp_text: String,
    /// 文件大小（字节）
    pub file_size: u64,
    /// 是否可解析（当前仅 JAVA PROFILE 1.0.x 家族可解析）
    pub supported: bool,
    /// 补充说明（不支持时的原因 / 后续指引）
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct MatHistogramRow {
    pub class_name: String,
    /// 对象数量（实例 + 数组）
    pub count: u64,
    /// 浅堆字节数（Shallow Heap）
    pub shallow_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct MatHistogramResult {
    /// 全部有实例的类数（实际行数，rows 仅在显式 limit 时截断）
    pub total_classes: usize,
    /// 总对象数（含所有类）
    pub total_objects: u64,
    /// 总浅堆字节
    pub total_shallow_bytes: u64,
    /// 是否因 limit 截断（默认不限制时为 false）
    pub truncated: bool,
    /// 直方图行（浅堆降序；默认返回全部类）
    pub rows: Vec<MatHistogramRow>,
    pub elapsed_ms: u64,
}

// ==================== 格式识别 ====================

/** HPROF 文件头 magic（HPROF / Android ART 均以此开头，后跟 \0 与版本号） */
const HPROF_MAGIC: &[u8] = b"JAVA PROFILE";

/// 识别堆转储格式：返回 (格式名, 是否 HPROF 家族可解析)
pub fn sniff_format(head: &[u8]) -> (String, bool) {
    if head.starts_with(HPROF_MAGIC) {
        // 取完整 label（第一个 \0 前）
        let end = head.iter().position(|&b| b == 0).unwrap_or(head.len());
        let label = String::from_utf8_lossy(&head[..end]).trim().to_string();
        return (if label.is_empty() { "JAVA PROFILE".into() } else { label }, true);
    }
    // 其它格式：取前 64 字节中最长的连续可打印 run 作标识
    // （IBM PHD 等二进制头部通常含 ASCII 魔数，但可能被非打印字节前缀干扰）
    let bytes = head.iter().take(64);
    let mut best: String = String::new();
    let mut cur: String = String::new();
    for &b in bytes {
        if b.is_ascii_graphic() || b == b' ' {
            cur.push(b as char);
        } else if !cur.is_empty() {
            if cur.len() > best.len() {
                best = cur.clone();
            }
            cur.clear();
        }
    }
    if cur.len() > best.len() {
        best = cur;
    }
    let name = best.trim().to_string();
    (if name.is_empty() { "UNKNOWN".to_string() } else { name }, false)
}

// ==================== Header 命令 ====================

/// 读取堆转储头部元信息（纯同步，调用方需 spawn_blocking 包裹）
pub fn mat_hprof_header(path: &str) -> Result<ResData<MatHeaderInfo>, String> {
    let p = Path::new(path);
    let meta = std::fs::metadata(p).map_err(|e| format!("无法读取文件: {e}"))?;
    let file_size = meta.len();
    let head = read_head(path, 1024)?;
    let (format, supported) = sniff_format(&head);

    if !supported {
        return Ok(ResData {
            code: 200,
            msg: "ok".to_string(),
            data: MatHeaderInfo {
                format,
                id_size: 0,
                timestamp_ms: 0,
                timestamp_text: String::new(),
                file_size,
                supported: false,
                detail: "当前仅支持解析 JAVA PROFILE 1.0.x（HotSpot / OpenJDK / Android Studio 同族格式）；PHD 等格式将在后续版本支持".to_string(),
            },
        });
    }

    // HPROF：用 jvm-hprof 解析头部细节
    let parse = catch_parse(|| parse_header_only(path));
    match parse {
        Ok(Some((id_size, ts_ms))) => {
            let ts_text = if ts_ms > 0 {
                Local
                    .timestamp_millis_opt(ts_ms as i64)
                    .earliest()
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            Ok(ResData {
                code: 200,
                msg: "ok".to_string(),
                data: MatHeaderInfo {
                    format,
                    id_size,
                    timestamp_ms: ts_ms as i64,
                    timestamp_text: ts_text,
                    file_size,
                    supported: true,
                    detail: String::new(),
                },
            })
        }
        Ok(None) => Ok(ResData {
            code: 200,
            msg: "ok".to_string(),
            data: MatHeaderInfo {
                format,
                id_size: 0,
                timestamp_ms: 0,
                timestamp_text: String::new(),
                file_size,
                supported: true,
                detail: "文件以 JAVA PROFILE 开头但头部解析异常，可能为不完整或损坏的转储".to_string(),
            },
        }),
        Err(e) => Err(e),
    }
}

/// 读取文件前 n 字节
pub(crate) fn read_head(path: &str, n: usize) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut f = File::open(path).map_err(|e| format!("无法打开文件: {e}"))?;
    let mut buf = vec![0u8; n];
    let got = f.read(&mut buf).map_err(|e| format!("读取文件失败: {e}"))?;
    buf.truncate(got);
    Ok(buf)
}

/// 用 jvm-hprof 解析 HPROF 头部：返回 (id 宽度, epoch 毫秒)
fn parse_header_only(path: &str) -> Result<Option<(u8, u64)>, String> {
    let f = File::open(path).map_err(|e| format!("无法打开文件: {e}"))?;
    let mm = unsafe { Mmap::map(&f).map_err(|e| format!("内存映射失败: {e}"))? };
    let hp = jvm_hprof::parse_hprof(&mm[..]).map_err(|e| format!("解析失败: {e:?}"))?;
    let h = hp.header();
    Ok(Some((
        match h.id_size() {
            jvm_hprof::IdSize::U32 => 4,
            jvm_hprof::IdSize::U64 => 8,
        },
        h.timestamp_millis(),
    )))
}

// ==================== 直方图命令 ====================

/// 生成按类聚合的浅堆直方图（纯同步，调用方需 spawn_blocking 包裹）
///
/// `limit` 语义：`None` 或 `0` = 不限制（返回全部类行，适合虚拟滚动表格展示）；
/// `Some(n)` = 只返回浅堆最大的前 n 行（预留，前端当前不传）。
pub fn mat_hprof_histogram(
    path: &str,
    limit: Option<usize>,
) -> Result<ResData<MatHistogramResult>, String> {
    let started = std::time::Instant::now();
    let head = read_head(path, 64)?;
    let (_format, supported) = sniff_format(&head);
    if !supported {
        return Err(format!(
            "不支持的堆转储格式（{})，当前仅支持 JAVA PROFILE 1.0.x",
            String::from_utf8_lossy(&head)
        ));
    }

    let result = catch_parse(|| build_histogram(path))?;
    let elapsed_ms = started.elapsed().as_millis() as u64;

    let total_classes = result.len();
    let total_objects: u64 = result.iter().map(|r| r.count).sum();
    let total_shallow: u64 = result.iter().map(|r| r.shallow_bytes).sum();

    let cap = limit.filter(|&l| l > 0); // None/0 = 不限制
    let truncated = cap.is_some_and(|l| total_classes > l);
    let mut rows = result;
    rows.sort_by(|a, b| b.shallow_bytes.cmp(&a.shallow_bytes));
    if let Some(l) = cap {
        rows.truncate(l);
    }

    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: MatHistogramResult {
            total_classes,
            total_objects,
            total_shallow_bytes: total_shallow,
            truncated,
            rows,
            elapsed_ms,
        },
    })
}

/// 解析并聚合直方图：返回 类名 → (count, shallow_bytes) 与各阶段计数
fn build_histogram(path: &str) -> Result<Vec<MatHistogramRow>, String> {
    let f = File::open(path).map_err(|e| format!("无法打开文件: {e}"))?;
    let mm = unsafe { Mmap::map(&f).map_err(|e| format!("内存映射失败: {e}"))? };
    let hp = jvm_hprof::parse_hprof(&mm[..]).map_err(|e| format!("解析失败: {e:?}"))?;
    let id_size = hp.header().id_size();
    let id_width: u64 = match id_size {
        jvm_hprof::IdSize::U32 => 4,
        jvm_hprof::IdSize::U64 => 8,
    };
    // 数组对象头（近似 HotSpot 64 位压缩对象头对齐后的 16B）
    let array_header: u64 = 16;

    // 第一遍：Utf8 名表 + LoadClass 类名 + ClassDump 实例大小
    let mut utf8: HashMap<u64, String> = HashMap::new();
    let mut class_name: HashMap<u64, String> = HashMap::new();
    let mut class_inst_size: HashMap<u64, u64> = HashMap::new();

    for rec in hp.records_iter() {
        let rec = rec.map_err(|e| format!("记录解析失败: {e:?}"))?;
        match rec.tag() {
            jvm_hprof::RecordTag::Utf8 => {
                if let Some(u) = rec.as_utf_8() {
                    let u = u.map_err(|e| format!("Utf8 解析失败: {e:?}"))?;
                    utf8.insert(
                        u.name_id().id(),
                        String::from_utf8_lossy(u.text()).into_owned(),
                    );
                }
            }
            jvm_hprof::RecordTag::LoadClass => {
                if let Some(lc) = rec.as_load_class() {
                    let lc = lc.map_err(|e| format!("LoadClass 解析失败: {e:?}"))?;
                    let name = utf8
                        .get(&lc.class_name_id().id())
                        .cloned()
                        .unwrap_or_default();
                    class_name.insert(lc.class_obj_id().id(), name);
                }
            }
            jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment => {
                if let Some(seg) = rec.as_heap_dump_segment() {
                    let seg = seg.map_err(|e| format!("HeapDump 解析失败: {e:?}"))?;
                    for sr in seg.sub_records() {
                        if let Ok(jvm_hprof::heap_dump::SubRecord::Class(c)) = sr {
                            class_inst_size.insert(c.obj_id().id(), c.instance_size_bytes() as u64);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // 第二遍：统计 Instance / 对象数组 / 基本类型数组
    let mut hist: HashMap<String, MatHistogramRow> = HashMap::new();
    let mut add = |name: String, count: u64, shallow: u64| {
        let e = hist.entry(name.clone()).or_insert_with(|| MatHistogramRow {
            class_name: name.clone(),
            count: 0,
            shallow_bytes: 0,
        });
        e.count += count;
        e.shallow_bytes += shallow;
    };

    for rec in hp.records_iter() {
        let rec = rec.map_err(|e| format!("记录解析失败: {e:?}"))?;
        if !matches!(
            rec.tag(),
            jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment
        ) {
            continue;
        }
        if let Some(seg) = rec.as_heap_dump_segment() {
            let seg = seg.map_err(|e| format!("HeapDump 解析失败: {e:?}"))?;
            for sr in seg.sub_records() {
                let sr = sr.map_err(|e| format!("子记录解析失败: {e:?}"))?;
                match sr {
                    jvm_hprof::heap_dump::SubRecord::Instance(d) => {
                        let cid = d.class_obj_id().id();
                        let shallow = class_inst_size.get(&cid).copied().unwrap_or(0);
                        let name = class_name
                            .get(&cid)
                            .cloned()
                            .unwrap_or_else(|| format!("#{:x}", cid));
                        add(name, 1, shallow);
                    }
                    jvm_hprof::heap_dump::SubRecord::ObjectArray(a) => {
                        let cid = a.array_class_obj_id().id();
                        let n = a.elements(id_size).count() as u64;
                        let shallow = array_header + n * id_width;
                        let name = class_name
                            .get(&cid)
                            .cloned()
                            .unwrap_or_else(|| format!("[L#{:x}", cid));
                        add(name, 1, shallow);
                    }
                    jvm_hprof::heap_dump::SubRecord::PrimitiveArray(p) => {
                        let (name, elem_w) = primitive_array_meta(p.primitive_type());
                        let n = count_primitive_elements(&p);
                        let shallow = array_header + n * elem_w;
                        add(name.to_string(), 1, shallow);
                    }
                    _ => {}
                }
            }
        }
    }

    let rows: Vec<MatHistogramRow> = hist.into_values().collect();
    Ok(rows)
}

/// 轻量对象计数：单遍扫描堆段，只统计会建为图节点的对象数
///（实例 + 对象数组 + 基本类型数组 + 类对象），不解析字段 / 元素 / 边、不建索引，
/// 内存近零。用于「超限预检」：对象总数超分析上限时直接截断返回，避免大 dump
/// 白跑一次 GB 级全量建图（解析峰值大头是 mmap 文件页 + 节点表 + id→索引哈希）。
pub(crate) fn count_heap_objects(path: &str) -> Result<u64, String> {
    let f = File::open(path).map_err(|e| format!("无法打开文件: {e}"))?;
    let mm = unsafe { Mmap::map(&f).map_err(|e| format!("内存映射失败: {e}"))? };
    let hp = jvm_hprof::parse_hprof(&mm[..]).map_err(|e| format!("解析失败: {e:?}"))?;
    let mut n: u64 = 0;
    for rec in hp.records_iter() {
        let rec = rec.map_err(|e| format!("记录解析失败: {e:?}"))?;
        if !matches!(
            rec.tag(),
            jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment
        ) {
            continue;
        }
        if let Some(seg) = rec.as_heap_dump_segment() {
            let seg = seg.map_err(|e| format!("HeapDump 解析失败: {e:?}"))?;
            for sr in seg.sub_records() {
                let sr = sr.map_err(|e| format!("子记录解析失败: {e:?}"))?;
                match sr {
                    jvm_hprof::heap_dump::SubRecord::Class(_)
                    | jvm_hprof::heap_dump::SubRecord::Instance(_)
                    | jvm_hprof::heap_dump::SubRecord::ObjectArray(_)
                    | jvm_hprof::heap_dump::SubRecord::PrimitiveArray(_) => n += 1,
                    _ => {}
                }
            }
        }
    }
    Ok(n)
}

/// 基本类型数组：类名（byte[] 风格，与 MAT 一致）+ 元素宽度
pub(crate) fn primitive_array_meta(t: jvm_hprof::heap_dump::PrimitiveArrayType) -> (&'static str, u64) {
    use jvm_hprof::heap_dump::PrimitiveArrayType as T;
    match t {
        T::Boolean => ("boolean[]", 1),
        T::Byte => ("byte[]", 1),
        T::Char => ("char[]", 2),
        T::Short => ("short[]", 2),
        T::Int => ("int[]", 4),
        T::Float => ("float[]", 4),
        T::Long => ("long[]", 8),
        T::Double => ("double[]", 8),
    }
}

/// 计算基本类型数组元素个数（PrimitiveArray 未暴露长度字段，通过迭代器计数）
pub(crate) fn count_primitive_elements(p: &jvm_hprof::heap_dump::PrimitiveArray) -> u64 {
    use jvm_hprof::heap_dump::PrimitiveArrayType as T;
    (match p.primitive_type() {
        T::Byte => p.bytes().map(|it| it.count()),
        T::Boolean => p.booleans().map(|it| it.count()),
        T::Char => p.chars().map(|it| it.count()),
        T::Short => p.shorts().map(|it| it.count()),
        T::Int => p.ints().map(|it| it.count()),
        T::Float => p.floats().map(|it| it.count()),
        T::Long => p.longs().map(|it| it.count()),
        T::Double => p.doubles().map(|it| it.count()),
    })
    .unwrap_or(0) as u64
}

// ==================== 兜底 ====================

/// jvm-hprof 对未知子记录会 panic（如 Android ART 变体），用 catch_unwind 兜底成错误
pub(crate) fn catch_parse<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .map_err(|_| "该转储包含当前解析器无法识别的记录（可能来自 Android/ART 或更新 JDK），暂不支持".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- 小工具：向 Vec<u8> 追加大端整数 ----
    fn u1(v: &mut Vec<u8>, x: u8) {
        v.push(x);
    }
    fn u2(v: &mut Vec<u8>, x: u16) {
        v.extend_from_slice(&x.to_be_bytes());
    }
    fn u4(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_be_bytes());
    }
    fn u8(v: &mut Vec<u8>, x: u64) {
        v.extend_from_slice(&x.to_be_bytes());
    }
    /// id 恒 8 字节
    fn id(v: &mut Vec<u8>, x: u64) {
        u8(v, x);
    }
    /// 写一条 record：tag + u4 微秒时间戳 + u4 长度 + payload
    fn rec(v: &mut Vec<u8>, tag: u8, payload: &[u8]) {
        u1(v, tag);
        u4(v, 0); // record time（微秒，构造为 0）
        u4(v, payload.len() as u32);
        v.extend_from_slice(payload);
    }

    /// 构造最小 HPROF 文件（JAVA PROFILE 1.0.2 / 8 字节 id / ts=1788834606862）
    fn build_min_hprof() -> Vec<u8> {
        let mut f: Vec<u8> = Vec::new();
        // header: label\0 + u4 id_size + u8 timestamp
        f.extend_from_slice(b"JAVA PROFILE 1.0.2\0");
        u4(&mut f, 8);
        u8(&mut f, 1_788_834_606_862);
        // ---- UTF8 名表 ----
        // utf8 id=1 -> java/lang/String
        let mut p = Vec::new();
        id(&mut p, 1);
        p.extend_from_slice(b"java/lang/String");
        rec(&mut f, 0x01, &p);
        // utf8 id=2 -> [Ljava/lang/String;
        let mut p = Vec::new();
        id(&mut p, 2);
        p.extend_from_slice(b"[Ljava/lang/String;");
        rec(&mut f, 0x01, &p);
        // ---- LOAD_CLASS ----
        // class_obj 0x100 -> utf8#1 (java/lang/String)
        let mut p = Vec::new();
        u4(&mut p, 1); // class serial
        id(&mut p, 0x100); // class obj id
        u4(&mut p, 0); // stack trace serial
        id(&mut p, 1); // name id
        rec(&mut f, 0x02, &p);
        // class_obj 0x300 -> utf8#2 ([Ljava/lang/String;)
        let mut p = Vec::new();
        u4(&mut p, 2);
        id(&mut p, 0x300);
        u4(&mut p, 0);
        id(&mut p, 2);
        rec(&mut f, 0x02, &p);
        // ---- HEAP_DUMP ----
        let mut dump = Vec::new();
        // class dump: java/lang/String instance size = 24
        u1(&mut dump, 0x20);
        id(&mut dump, 0x100); // class obj
        u4(&mut dump, 0); // trace
        id(&mut dump, 0); // super (null)
        id(&mut dump, 0); // loader (null)
        id(&mut dump, 0); // signers (null)
        id(&mut dump, 0); // protection domain (null)
        id(&mut dump, 0); // reserved
        id(&mut dump, 0); // reserved
        u4(&mut dump, 24); // instance_size
        u2(&mut dump, 0); // constant pool len = 0
        u2(&mut dump, 0); // static fields = 0
        u2(&mut dump, 0); // instance fields = 0
        // 2 个 String 实例：字段字节各 16（instance_size 24 - 头 8，内容随意）
        u1(&mut dump, 0x21);
        id(&mut dump, 0x201);
        u4(&mut dump, 0);
        id(&mut dump, 0x100);
        u4(&mut dump, 16);
        dump.extend_from_slice(&[0u8; 16]);
        u1(&mut dump, 0x21);
        id(&mut dump, 0x202);
        u4(&mut dump, 0);
        id(&mut dump, 0x100);
        u4(&mut dump, 16);
        dump.extend_from_slice(&[0u8; 16]);
        // 对象数组 [Ljava/lang/String; n=2
        u1(&mut dump, 0x22);
        id(&mut dump, 0x401);
        u4(&mut dump, 0);
        u4(&mut dump, 2); // n elements
        id(&mut dump, 0x300); // array class
        id(&mut dump, 0x201);
        id(&mut dump, 0x202);
        // 基本类型数组 byte[] n=4 type=0x08
        u1(&mut dump, 0x23);
        id(&mut dump, 0x501);
        u4(&mut dump, 0);
        u4(&mut dump, 4);
        u1(&mut dump, 0x08);
        dump.extend_from_slice(&[1u8, 2, 3, 4]);
        rec(&mut f, 0x0C, &dump); // HEAP_DUMP
        f
    }

    #[test]
    fn header_sniff_and_parse() {
        let bytes = build_min_hprof();
        let dir = std::env::temp_dir().join("mat_unit_test.hprof");
        std::fs::write(&dir, &bytes).unwrap();
        let head = read_head(dir.to_str().unwrap(), 64).unwrap();
        let (format, supported) = sniff_format(&head);
        assert!(supported);
        assert!(format.starts_with("JAVA PROFILE"));
        let res = mat_hprof_header(dir.to_str().unwrap()).unwrap();
        assert_eq!(res.code, 200);
        assert_eq!(res.data.id_size, 8);
        assert_eq!(res.data.file_size, bytes.len() as u64);
        assert_eq!(res.data.timestamp_ms, 1_788_834_606_862);
        assert!(!res.data.timestamp_text.is_empty());
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn histogram_counts_and_shallow() {
        let bytes = build_min_hprof();
        let dir = std::env::temp_dir().join("mat_unit_test_hist.hprof");
        std::fs::write(&dir, &bytes).unwrap();
        let res = mat_hprof_histogram(dir.to_str().unwrap(), Some(100)).unwrap();
        assert_eq!(res.code, 200);
        let rows = &res.data.rows;
        assert_eq!(res.data.total_objects, 4); // 2 String + 1 obj array + 1 byte[]
        assert_eq!(res.data.total_classes, 3);
        assert_eq!(res.data.total_shallow_bytes, 24 * 2 + (16 + 2 * 8) + (16 + 4 * 1));
        assert!(!res.data.truncated);
        // java/lang/String: count=2 shallow=48
        let s = rows.iter().find(|r| r.class_name == "java/lang/String").unwrap();
        assert_eq!(s.count, 2);
        assert_eq!(s.shallow_bytes, 48);
        // [Ljava/lang/String; count=1 shallow=32
        let arr = rows.iter().find(|r| r.class_name == "[Ljava/lang/String;").unwrap();
        assert_eq!(arr.count, 1);
        assert_eq!(arr.shallow_bytes, 32);
        // byte[] count=1 shallow=20
        let b = rows.iter().find(|r| r.class_name == "byte[]").unwrap();
        assert_eq!(b.count, 1);
        assert_eq!(b.shallow_bytes, 20);
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn histogram_no_limit_returns_all_rows() {
        let bytes = build_min_hprof();
        let dir = std::env::temp_dir().join("mat_unit_test_hist_all.hprof");
        std::fs::write(&dir, &bytes).unwrap();
        // 不传 limit（None）= 全量返回，truncated 恒 false
        let res = mat_hprof_histogram(dir.to_str().unwrap(), None).unwrap();
        assert_eq!(res.code, 200);
        assert_eq!(res.data.rows.len(), res.data.total_classes); // 3 类全部返回
        assert!(!res.data.truncated);
        // 与显式大 limit 结果一致
        let capped = mat_hprof_histogram(dir.to_str().unwrap(), Some(3)).unwrap();
        assert_eq!(capped.data.rows.len(), res.data.rows.len());
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn unknown_format_rejected() {
        let dir = std::env::temp_dir().join("mat_unit_test_unknown.bin");
        // PNG 魔数（\x89PNG...）是典型的非堆转储二进制
        std::fs::write(&dir, b"\x89PNG\r\n\x1a\nnot-a-java-heap-dump-at-all-just-printable-noise").unwrap();
        let head = read_head(dir.to_str().unwrap(), 16).unwrap();
        let (name, supported) = sniff_format(&head);
        assert!(!supported);
        assert!(!name.is_empty() && name != "UNKNOWN");
        let res = mat_hprof_header(dir.to_str().unwrap()).unwrap();
        assert_eq!(res.code, 200);
        assert!(!res.data.supported);
        assert!(mat_hprof_histogram(dir.to_str().unwrap(), None).is_err());
        let _ = std::fs::remove_file(&dir);
    }
}
