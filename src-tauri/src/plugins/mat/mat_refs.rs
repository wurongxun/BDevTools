//! # 对象引用追踪（Reference Tracking）
//!
//! 纯业务层模块：在 `mat_graph` 的对象引用图上做**会话式**多轮查询——
//! 类名搜索 → 该类实例样本 → 单个对象详情（outgoing / incoming / Path to GC Roots），
//! 任一引用对象可继续点击追踪（链式导航）。
//!
//! ## 会话缓存
//! 引用追踪的交互是「建图一次、查询多次」，而大 dump 建图可达分钟级，因此本模块
//! 维护 `path → Arc<Session>` 的全局会话表（LRU 逐出，`SESSION_CAP` 上限），
//! `Session` 内：对象图（与泄漏检测共用 `mat_graph::build_graph`，同一 dump 不存两份图）+
//! 若干 `OnceLock` 惰性派生索引：
//! - `idx`：obj_id → 节点索引（二分查找，供 inspect 定位）；
//! - `reachable`：从 GC Roots 正向 BFS 的可达标记（不可达 = 垃圾，无 GC Root 路径）；
//! - `pred`：反向引用索引（谁引用我，incoming 聚合 / Path BFS 共用）；
//! - `class_index`：类名 → 该类全部实例节点（搜索与实例样本）。
//! 惰性构建一次后所有查询毫秒级；`closeDump` 时应调用 `mat_refs_release` 释放。
//!
//! ## 引用点（edge_slot）
//! `mat_graph` 为每条边记录了「引用点」：实例字段在继承链中的位置 / 静态字段下标 /
//! 数组元素下标。本模块据此还原「通过哪个字段引用」，展示与 MAT 的
//! "Path to GC Roots" 引用链口径一致（GC Root →(字段) 对象 → …）。
//!
//! ## 规模保护
//! 引用追踪需要额外物化反向索引与类索引（内存约为泄漏检测的 1.8×），上限设得
//! 比泄漏检测更保守。上限不再是写死常量：按「分析内存预算」换算（见 `mat_cfg`，
//! 默认随本机物理内存自适应，用户可在「分析预算」设置中覆盖），超限时
//! build 返回 `truncated` 标记、查询返回明确提示，不强行分析。

use crate::plugins::mat::mat_graph::{
    build_graph, chain_fields_named, ClassMeta, Graph, Node, NodeKind,
};
use crate::plugins::mat::mat_cfg::{self, MatLimits};
use crate::plugins::mat::mat_hprof::{catch_parse, count_heap_objects, read_head, sniff_format};
use crate::response::resp_data::ResData;
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Mutex, OnceLock};

// ==================== 返回结构（与前端 mat api 字段一一对应，snake_case） ====================

/// build 会话后的图规模与可达性汇总
#[derive(Debug, Serialize)]
pub struct RefBuildInfo {
    pub total_nodes: u64,
    pub total_shallow: u64,
    pub root_nodes: u64,
    pub edge_count: u64,
    pub skipped_edge_count: u64,
    pub field_mismatch: u64,
    pub classes: u64,
    pub reachable_nodes: u64,
    pub reachable_shallow: u64,
    pub unreachable_nodes: u64,
    pub unreachable_shallow: u64,
    /// 是否因规模超限无法提供引用追踪（search/inspect 将返回提示）
    pub truncated: bool,
    /// 是否命中了已有会话缓存（无需重新解析）
    pub cached: bool,
    pub elapsed_ms: u64,
    pub notes: Vec<String>,
}

/// 类名搜索命中行
#[derive(Debug, Serialize)]
pub struct RefClassHit {
    pub class_name: String,
    /// 该类实例数（含对象数组；primitive 数组如 byte[] 亦统计）
    pub count: u64,
    /// 该类全部实例浅堆字节
    pub shallow_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct RefSearchResult {
    pub total: u64,
    pub hits: Vec<RefClassHit>,
    pub elapsed_ms: u64,
}

/// 实例样本行
#[derive(Debug, Serialize)]
pub struct RefInstanceRow {
    /// 对象 id（0x 十六进制展示）
    pub obj_id: String,
    pub shallow_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct RefInstances {
    pub class_name: String,
    /// 该类实例总数
    pub total: u64,
    /// 浅堆最大的前 N 个样本
    pub rows: Vec<RefInstanceRow>,
    pub elapsed_ms: u64,
}

/// outgoing 明细项 / incoming 样例引用点统一描述
#[derive(Debug, Serialize)]
pub struct RefOutItem {
    /// field（字段引用）/ elem（对象数组单元素）/ group（数组元素聚合组）
    pub kind: String,
    /// 字段名 / "[下标]" / 组的目标类名（由 count 配合）
    pub label: String,
    pub target_class: String,
    /// 目标对象 id（0x；group 为该组首个样例对象）
    pub target_obj_id: String,
    pub target_shallow: u64,
    /// 目标节点类型：instance / object-array / primitive-array / class
    pub target_kind: String,
    /// 聚合组元素数（field / elem 恒为 1）
    pub count: u64,
}

/// incoming 聚合：谁引用了我（按引用者类分组）
#[derive(Debug, Serialize)]
pub struct RefIncomingGroup {
    pub class_name: String,
    pub count: u64,
    pub shallow_bytes: u64,
}

/// incoming 样例（每引用者类前若干条），via = 引用点
#[derive(Debug, Serialize)]
pub struct RefIncomingSample {
    pub referrer_obj_id: String,
    pub referrer_class: String,
    /// 引用点：字段名 / <静态>: 字段名 / [下标]
    pub via: String,
    pub referrer_shallow: u64,
}

#[derive(Debug, Serialize)]
pub struct RefPathItem {
    pub class_name: String,
    pub obj_id: String,
    pub is_root: bool,
    /// 本对象如何被链上**前一节点**引用（root 首项为空）
    pub via: String,
}

#[derive(Debug, Serialize)]
pub struct RefObjectDetail {
    pub obj_id: String,
    pub class_name: String,
    /// instance / object-array / primitive-array / class
    pub kind: String,
    pub shallow_bytes: u64,
    pub reachable: bool,
    /// 该对象自身是否为 GC Root
    pub is_root: bool,
    /// GC Root 分类标签（is_root 时有效）
    pub root_kind: String,
    /// outgoing：它引用了谁
    pub outgoing_total: u64,
    pub outgoing_truncated: bool,
    pub outgoing: Vec<RefOutItem>,
    /// incoming：谁引用了它（聚合 + 样例）
    pub incoming_total: u64,
    pub incoming_truncated: bool,
    pub incoming_by_class: Vec<RefIncomingGroup>,
    pub incoming_samples: Vec<RefIncomingSample>,
    /// Path to GC Roots（GC Root → … → 本对象）
    pub path_state: String, // ok / unreachable / truncated / is-root
    pub path_note: String,
    pub path: Vec<RefPathItem>,
    pub elapsed_ms: u64,
}

// ==================== 规模阈值 ====================

// 节点/边上限由 mat_cfg 按「分析内存预算」动态换算（16GB 参考机 ≈ 20M/100M，
// 默认随物理内存自适应、可在前端「分析预算」设置中覆盖）。
/// 会话缓存容量（超限 LRU 逐出最久未用）
const SESSION_CAP: usize = 3;
/// 类名搜索返回命中上限
const SEARCH_LIMIT: usize = 40;
/// 每类实例样本上限（按浅堆降序取前 N）
const INSTANCE_SAMPLE_LIMIT: usize = 300;
/// 实例/类对象出边（字段引用）明细上限
const OUT_ITEM_LIMIT: usize = 400;
/// 对象数组元素引用 ≤ 该值直接列明细，否则按目标类聚合
const ARRAY_DETAIL_THRESHOLD: usize = 60;
/// 数组聚合组输出上限
const ARRAY_GROUP_LIMIT: usize = 30;
/// incoming 聚合组输出上限
const INCOMING_GROUP_LIMIT: usize = 40;
/// 每个引用者类取几个样例
const INCOMING_SAMPLE_PER: usize = 2;
/// Path BFS 访问节点上限（超限标记 truncated，防止巨大环形图把单次查询拖死）
const PATH_VISIT_LIMIT: usize = 3_000_000;

// ==================== 会话缓存 ====================

/// 对象 id → 节点索引（按 id 升序，二分定位）
struct IdxTable {
    pairs: Vec<(u64, u32)>,
}

impl IdxTable {
    fn build(nodes: &[Node]) -> Self {
        let mut pairs: Vec<(u64, u32)> = nodes
            .iter()
            .enumerate()
            .map(|(i, nd)| (nd.id, i as u32))
            .collect();
        pairs.sort_unstable_by_key(|p| p.0);
        IdxTable { pairs }
    }

    fn get(&self, id: u64) -> Option<u32> {
        self.pairs
            .binary_search_by_key(&id, |p| p.0)
            .ok()
            .map(|i| self.pairs[i].1)
    }
}

/// 反向引用索引：pred_head[v] → 引用 v 的对象节点链表
struct PredIndex {
    head: Vec<u32>,
    to: Vec<u32>,
    next: Vec<u32>,
}

fn build_pred(g: &Graph) -> PredIndex {
    let n = g.nodes.len();
    let mut head: Vec<u32> = vec![u32::MAX; n];
    let mut to: Vec<u32> = Vec::with_capacity(g.edge_to.len());
    let mut next: Vec<u32> = Vec::with_capacity(g.edge_to.len());
    for (u, &e0) in g.edge_head.iter().enumerate() {
        let mut e = e0;
        while e != u32::MAX {
            let v = g.edge_to[e as usize] as usize;
            let ne = to.len() as u32;
            to.push(u as u32);
            next.push(head[v]);
            head[v] = ne;
            e = g.edge_next[e as usize];
        }
    }
    PredIndex { head, to, next }
}

/// 类名 → 该类实例节点索引 + 浅堆合计
struct ClassIndex {
    map: HashMap<String, (Vec<u32>, u64)>,
}

fn build_class_index(g: &Graph) -> ClassIndex {
    let mut map: HashMap<String, (Vec<u32>, u64)> = HashMap::new();
    // 预填 primitive 伪类（保证空类也出现在搜索里）
    for m in g.meta.iter().take(8) {
        map.entry(m.name.clone()).or_default();
    }
    for (i, nd) in g.nodes.iter().enumerate() {
        if nd.kind == NodeKind::Class {
            continue; // 类对象不参与类名搜索
        }
        let name = g.meta[nd.class as usize].name.clone();
        let e = map.entry(name).or_default();
        e.0.push(i as u32);
        e.1 += nd.shallow;
    }
    ClassIndex { map }
}

/// 单个 dump 的引用追踪会话（建图一次、查询多次）。
/// `pub(crate)`：泄漏检测（mat_leak）经 `session_graph` / `session_reachable` 复用其图。
pub(crate) struct Session {
    g: Graph,
    truncated: bool,
    /// 建图时的预算上限快照：预算调高后旧截断会话据此自动重建（mat_refs_build 升级判定）
    lim: MatLimits,
    idx: OnceLock<IdxTable>,
    reachable: OnceLock<Vec<bool>>,
    pred: OnceLock<PredIndex>,
    class_index: OnceLock<ClassIndex>,
    /// 继承链字段 memo（chain_fields_named 缓存）
    chain_memo: Mutex<HashMap<usize, Option<Vec<(u8, String)>>>>,
}

impl Session {
    fn new(g: Graph) -> Session {
        let lim = mat_cfg::current_limits();
        let truncated = g.nodes.len() > lim.refs_nodes || g.edge_to.len() > lim.refs_edges;
        Session {
            g,
            truncated,
            lim,
            idx: OnceLock::new(),
            reachable: OnceLock::new(),
            pred: OnceLock::new(),
            class_index: OnceLock::new(),
            chain_memo: Mutex::new(HashMap::new()),
        }
    }

    fn idx(&self) -> &IdxTable {
        self.idx.get_or_init(|| IdxTable::build(&self.g.nodes))
    }

    fn reachable(&self) -> &[bool] {
        self.reachable.get_or_init(|| {
            let n = self.g.nodes.len();
            let mut mark = vec![false; n];
            let mut q: VecDeque<u32> = VecDeque::new();
            for &(ri, _) in self.g.roots.iter() {
                if !mark[ri as usize] {
                    mark[ri as usize] = true;
                    q.push_back(ri);
                }
            }
            while let Some(u) = q.pop_front() {
                let mut e = self.g.edge_head[u as usize];
                while e != u32::MAX {
                    let v = self.g.edge_to[e as usize] as usize;
                    if !mark[v] {
                        mark[v] = true;
                        q.push_back(v as u32);
                    }
                    e = self.g.edge_next[e as usize];
                }
            }
            mark
        })
    }

    fn pred(&self) -> &PredIndex {
        self.pred.get_or_init(|| build_pred(&self.g))
    }

    fn class_index(&self) -> &ClassIndex {
        self.class_index.get_or_init(|| build_class_index(&self.g))
    }

    /// 节点基本展示信息
    fn node_label(&self, ni: u32) -> (String, u64, String) {
        let nd = &self.g.nodes[ni as usize];
        (
            self.g.meta[nd.class as usize].name.clone(),
            nd.shallow,
            kind_str(nd.kind).to_string(),
        )
    }
}

fn kind_str(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Instance => "instance",
        NodeKind::ObjectArray => "object-array",
        NodeKind::PrimitiveArray => "primitive-array",
        NodeKind::Class => "class",
    }
}

// ---- 会话表（全局，LRU）----
struct Entry {
    last: std::time::Instant,
    session: std::sync::Arc<Session>,
}

static SESSIONS: OnceLock<Mutex<HashMap<String, Entry>>> = OnceLock::new();

fn sessions() -> &'static Mutex<HashMap<String, Entry>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 取（或构建）会话：命中刷新 LRU；未命中在锁外构建避免长图解析阻塞其他 dump。
/// `pub(crate)`：泄漏检测（mat_leak）也复用同一会话图，同一 dump 不二次全量解析。
pub(crate) fn ensure_session(path: &str) -> Result<std::sync::Arc<Session>, String> {
    {
        let mut map = sessions().lock().unwrap();
        if let Some(e) = map.get_mut(path) {
            e.last = std::time::Instant::now();
            return Ok(e.session.clone());
        }
    }
    // 锁外建图（可能耗时数十秒级）
    let head = read_head(path, 64)?;
    let (_format, supported) = sniff_format(&head);
    if !supported {
        return Err(format!(
            "不支持的堆转储格式（{}），当前仅支持 JAVA PROFILE 1.0.x",
            String::from_utf8_lossy(&head)
        ));
    }
    let graph = catch_parse(|| build_graph(path))?;
    let session = std::sync::Arc::new(Session::new(graph));

    let mut map = sessions().lock().unwrap();
    if let Some(e) = map.get_mut(path) {
        // 并发双建：他人已插入，用已有的（丢弃自己的，省内存）
        e.last = std::time::Instant::now();
        return Ok(e.session.clone());
    }
    // LRU 逐出
    while map.len() >= SESSION_CAP {
        let oldest = map
            .iter()
            .min_by_key(|(_, e)| e.last)
            .map(|(k, _)| k.clone());
        match oldest {
            Some(k) => {
                map.remove(&k);
            }
            None => break,
        }
    }
    map.insert(path.to_string(), Entry { last: std::time::Instant::now(), session: session.clone() });
    Ok(session)
}

fn release_session(path: &str) -> bool {
    sessions().lock().unwrap().remove(path).is_some()
}

// ---- 跨模块复用（泄漏检测共用同一会话图，避免同 dump 二次全量解析） ----

pub(crate) fn refs_has_session(path: &str) -> bool {
    sessions().lock().unwrap().contains_key(path)
}

pub(crate) fn session_graph(s: &Session) -> &Graph {
    &s.g
}

pub(crate) fn session_reachable(s: &Session) -> &[bool] {
    s.reachable()
}

/// 可达性统计（与泄漏检测 summary 同口径）
fn reach_stats(s: &Session) -> (u64, u64, u64, u64) {
    let reach = s.reachable();
    let mut r_cnt = 0u64;
    let mut r_sh = 0u64;
    let mut u_cnt = 0u64;
    let mut u_sh = 0u64;
    for (i, &ok) in reach.iter().enumerate() {
        if ok {
            r_cnt += 1;
            r_sh += s.g.nodes[i].shallow;
        } else {
            u_cnt += 1;
            u_sh += s.g.nodes[i].shallow;
        }
    }
    (r_cnt, r_sh, u_cnt, u_sh)
}

// ==================== 命令入口 ====================

pub fn mat_refs_build(path: &str) -> Result<ResData<RefBuildInfo>, String> {
    let started = std::time::Instant::now();
    let cached = refs_has_session(path);
    // 冷启动先轻量数对象：节点超限时原逻辑必然截断（节点数 OR 边数超限即截断），
    // 预检直接返回，避免大 dump 白跑一次 GB 级建图。会话已缓存时跳过预检走精确统计。
    let mut s = if cached {
        ensure_session(path)?
    } else {
        let head = read_head(path, 64)?;
        let (_format, supported) = sniff_format(&head);
        if !supported {
            return Err(format!(
                "不支持的堆转储格式（{}），当前仅支持 JAVA PROFILE 1.0.x",
                String::from_utf8_lossy(&head)
            ));
        }
        let cnt = catch_parse(|| count_heap_objects(path))?;
        let lim = mat_cfg::current_limits();
        if cnt > lim.refs_nodes as u64 {
            return Ok(ResData {
                code: 200,
                msg: "ok".to_string(),
                data: RefBuildInfo {
                    total_nodes: cnt,
                    total_shallow: 0,
                    root_nodes: 0,
                    edge_count: 0,
                    skipped_edge_count: 0,
                    field_mismatch: 0,
                    classes: 0,
                    reachable_nodes: 0,
                    reachable_shallow: 0,
                    unreachable_nodes: cnt,
                    unreachable_shallow: 0,
                    truncated: true,
                    cached: false,
                    elapsed_ms: started.elapsed().as_millis() as u64,
                    notes: vec![format!(
                        "对象总数 {} 超过当前分析上限（预算 {} GB → 约 {} 节点 / {} 边；本机物理内存 {} GB）。预检阶段直接跳过全量建图以降低内存峰值；可在「分析预算」设置中调高后重试",
                        cnt, lim.budget_gb, lim.refs_nodes, lim.refs_edges, mat_cfg::physical_mem_gb()
                    )],
                },
            });
        }
        ensure_session(path)?
    };
    // 预算调高后旧截断会话自动升级：释放重建（新预算下可能已可提供引用查询）
    if s.truncated {
        let cur = mat_cfg::current_limits();
        if cur.refs_nodes > s.lim.refs_nodes || cur.refs_edges > s.lim.refs_edges {
            release_session(path);
            s = ensure_session(path)?;
        }
    }
    let mut notes: Vec<String> = Vec::new();
    if s.g.field_mismatch > 0 {
        notes.push(format!(
            "{} 个实例的字段字节与类定义不符，未提取引用（多为继承链元数据缺失的类）",
            s.g.field_mismatch
        ));
    }
    if s.g.skipped > 0 {
        notes.push(format!("{} 条引用指向堆外对象，已忽略", s.g.skipped));
    }
    if s.truncated {
        let lim = mat_cfg::current_limits();
        notes.push(format!(
            "对象图规模超过当前分析上限（预算 {} GB → 约 {} 节点 / {} 边；本机物理内存 {} GB），无法提供引用查询。可在「分析预算」设置中调高后重试",
            lim.budget_gb, lim.refs_nodes, lim.refs_edges, mat_cfg::physical_mem_gb()
        ));
    }
    let (r_cnt, r_sh, u_cnt, u_sh) = reach_stats(&s);
    let total_shallow: u64 = s.g.nodes.iter().map(|nd| nd.shallow).sum();
    let data = RefBuildInfo {
        total_nodes: s.g.nodes.len() as u64,
        total_shallow,
        root_nodes: s.g.roots.len() as u64,
        edge_count: s.g.edge_to.len() as u64,
        skipped_edge_count: s.g.skipped,
        field_mismatch: s.g.field_mismatch,
        classes: s.g.meta.len() as u64,
        reachable_nodes: r_cnt,
        reachable_shallow: r_sh,
        unreachable_nodes: u_cnt,
        unreachable_shallow: u_sh,
        truncated: s.truncated,
        cached,
        elapsed_ms: started.elapsed().as_millis() as u64,
        notes,
    };
    Ok(ResData { code: 200, msg: "ok".to_string(), data })
}

pub fn mat_refs_release(path: &str) -> Result<ResData<bool>, String> {
    let ok = release_session(path);
    Ok(ResData { code: 200, msg: "ok".to_string(), data: ok })
}

/// 类名搜索：大小写不敏感子串匹配，按浅堆合计降序。
pub fn mat_refs_search_classes(
    path: &str,
    query: &str,
    limit: usize,
) -> Result<ResData<RefSearchResult>, String> {
    let started = std::time::Instant::now();
    let s = ensure_session(path)?;
    if s.truncated {
        return Err("对象图规模超过引用追踪上限，无法搜索".to_string());
    }
    let q = query.trim().to_lowercase();
    let mut hits: Vec<RefClassHit> = Vec::new();
    for (name, (nodes, shallow)) in s.class_index().map.iter() {
        let ok = if q.is_empty() {
            true
        } else {
            name.to_lowercase().contains(&q)
        };
        if ok {
            hits.push(RefClassHit {
                class_name: name.clone(),
                count: nodes.len() as u64,
                shallow_bytes: *shallow,
            });
        }
    }
    hits.sort_by(|a, b| b.shallow_bytes.cmp(&a.shallow_bytes).then(b.count.cmp(&a.count)));
    let total = hits.len() as u64;
    let lim = limit.min(SEARCH_LIMIT).max(1);
    hits.truncate(lim);
    let data = RefSearchResult { total, hits, elapsed_ms: started.elapsed().as_millis() as u64 };
    Ok(ResData { code: 200, msg: "ok".to_string(), data })
}

/// 取某类的实例样本（按浅堆降序 Top N）。
pub fn mat_refs_instances(
    path: &str,
    class_name: &str,
    limit: usize,
) -> Result<ResData<RefInstances>, String> {
    let started = std::time::Instant::now();
    let s = ensure_session(path)?;
    if s.truncated {
        return Err("对象图规模超过引用追踪上限，无法查询".to_string());
    }
    let nodes = match s.class_index().map.get(class_name) {
        Some((v, _)) => v,
        None => {
            return Ok(ResData {
                code: 200,
                msg: "ok".to_string(),
                data: RefInstances {
                    class_name: class_name.to_string(),
                    total: 0,
                    rows: Vec::new(),
                    elapsed_ms: started.elapsed().as_millis() as u64,
                },
            });
        }
    };
    let total = nodes.len() as u64;
    // Top-K（小顶堆按浅堆），K ≤ INSTANCE_SAMPLE_LIMIT
    let k = limit.min(INSTANCE_SAMPLE_LIMIT).max(1);
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let mut heap: BinaryHeap<Reverse<(u64, u32)>> = BinaryHeap::with_capacity(k + 1);
    for &ni in nodes.iter() {
        let sh = s.g.nodes[ni as usize].shallow;
        if heap.len() < k {
            heap.push(Reverse((sh, ni)));
        } else if let Some(&Reverse((top_sh, _))) = heap.peek() {
            if sh > top_sh {
                heap.pop();
                heap.push(Reverse((sh, ni)));
            }
        }
    }
    let mut picked: Vec<(u64, u32)> = heap.into_iter().map(|Reverse(p)| p).collect();
    picked.sort_by(|a, b| b.0.cmp(&a.0));
    let rows = picked
        .into_iter()
        .map(|(sh, ni)| RefInstanceRow {
            obj_id: format!("0x{:x}", s.g.nodes[ni as usize].id),
            shallow_bytes: sh,
        })
        .collect();
    let data = RefInstances {
        class_name: class_name.to_string(),
        total,
        rows,
        elapsed_ms: started.elapsed().as_millis() as u64,
    };
    Ok(ResData { code: 200, msg: "ok".to_string(), data })
}

/// 单个对象详情：outgoing / incoming / Path to GC Roots。
pub fn mat_refs_inspect(path: &str, obj_id: &str) -> Result<ResData<RefObjectDetail>, String> {
    let started = std::time::Instant::now();
    let s = ensure_session(path)?;
    if s.truncated {
        return Err("对象图规模超过引用追踪上限，无法查询".to_string());
    }
    let id = parse_obj_id(obj_id)?;
    let ni = match s.idx().get(id) {
        Some(v) => v,
        None => {
            return Err(format!("堆中不存在对象 0x{:x}（可能来自其他 dump）", id));
        }
    };
    let detail = inspect_object(&s, ni, id, started.elapsed().as_millis() as u64);
    Ok(ResData { code: 200, msg: "ok".to_string(), data: detail })
}

/// "0x…" / 纯十六进制 → u64（4/8 字节 id 都按无符号解析）
fn parse_obj_id(s: &str) -> Result<u64, String> {
    let t = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("无效的对象 id：{s}"));
    }
    u64::from_str_radix(t, 16).map_err(|_| format!("无效的对象 id：{s}"))
}

// ==================== inspect 核心 ====================

fn inspect_object(s: &Session, ni: u32, id: u64, elapsed: u64) -> RefObjectDetail {
    let g = &s.g;
    let nd: Node = g.nodes[ni as usize]; // Copy
    let (class_name, shallow, kind) = s.node_label(ni);

    // 自身 root 判定
    let mut is_root = false;
    let mut root_kind = String::new();
    if let Some((_, k)) = g.roots.iter().find(|(ri, _)| *ri == ni) {
        is_root = true;
        root_kind = k.to_string();
    }
    let reachable = s.reachable()[ni as usize];

    // ---- outgoing（它引用了谁）：先收集出边（引用点 slot, 目标），再按类型展示 ----
    let mut out_edges: Vec<(u32, u32)> = Vec::new();
    {
        let mut e = g.edge_head[ni as usize];
        while e != u32::MAX {
            out_edges.push((g.edge_slot[e as usize], g.edge_to[e as usize]));
            e = g.edge_next[e as usize];
        }
    }
    out_edges.sort_by_key(|x| x.0);
    let out_total = out_edges.len();
    let mut out_items: Vec<RefOutItem> = Vec::new();
    let mut out_trunc = false;
    match nd.kind {
        NodeKind::PrimitiveArray => {
            // 基本类型数组无引用
        }
        NodeKind::ObjectArray => {
            if out_total <= ARRAY_DETAIL_THRESHOLD {
                // 元素少：逐个列出（label = 元素下标）
                for (slot, ti) in out_edges {
                    out_items.push(make_out_item(g, "elem", &format!("[{}]", slot), ti, 1));
                }
            } else {
                // 元素多：按目标类聚合，每组给首个样例
                let mut by_class: HashMap<u32, (u64, u32)> = HashMap::new(); // class idx → (count, 样例节点)
                for (_slot, ti) in out_edges.iter() {
                    let cls = g.nodes[*ti as usize].class;
                    let e2 = by_class.entry(cls).or_insert((0, *ti));
                    e2.0 += 1;
                }
                let mut groups: Vec<(u32, u64, u32)> = by_class
                    .into_iter()
                    .map(|(cls, (cnt, sample))| (cls, cnt, sample))
                    .collect();
                groups.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                for (cls, cnt, sample) in groups.into_iter().take(ARRAY_GROUP_LIMIT) {
                    let mut it = make_out_item(g, "group", "", sample, cnt);
                    it.target_class = g.meta[cls as usize].name.clone();
                    it.label = format!("× {} 个 {}", cnt, it.target_class);
                    out_items.push(it);
                }
                out_trunc = out_total > ARRAY_GROUP_LIMIT;
            }
        }
        _ => {
            // 实例（字段引用）/ 类对象（静态引用）：按引用点排序输出明细
            for (slot, ti) in out_edges.into_iter().take(OUT_ITEM_LIMIT) {
                let label = if nd.kind == NodeKind::Class {
                    stat_name(g, &nd, slot)
                } else {
                    field_name(s, &nd, slot)
                };
                out_items.push(make_out_item(g, "field", &label, ti, 1));
            }
            out_trunc = out_total > OUT_ITEM_LIMIT;
        }
    }

    // ---- incoming（谁引用我）----
    let pred = s.pred();
    let mut in_total = 0usize;
    let mut e = pred.head[ni as usize];
    while e != u32::MAX {
        in_total += 1;
        e = pred.next[e as usize];
    }
    // 聚合 by referrer class + 收样例
    let mut by_class: HashMap<u32, (u64, u64, Vec<u32>)> = HashMap::new(); // class idx → (count, shallow, samples)
    {
        let mut e = pred.head[ni as usize];
        while e != u32::MAX {
            let p = pred.to[e as usize] as usize;
            let cls = g.nodes[p].class;
            let en = by_class.entry(cls).or_insert((0, 0, Vec::new()));
            en.0 += 1;
            en.1 += g.nodes[p].shallow;
            if en.2.len() < INCOMING_SAMPLE_PER {
                en.2.push(p as u32);
            }
            e = pred.next[e as usize];
        }
    }
    let mut groups: Vec<(u32, u64, u64, Vec<u32>)> = by_class
        .into_iter()
        .map(|(cls, (cnt, sh, smp))| (cls, cnt, sh, smp))
        .collect();
    groups.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let in_trunc = groups.len() > INCOMING_GROUP_LIMIT;
    let mut in_by_class: Vec<RefIncomingGroup> = Vec::new();
    let mut in_samples: Vec<RefIncomingSample> = Vec::new();
    for (cls, cnt, sh, smp) in groups.into_iter().take(INCOMING_GROUP_LIMIT) {
        in_by_class.push(RefIncomingGroup {
            class_name: g.meta[cls as usize].name.clone(),
            count: cnt,
            shallow_bytes: sh,
        });
        for p in smp {
            let via = locate_ref(s, p, ni);
            in_samples.push(RefIncomingSample {
                referrer_obj_id: format!("0x{:x}", g.nodes[p as usize].id),
                referrer_class: g.meta[g.nodes[p as usize].class as usize].name.clone(),
                via,
                referrer_shallow: g.nodes[p as usize].shallow,
            });
        }
    }

    // ---- Path to GC Roots ----
    let (path_state, path_note, path_items) = path_to_root(s, ni, is_root, reachable);

    RefObjectDetail {
        obj_id: format!("0x{:x}", id),
        class_name,
        kind: kind.to_string(),
        shallow_bytes: shallow,
        reachable,
        is_root,
        root_kind,
        outgoing_total: out_total as u64,
        outgoing_truncated: out_trunc,
        outgoing: out_items,
        incoming_total: in_total as u64,
        incoming_truncated: in_trunc,
        incoming_by_class: in_by_class,
        incoming_samples: in_samples,
        path_state,
        path_note,
        path: path_items,
        elapsed_ms: elapsed,
    }
}

/// 由「引用者节点 + 引用点 slot」构造 outgoing 项
fn make_out_item(g: &Graph, kind: &str, label: &str, ti: u32, count: u64) -> RefOutItem {
    let nd = &g.nodes[ti as usize];
    RefOutItem {
        kind: kind.to_string(),
        label: label.to_string(),
        target_class: g.meta[nd.class as usize].name.clone(),
        target_obj_id: format!("0x{:x}", nd.id),
        target_shallow: nd.shallow,
        target_kind: kind_str(nd.kind).to_string(),
        count,
    }
}

/// 类节点静态字段名（slot = statics 下标）
fn stat_name(g: &Graph, _nd: &Node, slot: u32) -> String {
    // 类节点出边 = 静态引用；静态字段名放在「目标类的 statics_names」不适用——
    // 需要该 class meta 的 statics_names。类节点的 class 字段指向自身 meta。
    let mi = _nd.class as usize;
    let m: &ClassMeta = &g.meta[mi];
    m.statics_names
        .get(slot as usize)
        .cloned()
        .unwrap_or_else(|| "<静态>".to_string())
}

/// 实例字段名（slot = 继承链位置）
fn field_name(s: &Session, nd: &Node, slot: u32) -> String {
    let mi = nd.class as usize;
    let mut memo = s.chain_memo.lock().unwrap();
    let mut busy = HashSet::new();
    match chain_fields_named(mi, &s.g.meta, &mut memo, &mut busy) {
        Some(chain) => chain
            .get(slot as usize)
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| "<字段>".to_string()),
        None => "<字段>".to_string(),
    }
}

/// 引用点标注：对象 p（引用者）通过什么引用指向对象 q
fn locate_ref(s: &Session, p: u32, q: u32) -> String {
    let g = &s.g;
    let pnd = &g.nodes[p as usize];
    // 在 p 的出边中找 target == q 的边，取其引用点
    let mut e = g.edge_head[p as usize];
    while e != u32::MAX {
        if g.edge_to[e as usize] == q {
            let slot = g.edge_slot[e as usize];
            return match pnd.kind {
                NodeKind::Class => stat_name(g, pnd, slot),
                NodeKind::ObjectArray => format!("[{}]", slot),
                NodeKind::Instance => field_name(s, pnd, slot),
                NodeKind::PrimitiveArray => "<引用>".to_string(),
            };
        }
        e = g.edge_next[e as usize];
    }
    "<引用>".to_string()
}

/// GC Root → … → 目标对象 的最短引用路径（反向 BFS）。
/// 返回 (state, note, items)：state ∈ ok / is-root / unreachable / truncated。
fn path_to_root(s: &Session, start: u32, is_root: bool, reachable: bool) -> (String, String, Vec<RefPathItem>) {
    let g = &s.g;
    if is_root {
        let nd = &g.nodes[start as usize];
        let item = RefPathItem {
            class_name: g.meta[nd.class as usize].name.clone(),
            obj_id: format!("0x{:x}", nd.id),
            is_root: true,
            via: String::new(),
        };
        return ("is-root".to_string(), String::new(), vec![item]);
    }
    if !reachable {
        return (
            "unreachable".to_string(),
            "该对象不可达（垃圾），不存在到 GC Roots 的路径".to_string(),
            Vec::new(),
        );
    }
    let pred = s.pred();
    // root 集合（去重）
    let root_set: HashSet<u32> = g.roots.iter().map(|(ri, _)| *ri).collect();
    // parent[x] = y：沿反向边从 y 扩展到达 x（x 更靠近 GC Root）
    let mut parent: HashMap<u32, u32> = HashMap::new();
    let mut q: VecDeque<u32> = VecDeque::new();
    parent.insert(start, u32::MAX);
    q.push_back(start);
    let mut goal: Option<u32> = None;
    let mut visited: usize = 0;
    while let Some(u) = q.pop_front() {
        visited += 1;
        if root_set.contains(&u) {
            goal = Some(u);
            break;
        }
        if visited > PATH_VISIT_LIMIT {
            break;
        }
        let mut e = pred.head[u as usize];
        while e != u32::MAX {
            let p = pred.to[e as usize];
            if !parent.contains_key(&p) {
                parent.insert(p, u);
                q.push_back(p);
            }
            e = pred.next[e as usize];
        }
    }
    let goal = match goal {
        Some(g) => g,
        None => {
            if visited > PATH_VISIT_LIMIT {
                return (
                    "truncated".to_string(),
                    format!("反向搜索超过 {} 个节点未收敛（超大引用环），路径可能不存在或极长", PATH_VISIT_LIMIT),
                    Vec::new(),
                );
            }
            // 理论不可达（与 reachable 冲突，防御）
            return ("unreachable".to_string(), "未找到 GC Root 路径".to_string(), Vec::new());
        }
    };
    // 重建：goal(root) → … → start（天然 GC Root 在前）
    let mut chain: Vec<u32> = Vec::new();
    let mut cur = goal;
    while cur != u32::MAX {
        chain.push(cur);
        if cur == start {
            break;
        }
        cur = match parent.get(&cur) {
            Some(&p) => p,
            None => break,
        };
    }
    let mut items: Vec<RefPathItem> = Vec::with_capacity(chain.len());
    for (i, &nix) in chain.iter().enumerate() {
        let nd = &g.nodes[nix as usize];
        let via = if i == 0 {
            String::new()
        } else {
            // chain[i-1]（更靠 GC Root）引用 chain[i]：引用点为前一跳的边
            locate_ref(s, chain[i - 1], chain[i])
        };
        items.push(RefPathItem {
            class_name: g.meta[nd.class as usize].name.clone(),
            obj_id: format!("0x{:x}", nd.id),
            is_root: root_set.contains(&nix),
            via,
        });
    }
    ("ok".to_string(), String::new(), items)
}


// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- 合成 HPROF 构造（tag 常量见 HPROF 规范 / mat_leak 测试）----
    fn u1(v: &mut Vec<u8>, x: u8) { v.push(x); }
    fn u2(v: &mut Vec<u8>, x: u16) { v.extend_from_slice(&x.to_be_bytes()); }
    fn u4(v: &mut Vec<u8>, x: u32) { v.extend_from_slice(&x.to_be_bytes()); }
    fn u8(v: &mut Vec<u8>, x: u64) { v.extend_from_slice(&x.to_be_bytes()); }
    fn id(v: &mut Vec<u8>, x: u64) { u8(v, x); }
    fn rec(v: &mut Vec<u8>, tag: u8, payload: &[u8]) {
        u1(v, tag);
        u4(v, 0);
        u4(v, payload.len() as u32);
        v.extend_from_slice(payload);
    }

    struct HprofBuilder { f: Vec<u8>, next_name: u64, next_serial: u32 }
    impl HprofBuilder {
        fn new() -> Self {
            let mut f = Vec::new();
            f.extend_from_slice(b"JAVA PROFILE 1.0.2\0");
            u4(&mut f, 8);
            u8(&mut f, 0);
            HprofBuilder { f, next_name: 0x1000, next_serial: 1 }
        }
        fn utf8(&mut self, text: &str) -> u64 {
            let nid = self.next_name;
            self.next_name += 1;
            let mut p = Vec::new();
            id(&mut p, nid);
            p.extend_from_slice(text.as_bytes());
            rec(&mut self.f, 0x01, &p);
            nid
        }
        fn load_class(&mut self, class_obj: u64, name_id: u64) {
            let mut p = Vec::new();
            u4(&mut p, self.next_serial);
            self.next_serial += 1;
            id(&mut p, class_obj);
            u4(&mut p, 0);
            id(&mut p, name_id);
            rec(&mut self.f, 0x02, &p);
        }
        fn heap(&mut self, sub: &[u8]) { rec(&mut self.f, 0x0C, sub); }
        fn into_bytes(self) -> Vec<u8> { self.f }
    }

    /// ClassDump：全静态/实例字段共用同一名字（name id），对象引用类型
    fn sub_class(
        class_obj: u64,
        super_obj: Option<u64>,
        instance_size: u32,
        static_refs: &[u64],
        inst_fields: usize,
        field_name_id: u64,
    ) -> Vec<u8> {
        let mut b = Vec::new();
        u1(&mut b, 0x20);
        id(&mut b, class_obj);
        u4(&mut b, 0);
        match super_obj { Some(s) => id(&mut b, s), None => id(&mut b, 0) }
        id(&mut b, 0); id(&mut b, 0); id(&mut b, 0); id(&mut b, 0); id(&mut b, 0);
        u4(&mut b, instance_size);
        u2(&mut b, 0);
        u2(&mut b, static_refs.len() as u16);
        for &t in static_refs {
            id(&mut b, field_name_id);
            u1(&mut b, 0x02);
            id(&mut b, t);
        }
        u2(&mut b, inst_fields as u16);
        for _ in 0..inst_fields {
            id(&mut b, field_name_id);
            u1(&mut b, 0x02);
        }
        b
    }

    fn sub_instance(obj: u64, class_obj: u64, fields: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        u1(&mut b, 0x21);
        id(&mut b, obj);
        u4(&mut b, 0);
        id(&mut b, class_obj);
        u4(&mut b, fields.len() as u32);
        b.extend_from_slice(fields);
        b
    }

    fn sub_obj_array(obj: u64, elem_class: u64, elems: &[u64]) -> Vec<u8> {
        let mut b = Vec::new();
        u1(&mut b, 0x22);
        id(&mut b, obj);
        u4(&mut b, 0);
        u4(&mut b, elems.len() as u32);
        id(&mut b, elem_class);
        for &e in elems {
            id(&mut b, e);
        }
        b
    }

    fn sub_prim_array(obj: u64, n: usize, ty: u8) -> Vec<u8> {
        let mut b = Vec::new();
        u1(&mut b, 0x23);
        id(&mut b, obj);
        u4(&mut b, 0);
        u4(&mut b, n as u32);
        u1(&mut b, ty);
        let w = if ty == 0x05 { 2 } else { 1 }; // char 2B，其余测试类型 1B
        b.extend(std::iter::repeat(0u8).take(n * w));
        b
    }

    fn sub_root_system_class(obj: u64) -> Vec<u8> {
        let mut b = Vec::new();
        u1(&mut b, 0x05);
        id(&mut b, obj);
        b
    }

    /// 合成 dump（9 节点 / 8 边）：
    ///   Root(0x100, 系统类根, 静态 ref→keeper, ref→array) → keeper(0x201) → leaf(0x301) → char[](0x501)
    ///   Root → array(0x410)[leaf2×3 + keeper]（leaf2 0x302 仅被数组持有）；
    ///   byte[](0x601) 无人引用 → 不可达。leaf(0x301) 的入边唯一（keeper），路径断言确定。
    fn write_synthetic_dump() -> String {
        let mut hb = HprofBuilder::new();
        let n_root = hb.utf8("mem/Root");
        let n_keeper = hb.utf8("mem/Keeper");
        let n_leaf = hb.utf8("mem/Leaf");
        let n_arrcls = hb.utf8("[Lmem/Leaf;");
        let ref_name = hb.utf8("ref");
        hb.load_class(0x100, n_root);
        hb.load_class(0x200, n_keeper);
        hb.load_class(0x300, n_leaf);
        hb.load_class(0x700, n_arrcls);

        let mut h1 = Vec::new();
        h1.extend_from_slice(&sub_root_system_class(0x100));
        // Root：静态引用 keeper + array
        h1.extend_from_slice(&sub_class(0x100, None, 0, &[0x201, 0x410], 0, ref_name));
        // Keeper：实例字段 1（对象引用）
        h1.extend_from_slice(&sub_class(0x200, None, 32, &[], 1, ref_name));
        // Leaf：实例字段 1
        h1.extend_from_slice(&sub_class(0x300, None, 24, &[], 1, ref_name));
        // keeper → leaf(0x301)
        let mut kf = Vec::new();
        id(&mut kf, 0x301);
        h1.extend_from_slice(&sub_instance(0x201, 0x200, &kf));
        // leaf(0x301) → char[]；另一个 leaf(0x302) 无字段引用（仅被数组持有）
        let mut lf = Vec::new();
        id(&mut lf, 0x501);
        h1.extend_from_slice(&sub_instance(0x301, 0x300, &lf));
        h1.extend_from_slice(&sub_instance(0x302, 0x300, &[0u8; 8]));
        // 对象数组：元素 [leaf2, leaf2, leaf2, keeper]（不引用 0x301，保证其路径唯一）
        h1.extend_from_slice(&sub_obj_array(0x410, 0x700, &[0x302, 0x302, 0x302, 0x201]));
        // char[] 2KB（被 leaf 引用）+ 无人引用的 byte[] 1KB
        h1.extend_from_slice(&sub_prim_array(0x501, 2000, 0x05));
        h1.extend_from_slice(&sub_prim_array(0x601, 1000, 0x08));
        hb.heap(&h1);

        let dir = std::env::temp_dir().join("mat_refs_e2e.hprof");
        std::fs::write(&dir, hb.into_bytes()).unwrap();
        dir.to_str().unwrap().to_string()
    }

    #[test]
    fn refs_synthetic_end_to_end() {
        let p = write_synthetic_dump();
        // 清理历史会话，保证从零 build
        let _ = mat_refs_release(&p);

        // ---- build ----
        let b = mat_refs_build(&p).unwrap();
        assert_eq!(b.code, 200);
        let info = &b.data;
        assert_eq!(info.total_nodes, 9, "nodes");
        assert_eq!(info.edge_count, 8, "edges");
        assert_eq!(info.root_nodes, 1);
        assert_eq!(info.reachable_nodes, 6);
        assert_eq!(info.unreachable_nodes, 3);
        assert!(!info.truncated);
        // 超限预检用的轻量计数应与图节点数一致（build 冷启动内部已跑过一次）
        assert_eq!(crate::plugins::mat::mat_hprof::count_heap_objects(&p).unwrap(), 9);

        // ---- 类名搜索 ----
        let r = mat_refs_search_classes(&p, "keeper", 20).unwrap().data;
        assert!(r.hits.iter().any(|h| h.class_name == "mem/Keeper" && h.count == 1));
        let r = mat_refs_search_classes(&p, "[L", 20).unwrap().data;
        assert!(r.hits.iter().any(|h| h.class_name == "[Lmem/Leaf;" && h.count == 1));
        let r = mat_refs_search_classes(&p, "char", 20).unwrap().data;
        assert!(r.hits.iter().any(|h| h.class_name == "char[]" && h.count == 1));

        // ---- 实例样本（按浅堆降序）----
        let inst = mat_refs_instances(&p, "mem/Keeper", 10).unwrap().data;
        assert_eq!(inst.total, 1);
        assert_eq!(inst.rows.len(), 1);
        assert_eq!(inst.rows[0].obj_id, "0x201");
        assert_eq!(inst.rows[0].shallow_bytes, 32);

        // ---- inspect char[] 0x501：出边空、入边来自 leaf、最短路径 Root→keeper→leaf→char[] ----
        let d = mat_refs_inspect(&p, "0x501").unwrap().data;
        assert_eq!(d.kind, "primitive-array");
        assert!(d.reachable);
        assert!(!d.is_root);
        assert_eq!(d.shallow_bytes, 16 + 2000 * 2);
        assert_eq!(d.outgoing_total, 0);
        assert_eq!(d.incoming_total, 1);
        assert_eq!(d.incoming_by_class[0].class_name, "mem/Leaf");
        assert_eq!(d.incoming_by_class[0].count, 1);
        assert_eq!(d.incoming_samples.len(), 1);
        assert_eq!(d.incoming_samples[0].via, "ref");
        assert_eq!(d.path_state, "ok");
        assert_eq!(d.path.len(), 4);
        assert!(d.path[0].is_root);
        assert_eq!(d.path[0].class_name, "mem/Root");
        assert_eq!(d.path[0].via, "");
        assert_eq!(d.path[1].class_name, "mem/Keeper");
        assert_eq!(d.path[1].via, "ref"); // Root 静态字段
        assert_eq!(d.path[2].class_name, "mem/Leaf");
        assert_eq!(d.path[2].via, "ref"); // keeper 字段
        assert_eq!(d.path[3].class_name, "char[]");
        assert_eq!(d.path[3].obj_id, "0x501");
        assert_eq!(d.path[3].via, "ref"); // leaf 字段

        // ---- inspect keeper 0x201：出边 1 字段、入边来自 Root(静态) + 数组(下标 [3]) ----
        let d = mat_refs_inspect(&p, "0x201").unwrap().data;
        assert_eq!(d.kind, "instance");
        assert_eq!(d.class_name, "mem/Keeper");
        assert_eq!(d.outgoing_total, 1);
        assert_eq!(d.outgoing[0].label, "ref");
        assert_eq!(d.outgoing[0].target_class, "mem/Leaf");
        assert_eq!(d.outgoing[0].target_obj_id, "0x301");
        assert_eq!(d.incoming_total, 2);
        let g1 = d.incoming_by_class.iter().find(|g| g.class_name == "mem/Root").unwrap();
        assert_eq!(g1.count, 1);
        let g2 = d.incoming_by_class.iter().find(|g| g.class_name == "[Lmem/Leaf;").unwrap();
        assert_eq!(g2.count, 1);
        // 数组的引用点应标出下标 [3]
        let smp = d.incoming_samples.iter().find(|s| s.referrer_obj_id == "0x410").unwrap();
        assert_eq!(smp.via, "[3]");
        // 类对象（静态）引用点
        let smp = d.incoming_samples.iter().find(|s| s.referrer_obj_id == "0x100").unwrap();
        assert_eq!(smp.via, "ref");

        // ---- inspect 对象数组 0x410：出边 4 元素逐个列出 ----
        let d = mat_refs_inspect(&p, "0x410").unwrap().data;
        assert_eq!(d.kind, "object-array");
        assert_eq!(d.outgoing_total, 4);
        assert!(!d.outgoing_truncated);
        assert_eq!(d.outgoing[0].label, "[0]");
        assert_eq!(d.outgoing[0].target_class, "mem/Leaf");
        assert_eq!(d.outgoing[3].label, "[3]");
        assert_eq!(d.outgoing[3].target_class, "mem/Keeper");

        // ---- inspect 类对象 Root 0x100：自身是 GC Root → path_state = is-root ----
        let d = mat_refs_inspect(&p, "0x100").unwrap().data;
        assert_eq!(d.kind, "class");
        assert!(d.is_root);
        assert_eq!(d.path_state, "is-root");
        assert_eq!(d.outgoing_total, 2);

        // ---- 不可达 byte[] 0x601 ----
        let d = mat_refs_inspect(&p, "0x601").unwrap().data;
        assert!(!d.reachable);
        assert_eq!(d.path_state, "unreachable");
        assert!(d.path.is_empty());

        // ---- 未知对象 / 坏 id ----
        assert!(mat_refs_inspect(&p, "0xffff").is_err());
        assert!(mat_refs_inspect(&p, "zz").is_err());

        // ---- 释放 ----
        assert!(mat_refs_release(&p).unwrap().data);
        assert!(!mat_refs_release(&p).unwrap().data);

        let _ = std::fs::remove_file(&p);
    }

    /// 真实 HotSpot dump 集成探针（cargo test real_dump_probe -- --ignored --nocapture）
    /// 前提：mat-realcheck/leak_demo.hprof 已用 LeakDemo.java 生成。
    #[test]
    #[ignore]
    fn real_dump_probe() {
        let p = "D:/ideaProject/BackendDevelopmentTools/mat-realcheck/leak_demo.hprof";
        let _ = mat_refs_release(p);
        let info = mat_refs_build(p).unwrap().data;
        println!(
            "build: nodes={} shallow={} roots={} edges={} reachable={} unreachable={} classes={} cached={} truncated={} elapsed={}ms",
            info.total_nodes, info.total_shallow, info.root_nodes, info.edge_count,
            info.reachable_nodes, info.unreachable_nodes, info.classes, info.cached,
            info.truncated, info.elapsed_ms
        );
        for n in &info.notes {
            println!("note: {n}");
        }
        assert!(!info.truncated, "synthetic dump 不应超限");

        // LeakDemo 类无实例（纯静态持有），类名搜索命中数应为 0
        let hits = mat_refs_search_classes(p, "leakdemo", 20).unwrap().data;
        assert!(
            hits.hits.is_empty(),
            "LeakDemo 无实例，搜索应为空：{}",
            serde_json::to_string(&hits.hits).unwrap()
        );
        // 展示根 char[]（main 栈局部）→ is-root
        let hit = mat_refs_search_classes(p, "char[]", 20).unwrap().data;
        let hit = hit.hits.iter().find(|h| h.class_name == "char[]").unwrap();
        println!("char[] count={} shallow={}", hit.count, hit.shallow_bytes);
        let inst = mat_refs_instances(p, "char[]", 5).unwrap().data;
        let top_obj = inst.rows[0].obj_id.clone();
        let d = mat_refs_inspect(p, &top_obj).unwrap().data;
        println!(
            "root char[] {}: shallow={} path_state={} path_len={}",
            top_obj, d.shallow_bytes, d.path_state, d.path.len()
        );
        assert_eq!(d.path_state, "is-root");

        // 选一个被静态持有的大 byte[]（LeakDemo.BIG 2MB）做 inspect → 长路径 + incoming
        let inst = mat_refs_instances(p, "byte[]", 10).unwrap().data;
        assert!(!inst.rows.is_empty(), "byte[] 应有实例");
        let top_obj = inst.rows[0].obj_id.clone();
        println!("top byte[]: obj={} shallow={}", top_obj, inst.rows[0].shallow_bytes);
        let d = mat_refs_inspect(p, &top_obj).unwrap().data;
        println!(
            "inspect {}: kind={} class={} shallow={} reachable={} outgoing={} incoming={} path_state={} path_len={}",
            d.obj_id, d.kind, d.class_name, d.shallow_bytes, d.reachable,
            d.outgoing_total, d.incoming_total, d.path_state, d.path.len()
        );
        let chain: Vec<String> = d
            .path
            .iter()
            .map(|x| format!("{}{}", x.class_name, if x.is_root { "*" } else { "" }))
            .collect();
        println!("path: {}", chain.join(" -> "));
        for g in d.incoming_by_class.iter().take(5) {
            println!("incoming: {} × {}", g.class_name, g.count);
        }
        assert_eq!(d.path_state, "ok", "被持有的 byte[] 应有 GC Root 路径");
        assert!(d.path.len() >= 2);
        assert!(
            d.path.last().unwrap().obj_id == d.obj_id,
            "路径末节点应是被检查对象自身"
        );

        // 双 build 命中缓存
        let info2 = mat_refs_build(p).unwrap().data;
        assert!(info2.cached);
        let _ = mat_refs_release(p);
    }
}
