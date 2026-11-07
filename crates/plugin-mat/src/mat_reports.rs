//! # 专用报告（线程 / 类加载器 / 内存浪费 / 不可达对象）
//!
//! 对应 Eclipse MAT 的 Java Basics 报告族。四个报告的数据来源不同，这里按来源分两类：
//!
//! - **需要值扫描**（线程名、集合填充率、重复字符串）：沿用 `mat_values` 的「按需扫描」
//!   策略，临时 mmap、扫 2~3 遍，函数返回即释放。不常驻内存，也不持有文件句柄。
//! - **复用会话图**（不可达对象直方图）：直接用 `mat_refs` 的会话（可达标记 + 类索引），
//!   与 Leak Suspects 的不可达计数天然一致。
//!
//! ## 明确的能力边界（前端必须如实展示）
//! - `jvm-hprof 0.1` 不解析 `START_THREAD` / `STACK_FRAME` 子记录，因此**拿不到调用栈**：
//!   线程报告只有「线程对象 + 名称 + 状态位 + 浅堆」，没有帧与局部变量。
//! - 类加载器维度只到「loader 对象 id + 加载的类数 + 同名类重复」，不解析 loader 的
//!   父子委派链（那需要读 `parent` 字段并逐层展开，属于 P3 索引持久化后的工作）。
//! - 浪费报告对 String 与集合做**采样**（上限 `mat_cfg::QueryLimits::waste_sample`），
//!   采样时 `note` 会写明，避免用户把采样值当精确值。

use crate::mat_cfg;
use crate::mat_graph::field_type_tag;
use crate::mat_hprof::{catch_parse, read_head, sniff_format};
use crate::mat_values::{chain_of, parse_fields, ClassInfo, MAX_STRING_CHARS};
use bdevtools_core::response::resp_data::ResData;
use jvm_hprof::heap_dump::SubRecord;
use memmap2::Mmap;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

// ==================== 返回结构 ====================

#[derive(Debug, Serialize)]
pub struct ThreadRow {
    pub obj_id: String,
    pub class_name: String,
    /// 线程名（未能解析时为 `(未命名)`）
    pub name: String,
    /// 名称字段指向的 String 对象 id（可继续用对象值面板查看）
    pub name_obj_id: String,
    pub shallow_bytes: u64,
    /// JVM 内部 `threadStatus` 位标记（不做 Thread.State 映射，避免给出错误语义）
    pub thread_status: i64,
    /// 是否被 `GcRootThreadObj` 标记为线程根
    pub is_root: bool,
}

#[derive(Debug, Serialize)]
pub struct ThreadsResult {
    pub total: u64,
    /// `GcRootThreadObj` 记录的线程根数量（可能 > total，含已结束线程）
    pub thread_roots: u64,
    pub rows: Vec<ThreadRow>,
    pub note: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct LoaderRow {
    pub loader_obj_id: String,
    /// loader 实例的类名（bootstrap loader 为空串）
    pub loader_class: String,
    pub classes: u64,
    pub duplicate_classes: u64,
    /// 该 loader 加载的类名样例（最多 8 个）
    pub sample: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DuplicateClassRow {
    /// 简单类名（不含包）
    pub simple_name: String,
    /// 完整类名 → 加载它的 loader 对象 id
    pub occurrences: Vec<String>,
    pub loaders: u64,
}

#[derive(Debug, Serialize)]
pub struct ClassLoadersResult {
    pub total_loaders: u64,
    pub total_classes: u64,
    pub rows: Vec<LoaderRow>,
    pub duplicates: Vec<DuplicateClassRow>,
    pub note: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct DupStringRow {
    pub value: String,
    pub count: u64,
    /// 该值的 String 对象浅堆合计
    pub shallow_bytes: u64,
    /// 可回收字节（= (count − 1) × 单个浅堆）
    pub wasted_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct CollectionFillRow {
    pub class_name: String,
    pub instances: u64,
    /// size == 0 的实例数
    pub empty_instances: u64,
    /// 平均填充率（size / 底层数组长度；无表实例不计入）
    pub avg_fill: f64,
    /// 最大实例 size
    pub max_size: u64,
    /// 浅堆合计（这些集合对象自身）
    pub shallow_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct WasteResult {
    pub duplicate_strings: Vec<DupStringRow>,
    /// 重复字符串浪费字节合计
    pub duplicate_wasted_bytes: u64,
    pub duplicate_groups: u64,
    pub collections: Vec<CollectionFillRow>,
    pub empty_collections: u64,
    /// 采样是否触顶（结果只是下界）
    pub sampled: bool,
    pub strings_scanned: u64,
    pub note: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct UnreachableRow {
    pub class_name: String,
    pub count: u64,
    pub shallow_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct UnreachableResult {
    pub total_objects: u64,
    pub total_shallow: u64,
    /// 可达对象数（用于与泄漏检测的 summary 对账）
    pub reachable_nodes: u64,
    pub rows: Vec<UnreachableRow>,
    pub note: String,
    pub elapsed_ms: u64,
}

// ==================== 公共准备 ====================

/// 打开并校验格式（只做 mmap；解析在各报告里就地做，避免自引用结构）
fn open(path: &str) -> Result<Mmap, String> {
    let head = read_head(path, 64)?;
    let (_f, supported) = sniff_format(&head);
    if !supported {
        return Err("不支持的堆转储格式（当前仅支持 JAVA PROFILE 1.0.x）".to_string());
    }
    crate::mat_source::map_file(path)
}

fn parse_mm<'a>(mm: &'a [u8]) -> Result<jvm_hprof::Hprof<'a>, String> {
    // ART 等变体的未知子记录会让解析器 panic，统一兜底
    catch_parse(|| jvm_hprof::parse_hprof(mm).map_err(|e| format!("解析失败: {e:?}")))
}

fn id_width(hp: &jvm_hprof::Hprof<'_>) -> usize {
    match hp.header().id_size() {
        jvm_hprof::IdSize::U32 => 4,
        jvm_hprof::IdSize::U64 => 8,
    }
}

/// 两遍扫描：字符串表 + LoadClass，再取类元数据（字段 / 父类 / loader）
fn load_classes(hp: &jvm_hprof::Hprof<'_>) -> (HashMap<u64, ClassInfo>, HashMap<u64, String>) {
    let mut utf8: HashMap<u64, String> = HashMap::new();
    let mut names: HashMap<u64, String> = HashMap::new();
    for rec in hp.records_iter() {
        let Ok(rec) = rec else { continue };
        match rec.tag() {
            jvm_hprof::RecordTag::Utf8 => {
                if let Some(Ok(u)) = rec.as_utf_8() {
                    utf8.insert(u.name_id().id(), String::from_utf8_lossy(u.text()).into_owned());
                }
            }
            jvm_hprof::RecordTag::LoadClass => {
                if let Some(Ok(lc)) = rec.as_load_class() {
                    names.insert(
                        lc.class_obj_id().id(),
                        utf8.get(&lc.class_name_id().id()).cloned().unwrap_or_default(),
                    );
                }
            }
            _ => {}
        }
    }
    let mut classes: HashMap<u64, ClassInfo> = HashMap::new();
    for rec in hp.records_iter() {
        let Ok(rec) = rec else { continue };
        if !matches!(
            rec.tag(),
            jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment
        ) {
            continue;
        }
        let Some(Ok(seg)) = rec.as_heap_dump_segment() else { continue };
        for sr in seg.sub_records() {
            let Ok(SubRecord::Class(c)) = sr else { continue };
            let cid = c.obj_id().id();
            let mut fields = Vec::new();
            for fd in c.instance_field_descriptors() {
                let Ok(fd) = fd else { continue };
                let nm = utf8
                    .get(&fd.name_id().id())
                    .cloned()
                    .unwrap_or_else(|| format!("#{:x}", fd.name_id().id()));
                fields.push((field_type_tag(fd.field_type()), nm));
            }
            classes.insert(
                cid,
                ClassInfo {
                    name: names.get(&cid).cloned().unwrap_or_default(),
                    super_id: c.super_class_obj_id().map(|i| i.id()),
                    loader_id: c.class_loader_obj_id().map(|i| i.id()),
                    instance_size: c.instance_size_bytes() as u64,
                    fields,
                    statics: Vec::new(),
                },
            );
        }
    }
    (classes, names)
}

fn leaf(name: &str) -> &str {
    name.rsplit(['/', '.']).next().unwrap_or(name)
}

/// 类继承链上是否有叶子名等于 `want` 的类
fn chain_has(classes: &HashMap<u64, ClassInfo>, cid: u64, want: &str) -> bool {
    let mut cur = Some(cid);
    let mut guard = 0;
    while let Some(id) = cur {
        guard += 1;
        if guard > 64 {
            return false;
        }
        match classes.get(&id) {
            Some(ci) => {
                if leaf(&ci.name).eq_ignore_ascii_case(want) {
                    return true;
                }
                cur = ci.super_id;
            }
            None => return false,
        }
    }
    false
}

fn field_i64(fields: &[crate::mat_values::ValueField], name: &str) -> Option<i64> {
    fields.iter().find(|f| f.name == name).and_then(|f| f.value.parse::<i64>().ok())
}

fn field_ref(fields: &[crate::mat_values::ValueField], name: &str) -> Option<u64> {
    fields
        .iter()
        .find(|f| f.name == name && !f.target_obj_id.is_empty())
        .and_then(|f| u64::from_str_radix(f.target_obj_id.trim_start_matches("0x"), 16).ok())
}

fn truncate_chars(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n).collect::<String>() + "…"
    }
}

// ==================== 线程报告 ====================

pub fn mat_threads(path: &str, limit: usize) -> Result<ResData<ThreadsResult>, String> {
    let started = std::time::Instant::now();
    let mm = open(path)?;
    let hp = parse_mm(&mm[..])?;
    let idw = id_width(&hp);
    // 浅堆口径与直方图 / 支配树同源（mat_layout）
    let layout = crate::mat_layout::HeapLayout::from_id_width(idw as u64);
    let (classes, _names) = load_classes(&hp);
    let limits = mat_cfg::query_limits();
    let page = limit.clamp(1, limits.report_rows);

    // ---- 线程根（GcRootThreadObj）----
    let mut root_ids: HashSet<u64> = HashSet::new();
    let mut thread_roots: u64 = 0;
    for rec in hp.records_iter() {
        let Ok(rec) = rec else { continue };
        if !matches!(
            rec.tag(),
            jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment
        ) {
            continue;
        }
        let Some(Ok(seg)) = rec.as_heap_dump_segment() else { continue };
        for sr in seg.sub_records() {
            if let Ok(SubRecord::GcRootThreadObj(r)) = sr {
                thread_roots += 1;
                if let Some(id) = r.thread_obj_id() {
                    root_ids.insert(id.id());
                }
            }
        }
    }

    // ---- Pass：Thread 实例（名称 / 状态）----
    struct Raw {
        obj_id: u64,
        class_name: String,
        name_obj: u64,
        shallow: u64,
        status: i64,
    }
    let mut raws: Vec<Raw> = Vec::new();
    for rec in hp.records_iter() {
        let Ok(rec) = rec else { continue };
        if !matches!(
            rec.tag(),
            jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment
        ) {
            continue;
        }
        let Some(Ok(seg)) = rec.as_heap_dump_segment() else { continue };
        for sr in seg.sub_records() {
            let Ok(SubRecord::Instance(inst)) = sr else { continue };
            let cid = inst.class_obj_id().id();
            if !chain_has(&classes, cid, "Thread") {
                continue;
            }
            let chain = match chain_of(cid, &classes) {
                Some(v) => v,
                None => continue,
            };
            let fields = parse_fields(inst.fields(), &chain, idw, 512).0;
            raws.push(Raw {
                obj_id: inst.obj_id().id(),
                class_name: classes.get(&cid).map(|c| c.name.clone()).unwrap_or_default(),
                name_obj: field_ref(&fields, "name").unwrap_or(0),
                shallow: layout.instance_bytes(
                    classes.get(&cid).map(|c| c.instance_size).unwrap_or(0),
                ),
                status: field_i64(&fields, "threadStatus").unwrap_or(-1),
            });
        }
    }

    // ---- 线程名：String 实例 → value 数组 → 文本 ----
    let want_strings: HashSet<u64> = raws.iter().map(|r| r.name_obj).filter(|v| *v != 0).collect();
    let texts = string_texts(&hp, idw, &classes, &want_strings);

    raws.sort_by(|a, b| b.shallow.cmp(&a.shallow).then(a.obj_id.cmp(&b.obj_id)));
    let total = raws.len() as u64;
    let rows: Vec<ThreadRow> = raws
        .into_iter()
        .take(page)
        .map(|r| {
            let name = texts
                .get(&r.name_obj)
                .cloned()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("(未命名线程 0x{:x})", r.obj_id));
            ThreadRow {
                obj_id: format!("0x{:x}", r.obj_id),
                class_name: r.class_name,
                name,
                name_obj_id: if r.name_obj == 0 {
                    String::new()
                } else {
                    format!("0x{:x}", r.name_obj)
                },
                shallow_bytes: r.shallow,
                thread_status: r.status,
                is_root: root_ids.contains(&r.obj_id),
            }
        })
        .collect();

    let mut note = String::new();
    if total > page as u64 {
        note.push_str(&format!("共 {total} 个线程对象，仅列出前 {page} 个"));
    }
    note.push_str(if note.is_empty() { "" } else { "；" });
    note.push_str(
        "当前解析器不支持 START_THREAD / STACK_FRAME 子记录，因此没有调用栈与局部变量；\
         `threadStatus` 是 JVM 内部位标记，未映射成 Thread.State",
    );

    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: ThreadsResult {
            total,
            thread_roots,
            rows,
            note,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    })
}

/// 给定一批 String 对象 id，返回 obj_id → 文本（最多 3 遍扫描，内存只存命中项）
fn string_texts(
    hp: &jvm_hprof::Hprof<'_>,
    idw: usize,
    classes: &HashMap<u64, ClassInfo>,
    wanted: &HashSet<u64>,
) -> HashMap<u64, String> {
    if wanted.is_empty() {
        return HashMap::new();
    }
    // ① String 实例 → (value 数组 id, 是否 UTF-16)
    let mut arr_map: HashMap<u64, (u64, bool)> = HashMap::new();
    for rec in hp.records_iter() {
        let Ok(rec) = rec else { continue };
        if !matches!(
            rec.tag(),
            jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment
        ) {
            continue;
        }
        let Some(Ok(seg)) = rec.as_heap_dump_segment() else { continue };
        for sr in seg.sub_records() {
            let Ok(SubRecord::Instance(inst)) = sr else { continue };
            if !wanted.contains(&inst.obj_id().id()) {
                continue;
            }
            let cid = inst.class_obj_id().id();
            let chain = match chain_of(cid, classes) {
                Some(v) => v,
                None => continue,
            };
            let fields = parse_fields(inst.fields(), &chain, idw, 64).0;
            if let Some(vid) = field_ref(&fields, "value") {
                let utf16 = field_i64(&fields, "coder").map(|c| c == 1).unwrap_or(false);
                arr_map.insert(inst.obj_id().id(), (vid, utf16));
            }
        }
    }
    if arr_map.is_empty() {
        return HashMap::new();
    }
    let want_arrays: HashSet<u64> = arr_map.values().map(|(v, _)| *v).collect();
    let mut arr_text: HashMap<u64, String> = HashMap::new();
    // ② 数组 → 文本
    for rec in hp.records_iter() {
        let Ok(rec) = rec else { continue };
        if !matches!(
            rec.tag(),
            jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment
        ) {
            continue;
        }
        let Some(Ok(seg)) = rec.as_heap_dump_segment() else { continue };
        for sr in seg.sub_records() {
            let Ok(SubRecord::PrimitiveArray(p)) = sr else { continue };
            if !want_arrays.contains(&p.obj_id().id()) {
                continue;
            }
            use jvm_hprof::heap_dump::PrimitiveArrayType as T;
            let text = match p.primitive_type() {
                T::Char => {
                    let cs: Vec<u16> = p
                        .chars()
                        .into_iter()
                        .flatten()
                        .filter_map(|r| r.ok())
                        .take(MAX_STRING_CHARS)
                        .collect();
                    String::from_utf16_lossy(&cs)
                }
                T::Byte => {
                    let bs: Vec<u8> = p
                        .bytes()
                        .into_iter()
                        .flatten()
                        .filter_map(|r| r.ok())
                        .map(|b| b as u8)
                        .take(MAX_STRING_CHARS)
                        .collect();
                    let utf16 = arr_map.iter().any(|(_, (a, u))| *a == p.obj_id().id() && *u);
                    if utf16 {
                        let cs: Vec<u16> = bs
                            .chunks_exact(2)
                            .map(|c| u16::from_be_bytes([c[0], c[1]]))
                            .collect();
                        String::from_utf16_lossy(&cs)
                    } else {
                        bs.iter().map(|&b| b as char).collect()
                    }
                }
                _ => continue,
            };
            arr_text.insert(p.obj_id().id(), text);
        }
    }
    // ③ 组装
    let mut out: HashMap<u64, String> = HashMap::new();
    for (sid, (vid, _)) in arr_map {
        if let Some(t) = arr_text.get(&vid) {
            out.insert(sid, t.clone());
        }
    }
    out
}

// ==================== 类加载器报告 ====================

pub fn mat_classloaders(path: &str, limit: usize) -> Result<ResData<ClassLoadersResult>, String> {
    let started = std::time::Instant::now();
    let mm = open(path)?;
    let hp = parse_mm(&mm[..])?;
    let (classes, _names) = load_classes(&hp);
    let limits = mat_cfg::query_limits();
    let page = limit.clamp(1, limits.report_rows);

    // loader id → 类名列表
    let mut by_loader: HashMap<u64, Vec<String>> = HashMap::new();
    let mut total_classes: u64 = 0;
    for ci in classes.values() {
        total_classes += 1;
        if let Some(lid) = ci.loader_id {
            by_loader.entry(lid).or_default().push(ci.name.clone());
        }
    }
    // 同名类（简单名相同）跨 loader 复用 → duplicate classes
    let mut simple_map: HashMap<String, Vec<(String, u64)>> = HashMap::new();
    for (lid, names) in by_loader.iter() {
        for n in names {
            simple_map
                .entry(leaf(n).to_string())
                .or_default()
                .push((n.clone(), *lid));
        }
    }
    let mut duplicates: Vec<DuplicateClassRow> = simple_map
        .into_iter()
        .filter(|(_, v)| {
            let mut loaders: Vec<u64> = v.iter().map(|(_, l)| *l).collect();
            loaders.sort_unstable();
            loaders.dedup();
            loaders.len() > 1
        })
        .map(|(simple, v)| {
            let mut loaders: Vec<u64> = v.iter().map(|(_, l)| *l).collect();
            loaders.sort_unstable();
            loaders.dedup();
            DuplicateClassRow {
                simple_name: simple,
                occurrences: v.iter().map(|(n, l)| format!("{n} @0x{l:x}")).collect(),
                loaders: loaders.len() as u64,
            }
        })
        .collect();
    duplicates.sort_by(|a, b| b.loaders.cmp(&a.loaders).then(a.simple_name.cmp(&b.simple_name)));
    duplicates.truncate(page);

    // loader 自身的类名（扫一遍实例，只为命中 loader id 的对象）
    let loader_ids: HashSet<u64> = by_loader.keys().copied().collect();
    let mut loader_class: HashMap<u64, String> = HashMap::new();
    if !loader_ids.is_empty() {
        for rec in hp.records_iter() {
            let Ok(rec) = rec else { continue };
            if !matches!(
                rec.tag(),
                jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment
            ) {
                continue;
            }
            let Some(Ok(seg)) = rec.as_heap_dump_segment() else { continue };
            for sr in seg.sub_records() {
                let Ok(SubRecord::Instance(inst)) = sr else { continue };
                let oid = inst.obj_id().id();
                if loader_ids.contains(&oid) {
                    loader_class.insert(
                        oid,
                        classes
                            .get(&inst.class_obj_id().id())
                            .map(|c| c.name.clone())
                            .unwrap_or_default(),
                    );
                }
            }
        }
    }

    let dup_by_loader: HashMap<u64, u64> = {
        let mut m: HashMap<u64, u64> = HashMap::new();
        for dd in duplicates.iter() {
            for occ in dd.occurrences.iter() {
                if let Some((_, hex)) = occ.rsplit_once("@0x") {
                    if let Ok(id) = u64::from_str_radix(hex, 16) {
                        *m.entry(id).or_insert(0) += 1;
                    }
                }
            }
        }
        m
    };

    let mut rows: Vec<LoaderRow> = by_loader
        .iter()
        .map(|(lid, names)| LoaderRow {
            loader_obj_id: format!("0x{lid:x}"),
            loader_class: loader_class.get(lid).cloned().unwrap_or_default(),
            classes: names.len() as u64,
            duplicate_classes: dup_by_loader.get(lid).copied().unwrap_or(0),
            sample: names.iter().take(8).cloned().collect(),
        })
        .collect();
    rows.sort_by(|a, b| b.classes.cmp(&a.classes));
    let total_loaders = rows.len() as u64;
    rows.truncate(page);

    let note = if duplicates.is_empty() {
        "未发现同名类被多个类加载器加载".to_string()
    } else {
        format!(
            "发现 {} 个简单类名存在多个加载器（duplicate classes），常见于热部署 / 多 ClassLoader 容器",
            duplicates.len()
        )
    };

    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: ClassLoadersResult {
            total_loaders,
            total_classes,
            rows,
            duplicates,
            note,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    })
}

// ==================== 内存浪费报告 ====================

const COLLECTION_LEAVES: [&str; 8] = [
    "HashMap",
    "LinkedHashMap",
    "ConcurrentHashMap",
    "Hashtable",
    "HashSet",
    "LinkedHashSet",
    "ArrayList",
    "Vector",
];

pub fn mat_waste_report(path: &str, limit: usize) -> Result<ResData<WasteResult>, String> {
    let started = std::time::Instant::now();
    let mm = open(path)?;
    let hp = parse_mm(&mm[..])?;
    let idw = id_width(&hp);
    // 浅堆口径与直方图 / 支配树同源（mat_layout）
    let layout = crate::mat_layout::HeapLayout::from_id_width(idw as u64);
    let (classes, _names) = load_classes(&hp);
    let limits = mat_cfg::query_limits();
    let page = limit.clamp(1, limits.report_rows);
    let sample_cap = limits.waste_sample;

    // ---- Pass A：String 实例 + 集合实例 ----
    struct StrRaw {
        obj_id: u64,
        shallow: u64,
    }
    struct CollRaw {
        class_name: String,
        size: u64,
        arr: u64,
        shallow: u64,
    }
    let mut strings: Vec<StrRaw> = Vec::new();
    let mut colls: Vec<CollRaw> = Vec::new();
    let mut sampled = false;
    for rec in hp.records_iter() {
        let Ok(rec) = rec else { continue };
        if !matches!(
            rec.tag(),
            jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment
        ) {
            continue;
        }
        let Some(Ok(seg)) = rec.as_heap_dump_segment() else { continue };
        for sr in seg.sub_records() {
            let Ok(SubRecord::Instance(inst)) = sr else { continue };
            let cid = inst.class_obj_id().id();
            let class_name = classes.get(&cid).map(|c| c.name.clone()).unwrap_or_default();
            let sharp = leaf(&class_name).to_string();
            if sharp == "String" {
                if strings.len() >= sample_cap {
                    sampled = true;
                    continue;
                }
                strings.push(StrRaw {
                    obj_id: inst.obj_id().id(),
                    shallow: layout.instance_bytes(
                    classes.get(&cid).map(|c| c.instance_size).unwrap_or(0),
                ),
                });
            } else if COLLECTION_LEAVES.contains(&sharp.as_str()) {
                let chain = match chain_of(cid, &classes) {
                    Some(v) => v,
                    None => continue,
                };
                let fields = parse_fields(inst.fields(), &chain, idw, 64).0;
                let size = field_i64(&fields, "size").unwrap_or(-1);
                if size < 0 {
                    continue;
                }
                let arr = field_ref(&fields, "table")
                    .or_else(|| field_ref(&fields, "elementData"))
                    .unwrap_or(0);
                colls.push(CollRaw {
                    class_name,
                    size: size as u64,
                    arr,
                    shallow: layout.instance_bytes(
                    classes.get(&cid).map(|c| c.instance_size).unwrap_or(0),
                ),
                });
            }
        }
    }

    // ---- 字符串文本 ----
    let want_strings: HashSet<u64> = strings.iter().map(|s| s.obj_id).collect();
    let texts = string_texts(&hp, idw, &classes, &want_strings);

    // ---- 集合底层数组长度 ----
    let want_arrays: HashSet<u64> = colls.iter().map(|c| c.arr).filter(|v| *v != 0).collect();
    let mut arr_len: HashMap<u64, u64> = HashMap::new();
    if !want_arrays.is_empty() {
        for rec in hp.records_iter() {
            let Ok(rec) = rec else { continue };
            if !matches!(
                rec.tag(),
                jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment
            ) {
                continue;
            }
            let Some(Ok(seg)) = rec.as_heap_dump_segment() else { continue };
            for sr in seg.sub_records() {
                let Ok(SubRecord::ObjectArray(a)) = sr else { continue };
                if want_arrays.contains(&a.obj_id().id()) {
                    arr_len.insert(a.obj_id().id(), a.elements(hp.header().id_size()).count() as u64);
                }
            }
        }
    }

    // ---- 重复字符串 ----
    let shallow_of: HashMap<u64, u64> = strings.iter().map(|s| (s.obj_id, s.shallow)).collect();
    let mut by_value: HashMap<String, (u64, u64)> = HashMap::new(); // value → (count, shallow_sum)
    for (sid, text) in texts.iter() {
        if text.is_empty() {
            continue;
        }
        let sh = shallow_of.get(sid).copied().unwrap_or(0);
        let e = by_value.entry(text.clone()).or_insert((0, 0));
        e.0 += 1;
        e.1 += sh;
    }
    let mut dups: Vec<DupStringRow> = by_value
        .into_iter()
        .filter(|(_, (c, _))| *c > 1)
        .map(|(value, (count, shallow))| {
            let one = if count > 0 { shallow / count } else { 0 };
            DupStringRow {
                value: truncate_chars(&value, 96),
                count,
                shallow_bytes: shallow,
                wasted_bytes: one * (count - 1),
            }
        })
        .collect();
    dups.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes).then(b.count.cmp(&a.count)));
    let duplicate_groups = dups.len() as u64;
    let duplicate_wasted_bytes: u64 = dups.iter().map(|d| d.wasted_bytes).sum();
    dups.truncate(page);

    // ---- 集合填充率 ----
    struct Agg {
        instances: u64,
        empty: u64,
        size_sum: u64,
        fill_sum: f64,
        fill_n: u64,
        max_size: u64,
        shallow: u64,
    }
    let mut agg: HashMap<String, Agg> = HashMap::new();
    let mut empty_collections: u64 = 0;
    for c in colls.iter() {
        let a = agg.entry(c.class_name.clone()).or_insert(Agg {
            instances: 0,
            empty: 0,
            size_sum: 0,
            fill_sum: 0.0,
            fill_n: 0,
            max_size: 0,
            shallow: 0,
        });
        a.instances += 1;
        a.size_sum += c.size;
        a.shallow += c.shallow;
        a.max_size = a.max_size.max(c.size);
        if c.size == 0 {
            a.empty += 1;
            empty_collections += 1;
        }
        if let Some(&len) = arr_len.get(&c.arr) {
            if len > 0 {
                a.fill_sum += (c.size as f64 / len as f64).min(1.0);
                a.fill_n += 1;
            }
        }
    }
    let mut collections: Vec<CollectionFillRow> = agg
        .into_iter()
        .map(|(class_name, a)| CollectionFillRow {
            class_name,
            instances: a.instances,
            empty_instances: a.empty,
            avg_fill: if a.fill_n > 0 { a.fill_sum / a.fill_n as f64 } else { 0.0 },
            max_size: a.max_size,
            shallow_bytes: a.shallow,
        })
        .collect();
    collections.sort_by(|a, b| b.shallow_bytes.cmp(&a.shallow_bytes));
    collections.truncate(page);

    let mut note = String::new();
    if sampled {
        note.push_str(&format!(
            "String 采样已达上限 {sample_cap} 个，重复字符串统计是**下界**"
        ));
    }
    if !note.is_empty() {
        note.push_str("；");
    }
    note.push_str("重复字符串按解码后的值聚合（JDK8 char[] / JDK9+ byte[]+coder 都已处理）");

    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: WasteResult {
            duplicate_strings: dups,
            duplicate_wasted_bytes,
            duplicate_groups,
            collections,
            empty_collections,
            sampled,
            strings_scanned: strings.len() as u64,
            note,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    })
}

// ==================== 不可达对象直方图 ====================

/// 复用引用追踪会话：不可达 = 从 GC Roots 不可达（与 Leak Suspects 同口径）。
pub fn mat_unreachable_histogram(
    path: &str,
    limit: usize,
) -> Result<ResData<UnreachableResult>, String> {
    let started = std::time::Instant::now();
    let s = crate::mat_refs::ensure_session(path)?;
    let limits = mat_cfg::query_limits();
    let page = limit.clamp(1, limits.report_rows);
    let reach = crate::mat_refs::session_reachable(&s);
    let g = crate::mat_refs::session_graph(&s);

    let mut agg: HashMap<String, (u64, u64)> = HashMap::new();
    let mut total_objects: u64 = 0;
    let mut total_shallow: u64 = 0;
    let mut reachable_nodes: u64 = 0;
    for (i, ok) in reach.iter().enumerate() {
        if *ok {
            reachable_nodes += 1;
            continue;
        }
        let nd = &g.nodes[i];
        total_objects += 1;
        total_shallow += nd.shallow;
        let name = g.meta[nd.class as usize].name.clone();
        let e = agg.entry(name).or_insert((0, 0));
        e.0 += 1;
        e.1 += nd.shallow;
    }
    let mut rows: Vec<UnreachableRow> = agg
        .into_iter()
        .map(|(class_name, (count, shallow_bytes))| UnreachableRow {
            class_name,
            count,
            shallow_bytes,
        })
        .collect();
    rows.sort_by(|a, b| b.shallow_bytes.cmp(&a.shallow_bytes));
    let classes_total = rows.len() as u64;
    rows.truncate(page);

    let note = if total_objects == 0 {
        "没有不可达对象（所有对象都能从 GC Roots 到达）".to_string()
    } else {
        format!(
            "不可达对象 {} 个 / {} 字节，分布在 {} 个类（按堆字节降序）",
            total_objects, total_shallow, classes_total
        )
    };

    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: UnreachableResult {
            total_objects,
            total_shallow,
            reachable_nodes,
            rows,
            note,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    })
}
