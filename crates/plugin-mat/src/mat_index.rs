/*! # 本地索引持久化（`.bdtidx` / `.bdtidx.dom`）

打开一份 GB 级 dump 最贵的一步不是查询，而是**解析 + 建图**：`jvm-hprof` 要顺序
扫完整个文件、按类继承链展开每个实例的字段、再拼出 CSR 邻接表。同一份 dump 反复
打开（调参、对比、隔天回看）就要反复付这笔钱。本模块把「解析结果」缓存到 dump
旁边，第二次打开直接 mmap 读回。

## 两个文件，各管一段

| 文件 | 内容 | 什么时候写 |
|---|---|---|
| `<dump>.bdtidx` | 对象图：类元数据 / 节点 / CSR 邻接 / GC Roots / 统计计数 | 首次建图后 |
| `<dump>.bdtidx.dom` | 支配树：inv / idom / retained / 孩子链表 / 根标记 / 泄漏嫌疑 | 首次算完支配树后 |

**刻意分成两个文件**：支配树是懒加载的（不点「泄漏检测」就不该花几分钟去算），
它的产生时间远晚于建图。合成一个文件就得在算完支配树后**重写整份索引**（GB 级
写盘），分成两个各自独立、随时可失效，代价小得多。

## 失效判定 = 指纹

指纹 = `(源路径哈希, 源文件大小, mtime 毫秒)`。三者任一变化即视为不同文件：

- **必须带路径**：`jmap -dump` 覆盖写同名文件是常规操作，只按名字做键会拿到上一次的图；
- **必须带 size + mtime**：覆盖写之后大小可能恰好相同（监控工具定时 dump 同一应用）；
- 路径用 FNV-1a 哈希而非原文：避免超长路径与 Windows 非法字符。

`dom` 文件额外记录 `strong_only`（弱引用口径）—— 口径一变，可达集与支配树全变。
图本身与口径无关（边上的 `ref_kind` 是原始数据），所以 graph 段不校验它。

## 健壮性约定

索引**永远只是缓存**：任何异常（文件缺失 / magic 不符 / 版本不符 / 指纹不符 /
长度不符 / 结构越界）一律返回 `None` 让调用方重新解析，**绝不返回 Err 挡住主流程**。
写入失败（目录只读、磁盘满）同样只忽略，业务照常。

- 写：先写同目录唯一临时名 → `rename` 覆盖（Windows 的 `rename` 不覆盖已存在文件，
  故先 `remove_file`）。中途崩溃最多留一个 `.tmp`，不会污染已有索引。
- 读：mmap（不把 GB 级索引读进堆），头部 `total_len` 必须等于文件长度 —— 这是对
  截断的确定性检查（不靠逐字节 checksum，省一次全量扫描）。
- 结构校验覆盖所有后续会被当数组下标用的字段（`node.class` / `edge_to` / `inv` /
  `idom` / `child_next`），宁可整份判废，也不让坏索引在查询时 panic。
*/

use crate::mat_dom::DomTree;
use crate::mat_graph::{ClassMeta, Graph, Node, NodeKind};
use crate::mat_leak::{MatLeakPathNode, MatLeakSuspect};
use crate::mat_refs::Session;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// 图索引 magic（8 字节）
const GRAPH_MAGIC: &[u8; 8] = b"BDTMATGR";
/// 支配树索引 magic（8 字节）
const DOM_MAGIC: &[u8; 8] = b"BDTMATDM";
/// 格式版本。**改任何字段布局都必须 +1**，否则老索引会被按新布局解读。
/// v2：头部加「堆布局」段（浅堆口径参与失效判定，P3-4）。
const VERSION: u32 = 2;
/// 头部长度：magic(8) + version(4) + total_len(8) + path_hash(8) + size(8) + mtime(8)
/// + layout(16) = 60
const HEADER_LEN: usize = 60;

/// GC Root 标签表（与 `mat_graph::gc_root_kind` 的返回值一一对应）。
/// 落盘只存下标：这些标签是编译期常量，没必要为每个根存一遍字符串。
const ROOT_KINDS: [&str; 9] = [
    "unknown",
    "thread",
    "jni-global",
    "jni-local",
    "stack-frame",
    "native-stack",
    "system-class",
    "thread-block",
    "monitor",
];

// ==================== 对外结构 ====================

/// 索引状态（`mat_index_info` 返回，前端概览页展示）
#[derive(Debug, Serialize)]
pub struct IndexInfo {
    /// 是否启用索引（`plugins/mat.json` 的 `indexEnabled`）
    pub enabled: bool,
    /// 图索引是否存在且指纹匹配
    pub graph_valid: bool,
    /// 图索引文件路径（存在时）
    pub graph_path: String,
    pub graph_bytes: u64,
    /// 支配树索引是否存在且与当前图 + 口径匹配
    pub dom_valid: bool,
    pub dom_path: String,
    pub dom_bytes: u64,
    /// 超过该体积的 dump 只读不写（避免为超大堆生成同量级索引）
    pub write_max_gb: f64,
    pub note: String,
}

/// 删除索引的结果
#[derive(Debug, Serialize)]
pub struct IndexClearResult {
    pub removed: u32,
    pub removed_bytes: u64,
    /// 删不掉的文件（仍被 mmap 占用 / 权限不足）
    pub skipped: Vec<String>,
}

// ==================== 指纹与路径 ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fp {
    path_hash: u64,
    size: u64,
    mtime: i64,
}

/// 源文件指纹；文件不存在 / 元数据读不到 ⇒ None（此时不该写索引）
fn fingerprint(path: &str) -> Option<Fp> {
    let meta = fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Some(Fp {
        path_hash: crate::mat_source::fnv1a64(path.as_bytes()),
        size: meta.len(),
        mtime,
    })
}

/// 索引目录（写在 dump 旁边失败时的回退位置）
fn fallback_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("BDT_MAT_INDEX_DIR") {
        return PathBuf::from(d);
    }
    let app_dir = bdevtools_core::app::app_store::APP.lock().app_dir.clone();
    let base = if app_dir.as_os_str().is_empty() {
        std::env::temp_dir().join("bdevtools")
    } else {
        app_dir
    };
    base.join("mat_index")
}

/// 候选路径：① dump 同目录（用户可见、随文件移动）② 应用数据目录（源目录只读时）
fn candidates(path: &str, suffix: &str) -> [PathBuf; 2] {
    let colocated = PathBuf::from(format!("{path}{suffix}"));
    let key = crate::mat_source::fnv1a64(path.as_bytes());
    let fallback = fallback_dir().join(format!("{key:016x}{suffix}"));
    [colocated, fallback]
}

/// 写入（原子替换）：先写唯一临时名，再 rename 覆盖
fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    let name = target
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "index".to_string());
    let tmp = target.with_file_name(format!("{name}.{}.tmp", std::process::id()));
    fs::write(&tmp, bytes).map_err(|e| format!("写入临时文件失败: {e}"))?;
    // Windows：rename 不覆盖已存在文件，先删旧文件（删不掉就让 rename 报错，调用方回退）
    let _ = fs::remove_file(target);
    if let Err(e) = fs::rename(&tmp, target) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("替换索引文件失败: {e}"));
    }
    Ok(())
}

/// 依次尝试候选路径写入；返回实际写入的位置
fn write_sidecar(path: &str, suffix: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let mut last = String::new();
    for cand in candidates(path, suffix) {
        match write_atomic(&cand, bytes) {
            Ok(()) => return Ok(cand),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// 打开候选索引并校验头部；返回 (payload, 映射对象, 实际路径, 布局)。
/// `Mmap` 必须由调用方持有到解码结束（它才是文件存活的保证）。
fn open_sidecar(
    path: &str,
    suffix: &str,
    magic: &[u8; 8],
) -> Option<(memmap2::Mmap, usize, PathBuf, crate::mat_layout::HeapLayout)> {
    let want = fingerprint(path)?;
    // 配置里的堆布局口径变了（对齐 / 压缩指针）⇒ 索引里的浅堆全部作废
    let cur_tag = crate::mat_layout::cfg_tag();
    for cand in candidates(path, suffix) {
        let Some(s) = cand.to_str() else { continue };
        let mm = match crate::mat_source::map_file(s) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if mm.len() < HEADER_LEN || &mm[0..8] != &magic[..] {
            continue;
        }
        let ver = u32::from_le_bytes(mm[8..12].try_into().ok()?);
        if ver != VERSION {
            continue;
        }
        let total = u64::from_le_bytes(mm[12..20].try_into().ok()?);
        if total != mm.len() as u64 {
            continue; // 截断 / 被追加
        }
        let fp = Fp {
            path_hash: u64::from_le_bytes(mm[20..28].try_into().ok()?),
            size: u64::from_le_bytes(mm[28..36].try_into().ok()?),
            mtime: i64::from_le_bytes(mm[36..44].try_into().ok()?),
        };
        if fp != want {
            continue; // dump 被改写 / 索引被复制到别的 dump
        }
        let layout = crate::mat_layout::HeapLayout::from_stored(
            mm[44],
            mm[45],
            mm[46] != 0,
            u32::from_le_bytes(mm[48..52].try_into().ok()?),
            u32::from_le_bytes(mm[52..56].try_into().ok()?),
            u32::from_le_bytes(mm[56..60].try_into().ok()?),
        )?;
        if layout.tag() != cur_tag {
            continue; // 配置改过（heapLayout），旧浅堆不可信
        }
        return Some((mm, HEADER_LEN, cand, layout));
    }
    None
}

// ==================== 二进制编解码 ====================

/// 写入器：小端 + 长度前缀。体积优先，不做自描述（版本号负责兼容边界）。
struct Wr {
    b: Vec<u8>,
}

impl Wr {
    fn with_capacity(n: usize) -> Wr {
        Wr { b: Vec::with_capacity(n) }
    }
    fn u8(&mut self, v: u8) {
        self.b.push(v);
    }
    fn bool(&mut self, v: bool) {
        self.b.push(v as u8);
    }
    fn u32(&mut self, v: u32) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }
    fn f64(&mut self, v: f64) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }
    fn bytes(&mut self, v: &[u8]) {
        self.u32(v.len() as u32);
        self.b.extend_from_slice(v);
    }
    fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
    fn u32s(&mut self, v: &[u32]) {
        self.u32(v.len() as u32);
        for &x in v {
            self.b.extend_from_slice(&x.to_le_bytes());
        }
    }
    fn u64s(&mut self, v: &[u64]) {
        self.u32(v.len() as u32);
        for &x in v {
            self.b.extend_from_slice(&x.to_le_bytes());
        }
    }
}

/// 读取器：所有越界都返回 Err（由调用方整份判废），绝不 panic
struct Rd<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Rd<'a> {
    fn new(b: &'a [u8]) -> Rd<'a> {
        Rd { b, p: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.p.checked_add(n).ok_or("索引长度溢出")?;
        if end > self.b.len() {
            return Err("索引文件被截断".to_string());
        }
        let s = &self.b[self.p..end];
        self.p = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn bool(&mut self) -> Result<bool, String> {
        Ok(self.u8()? != 0)
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn len(&mut self) -> Result<usize, String> {
        Ok(self.u32()? as usize)
    }
    fn bytes(&mut self) -> Result<&'a [u8], String> {
        let n = self.len()?;
        self.take(n)
    }
    fn str(&mut self) -> Result<String, String> {
        let b = self.bytes()?;
        String::from_utf8(b.to_vec()).map_err(|_| "索引含非法 UTF-8".to_string())
    }
    fn u32s(&mut self) -> Result<Vec<u32>, String> {
        let n = self.len()?;
        let raw = self.take(n.checked_mul(4).ok_or("索引长度溢出")?)?;
        Ok(raw.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect())
    }
    fn u64s(&mut self) -> Result<Vec<u64>, String> {
        let n = self.len()?;
        let raw = self.take(n.checked_mul(8).ok_or("索引长度溢出")?)?;
        Ok(raw.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect())
    }
    fn strs(&mut self) -> Result<Vec<String>, String> {
        let n = self.len()?;
        let mut out = Vec::with_capacity(n.min(1 << 20));
        for _ in 0..n {
            out.push(self.str()?);
        }
        Ok(out)
    }
}

/// 组帧：magic + version + total_len + 指纹 + 堆布局 + body
fn frame(
    magic: &[u8; 8],
    fp: &Fp,
    layout: &crate::mat_layout::HeapLayout,
    body: Vec<u8>,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + body.len());
    out.extend_from_slice(magic);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&0u64.to_le_bytes()); // 占位，写完回填
    out.extend_from_slice(&fp.path_hash.to_le_bytes());
    out.extend_from_slice(&fp.size.to_le_bytes());
    out.extend_from_slice(&fp.mtime.to_le_bytes());
    // 堆布局（16B）：浅堆口径必须随索引一起走，否则读回的口径与写出的不一致
    out.push(layout.id_width);
    out.push(layout.align);
    out.push(layout.compressed_oops as u8);
    out.push(0); // 对齐填充
    out.extend_from_slice(&layout.instance_header.to_le_bytes());
    out.extend_from_slice(&layout.array_header.to_le_bytes());
    out.extend_from_slice(&layout.class_shallow.to_le_bytes());
    debug_assert_eq!(out.len(), HEADER_LEN);
    out.extend_from_slice(&body);
    let total = out.len() as u64;
    out[12..20].copy_from_slice(&total.to_le_bytes());
    out
}

// ==================== NodeKind / Root 标签编码 ====================

fn kind_code(k: NodeKind) -> u8 {
    match k {
        NodeKind::Instance => 0,
        NodeKind::ObjectArray => 1,
        NodeKind::PrimitiveArray => 2,
        NodeKind::Class => 3,
    }
}

fn kind_from(code: u8) -> Result<NodeKind, String> {
    Ok(match code {
        0 => NodeKind::Instance,
        1 => NodeKind::ObjectArray,
        2 => NodeKind::PrimitiveArray,
        3 => NodeKind::Class,
        _ => return Err(format!("未知节点类型 {code}")),
    })
}

fn root_code(label: &str) -> Result<u8, String> {
    ROOT_KINDS
        .iter()
        .position(|k| *k == label)
        .map(|i| i as u8)
        .ok_or_else(|| format!("未知 GC Root 标签 {label}"))
}

fn root_from(code: u8) -> Result<&'static str, String> {
    ROOT_KINDS.get(code as usize).copied().ok_or_else(|| format!("未知 GC Root 下标 {code}"))
}

// ==================== 图：编码 / 解码 ====================

fn encode_graph(g: &Graph) -> Result<Vec<u8>, String> {
    // 粗估：节点 21B + 边 13B + 元数据字符串。多留 20% 免得反复扩容。
    let est = g.nodes.len() * 24 + g.edge_to.len() * 16 + g.meta.len() * 96;
    let mut w = Wr::with_capacity(est);

    w.u64(g.nodes.len() as u64);
    w.u64(g.meta.len() as u64);
    w.u64(g.edge_to.len() as u64);
    w.u64(g.roots.len() as u64);
    w.u64(g.skipped);
    w.u64(g.field_mismatch);

    for m in &g.meta {
        w.u64(m.obj_id);
        w.str(&m.name);
        w.i32(m.super_idx);
        w.u32(m.instance_size);
        w.u32(m.statics.len() as u32);
        for &t in &m.statics {
            w.u64(t);
        }
        w.u32(m.statics_names.len() as u32);
        for s in &m.statics_names {
            w.str(s);
        }
        w.bytes(&m.fields);
        w.u32(m.fields_names.len() as u32);
        for s in &m.fields_names {
            w.str(s);
        }
        w.bool(m.chain_ok);
        w.u8(m.ref_kind);
    }

    for nd in &g.nodes {
        w.u64(nd.id);
        w.u8(kind_code(nd.kind));
        w.u32(nd.class);
        w.u64(nd.shallow);
    }

    w.u32s(&g.edge_head);
    w.u32s(&g.edge_to);
    w.u32s(&g.edge_next);
    w.u32s(&g.edge_slot);
    w.bytes(&g.edge_kind);

    for (ni, label) in &g.roots {
        w.u32(*ni);
        w.u8(root_code(label)?);
    }
    Ok(w.b)
}

fn decode_graph(body: &[u8]) -> Result<Graph, String> {
    let mut r = Rd::new(body);
    let node_n = r.u64()? as usize;
    let meta_n = r.u64()? as usize;
    let edge_n = r.u64()? as usize;
    let root_n = r.u64()? as usize;
    let skipped = r.u64()?;
    let field_mismatch = r.u64()?;

    // 粗筛：声明的数量不该超过剩余字节能容纳的量（防坏头部直接申请几个 GB）
    if meta_n.saturating_mul(16) > body.len() || node_n.saturating_mul(21) > body.len() {
        return Err("索引头部数量与体积不符".to_string());
    }

    let mut meta: Vec<ClassMeta> = Vec::with_capacity(meta_n);
    for _ in 0..meta_n {
        let obj_id = r.u64()?;
        let name = r.str()?;
        let super_idx = r.i32()?;
        let instance_size = r.u32()?;
        let sn = r.len()?;
        let mut statics = Vec::with_capacity(sn.min(1 << 16));
        for _ in 0..sn {
            statics.push(r.u64()?);
        }
        let statics_names = r.strs()?;
        let fields = r.bytes()?.to_vec();
        let fields_names = r.strs()?;
        let chain_ok = r.bool()?;
        let ref_kind = r.u8()?;
        meta.push(ClassMeta {
            obj_id,
            name,
            super_idx,
            instance_size,
            statics,
            statics_names,
            fields,
            fields_names,
            chain_ok,
            ref_kind,
        });
    }
    if meta.len() < 8 {
        return Err("类元数据不足（缺 primitive 伪类表头）".to_string());
    }

    let mut nodes: Vec<Node> = Vec::with_capacity(node_n);
    for _ in 0..node_n {
        let id = r.u64()?;
        let kind = kind_from(r.u8()?)?;
        let class = r.u32()?;
        let shallow = r.u64()?;
        if class as usize >= meta.len() {
            return Err(format!("节点类索引越界（{class} >= {}）", meta.len()));
        }
        nodes.push(Node { id, kind, class, shallow });
    }

    let edge_head = r.u32s()?;
    let edge_to = r.u32s()?;
    let edge_next = r.u32s()?;
    let edge_slot = r.u32s()?;
    let edge_kind = r.bytes()?.to_vec();

    if edge_head.len() != nodes.len() {
        return Err("邻接表头长度与节点数不符".to_string());
    }
    if edge_to.len() != edge_n
        || edge_next.len() != edge_n
        || edge_slot.len() != edge_n
        || edge_kind.len() != edge_n
    {
        return Err("边数组长度不一致".to_string());
    }
    // 全量校验后续会被当索引用的值：坏索引宁可整份判废，也不能留到查询时 panic
    for (i, &v) in edge_to.iter().enumerate() {
        if v as usize >= nodes.len() {
            return Err(format!("边 {i} 目标越界（{v}）"));
        }
        let nx = edge_next[i];
        if nx != u32::MAX && nx as usize >= edge_n {
            return Err(format!("边 {i} 链表指针越界（{nx}）"));
        }
    }
    for (i, &h) in edge_head.iter().enumerate() {
        if h != u32::MAX && h as usize >= edge_n.max(1) {
            return Err(format!("节点 {i} 首边越界（{h}）"));
        }
    }

    let mut roots: Vec<(u32, &'static str)> = Vec::with_capacity(root_n);
    for _ in 0..root_n {
        let ni = r.u32()?;
        let label = root_from(r.u8()?)?;
        if ni as usize >= nodes.len() {
            return Err(format!("GC Root 节点越界（{ni}）"));
        }
        roots.push((ni, label));
    }

    Ok(Graph {
        nodes,
        meta,
        edge_head,
        edge_to,
        edge_next,
        edge_slot,
        edge_kind,
        roots,
        skipped,
        field_mismatch,
        // 由调用方（`load_graph`）用索引头里的布局覆盖；这里给个安全的默认值
        layout: crate::mat_layout::HeapLayout::from_id_width(8),
    })
}

// ==================== 支配树：编码 / 解码 ====================

fn encode_dom(d: &DomTree, strong_only: bool) -> Vec<u8> {
    let m = d.inv.len();
    let mut w = Wr::with_capacity(m * 34);
    w.bool(strong_only);
    w.u64(d.reach_cnt);
    w.u64(d.reach_shallow);
    w.u32s(&d.inv);
    w.u32s(&d.idom);
    w.u64s(&d.ret_bytes);
    w.u64s(&d.ret_objs);
    w.u32s(&d.child_head);
    w.u32s(&d.child_next);
    // root_mark 位图：reach 域一位一节点
    let bytes = m.div_ceil(8);
    let mut bits = vec![0u8; bytes];
    for (i, &b) in d.root_mark.iter().enumerate() {
        if b {
            bits[i / 8] |= 1 << (i % 8);
        }
    }
    w.bytes(&bits);

    w.u32(d.suspects.len() as u32);
    for s in &d.suspects {
        w.u32(s.rank);
        w.str(&s.obj_id);
        w.str(&s.class_name);
        w.bool(s.is_class);
        w.str(&s.root_kind);
        w.u64(s.shallow_bytes);
        w.u64(s.retained_bytes);
        w.u64(s.retained_objects);
        w.f64(s.pct);
        w.u32(s.path.len() as u32);
        for p in &s.path {
            w.str(&p.class_name);
            w.bool(p.is_root);
        }
    }
    w.b
}

fn decode_dom(body: &[u8], node_n: usize, strong_only: bool) -> Result<DomTree, String> {
    let mut r = Rd::new(body);
    if r.bool()? != strong_only {
        return Err("支配树索引的引用口径与当前配置不同".to_string());
    }
    let reach_cnt = r.u64()?;
    let reach_shallow = r.u64()?;
    let inv = r.u32s()?;
    let idom = r.u32s()?;
    let ret_bytes = r.u64s()?;
    let ret_objs = r.u64s()?;
    let child_head = r.u32s()?;
    let child_next = r.u32s()?;
    let m = inv.len();
    let bit_len = r.len()?;
    let bits = r.take(bit_len)?.to_vec();

    if m < 1 || inv[0] != u32::MAX {
        return Err("支配树 inv 头不合法".to_string());
    }
    if reach_cnt as usize + 1 != m {
        return Err("支配树可达数与 inv 长度不符".to_string());
    }
    for &v in &inv[1..] {
        if v as usize >= node_n {
            return Err(format!("支配树 inv 节点越界（{v} >= {node_n}）"));
        }
    }
    if idom.len() != m || ret_bytes.len() != m || ret_objs.len() != m || child_head.len() != m {
        return Err("支配树数组长度不一致".to_string());
    }
    for (i, &v) in idom.iter().enumerate() {
        if v as usize >= m {
            return Err(format!("idom[{i}] 越界（{v} >= {m}）"));
        }
    }
    if child_next.len() != m {
        // child_next 用「与 child_head 等长的链表」表示：长度应为孩子总数
        return Err("支配树孩子表长度不一致".to_string());
    }
    if bits.len() != m.div_ceil(8) {
        return Err("支配树根标记位图长度不符".to_string());
    }

    // relabel 由 inv 反推，不落盘（省 4B/节点，且不会与 inv 不一致）
    let mut relabel = vec![u32::MAX; node_n];
    for (i, &v) in inv.iter().enumerate().skip(1) {
        relabel[v as usize] = i as u32;
    }
    let mut root_mark = vec![false; m];
    for i in 0..m {
        root_mark[i] = bits[i / 8] & (1 << (i % 8)) != 0;
    }

    let sn = r.len()?;
    let mut suspects = Vec::with_capacity(sn.min(4096));
    for _ in 0..sn {
        let rank = r.u32()?;
        let obj_id = r.str()?;
        let class_name = r.str()?;
        let is_class = r.bool()?;
        let root_kind = r.str()?;
        let shallow_bytes = r.u64()?;
        let retained_bytes = r.u64()?;
        let retained_objects = r.u64()?;
        let pct = r.f64()?;
        let pn = r.len()?;
        let mut path = Vec::with_capacity(pn.min(4096));
        for _ in 0..pn {
            path.push(MatLeakPathNode { class_name: r.str()?, is_root: r.bool()? });
        }
        suspects.push(MatLeakSuspect {
            rank,
            obj_id,
            class_name,
            is_class,
            root_kind,
            shallow_bytes,
            retained_bytes,
            retained_objects,
            pct,
            path,
        });
    }

    // child_next 的每个指针都必须落在孩子链表域内（越界会让遍历死循环 / panic）
    for (i, &v) in child_next.iter().enumerate() {
        if v != u32::MAX && v as usize >= m {
            return Err(format!("支配树孩子指针 {i} 越界（{v}）"));
        }
    }
    for (i, &h) in child_head.iter().enumerate() {
        if h != u32::MAX && h as usize >= m {
            return Err(format!("支配树孩子头 {i} 越界（{h}）"));
        }
    }

    Ok(DomTree {
        inv,
        relabel,
        idom,
        ret_bytes,
        ret_objs,
        child_head,
        child_next,
        root_mark,
        reach_cnt,
        reach_shallow,
        suspects,
    })
}

// ==================== 对外读写 ====================

/// 读图索引。任何异常都返回 `None`（调用方重新解析即可），不阻断主流程。
pub(crate) fn load_graph(path: &str) -> Option<Graph> {
    if !crate::mat_cfg::index_cfg().enabled {
        return None;
    }
    let (mm, off, _, layout) = open_sidecar(path, ".bdtidx", GRAPH_MAGIC)?;
    match decode_graph(&mm[off..]) {
        Ok(mut g) => {
            g.layout = layout; // 浅堆口径以索引头为准（与节点里的 shallow 同源）
            Some(g)
        }
        Err(e) => {
            // 坏索引不删（也许是并发写入中被读到），下次写入会整体覆盖
            eprintln!("[mat] 索引不可用，回退解析: {e}");
            None
        }
    }
}

/// 图索引是否可用（只读头部，供 `mat_refs_build` 跳过「先数对象」的预检）
pub(crate) fn graph_valid(path: &str) -> bool {
    if !crate::mat_cfg::index_cfg().enabled {
        return false;
    }
    open_sidecar(path, ".bdtidx", GRAPH_MAGIC).is_some()
}

/// 落图索引。失败只返回 Err，调用方决定是否忽略（当前一律忽略并继续分析）。
pub(crate) fn save_graph(path: &str, g: &Graph) -> Result<PathBuf, String> {
    let cfg = crate::mat_cfg::index_cfg();
    if !cfg.enabled {
        return Err("索引已禁用".to_string());
    }
    let fp = fingerprint(path).ok_or_else(|| "无法读取源文件元数据".to_string())?;
    if fp.size as f64 > cfg.write_max_dump_gb * 1e9 {
        return Err(format!(
            "源文件 {} 超过索引写入上限 {} GB",
            crate::mat_source::size_hint(fp.size),
            cfg.write_max_dump_gb
        ));
    }
    let body = encode_graph(g)?;
    let bytes = frame(GRAPH_MAGIC, &fp, &g.layout, body);
    write_sidecar(path, ".bdtidx", &bytes)
}

/// 读支配树索引（需与当前会话的可达口径一致，且与图索引同指纹）
pub(crate) fn load_dom(s: &Session) -> Option<std::sync::Arc<DomTree>> {
    if !crate::mat_cfg::index_cfg().enabled {
        return None;
    }
    let (mm, off, _, _) = open_sidecar(&s.path, ".bdtidx.dom", DOM_MAGIC)?;
    let strong_only = crate::mat_refs::session_strong_only(s);
    match decode_dom(&mm[off..], crate::mat_refs::session_graph(s).nodes.len(), strong_only) {
        Ok(d) => {
            // 可达数与当前图对不上 ⇒ 索引来自另一份图（指纹相同但布局变过，理论上不会）
            if d.reach_cnt as usize != crate::mat_refs::reach_stats(s).0 as usize {
                return None;
            }
            Some(std::sync::Arc::new(d))
        }
        Err(e) => {
            eprintln!("[mat] 支配树索引不可用，重新计算: {e}");
            None
        }
    }
}

pub(crate) fn save_dom(s: &Session, d: &DomTree) -> Result<PathBuf, String> {
    let cfg = crate::mat_cfg::index_cfg();
    if !cfg.enabled {
        return Err("索引已禁用".to_string());
    }
    let fp = fingerprint(&s.path).ok_or_else(|| "无法读取源文件元数据".to_string())?;
    if fp.size as f64 > cfg.write_max_dump_gb * 1e9 {
        return Err("源文件超过索引写入上限".to_string());
    }
    let body = encode_dom(d, crate::mat_refs::session_strong_only(s));
    let bytes = frame(DOM_MAGIC, &fp, &crate::mat_refs::session_layout(s), body);
    write_sidecar(&s.path, ".bdtidx.dom", &bytes)
}

// ==================== 命令入口 ====================

/// 索引状态（前端概览页展示）
pub fn mat_index_info(path: &str) -> Result<bdevtools_core::response::resp_data::ResData<IndexInfo>, String> {
    use bdevtools_core::response::resp_data::ResData;
    let cfg = crate::mat_cfg::index_cfg();
    let (g_path, g_bytes, g_ok) = probe(path, ".bdtidx", GRAPH_MAGIC);
    let (d_path, d_bytes, d_ok) = probe(path, ".bdtidx.dom", DOM_MAGIC);
    let note = if !cfg.enabled {
        "索引已关闭（plugins/mat.json 的 indexEnabled = false），每次打开都会重新解析".to_string()
    } else if !g_ok {
        "尚无索引：下次打开该 dump 时生成（分析结束后即可复用）".to_string()
    } else if !d_ok {
        "图索引已就绪；支配树索引在首次做泄漏检测 / 支配树分析后生成".to_string()
    } else {
        "图与支配树索引均已就绪，再次打开可直接复用".to_string()
    };
    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: IndexInfo {
            enabled: cfg.enabled,
            graph_valid: g_ok,
            graph_path: g_path,
            graph_bytes: g_bytes,
            dom_valid: d_ok,
            dom_path: d_path,
            dom_bytes: d_bytes,
            write_max_gb: cfg.write_max_dump_gb,
            note,
        },
    })
}

fn probe(path: &str, suffix: &str, magic: &[u8; 8]) -> (String, u64, bool) {
    // 用 open_sidecar 判定有效性（它会比指纹），再补一次路径与体积给前端展示
    let valid = open_sidecar(path, suffix, magic);
    match valid {
        Some((mm, _, p, _)) => (p.to_string_lossy().to_string(), mm.len() as u64, true),
        None => {
            // 无效但可能存在（指纹过期），仍报出路径与体积，方便用户知道「为什么没命中」
            for cand in candidates(path, suffix) {
                if let Ok(m) = fs::metadata(&cand) {
                    return (cand.to_string_lossy().to_string(), m.len(), false);
                }
            }
            (String::new(), 0, false)
        }
    }
}

/// 删除索引文件（正在被 mmap 的支配树可能删不掉，如实报告跳过项）
pub fn mat_index_clear(path: &str) -> Result<bdevtools_core::response::resp_data::ResData<IndexClearResult>, String> {
    use bdevtools_core::response::resp_data::ResData;
    let mut removed = 0u32;
    let mut removed_bytes = 0u64;
    let mut skipped: Vec<String> = Vec::new();
    for suffix in [".bdtidx", ".bdtidx.dom"] {
        for cand in candidates(path, suffix) {
            let Ok(m) = fs::metadata(&cand) else { continue };
            match fs::remove_file(&cand) {
                Ok(()) => {
                    removed += 1;
                    removed_bytes += m.len();
                }
                Err(e) => skipped.push(format!("{}：{e}", cand.to_string_lossy())),
            }
        }
    }
    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: IndexClearResult { removed, removed_bytes, skipped },
    })
}

// ==================== 单测 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sample_graph() -> Graph {
        let mut meta = Vec::new();
        for n in ["boolean[]", "byte[]", "char[]", "short[]", "int[]", "float[]", "long[]", "double[]"] {
            meta.push(ClassMeta {
                obj_id: 0,
                name: n.to_string(),
                super_idx: -1,
                instance_size: 0,
                statics: Vec::new(),
                statics_names: Vec::new(),
                fields: Vec::new(),
                fields_names: Vec::new(),
                chain_ok: true,
                ref_kind: 0,
            });
        }
        meta.push(ClassMeta {
            obj_id: 0x100,
            name: "demo.Node".to_string(),
            super_idx: -1,
            instance_size: 32,
            statics: vec![0x201, 0x202],
            statics_names: vec!["HEAD".to_string(), "TAIL".to_string()],
            fields: vec![0, 7],
            fields_names: vec!["next".to_string(), "value".to_string()],
            chain_ok: true,
            ref_kind: 0,
        });
        let nodes = vec![
            Node { id: 0x100, kind: NodeKind::Class, class: 8, shallow: 0 },
            Node { id: 0x201, kind: NodeKind::Instance, class: 8, shallow: 32 },
            Node { id: 0x202, kind: NodeKind::Instance, class: 8, shallow: 40 },
            Node { id: 0x300, kind: NodeKind::ObjectArray, class: 8, shallow: 24 },
        ];
        // 0 → 1 → 2, 3 → 2；1 的字段 0 指向 2
        Graph {
            nodes,
            meta,
            edge_head: vec![0, 2, u32::MAX, 3],
            edge_to: vec![1, 2, 2, 2],
            edge_next: vec![u32::MAX, u32::MAX, u32::MAX, u32::MAX],
            edge_slot: vec![0, 1, 0, 0],
            edge_kind: vec![0, 0, 1, 0],
            roots: vec![(0, "thread"), (3, "jni-global")],
            skipped: 7,
            field_mismatch: 2,
            layout: crate::mat_layout::HeapLayout::with_params(8, 8, Some(true)),
        }
    }

    #[test]
    fn graph_roundtrip() {
        let g = sample_graph();
        let body = encode_graph(&g).unwrap();
        let back = decode_graph(&body).unwrap();
        assert_eq!(back.nodes.len(), 4);
        assert_eq!(back.edge_to, g.edge_to);
        assert_eq!(back.edge_kind, g.edge_kind);
        assert_eq!(back.edge_slot, g.edge_slot);
        assert_eq!(back.roots, g.roots);
        assert_eq!(back.skipped, 7);
        assert_eq!(back.field_mismatch, 2);
        assert_eq!(back.meta.len(), g.meta.len());
        assert_eq!(back.meta[8].statics_names, vec!["HEAD", "TAIL"]);
        assert_eq!(back.meta[8].fields, vec![0, 7]);
        assert_eq!(back.nodes[3].kind, NodeKind::ObjectArray);
        assert_eq!(back.nodes[2].shallow, 40);
    }

    #[test]
    fn truncated_body_is_rejected() {
        let g = sample_graph();
        let body = encode_graph(&g).unwrap();
        // 任何截断都必须判废，而不是带着半个数组去解码
        for cut in [0usize, 8, body.len() / 2, body.len() - 1] {
            assert!(decode_graph(&body[..cut]).is_err(), "cut={cut} 应判废");
        }
    }

    #[test]
    fn out_of_range_edge_is_rejected() {
        // 编码器不做业务校验，解码器必须拦住越界值（否则查询时按索引会 panic）
        let mut g = sample_graph();
        g.edge_to[1] = 999;
        let body = encode_graph(&g).unwrap();
        let e = decode_graph(&body).err().expect("越界值必须判废");
        assert!(e.contains("目标越界"), "unexpected: {e}");

        let mut g2 = sample_graph();
        g2.nodes[2].class = 99;
        let body2 = encode_graph(&g2).unwrap();
        assert!(decode_graph(&body2).err().unwrap().contains("类索引越界"));

        let mut g3 = sample_graph();
        g3.roots.push((42, "thread"));
        let body3 = encode_graph(&g3).unwrap();
        assert!(decode_graph(&body3).err().unwrap().contains("GC Root 节点越界"));
    }

    #[test]
    fn dom_roundtrip() {
        let inv = vec![u32::MAX, 1, 2, 3];
        let mut relabel = vec![u32::MAX; 4];
        relabel[1] = 1;
        relabel[2] = 2;
        relabel[3] = 3;
        let d = DomTree {
            inv,
            relabel,
            idom: vec![0, 0, 1, 2],
            ret_bytes: vec![0, 100, 60, 20],
            ret_objs: vec![0, 3, 2, 1],
            child_head: vec![1, 2, 3, u32::MAX],
            child_next: vec![u32::MAX, u32::MAX, u32::MAX, u32::MAX],
            root_mark: vec![true, true, false, false],
            reach_cnt: 3,
            reach_shallow: 100,
            suspects: vec![MatLeakSuspect {
                rank: 1,
                obj_id: "0x201".to_string(),
                class_name: "demo.Node".to_string(),
                is_class: false,
                root_kind: "thread".to_string(),
                shallow_bytes: 32,
                retained_bytes: 100,
                retained_objects: 3,
                pct: 88.5,
                path: vec![
                    MatLeakPathNode { class_name: "thread".to_string(), is_root: true },
                    MatLeakPathNode { class_name: "demo.Node".to_string(), is_root: false },
                ],
            }],
        };
        let body = encode_dom(&d, true);
        let back = decode_dom(&body, 4, true).unwrap();
        assert_eq!(back.inv, d.inv);
        assert_eq!(back.idom, d.idom);
        assert_eq!(back.ret_bytes, d.ret_bytes);
        assert_eq!(back.ret_objs, d.ret_objs);
        assert_eq!(back.root_mark, d.root_mark);
        assert_eq!(back.reach_cnt, 3);
        assert_eq!(back.suspects.len(), 1);
        assert_eq!(back.suspects[0].class_name, "demo.Node");
        assert_eq!(back.suspects[0].path.len(), 2);
        assert!(back.suspects[0].path[0].is_root);
        // relabel 由 inv 反推
        assert_eq!(back.relabel[2], 2);
        assert_eq!(back.relabel[0], u32::MAX);
        // 口径不一致必须判废
        assert!(decode_dom(&body, 4, false).is_err());
    }

    /// 端到端：写盘 → 读回 → 改 mtime 后失效
    #[test]
    fn save_then_load_and_invalidate_on_touch() {
        let dir = std::env::temp_dir().join(format!("mat_index_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dump = dir.join("sample.hprof");
        {
            let mut f = fs::File::create(&dump).unwrap();
            f.write_all(b"JAVA PROFILE 1.0.2\0fake").unwrap();
        }
        let path = dump.to_str().unwrap().to_string();
        // 索引写在 dump 同目录（不依赖 app_dir）
        let g = sample_graph();
        let saved = save_graph(&path, &g).unwrap();
        assert_eq!(saved, PathBuf::from(format!("{path}.bdtidx")));
        assert!(graph_valid(&path));

        let back = load_graph(&path).expect("应命中索引");
        assert_eq!(back.nodes.len(), 4);
        assert_eq!(back.edge_to, g.edge_to);

        // 改文件内容 + mtime ⇒ 指纹变化，必须失效
        std::thread::sleep(std::time::Duration::from_millis(20));
        {
            let mut f = fs::File::create(&dump).unwrap();
            f.write_all(b"JAVA PROFILE 1.0.2\0fake-but-longer").unwrap();
        }
        assert!(!graph_valid(&path), "文件变化后索引必须失效");
        assert!(load_graph(&path).is_none());
        assert!(encode_graph(&sample_graph()).is_ok());

        let _ = fs::remove_file(&saved);
        let _ = fs::remove_file(&dump);
        let _ = fs::remove_dir(&dir);
    }
}
