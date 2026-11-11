//! # 对象值解析（字段值 / 数组内容 / String 值 / 类加载器）
//!
//! 纯业务层，不依赖 `tauri`。与 `mat_graph` 的取舍：
//!
//! - 对象图只保留**引用边**（GB 级 dump 能压进内存的前提），非引用字段字节、primitive
//!   数组内容在建图时是跳过的。若把值也塞进 `Graph` 或把 `Mmap` 存进 LRU 会话，
//!   要么内存峰值翻倍，要么 Windows 上文件句柄一直被占用到会话逐出（用户删不掉 dump）。
//! - 因此本模块采用**按需扫描**：`mat_object_values` 被调用时临时 mmap，扫 1~2 遍
//!   HPROF 定位目标对象并解析其值，函数返回即释放映射。低频交互（点开对象详情）
//!   多花几百毫秒～数秒，换来零常驻内存与零句柄占用。
//! - P3-2 的索引持久化落地后，可把「按 id 定位记录」换成 O(1) 偏移查表。
//!
//! ## 覆盖
//! - 实例字段：8 种 primitive + 对象引用（含继承链展开）
//! - String：JDK8 `char[] value` / JDK9+ `byte[] value` + `coder`（Latin1 / UTF-16）
//! - 数组：primitive 前 N 个元素预览、对象数组前 N 个元素 id
//! - 类对象：静态字段值 + class loader 对象 id
//! - 未覆盖（留给专用报告批次）：START_THREAD / STACK_FRAME、weak/soft/phantom 引用强度标注。

use crate::mat_graph::{field_type_tag, field_width, FT_OBJECT};
use crate::mat_hprof::{catch_parse, count_primitive_elements, read_head, sniff_format};
use bdevtools_core::response::resp_data::ResData;
use jvm_hprof::heap_dump::{FieldValue, PrimitiveArrayType, SubRecord};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

// ==================== 返回结构（与前端 matApi.ts 一一对应，snake_case） ====================

#[derive(Debug, Serialize)]
pub struct ValueField {
    pub name: String,
    /// object / boolean / char / float / double / byte / short / int / long
    pub type_name: String,
    /// 展示值（引用字段为 0x 十六进制，null 显示 "null"）
    pub value: String,
    /// 引用字段的目标对象 id（非引用为空串）
    pub target_obj_id: String,
}

#[derive(Debug, Serialize)]
pub struct ObjectValues {
    pub obj_id: String,
    pub class_name: String,
    /// instance / primitive-array / object-array / class
    pub kind: String,
    pub shallow_bytes: u64,
    pub fields: Vec<ValueField>,
    /// 字段总数（fields 可能被 limit 截断）
    pub fields_total: u64,
    /// 数组内容预览（对象数组为元素 id，primitive 数组为值序列）
    pub array_preview: String,
    pub array_total: u64,
    /// String 实例解析出的文本（最长 4096 字符）
    pub string_value: Option<String>,
    /// 该对象所属类的 class loader 对象 id
    pub loader_obj_id: String,
    pub note: String,
    pub elapsed_ms: u64,
}

// ==================== 内部数据结构 ====================

pub(crate) struct ClassInfo {
    pub name: String,
    pub super_id: Option<u64>,
    pub loader_id: Option<u64>,
    pub instance_size: u64,
    pub fields: Vec<(u8, String)>,
    pub statics: Vec<(String, FieldValue)>,
}

/// 扫描到的目标对象原始数据（只保存一个对象，内存可忽略）
enum Target {
    Instance { class_id: u64, data: Vec<u8> },
    PrimArray { array: PrimArrayData },
    ObjArray { total: u64, preview: String },
    ClassObj { class_id: u64 },
}

pub(crate) struct PrimArrayData {
    ty: PrimitiveArrayType,
    total: u64,
    preview: String,
    /// char[] / byte[] 的完整（截断到 MAX_STRING_CHARS）文本
    text: Option<String>,
}

pub(crate) const MAX_STRING_CHARS: usize = 4096;

fn type_name(tag: u8) -> &'static str {
    match tag {
        FT_OBJECT => "object",
        1 => "boolean",
        2 => "char",
        3 => "float",
        4 => "double",
        5 => "byte",
        6 => "short",
        7 => "int",
        8 => "long",
        _ => "unknown",
    }
}

pub(crate) fn be_u(data: &[u8], off: usize, width: usize) -> Option<u64> {
    if off + width > data.len() {
        return None;
    }
    let mut v = 0u64;
    for &b in &data[off..off + width] {
        v = (v << 8) | b as u64;
    }
    Some(v)
}

/// 把实例字段字节按继承链字段布局解析成可展示的值
pub(crate) fn parse_fields(
    data: &[u8],
    chain: &[(u8, String)],
    id_width: usize,
    limit: usize,
) -> (Vec<ValueField>, u64) {
    let mut out = Vec::new();
    let mut off = 0usize;
    for (tag, name) in chain {
        let w = if *tag == FT_OBJECT { id_width } else { field_width(*tag) };
        if w == 0 {
            break;
        }
        let raw = match be_u(data, off, w) {
            Some(v) => v,
            None => break, // 字段字节与类定义不符：保留已解析部分
        };
        off += w;
        if out.len() >= limit {
            continue; // 继续走完字节偏移，但不再追加
        }
        let (value, target) = match *tag {
            FT_OBJECT => {
                if raw == 0 {
                    ("null".to_string(), String::new())
                } else {
                    let hex = format!("0x{raw:x}");
                    (hex.clone(), hex)
                }
            }
            1 => (if raw != 0 { "true" } else { "false" }.to_string(), String::new()),
            2 => {
                let c = char::from_u32(raw as u32).unwrap_or('\u{fffd}');
                (format!("'{c}' (U+{:04X})", raw), String::new())
            }
            3 => (format!("{}", f32::from_bits(raw as u32)), String::new()),
            4 => (format!("{}", f64::from_bits(raw)), String::new()),
            5 => (format!("{}", raw as i8), String::new()),
            6 => (format!("{}", raw as i16), String::new()),
            7 => (format!("{}", raw as i32), String::new()),
            8 => (format!("{}", raw as i64), String::new()),
            _ => (format!("0x{raw:x}"), String::new()),
        };
        out.push(ValueField {
            name: name.clone(),
            type_name: type_name(*tag).to_string(),
            value,
            target_obj_id: target,
        });
    }
    (out, chain.len() as u64)
}

/// 读取一个 primitive 数组的内容（预览 + 可解码文本）
pub(crate) fn read_prim_array(
    p: &jvm_hprof::heap_dump::PrimitiveArray,
    limit: usize,
) -> PrimArrayData {
    let total = count_primitive_elements(p);
    let ty = p.primitive_type();
    let mut items: Vec<String> = Vec::new();
    let mut chars: Vec<u16> = Vec::new();
    let mut bytes: Vec<u8> = Vec::new();

    macro_rules! collect {
        ($it:expr, $fmt:expr) => {{
            for v in $it.take(limit.min(MAX_STRING_CHARS)) {
                let v = match v {
                    Ok(x) => x,
                    Err(_) => break,
                };
                if items.len() < limit {
                    items.push(format!($fmt, v));
                }
            }
        }};
    }
    match ty {
        PrimitiveArrayType::Boolean => collect!(p.booleans().into_iter().flatten(), "{}"),
        PrimitiveArrayType::Char => {
            for v in p.chars().into_iter().flatten().filter_map(|r| r.ok()).take(MAX_STRING_CHARS) {
                chars.push(v);
                if items.len() < limit {
                    items.push(format!("'{}'", char::from_u32(v as u32).unwrap_or('\u{fffd}')));
                }
            }
        }
        PrimitiveArrayType::Float => collect!(p.floats().into_iter().flatten(), "{}"),
        PrimitiveArrayType::Double => collect!(p.doubles().into_iter().flatten(), "{}"),
        PrimitiveArrayType::Byte => {
            for v in p.bytes().into_iter().flatten().filter_map(|r| r.ok()).take(MAX_STRING_CHARS) {
                bytes.push(v as u8);
                if items.len() < limit {
                    items.push(v.to_string());
                }
            }
        }
        PrimitiveArrayType::Short => collect!(p.shorts().into_iter().flatten(), "{}"),
        PrimitiveArrayType::Int => collect!(p.ints().into_iter().flatten(), "{}"),
        PrimitiveArrayType::Long => collect!(p.longs().into_iter().flatten(), "{}"),
    }
    let text = match ty {
        PrimitiveArrayType::Char => Some(String::from_utf16_lossy(&chars)),
        PrimitiveArrayType::Byte => Some(bytes.iter().map(|&b| b as char).collect()),
        _ => None,
    };
    let mut preview = items.join(", ");
    if total as usize > items.len() {
        preview.push_str(&format!(", … (共 {total} 个元素)"));
    }
    PrimArrayData { ty, total, preview, text }
}

fn field_value_text(v: FieldValue) -> (String, String) {
    match v {
        FieldValue::ObjectId(Some(id)) => {
            let h = format!("0x{:x}", id.id());
            (h.clone(), h)
        }
        FieldValue::ObjectId(None) => ("null".to_string(), String::new()),
        FieldValue::Boolean(b) => (b.to_string(), String::new()),
        FieldValue::Char(c) => (format!("'{}'", char::from_u32(c as u32).unwrap_or('\u{fffd}')), String::new()),
        FieldValue::Float(f) => (f.to_string(), String::new()),
        FieldValue::Double(f) => (f.to_string(), String::new()),
        FieldValue::Byte(b) => (b.to_string(), String::new()),
        FieldValue::Short(s) => (s.to_string(), String::new()),
        FieldValue::Int(i) => (i.to_string(), String::new()),
        FieldValue::Long(l) => (l.to_string(), String::new()),
    }
}

fn field_value_tag(v: &FieldValue) -> u8 {
    match v {
        FieldValue::ObjectId(_) => FT_OBJECT,
        FieldValue::Boolean(_) => 1,
        FieldValue::Char(_) => 2,
        FieldValue::Float(_) => 3,
        FieldValue::Double(_) => 4,
        FieldValue::Byte(_) => 5,
        FieldValue::Short(_) => 6,
        FieldValue::Int(_) => 7,
        FieldValue::Long(_) => 8,
    }
}

/// 继承链字段展开：本类声明在前，再逐级父类（与 HPROF 实例数据顺序一致）
pub(crate) fn chain_of(cid: u64, classes: &HashMap<u64, ClassInfo>) -> Option<Vec<(u8, String)>> {
    let mut out = Vec::new();
    let mut cur = Some(cid);
    let mut seen = HashSet::new();
    while let Some(id) = cur {
        if !seen.insert(id) {
            return None; // 循环继承（畸形）
        }
        let ci = classes.get(&id)?;
        out.extend(ci.fields.iter().cloned());
        cur = ci.super_id;
    }
    Some(out)
}

fn empty_result(obj_id: &str, note: String, elapsed_ms: u64) -> ResData<ObjectValues> {
    ResData {
        code: 200,
        msg: "ok".to_string(),
        data: ObjectValues {
            obj_id: obj_id.to_string(),
            class_name: String::new(),
            kind: String::new(),
            shallow_bytes: 0,
            fields: Vec::new(),
            fields_total: 0,
            array_preview: String::new(),
            array_total: 0,
            string_value: None,
            loader_obj_id: String::new(),
            note,
            elapsed_ms,
        },
    }
}

// ==================== 命令入口 ====================

/// 读取单个对象的字段值 / 数组内容 / String 值（纯同步，调用方需 spawn_blocking）
pub fn mat_object_values(
    path: &str,
    obj_id: &str,
    limit: usize,
) -> Result<ResData<ObjectValues>, String> {
    let started = std::time::Instant::now();
    let limit = limit.clamp(1, 2_000);
    let raw = obj_id.trim().trim_start_matches("0x").trim_start_matches("0X");
    let target = u64::from_str_radix(raw, 16).map_err(|_| format!("无效对象 id：{obj_id}"))?;

    let head = read_head(path, 64)?;
    let (_format, supported) = sniff_format(&head);
    if !supported {
        return Err("不支持的堆转储格式（当前仅支持 JAVA PROFILE 1.0.x）".to_string());
    }

    let mm = crate::mat_source::map_file(path)?;
    // ART 等变体的未知子记录会让解析器 panic，沿用全局约定统一兜底
    let hp = catch_parse(|| {
        jvm_hprof::parse_hprof(&mm[..]).map_err(|e| format!("解析失败: {e:?}"))
    })?;
    let id_size = hp.header().id_size();
    let id_width = match id_size { jvm_hprof::IdSize::U32 => 4, jvm_hprof::IdSize::U64 => 8 };

    // ---- Pass 1：字符串表 + LoadClass（顶层记录，扫描成本远低于堆段）----
    let mut utf8: HashMap<u64, String> = HashMap::new();
    let mut class_names: HashMap<u64, String> = HashMap::new();
    for rec in hp.records_iter() {
        let rec = rec.map_err(|e| format!("记录解析失败: {e:?}"))?;
        match rec.tag() {
            jvm_hprof::RecordTag::Utf8 => {
                if let Some(u) = rec.as_utf_8() {
                    let u = u.map_err(|e| format!("Utf8 解析失败: {e:?}"))?;
                    utf8.insert(u.name_id().id(), String::from_utf8_lossy(u.text()).into_owned());
                }
            }
            jvm_hprof::RecordTag::LoadClass => {
                if let Some(lc) = rec.as_load_class() {
                    let lc = lc.map_err(|e| format!("LoadClass 解析失败: {e:?}"))?;
                    let name = utf8.get(&lc.class_name_id().id()).cloned().unwrap_or_default();
                    class_names.insert(lc.class_obj_id().id(), name);
                }
            }
            _ => {}
        }
    }

    // ---- Pass 2：类元数据 + 定位目标对象 ----
    let mut classes: HashMap<u64, ClassInfo> = HashMap::new();
    let mut hit: Option<Target> = None;
    for rec in hp.records_iter() {
        let rec = rec.map_err(|e| format!("记录解析失败: {e:?}"))?;
        if !matches!(rec.tag(), jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment) {
            continue;
        }
        let Some(seg) = rec.as_heap_dump_segment() else { continue };
        let seg = seg.map_err(|e| format!("HeapDump 解析失败: {e:?}"))?;
        for sr in seg.sub_records() {
            let sr = match sr {
                Ok(v) => v,
                Err(_) => continue,
            };
            match sr {
                SubRecord::Class(c) => {
                    let cid = c.obj_id().id();
                    let mut fields = Vec::new();
                    for fd in c.instance_field_descriptors() {
                        let Ok(fd) = fd else { continue };
                        let nm = utf8.get(&fd.name_id().id()).cloned()
                            .unwrap_or_else(|| format!("#{:x}", fd.name_id().id()));
                        fields.push((field_type_tag(fd.field_type()), nm));
                    }
                    let mut statics = Vec::new();
                    for se in c.static_fields() {
                        let Ok(se) = se else { continue };
                        let nm = utf8.get(&se.name_id().id()).cloned()
                            .unwrap_or_else(|| format!("#{:x}", se.name_id().id()));
                        statics.push((nm, se.value()));
                    }
                    classes.insert(cid, ClassInfo {
                        name: class_names.get(&cid).cloned().unwrap_or_default(),
                        super_id: c.super_class_obj_id().map(|i| i.id()),
                        loader_id: c.class_loader_obj_id().map(|i| i.id()),
                        instance_size: c.instance_size_bytes() as u64,
                        fields,
                        statics,
                    });
                    if cid == target {
                        hit = Some(Target::ClassObj { class_id: cid });
                    }
                }
                SubRecord::Instance(d) => {
                    if d.obj_id().id() == target {
                        hit = Some(Target::Instance {
                            class_id: d.class_obj_id().id(),
                            data: d.fields().to_vec(),
                        });
                    }
                }
                SubRecord::PrimitiveArray(p) => {
                    if p.obj_id().id() == target {
                        hit = Some(Target::PrimArray { array: read_prim_array(&p, limit) });
                    }
                }
                SubRecord::ObjectArray(a) => {
                    if a.obj_id().id() == target {
                        let total = a.elements(id_size).count() as u64;
                        let mut ids = Vec::new();
                        for (i, el) in a.elements(id_size).enumerate().take(limit) {
                            let id = el.ok().flatten().map(|x| x.id()).unwrap_or(0);
                            ids.push(format!("[{i}] 0x{id:x}"));
                        }
                        let mut preview = ids.join(", ");
                        if total as usize > ids.len() {
                            preview.push_str(&format!(", … (共 {total} 个元素)"));
                        }
                        hit = Some(Target::ObjArray { total, preview });
                    }
                }
                _ => {}
            }
        }
    }

    let Some(t) = hit else {
        return Ok(empty_result(obj_id, format!("未在堆转储中找到对象 {obj_id}"), started.elapsed().as_millis() as u64));
    };

    let mut out = ObjectValues {
        obj_id: obj_id.to_string(),
        class_name: String::new(),
        kind: String::new(),
        shallow_bytes: 0,
        fields: Vec::new(),
        fields_total: 0,
        array_preview: String::new(),
        array_total: 0,
        string_value: None,
        loader_obj_id: String::new(),
        note: String::new(),
        elapsed_ms: 0,
    };

    match t {
        Target::ClassObj { class_id } => {
            let ci = classes.get(&class_id);
            out.kind = "class".to_string();
            out.class_name = ci.map(|c| c.name.clone()).unwrap_or_default();
            out.loader_obj_id = ci.and_then(|c| c.loader_id).map(|id| format!("0x{id:x}")).unwrap_or_default();
            if let Some(ci) = ci {
                out.fields_total = ci.statics.len() as u64;
                for (name, v) in ci.statics.iter().take(limit) {
                    let (value, target_obj_id) = field_value_text(*v);
                    out.fields.push(ValueField {
                        name: format!("<static> {name}"),
                        type_name: type_name(field_value_tag(v)).to_string(),
                        value,
                        target_obj_id,
                    });
                }
            }
        }
        Target::Instance { class_id, data } => {
            let ci = classes.get(&class_id);
            out.kind = "instance".to_string();
            out.class_name = ci.map(|c| c.name.clone()).unwrap_or_default();
            out.shallow_bytes = ci.map(|c| c.instance_size).unwrap_or(0);
            out.loader_obj_id = ci.and_then(|c| c.loader_id).map(|id| format!("0x{id:x}")).unwrap_or_default();
            match chain_of(class_id, &classes) {
                Some(chain) => {
                    let (fields, total) = parse_fields(&data, &chain, id_width, limit);
                    out.fields = fields;
                    out.fields_total = total;
                    if out.fields_total > limit as u64 {
                        out.note = format!("仅展示前 {limit} 个字段");
                    }
                    // String：JDK8 char[] / JDK9+ byte[] + coder
                    if out.class_name == "java.lang.String" || out.class_name == "java/lang/String" {
                        let value_id = out.fields.iter()
                            .find(|f| f.name == "value" && !f.target_obj_id.is_empty())
                            .and_then(|f| u64::from_str_radix(f.target_obj_id.trim_start_matches("0x"), 16).ok());
                        let coder = out.fields.iter()
                            .find(|f| f.name == "coder")
                            .map(|f| f.value.as_str()).unwrap_or("0");
                        let is_utf16 = coder.contains('1');
                        if let Some(vid) = value_id {
                            if let Some(text) = scan_string_value(&hp, vid, is_utf16) {
                                out.string_value = Some(text);
                            } else {
                                out.note.push_str("；String.value 数组未找到（可能位于不可解析的堆段）");
                            }
                        }
                    }
                }
                None => out.note = "类继承链元数据缺失，无法解析实例字段布局".to_string(),
            }
        }
        Target::PrimArray { array } => {
            out.kind = "primitive-array".to_string();
            out.class_name = match array.ty {
                PrimitiveArrayType::Boolean => "boolean[]",
                PrimitiveArrayType::Char => "char[]",
                PrimitiveArrayType::Float => "float[]",
                PrimitiveArrayType::Double => "double[]",
                PrimitiveArrayType::Byte => "byte[]",
                PrimitiveArrayType::Short => "short[]",
                PrimitiveArrayType::Int => "int[]",
                PrimitiveArrayType::Long => "long[]",
            }.to_string();
            let w = match array.ty {
                PrimitiveArrayType::Boolean | PrimitiveArrayType::Byte => 1,
                PrimitiveArrayType::Char | PrimitiveArrayType::Short => 2,
                PrimitiveArrayType::Float | PrimitiveArrayType::Int => 4,
                PrimitiveArrayType::Double | PrimitiveArrayType::Long => 8,
            };
            out.shallow_bytes = 16 + array.total * w;
            out.array_total = array.total;
            out.array_preview = array.preview;
            out.string_value = array.text;
        }
        Target::ObjArray { total, preview } => {
            out.kind = "object-array".to_string();
            out.class_name = "object[]".to_string();
            out.shallow_bytes = 16 + total * id_width as u64;
            out.array_total = total;
            out.array_preview = preview;
        }
    }

    out.elapsed_ms = started.elapsed().as_millis() as u64;
    Ok(ResData { code: 200, msg: "ok".to_string(), data: out })
}

/// Pass 3（仅 String 实例需要）：按 id 找到 value 数组并解码文本
fn scan_string_value(
    hp: &jvm_hprof::Hprof<'_>,
    array_id: u64,
    is_utf16: bool,
) -> Option<String> {
    for rec in hp.records_iter() {
        let Ok(rec) = rec else { continue };
        if !matches!(rec.tag(), jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment) {
            continue;
        }
        let Some(seg) = rec.as_heap_dump_segment() else { continue };
        let Ok(seg) = seg else { continue };
        for sr in seg.sub_records() {
            let Ok(SubRecord::PrimitiveArray(p)) = sr else { continue };
            if p.obj_id().id() != array_id {
                continue;
            }
            match p.primitive_type() {
                PrimitiveArrayType::Char => {
                    let cs: Vec<u16> = p.chars().into_iter().flatten().filter_map(|r| r.ok()).take(MAX_STRING_CHARS).collect();
                    return Some(String::from_utf16_lossy(&cs));
                }
                PrimitiveArrayType::Byte => {
                    let bs: Vec<u8> = p.bytes().into_iter().flatten().filter_map(|r| r.ok()).take(MAX_STRING_CHARS).map(|b| b as u8).collect();
                    if is_utf16 {
                        let cs: Vec<u16> = bs.chunks_exact(2)
                            .map(|c| u16::from_be_bytes([c[0], c[1]]))
                            .collect();
                        return Some(String::from_utf16_lossy(&cs));
                    }
                    return Some(bs.iter().map(|&b| b as char).collect());
                }
                _ => return None,
            }
        }
    }
    None
}
