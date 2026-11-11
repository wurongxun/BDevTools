//! # 支配树（Dominator Tree）会话缓存与查询
//!
//! 纯业务层模块，不依赖 `tauri`。动机：Leak Suspects 原本在 `mat_leak::analyze_suspects`
//! 里跑完 Lengauer-Tarjan、算出 Retained Heap 后**整棵支配树即被丢弃**，导致
//! Dominator Tree 视图 / 直方图 Retained 列 / Top Consumers 都得重算（大堆不可接受）。
//!
//! 本模块把支配结果提升到 `mat_refs::Session` 的 `OnceLock<Arc<DomTree>>`：
//! - **同一 dump 只算一次 LT**，泄漏检测与支配树查询共用；
//! - 缓存随 `Session` 生命周期走 —— 预算升级触发会话重建时（`mat_refs_build` 的
//!   `release_session` + `ensure_session`）新 Session 的 OnceLock 为空，天然避免
//!   「新算 leak + 旧 dom」错配，不需要额外的失效广播；
//! - 计算过程中的**反向边 CSR（pred_*）只在生成 GC Root 路径前缀时用一次**，
//!   不进缓存（那是 LT 阶段内存大头），缓存的是精简的支配树结构。
//!
//! ## 口径
//! - `retained(x)` = x 及仅由 x 支配的子树对象浅堆之和（与 Eclipse MAT 一致）。
//! - 分组视图（class / package）的 retained 是**各对象 retained 之和**，含支配重叠，
//!   只用于排序与相对比较，`note` 会明确提示，不可与堆总量直接相加。
//! - 规模闸走 `mat_cfg::current_limits()`（`dom_nodes` / `dom_edges`），不在此写裸常量。

use crate::mat_cfg;
use crate::mat_graph::{Graph, NodeKind};
use crate::mat_leak::{lengauer_tarjan, MatLeakPathNode, MatLeakSuspect};
use crate::mat_refs::{
    ensure_session, reach_stats, session_dom, session_graph, session_node_by_id,
    session_reachable, session_set_dom, Session,
};
use bdevtools_core::response::resp_data::ResData;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// ==================== 返回结构（与前端 matApi.ts 一一对应，snake_case） ====================

/// 支配树的一行：对象级或分组聚合级
#[derive(Debug, Serialize)]
pub struct DomRow {
    /// 对象 id（0x 十六进制）；分组行为空
    pub obj_id: String,
    pub class_name: String,
    /// instance / object-array / primitive-array / class；分组行为 "group"
    pub kind: String,
    pub shallow_bytes: u64,
    pub retained_bytes: u64,
    pub retained_objects: u64,
    /// retained / 可达堆浅堆 的百分比
    pub pct: f64,
    /// 支配树直接孩子数（分组行恒 0）
    pub children: u64,
    /// 该对象自身是否为 GC Root
    pub is_root: bool,
    /// 支配者对象 id（超根直接孩子为空串）
    pub parent_obj_id: String,
}

#[derive(Debug, Serialize)]
pub struct DomTreeResult {
    /// 分页前的总行数
    pub total: u64,
    pub rows: Vec<DomRow>,
    pub reach_nodes: u64,
    pub reach_shallow: u64,
    /// 因规模闸 / 无 GC Roots 未提供支配树（rows 为空，note 说明原因）
    pub truncated: bool,
    pub note: String,
    pub cached: bool,
    pub elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct RetainedSetResult {
    pub obj_id: String,
    /// 支配子树对象数（含自身）
    pub total: u64,
    /// 子树浅堆合计（= 该对象 retained_bytes）
    pub bytes: u64,
    /// 按 retained 降序的 Top N 成员
    pub rows: Vec<DomRow>,
    pub truncated: bool,
    pub note: String,
    pub elapsed_ms: u64,
}

// ==================== 会话缓存中的支配树 ====================

/// LT 结果（relabel 域：0 = 超根，1..=reach_cnt 为可达对象）
pub(crate) struct DomTree {
    /// rl → 原节点索引（inv[0] = u32::MAX）
    pub inv: Vec<u32>,
    /// 原节点索引 → rl（不可达 = u32::MAX）
    pub relabel: Vec<u32>,
    /// rl → 最近支配者 rl
    pub idom: Vec<u32>,
    /// rl → retained 字节 / 对象数
    pub ret_bytes: Vec<u64>,
    pub ret_objs: Vec<u64>,
    /// 支配树孩子链表（头插 CSR）
    pub child_head: Vec<u32>,
    pub child_next: Vec<u32>,
    /// rl 是否为 GC Root
    pub root_mark: Vec<bool>,
    pub reach_cnt: u64,
    pub reach_shallow: u64,
    /// Leak Suspects（生成时必须用到临时 pred_*，故在此一次算好并缓存）
    pub suspects: Vec<MatLeakSuspect>,
}

/// 支配树可用性：`Unavailable` 不是错误，前端要显示「为什么没有结果」而不是红字报错。
pub(crate) enum DomReady {
    Ready(Arc<DomTree>),
    Unavailable(String),
}

/// 取（或计算）会话的支配树。并发下用 Session 内的锁避免同一 dump 重复跑 LT。
pub(crate) fn ensure_dom(s: &Arc<Session>) -> Result<DomReady, String> {
    if let Some(d) = session_dom(s) {
        return Ok(DomReady::Ready(d));
    }
    let _guard = s.dom_lock.lock().map_err(|_| "支配树计算锁已失效".to_string())?;
    if let Some(d) = session_dom(s) {
        return Ok(DomReady::Ready(d));
    }

    // 本地支配树索引（`.bdtidx.dom`）：与图同指纹且引用口径一致才有意义，
    // 命中直接省掉一次分钟级的 Lengauer-Tarjan。校验失败返回 None，走正常计算。
    if let Some(d) = crate::mat_index::load_dom(s) {
        session_set_dom(s, d.clone());
        return Ok(DomReady::Ready(d));
    }

    let g = session_graph(s);
    let (r_cnt, r_sh, _, _) = reach_stats(s);
    let lim = mat_cfg::current_limits();
    if r_cnt == 0 {
        return Ok(DomReady::Unavailable("未发现任何 GC Roots，无法计算支配树".to_string()));
    }
    if s.truncated {
        return Ok(DomReady::Unavailable(format!(
            "对象图规模超过引用追踪上限（约 {} 节点 / {} 边），无法提供支配树。可在「分析预算」设置中调高后重试",
            lim.refs_nodes, lim.refs_edges
        )));
    }
    if r_cnt as usize > lim.dom_nodes || g.edge_to.len() > lim.dom_edges {
        return Ok(DomReady::Unavailable(format!(
            "可达对象 {}（边 {}）超过支配树闸（预算 {} GB → 约 {} 可达对象 / {} 边），已跳过支配树分析。可在「分析预算」设置中调高后重试",
            r_cnt, g.edge_to.len(), lim.budget_gb, lim.dom_nodes, lim.dom_edges
        )));
    }

    let tree = Arc::new(compute_dom(
        g,
        session_reachable(s),
        r_cnt as usize,
        r_sh,
        crate::mat_refs::session_strong_only(s),
    ));
    // 落支配树索引：失败只影响下次打开速度，不影响本次结果
    let _ = crate::mat_index::save_dom(s, &tree);
    session_set_dom(s, tree.clone());
    Ok(DomReady::Ready(tree))
}

// ==================== 支配树计算（LT + retained + 嫌疑报告） ====================

/// 跑一次 Lengauer-Tarjan，产出可复用的支配树（含 Leak Suspects）。
///
/// 预计内存峰值与原 `analyze_suspects` 一致：pred_* 反向 CSR 与 relabel 域数组都是临时的，
/// 只有精简的 DomTree 常驻。调用方需已通过 `mat_cfg` 的规模闸。
pub(crate) fn compute_dom(
    g: &Graph,
    reachable: &[bool],
    reach_cnt: usize,
    reach_shallow: u64,
    strong_only: bool,
) -> DomTree {
    let nodes = &g.nodes;
    let meta = &g.meta;
    let edge_head = &g.edge_head;
    let edge_to = &g.edge_to;
    let edge_next = &g.edge_next;
    let roots = &g.roots;

    crate::mat_progress::stage(
        "dom",
        Some(reach_cnt as u64),
        format!("计算支配树：{} 个可达对象", reach_cnt),
    );

    // ---- 可达节点重标号：0 = 超根，1..=reach_cnt = 原节点 ----
    let nn = nodes.len();
    let mut relabel: Vec<u32> = vec![u32::MAX; nn];
    let mut inv: Vec<u32> = Vec::with_capacity(reach_cnt + 1);
    inv.push(u32::MAX);
    for i in 0..nn {
        if reachable[i] {
            relabel[i] = inv.len() as u32;
            inv.push(i as u32);
        }
    }
    let m = reach_cnt + 1;

    // ---- relabel 域反向边（LT semi 阶段 + GC Root 路径前缀 BFS 共用）----
    let mut pred_head: Vec<u32> = vec![u32::MAX; m];
    let mut pred_to: Vec<u32> = Vec::new();
    let mut pred_next: Vec<u32> = Vec::new();
    macro_rules! add_pred {
        ($to:expr, $from:expr) => {{
            let _e = pred_to.len() as u32;
            pred_to.push($from);
            pred_next.push(pred_head[$to as usize]);
            pred_head[$to as usize] = _e;
        }};
    }
    for &(ri, _) in roots.iter() {
        let a = relabel[ri as usize];
        if a != u32::MAX && a != 0 {
            add_pred!(a, 0);
        }
    }
    for u in 0..nn {
        crate::mat_progress::tick(
            "dom",
            u as u64,
            Some(reach_cnt as u64),
            "构建反向边（semi 阶段前置）",
        );
        let a = relabel[u];
        if a == u32::MAX {
            continue;
        }
        let mut e = edge_head[u];
        while e != u32::MAX {
            // 强引用口径：弱/软/虚/终结器引用不参与支配关系与 retained 计算
            if !strong_only || g.edge_is_strong(e) {
                let v = edge_to[e as usize] as usize;
                let b = relabel[v];
                if b != u32::MAX && b != a {
                    add_pred!(b, a);
                }
            }
            e = edge_next[e as usize];
        }
    }

    let mut roots_rl: Vec<u32> = Vec::with_capacity(roots.len());
    let mut root_set: HashSet<u32> = HashSet::with_capacity(roots.len());
    for &(ri, _) in roots.iter() {
        let a = relabel[ri as usize];
        if a != u32::MAX && a != 0 && root_set.insert(a) {
            roots_rl.push(a);
        }
    }

    crate::mat_progress::flush(
        "dom",
        nn as u64,
        Some(reach_cnt as u64),
        "反向边就绪，进入 Lengauer-Tarjan",
    );
    crate::mat_progress::stage("dom-lt", None, "Lengauer-Tarjan 求直接支配者");
    let idom = lengauer_tarjan(
        m, &relabel, &inv, edge_head, edge_to, edge_next, &roots_rl,
        &pred_head, &pred_to, &pred_next,
    );
    if idom.len() != m {
        // 理论上不会发生；返回一棵空树，让上层显示「无法计算」而不是 panic。
        return DomTree {
            inv, relabel, idom: vec![0; m], ret_bytes: vec![0; m], ret_objs: vec![0; m],
            child_head: vec![u32::MAX; m], child_next: vec![u32::MAX; m],
            root_mark: vec![false; m], reach_cnt: reach_cnt as u64, reach_shallow,
            suspects: Vec::new(),
        };
    }

    crate::mat_progress::stage("dom-retained", None, "累加 Retained Heap");
    // ---- 支配树孩子链表 + retained 自底向上累加 ----
    let mut child_head: Vec<u32> = vec![u32::MAX; m];
    let mut child_next: Vec<u32> = vec![u32::MAX; m];
    for i in 1..m {
        let p = idom[i] as usize;
        if p < m && p != i {
            child_next[i] = child_head[p];
            child_head[p] = i as u32;
        }
    }
    let mut ret_bytes: Vec<u64> = vec![0; m];
    let mut ret_objs: Vec<u64> = vec![0; m];
    for i in 1..m {
        let orig = inv[i as usize] as usize;
        ret_bytes[i] = nodes[orig].shallow;
        ret_objs[i] = 1;
    }
    let mut order: Vec<u32> = Vec::with_capacity(m);
    order.push(0);
    let mut qi = 0usize;
    while qi < order.len() {
        let u = order[qi] as usize;
        qi += 1;
        let mut c = child_head[u];
        while c != u32::MAX {
            order.push(c);
            c = child_next[c as usize];
        }
    }
    for &u in order.iter().skip(1).rev() {
        let p = idom[u as usize] as usize;
        if p < m && p != u as usize {
            ret_bytes[p] += ret_bytes[u as usize];
            ret_objs[p] += ret_objs[u as usize];
        }
    }

    let suspects = build_suspects(
        nodes, meta, &relabel, &inv, &idom, &ret_bytes, &ret_objs,
        &pred_head, &pred_to, &pred_next, &root_set, roots, m, reach_shallow,
    );

    let mut root_mark = vec![false; m];
    for &r in roots_rl.iter() {
        root_mark[r as usize] = true;
    }

    DomTree {
        inv, relabel, idom, ret_bytes, ret_objs, child_head, child_next,
        root_mark, reach_cnt: reach_cnt as u64, reach_shallow, suspects,
    }
}

/// Top N 泄漏嫌疑 + 「GC Root → … → 嫌疑」支配链（原 mat_leak::analyze_suspects 后半段）。
const SUSPECT_LIMIT: usize = 25;

fn build_suspects(
    nodes: &[crate::mat_graph::Node],
    meta: &[crate::mat_graph::ClassMeta],
    relabel: &[u32],
    inv: &[u32],
    idom: &[u32],
    ret_bytes: &[u64],
    ret_objs: &[u64],
    pred_head: &[u32],
    pred_to: &[u32],
    pred_next: &[u32],
    root_set: &HashSet<u32>,
    roots: &[(u32, &'static str)],
    m: usize,
    reach_shallow: u64,
) -> Vec<MatLeakSuspect> {
    struct Cand {
        rl: u32,
        retained: u64,
        objs: u64,
    }
    let mut cands: Vec<Cand> = Vec::new();
    for i in 1..m {
        if root_set.contains(&(i as u32)) || ret_bytes[i] == 0 {
            continue;
        }
        cands.push(Cand { rl: i as u32, retained: ret_bytes[i], objs: ret_objs[i] });
    }
    cands.sort_by(|a, b| b.retained.cmp(&a.retained).then(b.objs.cmp(&a.objs)));
    if cands.len() > SUSPECT_LIMIT {
        cands.truncate(SUSPECT_LIMIT);
    }

    let class_name_of = |orig: usize| -> String {
        let nd = &nodes[orig];
        meta[nd.class as usize].name.clone()
    };

    /// 沿 preds（a ∈ pred[b] ⇔ a 引用 b，朝超根方向）从链顶找最近 GC Root。
    fn bfs_root_prefix(
        start: u32,
        pred_head: &[u32],
        pred_to: &[u32],
        pred_next: &[u32],
        root_set: &HashSet<u32>,
        m: usize,
    ) -> Option<Vec<u32>> {
        let mut parent: HashMap<u32, u32> = HashMap::new();
        let mut seen: Vec<bool> = vec![false; m];
        let mut queue: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
        seen[start as usize] = true;
        queue.push_back(start);
        let mut scanned = 0usize;
        while let Some(u) = queue.pop_front() {
            scanned += 1;
            if scanned > 200_000 {
                return None;
            }
            if root_set.contains(&u) {
                let mut path = vec![u];
                let mut cur = u;
                while let Some(&p) = parent.get(&cur) {
                    path.push(p);
                    cur = p;
                }
                path.pop();
                return Some(path);
            }
            let mut e = pred_head[u as usize];
            while e != u32::MAX {
                let a = pred_to[e as usize];
                if !seen[a as usize] {
                    seen[a as usize] = true;
                    parent.insert(a, u);
                    queue.push_back(a);
                }
                e = pred_next[e as usize];
            }
        }
        None
    }

    let mut out: Vec<MatLeakSuspect> = Vec::with_capacity(cands.len());
    for (rank, c) in cands.iter().enumerate() {
        let orig = inv[c.rl as usize] as usize;
        let nd = &nodes[orig];

        let mut chain: Vec<u32> = vec![c.rl];
        let mut cur = c.rl;
        loop {
            let up = idom[cur as usize];
            if up == 0 || up == cur {
                break;
            }
            chain.push(up);
            cur = up;
        }
        chain.reverse();
        let top = *chain.first().unwrap();

        let mut prefix: Vec<u32> = Vec::new();
        if !root_set.contains(&top) {
            if let Some(pr) = bfs_root_prefix(top, pred_head, pred_to, pred_next, root_set, m) {
                prefix = pr;
            }
        }
        let kind_of = |rl: u32| -> &'static str {
            roots
                .iter()
                .find(|&&(ri, _)| relabel[ri as usize] == rl)
                .map(|&(_, k)| k)
                .unwrap_or("gc-root")
        };
        let root_kind = if !prefix.is_empty() { kind_of(prefix[0]) } else { kind_of(top) };

        let mut path: Vec<MatLeakPathNode> = Vec::with_capacity(prefix.len() + chain.len());
        let mut pos = 0usize;
        for &rl in prefix.iter().chain(chain.iter()) {
            let o = inv[rl as usize] as usize;
            path.push(MatLeakPathNode {
                class_name: class_name_of(o),
                is_root: pos == 0,
            });
            pos += 1;
        }

        let pct = if reach_shallow > 0 {
            (c.retained as f64) / (reach_shallow as f64) * 100.0
        } else {
            0.0
        };
        out.push(MatLeakSuspect {
            rank: (rank + 1) as u32,
            obj_id: format!("0x{:x}", nd.id),
            class_name: class_name_of(orig),
            is_class: nd.kind == NodeKind::Class,
            root_kind: root_kind.to_string(),
            shallow_bytes: nd.shallow,
            retained_bytes: c.retained,
            retained_objects: c.objs,
            pct,
            path,
        });
    }
    out
}

// ==================== 查询辅助 ====================

fn kind_str(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Instance => "instance",
        NodeKind::ObjectArray => "object-array",
        NodeKind::PrimitiveArray => "primitive-array",
        NodeKind::Class => "class",
    }
}

fn obj_id_hex(nodes: &[crate::mat_graph::Node], ni: usize) -> String {
    format!("0x{:x}", nodes[ni].id)
}

fn class_of(g: &Graph, ni: usize) -> &str {
    let ci = g.nodes[ni].class as usize;
    g.meta.get(ci).map(|m| m.name.as_str()).unwrap_or("?")
}

/// group_by = class / package 时的聚合键
fn group_key(g: &Graph, ni: usize, group_by: &str) -> String {
    let name = class_of(g, ni);
    if group_by == "package" {
        // HPROF 类名多为 "java/util/HashMap" 或 "java.util.HashMap"，统一取包名
        let norm = name.replace('.', "/");
        match norm.rfind('/') {
            Some(i) => norm[..i].to_string(),
            None => "(default)".to_string(),
        }
    } else {
        name.to_string()
    }
}

fn row_from_rl(g: &Graph, d: &DomTree, rl: u32) -> DomRow {
    let ni = d.inv[rl as usize] as usize;
    let nd = &g.nodes[ni];
    let pct = if d.reach_shallow > 0 {
        d.ret_bytes[rl as usize] as f64 / d.reach_shallow as f64 * 100.0
    } else {
        0.0
    };
    let mut children = 0u64;
    let mut c = d.child_head[rl as usize];
    while c != u32::MAX {
        children += 1;
        c = d.child_next[c as usize];
    }
    let p = d.idom[rl as usize] as usize;
    let parent_obj_id = if p == 0 || p >= d.inv.len() {
        String::new()
    } else {
        obj_id_hex(&g.nodes, d.inv[p] as usize)
    };
    DomRow {
        obj_id: obj_id_hex(&g.nodes, ni),
        class_name: class_of(g, ni).to_string(),
        kind: kind_str(nd.kind).to_string(),
        shallow_bytes: nd.shallow,
        retained_bytes: d.ret_bytes[rl as usize],
        retained_objects: d.ret_objs[rl as usize],
        pct,
        children,
        is_root: d.root_mark[rl as usize],
        parent_obj_id,
    }
}

fn empty_result(note: String, elapsed_ms: u64) -> ResData<DomTreeResult> {
    ResData {
        code: 200,
        msg: "ok".to_string(),
        data: DomTreeResult {
            total: 0,
            rows: Vec::new(),
            reach_nodes: 0,
            reach_shallow: 0,
            truncated: true,
            note,
            cached: false,
            elapsed_ms,
        },
    }
}

/// 取会话（复用 `mat_refs_build` 的预检 / LRU / 预算升级判定），再取支配树。
fn open_dom(path: &str) -> Result<(Arc<Session>, Arc<DomTree>, bool), String> {
    let build = crate::mat_refs::mat_refs_build(path)?;
    if build.data.truncated {
        let note = build.data.notes.first().cloned().unwrap_or_else(|| {
            "对象图规模超过引用追踪上限，无法提供支配树".to_string()
        });
        return Err(format!("DOM_UNAVAILABLE:{note}"));
    }
    let s = ensure_session(path)?;
    let cached = session_dom(&s).is_some();
    match ensure_dom(&s)? {
        DomReady::Ready(d) => Ok((s, d, cached)),
        DomReady::Unavailable(reason) => Err(format!("DOM_UNAVAILABLE:{reason}")),
    }
}

// ==================== 命令入口（纯函数，由 commands/mat.rs 包 spawn_blocking） ====================

/// Dominator Tree 查询：对象级 / 按类 / 按包，支持子树展开、类名过滤与分页。
pub fn mat_dom_tree(
    path: &str,
    group_by: Option<&str>,
    parent_obj_id: Option<&str>,
    query: &str,
    offset: usize,
    limit: usize,
) -> Result<ResData<DomTreeResult>, String> {
    let started = std::time::Instant::now();
    let (s, d, cached) = match open_dom(path) {
        Ok(v) => v,
        Err(e) => {
            if let Some(note) = e.strip_prefix("DOM_UNAVAILABLE:") {
                return Ok(empty_result(note.to_string(), started.elapsed().as_millis() as u64));
            }
            return Err(e);
        }
    };
    let g = session_graph(&s);
    let group_by = group_by.unwrap_or("object");
    let q = query.trim().to_lowercase();
    let limit = limit.clamp(1, 2_000);
    let mut note = String::new();
    if group_by != "object" {
        note = "分组行的 Retained 是该组各对象 retained 之和，包含支配重叠，仅用于排序与相对比较，不可与堆总量相加。".to_string();
    }

    // ---- 子树展开：只列 parent 的支配树直接孩子 ----
    if let Some(pid) = parent_obj_id.filter(|v| !v.is_empty()) {
        let rl = match resolve_rl(&s, &d, pid) {
            Some(v) => v,
            None => {
                return Ok(ResData {
                    code: 200,
                    msg: "ok".to_string(),
                    data: DomTreeResult {
                        total: 0, rows: Vec::new(), reach_nodes: d.reach_cnt,
                        reach_shallow: d.reach_shallow, truncated: false,
                        note: format!("对象 {pid} 不在可达支配树中"), cached,
                        elapsed_ms: started.elapsed().as_millis() as u64,
                    },
                });
            }
        };
        let mut rls: Vec<u32> = Vec::new();
        let mut c = d.child_head[rl as usize];
        while c != u32::MAX {
            if q.is_empty() || class_of(g, d.inv[c as usize] as usize).to_lowercase().contains(&q) {
                rls.push(c);
            }
            c = d.child_next[c as usize];
        }
        rls.sort_by(|a, b| d.ret_bytes[*b as usize].cmp(&d.ret_bytes[*a as usize]));
        let total = rls.len() as u64;
        let rows: Vec<DomRow> = rls
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|rl| row_from_rl(g, &d, rl))
            .collect();
        return Ok(ResData {
            code: 200, msg: "ok".to_string(),
            data: DomTreeResult {
                total, rows, reach_nodes: d.reach_cnt, reach_shallow: d.reach_shallow,
                truncated: false, note, cached,
                elapsed_ms: started.elapsed().as_millis() as u64,
            },
        });
    }

    // ---- 顶层：对象级或分组聚合 ----
    if group_by == "object" {
        let mut rls: Vec<u32> = (1..d.inv.len() as u32)
            .filter(|&rl| {
                q.is_empty() || class_of(g, d.inv[rl as usize] as usize).to_lowercase().contains(&q)
            })
            .collect();
        rls.sort_by(|a, b| d.ret_bytes[*b as usize].cmp(&d.ret_bytes[*a as usize]));
        let total = rls.len() as u64;
        let page: Vec<DomRow> = rls
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|rl| row_from_rl(g, &d, rl))
            .collect();
        let rows: Vec<DomRow> = page;
        return Ok(ResData {
            code: 200, msg: "ok".to_string(),
            data: DomTreeResult {
                total, rows, reach_nodes: d.reach_cnt, reach_shallow: d.reach_shallow,
                truncated: false, note, cached,
                elapsed_ms: started.elapsed().as_millis() as u64,
            },
        });
    }

    // 分组：按类 / 包累加 shallow 与 retained（retained 有重叠，见 note）
    #[derive(Default)]
    struct Agg {
        count: u64,
        shallow: u64,
        retained: u64,
    }
    let mut agg: HashMap<String, Agg> = HashMap::new();
    for rl in 1..d.inv.len() as u32 {
        let ni = d.inv[rl as usize] as usize;
        let key = group_key(g, ni, group_by);
        if !q.is_empty() && !key.to_lowercase().contains(&q) {
            continue;
        }
        let a = agg.entry(key).or_default();
        a.count += 1;
        a.shallow += g.nodes[ni].shallow;
        a.retained += d.ret_bytes[rl as usize];
    }
    let mut groups: Vec<(String, Agg)> = agg.into_iter().collect();
    groups.sort_by(|a, b| b.1.retained.cmp(&a.1.retained));
    let total = groups.len() as u64;
    let rows: Vec<DomRow> = groups
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(name, a)| DomRow {
            obj_id: String::new(),
            class_name: name,
            kind: "group".to_string(),
            shallow_bytes: a.shallow,
            retained_bytes: a.retained,
            retained_objects: a.count,
            pct: if d.reach_shallow > 0 {
                a.retained as f64 / d.reach_shallow as f64 * 100.0
            } else {
                0.0
            },
            children: 0,
            is_root: false,
            parent_obj_id: String::new(),
        })
        .collect();
    Ok(ResData {
        code: 200, msg: "ok".to_string(),
        data: DomTreeResult {
            total, rows, reach_nodes: d.reach_cnt, reach_shallow: d.reach_shallow,
            truncated: false, note, cached,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    })
}

/// 把 0x 十六进制对象 id 解析为支配树 relabel 域下标
fn resolve_rl(s: &Session, d: &DomTree, obj_id: &str) -> Option<u32> {
    let raw = obj_id.trim().trim_start_matches("0x").trim_start_matches("0X");
    let id = u64::from_str_radix(raw, 16).ok()?;
    let ni = session_node_by_id(s, id)? as usize;
    if ni >= d.relabel.len() {
        return None;
    }
    let rl = d.relabel[ni];
    (rl != u32::MAX).then_some(rl)
}

/// Show Retained Set：列出某对象支配子树中的对象（按 retained 降序 Top N）。
pub fn mat_dom_retained_set(
    path: &str,
    obj_id: &str,
    limit: usize,
) -> Result<ResData<RetainedSetResult>, String> {
    let started = std::time::Instant::now();
    let (s, d, _cached) = match open_dom(path) {
        Ok(v) => v,
        Err(e) => {
            if let Some(note) = e.strip_prefix("DOM_UNAVAILABLE:") {
                return Ok(ResData {
                    code: 200, msg: "ok".to_string(),
                    data: RetainedSetResult {
                        obj_id: obj_id.to_string(), total: 0, bytes: 0, rows: Vec::new(),
                        truncated: true, note: note.to_string(),
                        elapsed_ms: started.elapsed().as_millis() as u64,
                    },
                });
            }
            return Err(e);
        }
    };
    let g = session_graph(&s);
    let rl = match resolve_rl(&s, &d, obj_id) {
        Some(v) => v,
        None => {
            return Ok(ResData {
                code: 200, msg: "ok".to_string(),
                data: RetainedSetResult {
                    obj_id: obj_id.to_string(), total: 0, bytes: 0, rows: Vec::new(),
                    truncated: false, note: format!("对象 {obj_id} 不在可达支配树中"),
                    elapsed_ms: started.elapsed().as_millis() as u64,
                },
            });
        }
    };

    // BFS 收集支配子树（含自身）；超根的子树会是整个堆，设访问上限防卡死
    const MAX_VISIT: usize = 1_000_000;
    let mut rls: Vec<u32> = Vec::new();
    let mut stack = vec![rl];
    while let Some(u) = stack.pop() {
        rls.push(u);
        if rls.len() >= MAX_VISIT {
            break;
        }
        let mut c = d.child_head[u as usize];
        while c != u32::MAX {
            stack.push(c);
            c = d.child_next[c as usize];
        }
    }
    let truncated = rls.len() >= MAX_VISIT;
    rls.sort_by(|a, b| d.ret_bytes[*b as usize].cmp(&d.ret_bytes[*a as usize]));
    let total = d.ret_objs[rl as usize];
    let bytes = d.ret_bytes[rl as usize];
    let rows: Vec<DomRow> = rls
        .into_iter()
        .take(limit.clamp(1, 2_000))
        .map(|r| row_from_rl(g, &d, r))
        .collect();
    let note = if truncated {
        format!("支配子树超过 {MAX_VISIT} 个节点，仅列出 retained 最大的部分（合计仍为精确值）")
    } else {
        String::new()
    };
    Ok(ResData {
        code: 200, msg: "ok".to_string(),
        data: RetainedSetResult {
            obj_id: obj_id.to_string(), total, bytes, rows, truncated, note,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    })
}

// ==================== Top Consumers（按类 / 包聚合的 retained 排行） ====================

/// Top Consumers 一行
#[derive(Debug, Serialize)]
pub struct ConsumerRow {
    /// 分组名（类名 / 包名）
    pub name: String,
    /// class / package / classloader
    pub kind: String,
    /// 该组包含的「顶层支配对象」个数
    pub objects: u64,
    pub shallow_bytes: u64,
    pub retained_bytes: u64,
    /// retained / 可达堆 的百分比
    pub pct: f64,
    /// 该组里 retained 最大的类（下钻入口提示）
    pub top_class: String,
}

#[derive(Debug, Serialize)]
pub struct TopConsumersResult {
    pub group_by: String,
    /// 分组总数（分页前）
    pub total: u64,
    pub rows: Vec<ConsumerRow>,
    pub reach_nodes: u64,
    pub reach_shallow: u64,
    pub cached: bool,
    pub note: String,
    pub elapsed_ms: u64,
}

/// Top Consumers：只看**支配树顶层**（超根的直接孩子）。
///
/// 这是 MAT 的口径要害：顶层支配对象的 retained 集合**互不相交**，求和才等于可达堆；
/// 若对所有对象按类累加 retained（`mat_dom_tree` 的 groupBy 模式），会因支配重叠
/// 得到大于堆总量的数字。
pub fn mat_top_consumers(
    path: &str,
    group_by: &str,
    limit: usize,
) -> Result<ResData<TopConsumersResult>, String> {
    let started = std::time::Instant::now();
    let (s, d, cached) = match open_dom(path) {
        Ok(v) => v,
        Err(e) => {
            if let Some(note) = e.strip_prefix("DOM_UNAVAILABLE:") {
                return Ok(ResData {
                    code: 200,
                    msg: "ok".to_string(),
                    data: TopConsumersResult {
                        group_by: group_by.to_string(),
                        total: 0,
                        rows: Vec::new(),
                        reach_nodes: 0,
                        reach_shallow: 0,
                        cached: false,
                        note: note.to_string(),
                        elapsed_ms: started.elapsed().as_millis() as u64,
                    },
                });
            }
            return Err(e);
        }
    };
    let g = session_graph(&s);
    let group_by = match group_by {
        "package" => "package",
        "classloader" => "classloader",
        _ => "class",
    };
    let mut note = String::new();
    if group_by == "classloader" {
        note.push_str(
            "类加载器维度需要解析类元数据的 loader 字段，当前按类聚合代替；精确的 loader 排行见「类加载器」报告。",
        );
    }

    #[derive(Default)]
    struct Agg {
        objects: u64,
        shallow: u64,
        retained: u64,
        best: u64,
        best_class: String,
    }
    let mut agg: HashMap<String, Agg> = HashMap::new();
    let mut top_level: u64 = 0;
    let mut c = d.child_head[0];
    while c != u32::MAX {
        let ni = d.inv[c as usize] as usize;
        if ni != u32::MAX as usize {
            top_level += 1;
            let class_name = class_of(g, ni).to_string();
            let key = if group_by == "package" {
                group_key(g, ni, "package")
            } else {
                class_name.clone()
            };
            let retained = d.ret_bytes[c as usize];
            let a = agg.entry(key).or_default();
            a.objects += 1;
            a.shallow += g.nodes[ni].shallow;
            a.retained += retained;
            if retained > a.best {
                a.best = retained;
                a.best_class = class_name;
            }
        }
        c = d.child_next[c as usize];
    }

    let mut rows: Vec<ConsumerRow> = agg
        .into_iter()
        .map(|(name, a)| ConsumerRow {
            name,
            kind: group_by.to_string(),
            objects: a.objects,
            shallow_bytes: a.shallow,
            retained_bytes: a.retained,
            pct: if d.reach_shallow > 0 {
                a.retained as f64 / d.reach_shallow as f64 * 100.0
            } else {
                0.0
            },
            top_class: a.best_class,
        })
        .collect();
    rows.sort_by(|a, b| b.retained_bytes.cmp(&a.retained_bytes));
    let total = rows.len() as u64;
    rows.truncate(limit.clamp(1, 200));

    if top_level == 0 {
        note.push_str("支配树顶层为空：该 dump 没有可达对象");
    } else if note.is_empty() {
        note.push_str(&format!(
            "基于支配树顶层 {top_level} 个对象聚合；retained 互不重叠，求和约等于可达堆"
        ));
    }

    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: TopConsumersResult {
            group_by: group_by.to_string(),
            total,
            rows,
            reach_nodes: d.reach_cnt,
            reach_shallow: d.reach_shallow,
            cached,
            note,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    })
}
