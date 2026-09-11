//! # 公共对象引用图（HPROF → 对象图）
//!
//! 从 `mat_leak` 抽出的**纯业务层**共享模块：把 HPROF 堆转储解析为一张对象引用图，
//! 供泄漏检测（支配树 / Retained Heap）与对象引用追踪（outgoing / incoming /
//! Path to GC Roots）复用，避免同一 dump 被两个功能各自全量解析、各存一份图。
//!
//! 本模块不出现 `#[tauri::command]`、不依赖 `tauri` crate，命令包装统一放在
//! `commands.rs`（见该文件头部的架构约定）。
//!
//! ## 图语义
//! - **节点** = 堆对象：实例 / 对象数组 / 基本类型数组 / 类对象（类对象浅堆按 0 计）。
//! - **边** u→v = u 持有对 v 的引用：
//!   - 实例的**对象引用字段**（实例字段数据按「具体类 → 父类 → … → Object」的顺序序列化，
//!     配合类继承链的字段描述符逐字段解析）；
//!   - 对象数组的元素；
//!   - 类对象的**静态引用字段**（static 持有是常见泄漏源）。
//! - 边以 CSR 三数组（`edge_head` / `edge_to` / `edge_next`，头插）存储，
//!   消除 `Vec<Vec>` 的每节点头开销与扩容碎片，支撑 2000 万+ 节点规模。
//!
//! ## 已知口径
//! - 浅堆：实例 = 类 `instance_size`；对象数组 = 16 + n × id 宽；基本类型数组 = 16 + n × 元素宽；
//!   类对象自身按 0 计。
//! - `ClassMeta.fields` / `fields_names` 记录**本类声明**的实例字段（类型 tag + 名字，
//!   一一对应）；实例字段字节布局需要按继承链展开（见 `chain_fields_named`）。
//! - GC Root 种类标签与 Eclipse MAT 的 "Path to GC Roots" 根分类对应。

use crate::command::mat_hprof::{count_primitive_elements, primitive_array_meta};
use memmap2::Mmap;
use std::collections::{HashMap, HashSet};
use std::fs::File;

// ==================== 内部对象图 ====================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
    Instance,
    ObjectArray,
    PrimitiveArray,
    Class,
}

/// 图节点：一个堆对象（或类对象）。浅堆与类元数据索引在收集后回填。
#[derive(Clone, Copy)]
pub(crate) struct Node {
    pub id: u64,
    pub kind: NodeKind,
    /// class_meta 索引（primitive 数组为 0..7 的伪类索引）
    pub class: u32,
    pub shallow: u64,
}

/// 类元数据（来自 ClassDump / LoadClass，primitive 数组为伪类）
pub(crate) struct ClassMeta {
    /// 类对象 id（primitive 伪类为 0）
    pub obj_id: u64,
    pub name: String,
    /// class_meta 索引；-1 = 无父类（Object / primitive 伪类 / 数组伪类）
    pub super_idx: i32,
    pub instance_size: u32,
    /// 静态引用字段指向的对象 id（class 节点的出边）
    pub statics: Vec<u64>,
    /// 与 `statics` 一一对应的静态字段名（展示 / 引用点标注用）
    pub statics_names: Vec<String>,
    /// 本类声明的实例字段类型 tag（0=ObjectId，1..9 见 field_type_tag）
    pub fields: Vec<u8>,
    /// 与 `fields` 一一对应的字段名（展示 / 引用点标注用）
    pub fields_names: Vec<String>,
    /// 继承链完整（本类+全部祖先的 class 元数据齐全）→ 实例可解析引用
    pub chain_ok: bool,
}

/// 对象引用图（构建完成后所有字段只读）
pub(crate) struct Graph {
    pub nodes: Vec<Node>,
    /// class 元数据表；primitive 伪类在表头 0..=7
    pub meta: Vec<ClassMeta>,
    /// 邻接表（CSR 三数组头插，避免 Vec<Vec> 的每节点头开销与扩容碎片）
    pub edge_head: Vec<u32>,
    pub edge_to: Vec<u32>,
    pub edge_next: Vec<u32>,
    /// 与 edge_to/edge_next 等长的「引用点」编码，语义取决于引用者节点类型：
    /// - 实例：字段在继承链（chain_fields_named 顺序）中的位置 → 字段名；
    /// - 类对象：静态字段下标（对齐 `ClassMeta.statics/statics_names`）；
    /// - 对象数组：被引用元素在数组中的下标。
    pub edge_slot: Vec<u32>,
    /// (节点索引, GC Root 分类标签)
    pub roots: Vec<(u32, &'static str)>,
    /// 指向图外对象的引用数
    pub skipped: u64,
    /// 实例字段字节与声明不符导致的解析中断次数
    pub field_mismatch: u64,
}

// ==================== 字段类型 tag ====================

/// 实例字段类型 tag（见 heapDumper 的 basic type 常量）
pub(crate) const FT_OBJECT: u8 = 0;
pub(crate) const FT_BOOLEAN: u8 = 1;
pub(crate) const FT_CHAR: u8 = 2;
pub(crate) const FT_FLOAT: u8 = 3;
pub(crate) const FT_DOUBLE: u8 = 4;
pub(crate) const FT_BYTE: u8 = 5;
pub(crate) const FT_SHORT: u8 = 6;
pub(crate) const FT_INT: u8 = 7;
pub(crate) const FT_LONG: u8 = 8;

/// 字段类型 → 字节宽度（object 返回 0，需按 id 宽另算）
pub(crate) fn field_width(tag: u8) -> usize {
    match tag {
        FT_OBJECT => 0,
        FT_BOOLEAN | FT_BYTE => 1,
        FT_CHAR | FT_SHORT => 2,
        FT_FLOAT | FT_INT => 4,
        FT_DOUBLE | FT_LONG => 8,
        _ => 0,
    }
}

/// primitive 数组类型 → 伪类索引 0..=7（与 meta 表头一致）
pub(crate) fn prim_array_idx(p: &jvm_hprof::heap_dump::PrimitiveArray) -> u32 {
    use jvm_hprof::heap_dump::PrimitiveArrayType as T;
    match p.primitive_type() {
        T::Boolean => 0,
        T::Byte => 1,
        T::Char => 2,
        T::Short => 3,
        T::Int => 4,
        T::Float => 5,
        T::Long => 6,
        T::Double => 7,
    }
}

/// 数组对象头（近似 HotSpot 对齐后 16B，与直方图口径一致）
pub(crate) const ARRAY_HEADER: u64 = 16;

// ==================== 图构建 ====================

pub(crate) fn build_graph(path: &str) -> Result<Graph, String> {
    let f = File::open(path).map_err(|e| format!("无法打开文件: {e}"))?;
    let mm = unsafe { Mmap::map(&f).map_err(|e| format!("内存映射失败: {e}"))? };
    let hp = jvm_hprof::parse_hprof(&mm[..]).map_err(|e| format!("解析失败: {e:?}"))?;
    let id_size = hp.header().id_size();
    let id_width = match id_size {
        jvm_hprof::IdSize::U32 => 4,
        jvm_hprof::IdSize::U64 => 8,
    } as usize;

    // ---- class 元数据表：表头 8 个 primitive 数组伪类 ----
    let prim_names = ["boolean[]", "byte[]", "char[]", "short[]", "int[]", "float[]", "long[]", "double[]"];
    let mut meta: Vec<ClassMeta> = prim_names
        .iter()
        .map(|n| ClassMeta {
            obj_id: 0,
            name: n.to_string(),
            super_idx: -1,
            instance_size: 0,
            statics: Vec::new(),
            statics_names: Vec::new(),
            fields: Vec::new(),
            fields_names: Vec::new(),
            chain_ok: true,
        })
        .collect();
    let mut class_by_obj: HashMap<u64, usize> = HashMap::new();

    // 取（或创建）类元数据；name 仅首次创建 / 名字为空时填入
    fn ensure_class(
        meta: &mut Vec<ClassMeta>,
        class_by_obj: &mut HashMap<u64, usize>,
        cid: u64,
        name: Option<String>,
    ) -> usize {
        if let Some(&i) = class_by_obj.get(&cid) {
            if name.is_some() && meta[i].name.is_empty() {
                meta[i].name = name.unwrap();
            }
            return i;
        }
        let i = meta.len();
        meta.push(ClassMeta {
            obj_id: cid,
            name: name.unwrap_or_default(),
            super_idx: -1,
            instance_size: 0,
            statics: Vec::new(),
            statics_names: Vec::new(),
            fields: Vec::new(),
            fields_names: Vec::new(),
            chain_ok: false,
        });
        class_by_obj.insert(cid, i);
        i
    }

    let mut utf8: HashMap<u64, String> = HashMap::new();
    // 节点表
    let mut nodes: Vec<Node> = Vec::new();
    let mut node_by_id: HashMap<u64, u32> = HashMap::new();
    // class 节点（ClassDump 建立的节点）的 node 索引 → 补静态边用
    let mut class_node_idx: HashMap<u64, u32> = HashMap::new();
    // 引用计数（PrimitiveArray 需要先 count 才知浅堆，放到第二遍）
    let mut roots: Vec<(u64, &'static str)> = Vec::new();

    // ---- 第一遍：名字 / 类元数据 / 节点全集 / GC Roots ----
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
                    let cid = lc.class_obj_id().id();
                    let name = utf8
                        .get(&lc.class_name_id().id())
                        .cloned()
                        .unwrap_or_default();
                    ensure_class(&mut meta, &mut class_by_obj, cid, Some(name));
                }
            }
            jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment => {
                if let Some(seg) = rec.as_heap_dump_segment() {
                    let seg = seg.map_err(|e| format!("HeapDump 解析失败: {e:?}"))?;
                    for sr in seg.sub_records() {
                        let sr = sr.map_err(|e| format!("子记录解析失败: {e:?}"))?;
                        match sr {
                            jvm_hprof::heap_dump::SubRecord::Class(c) => {
                                let cid = c.obj_id().id();
                                let mi = ensure_class(&mut meta, &mut class_by_obj, cid, None);
                                // 先算 super 索引（可能创建超类元数据，须在 &mut meta[mi] 借用前完成）
                                let super_idx = match c.super_class_obj_id() {
                                    Some(s) => {
                                        let sid = s.id();
                                        class_by_obj.get(&sid).copied().unwrap_or_else(|| {
                                            ensure_class(&mut meta, &mut class_by_obj, sid, None)
                                        }) as i32
                                    }
                                    None => -1,
                                };
                                let m = &mut meta[mi];
                                m.instance_size = c.instance_size_bytes();
                                m.super_idx = super_idx;
                                // 实例字段：解析成本类字段 tag + 名字列表（顺序一一对应）
                                m.fields.clear();
                                m.fields_names.clear();
                                for fd in c.instance_field_descriptors() {
                                    let fd = fd
                                        .map_err(|e| format!("类字段描述解析失败: {e:?}"))?;
                                    m.fields.push(field_type_tag(fd.field_type()));
                                    let nm = utf8
                                        .get(&fd.name_id().id())
                                        .cloned()
                                        .unwrap_or_else(|| format!("#{:x}", fd.name_id().id()));
                                    m.fields_names.push(nm);
                                }
                                // 静态字段：对象引用目标 + 名字（仅对象引用字段，保证一一对应）
                                m.statics.clear();
                                m.statics_names.clear();
                                for se in c.static_fields() {
                                    let se =
                                        se.map_err(|e| format!("静态字段解析失败: {e:?}"))?;
                                    if let jvm_hprof::heap_dump::FieldValue::ObjectId(Some(id)) =
                                        se.value()
                                    {
                                        let nm = utf8
                                            .get(&se.name_id().id())
                                            .cloned()
                                            .unwrap_or_else(|| {
                                                format!("#{:x}", se.name_id().id())
                                            });
                                        m.statics_names.push(nm);
                                        m.statics.push(id.id());
                                    }
                                }
                                // 类对象自身作为图节点
                                let ni = node_by_id.len() as u32;
                                nodes.push(Node { id: cid, kind: NodeKind::Class, class: mi as u32, shallow: 0 });
                                node_by_id.insert(cid, ni);
                                class_node_idx.insert(cid, ni);
                            }
                            jvm_hprof::heap_dump::SubRecord::Instance(d) => {
                                let cid = d.class_obj_id().id();
                                let mi = ensure_class(&mut meta, &mut class_by_obj, cid, None);
                                let ni = node_by_id.len() as u32;
                                nodes.push(Node { id: d.obj_id().id(), kind: NodeKind::Instance, class: mi as u32, shallow: 0 });
                                node_by_id.insert(d.obj_id().id(), ni);
                            }
                            jvm_hprof::heap_dump::SubRecord::ObjectArray(a) => {
                                let acid = a.array_class_obj_id().id();
                                let mi = ensure_class(&mut meta, &mut class_by_obj, acid, None);
                                let ni = node_by_id.len() as u32;
                                nodes.push(Node { id: a.obj_id().id(), kind: NodeKind::ObjectArray, class: mi as u32, shallow: 0 });
                                node_by_id.insert(a.obj_id().id(), ni);
                            }
                            jvm_hprof::heap_dump::SubRecord::PrimitiveArray(p) => {
                                let ni = node_by_id.len() as u32;
                                nodes.push(Node { id: p.obj_id().id(), kind: NodeKind::PrimitiveArray, class: prim_array_idx(&p), shallow: 0 });
                                node_by_id.insert(p.obj_id().id(), ni);
                            }
                            other => {
                                if let Some(kind) = gc_root_kind(&other) {
                                    roots.push((gc_root_id(&other), kind));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // 名字兜底：LoadClass 缺失的类给十六进制占位名
    for m in meta.iter_mut() {
        if m.name.is_empty() && m.obj_id != 0 {
            m.name = format!("#{:x}", m.obj_id);
        }
    }

    // ---- 解析继承链字段布局（memo 递归，链断 → None）----
    let mut memo: Vec<Option<Vec<u8>>> = vec![None; meta.len()];
    // 仅 primitive 伪类（表头固定 8 个）预置为空字段链；真实类留 None 待递归求解
    for i in 0..8.min(meta.len()) {
        memo[i] = Some(Vec::new());
    }
    fn chain_fields(
        mi: usize,
        meta: &[ClassMeta],
        memo: &mut Vec<Option<Vec<u8>>>,
        busy: &mut Vec<bool>,
    ) -> Option<Vec<u8>> {
        if busy[mi] {
            return None; // 循环继承（畸形）
        }
        if let Some(v) = memo[mi].as_ref() {
            return Some(v.clone());
        }
        busy[mi] = true;
        let m = &meta[mi];
        let mut out = m.fields.clone();
        let super_ok = if m.super_idx >= 0 {
            match chain_fields(m.super_idx as usize, meta, memo, busy) {
                Some(sup) => {
                    out.extend(sup);
                    true
                }
                None => false,
            }
        } else {
            true
        };
        busy[mi] = false;
        if super_ok {
            memo[mi] = Some(out.clone());
            Some(out)
        } else {
            None
        }
    }
    let mut busy = vec![false; meta.len()];
    for i in 0..meta.len() {
        let ok = chain_fields(i, &meta, &mut memo, &mut busy).is_some();
        meta[i].chain_ok = ok;
    }
    let chain_full = memo;

    // ---- 第二遍：浅堆回填 + 解析引用边 ----
    let n = nodes.len();
    // CSR 头插邻接：edge_head[u]（u32::MAX=空）→ 边链表
    let mut edge_head: Vec<u32> = vec![u32::MAX; n];
    let mut edge_to: Vec<u32> = Vec::new();
    let mut edge_next: Vec<u32> = Vec::new();
    let mut edge_slot: Vec<u32> = Vec::new();
    let mut skipped: u64 = 0;
    let mut field_mismatch: u64 = 0;

    // 头插：把 v 挂到 u 的出边链表头（自环/堆外过滤由调用点负责）
    macro_rules! add_edge {
        ($u:expr, $v:expr, $slot:expr) => {{
            let _e = edge_to.len() as u32;
            edge_to.push($v as u32);
            edge_next.push(edge_head[$u as usize]);
            edge_slot.push($slot);
            edge_head[$u as usize] = _e;
        }};
    }

    {
        // 局部函数：把对象 id 转为节点索引（找不到 → None）
        macro_rules! target_idx {
            ($tid:expr) => {{
                node_by_id.get(&($tid)).copied()
            }};
        }

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
                        jvm_hprof::heap_dump::SubRecord::Class(_) => {
                            // 静态边在类收集完毕后统一补（见下方循环）
                        }
                        jvm_hprof::heap_dump::SubRecord::Instance(d) => {
                            let oid = d.obj_id().id();
                            let &ni = node_by_id.get(&oid).ok_or("对象索引缺失")?;
                            let mi = nodes[ni as usize].class as usize;
                            nodes[ni as usize].shallow = meta[mi].instance_size as u64;
                            // 解析对象引用字段（需要完整继承链）
                            let field_chain = match chain_full[mi].as_ref() {
                                Some(v) if meta[mi].chain_ok => v,
                                _ => {
                                    field_mismatch += 1;
                                    continue;
                                }
                            };
                            let data = d.fields();
                            let mut off = 0usize;
                            let mut ok = true;
                            for (slot, &tag) in field_chain.iter().enumerate() {
                                if tag == FT_OBJECT {
                                    if off + id_width > data.len() {
                                        ok = false;
                                        break;
                                    }
                                    let mut v: u64 = 0;
                                    for b in &data[off..off + id_width] {
                                        v = (v << 8) | (*b as u64);
                                    }
                                    off += id_width;
                                    if v != 0 {
                                        match target_idx!(v) {
                                            Some(ti) if ti != ni => {
                                                add_edge!(ni, ti, slot as u32);
                                            }
                                            Some(_) => {} // 自环忽略
                                            None => skipped += 1,
                                        }
                                    }
                                } else {
                                    let w = field_width(tag);
                                    if w == 0 || off + w > data.len() {
                                        ok = false;
                                        break;
                                    }
                                    off += w;
                                }
                            }
                            if !ok {
                                field_mismatch += 1;
                            }
                        }
                        jvm_hprof::heap_dump::SubRecord::ObjectArray(a) => {
                            let oid = a.obj_id().id();
                            let &ni = node_by_id.get(&oid).ok_or("对象索引缺失")?;
                            let mut elem_n: u64 = 0;
                            for (ei, el) in a.elements(id_size).enumerate() {
                                let el = el.map_err(|e| format!("数组元素解析失败: {e:?}"))?;
                                if let Some(id) = el {
                                    if let Some(ti) = target_idx!(id.id()) {
                                        if ti != ni {
                                            elem_n += 1;
                                            add_edge!(ni, ti, ei as u32);
                                        }
                                    } else {
                                        skipped += 1;
                                    }
                                }
                            }
                            nodes[ni as usize].shallow = ARRAY_HEADER + elem_n * id_width as u64;
                        }
                        jvm_hprof::heap_dump::SubRecord::PrimitiveArray(p) => {
                            let oid = p.obj_id().id();
                            let &ni = node_by_id.get(&oid).ok_or("对象索引缺失")?;
                            let (_name, elem_w) = primitive_array_meta(p.primitive_type());
                            let elem_n = count_primitive_elements(&p);
                            nodes[ni as usize].shallow = ARRAY_HEADER + elem_n * elem_w;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // 类对象静态引用边（依赖 node_by_id 全集已就绪，故在第二遍之后补）
    for (&cid, &ni) in class_node_idx.iter() {
        let mi = match class_by_obj.get(&cid) {
            Some(&m) => m,
            None => continue,
        };
        for (si, &t) in meta[mi].statics.iter().enumerate() {
            match node_by_id.get(&t) {
                Some(&ti) if ti != ni => add_edge!(ni, ti, si as u32),
                Some(_) => {}
                None => skipped += 1,
            }
        }
    }

    let root_nodes: Vec<(u32, &'static str)> = {
        // GC Root 去重：同一对象多个根类型保留第一个
        let mut seen: HashMap<u32, &'static str> = HashMap::new();
        for (id, kind) in roots {
            if let Some(&ni) = node_by_id.get(&id) {
                seen.entry(ni).or_insert(kind);
            }
        }
        seen.into_iter().collect()
    };

    Ok(Graph {
        nodes,
        meta,
        edge_head,
        edge_to,
        edge_next,
        edge_slot,
        roots: root_nodes,
        skipped,
        field_mismatch,
    })
}

/// FieldType → 内部 tag（见 FT_* 常量）
pub(crate) fn field_type_tag(t: jvm_hprof::heap_dump::FieldType) -> u8 {
    use jvm_hprof::heap_dump::FieldType as F;
    match t {
        F::ObjectId => FT_OBJECT,
        F::Boolean => FT_BOOLEAN,
        F::Char => FT_CHAR,
        F::Float => FT_FLOAT,
        F::Double => FT_DOUBLE,
        F::Byte => FT_BYTE,
        F::Short => FT_SHORT,
        F::Int => FT_INT,
        F::Long => FT_LONG,
    }
}

/// GC Root 子记录 → 分类标签（None = 不是 root 记录）
pub(crate) fn gc_root_kind(sr: &jvm_hprof::heap_dump::SubRecord) -> Option<&'static str> {
    use jvm_hprof::heap_dump::SubRecord as S;
    Some(match sr {
        S::GcRootUnknown(_) => "unknown",
        S::GcRootThreadObj(_) => "thread",
        S::GcRootJniGlobal(_) => "jni-global",
        S::GcRootJniLocalRef(_) => "jni-local",
        S::GcRootJavaStackFrame(_) => "stack-frame",
        S::GcRootNativeStack(_) => "native-stack",
        S::GcRootSystemClass(_) => "system-class",
        S::GcRootThreadBlock(_) => "thread-block",
        S::GcRootBusyMonitor(_) => "monitor",
        _ => return None,
    })
}

/// GC Root 子记录 → 根对象 id
pub(crate) fn gc_root_id(sr: &jvm_hprof::heap_dump::SubRecord) -> u64 {
    use jvm_hprof::heap_dump::SubRecord as S;
    match sr {
        S::GcRootUnknown(r) => r.obj_id().id(),
        S::GcRootThreadObj(r) => r.thread_obj_id().map(|i| i.id()).unwrap_or(0),
        S::GcRootJniGlobal(r) => r.obj_id().id(),
        S::GcRootJniLocalRef(r) => r.obj_id().id(),
        S::GcRootJavaStackFrame(r) => r.obj_id().id(),
        S::GcRootNativeStack(r) => r.obj_id().id(),
        S::GcRootSystemClass(r) => r.obj_id().id(),
        S::GcRootThreadBlock(r) => r.obj_id().id(),
        S::GcRootBusyMonitor(r) => r.obj_id().id(),
        _ => 0,
    }
}

/// 类 `mi` 的**继承链**字段序列（类型 tag + 字段名），顺序与实例字段数据字节一致
/// （具体类字段 → 父类字段 → … → Object），供引用追踪做「字段名 → 引用目标」的
/// 逐字段解析。链断（循环继承 / 超类元数据缺失）返回 `None`。
///
/// `memo` 跨查询共享（解析过的类不再递归）；`busy` 每次调用新建，防循环继承栈溢出。
pub(crate) fn chain_fields_named(
    mi: usize,
    meta: &[ClassMeta],
    memo: &mut HashMap<usize, Option<Vec<(u8, String)>>>,
    busy: &mut HashSet<usize>,
) -> Option<Vec<(u8, String)>> {
    if let Some(v) = memo.get(&mi) {
        return v.clone();
    }
    if !busy.insert(mi) {
        return None; // 循环继承（畸形）
    }
    let m = &meta[mi];
    let mut out: Vec<(u8, String)> = m
        .fields
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            let nm = m
                .fields_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("#f{i}"));
            (t, nm)
        })
        .collect();
    let super_ok = if m.super_idx >= 0 {
        match chain_fields_named(m.super_idx as usize, meta, memo, busy) {
            Some(sup) => {
                out.extend(sup);
                true
            }
            None => false,
        }
    } else {
        true
    };
    busy.remove(&mi);
    if super_ok {
        let r = Some(out);
        memo.insert(mi, r.clone());
        r
    } else {
        memo.insert(mi, None);
        None
    }
}
