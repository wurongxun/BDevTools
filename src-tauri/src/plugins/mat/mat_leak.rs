//! # 内存泄漏检测（Leak Suspects）—— 支配树 + Retained Heap
//!
//! 本模块是**纯业务层**：不出现 `#[tauri::command]`、不依赖 `tauri` crate，
//! 命令包装统一放在 `commands.rs`（见该文件头部的架构约定）。
//!
//! ## 分析管线（与 Eclipse MAT 的 Leak Suspects 报告同思路）
//! 1. **构建对象引用图**：遍历 HPROF，把所有对象（实例 / 对象数组 / 基本类型数组 / 类对象）
//!    建成图节点；引用即边：
//!    - 实例的**对象引用字段**：实例字段数据按「具体类 → 父类 → … → Object」的顺序序列化
//!      （jvm-hprof 依 OpenJDK heapDumper 确认），配合类继承链的字段描述符逐字段解析；
//!    - 对象数组的元素；
//!    - 类对象的**静态引用字段**（static 持有是常见泄漏源，MAT 会把它作为嫌疑起点）。
//! 2. **可达性**：从 GC Roots（线程 / JNI / Java 栈帧局部变量 / 系统类 / 监视器等）BFS，
//!    区分可达对象与不可达对象（不可达对象下次 GC 即可回收，不构成泄漏）。
//! 3. **支配树**：对可达子图跑 Lengauer-Tarjan，得到每个节点的最近支配者（idom）。
//! 4. **Retained Heap**：支配树自底向上累加浅堆 —— `retained(x)` = x 及**仅由 x 支配**的
//!    子树对象浅堆之和（与 MAT 口径一致：移除 x 后能被 GC 释放的内存量）。
//! 5. **嫌疑报告**：非 GC Root 节点按 retained 降序取 Top N；每条嫌疑给出
//!    「GC Root → … → 嫌疑对象」的支配链路径（沿 idom 上溯），可直接看出是谁把对象群扣在堆上。
//!
//! ## 规模保护
//! 支配树阶段内存与图规模线性相关，超大 dump（可达对象过多 / 引用边过多）时跳过图分析，
//! 返回带 summary 的空嫌疑列表 + 提示（summary 统计不受影响，仍然有效）。
//!
//! ## 已知口径
//! - 浅堆：实例 = 类 `instance_size`；对象数组 = 16 + n × id 宽；基本类型数组 = 16 + n × 元素宽；
//!   类对象自身按 0 计（MAT 直方图同样不计类对象内存）。
//! - GC Root 种类标签与 Eclipse MAT 的 "Path to GC Roots" 根分类对应。
//! - Android ART 变体会使解析器 panic，由 `catch_unwind` 兜底（同 mat_hprof）。

use crate::plugins::mat::mat_graph::{ClassMeta, Node, NodeKind};
use crate::plugins::mat::mat_hprof::{catch_parse, count_heap_objects, read_head, sniff_format};
use crate::plugins::mat::mat_cfg;
use crate::plugins::mat::mat_refs::{ensure_session, refs_has_session, session_graph, session_reachable};
use crate::response::resp_data::ResData;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

// ==================== 返回结构（与前端 mat api 字段一一对应，snake_case） ====================

#[derive(Debug, Serialize)]
pub struct MatLeakSummary {
    /// 图中全部节点（实例 + 各类数组 + 类对象）
    pub total_nodes: u64,
    /// 全部节点浅堆字节
    pub total_shallow: u64,
    /// GC Roots 去重后的节点数
    pub root_nodes: u64,
    /// 引用边总数（含自环去重后的有效边）
    pub edge_count: u64,
    /// 指向图外对象的引用数（畸形数据，已忽略）
    pub skipped_edge_count: u64,
    /// 从 GC Roots 可达的对象数
    pub reachable_nodes: u64,
    /// 可达对象浅堆字节
    pub reachable_shallow: u64,
    /// 不可达对象数（垃圾，不构成泄漏）
    pub unreachable_nodes: u64,
    /// 不可达对象浅堆字节
    pub unreachable_shallow: u64,
    /// 是否因规模超限跳过了支配树分析（suspects 为空）
    pub truncated: bool,
    pub elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct MatLeakPathNode {
    pub class_name: String,
    /// 该节点是否是 GC Root 对象
    pub is_root: bool,
}

#[derive(Debug, Serialize)]
pub struct MatLeakSuspect {
    pub rank: u32,
    /// 对象 id（十六进制展示）
    pub obj_id: String,
    pub class_name: String,
    /// 是否为类对象（泄漏源在静态字段上）
    pub is_class: bool,
    /// 所在 GC Root 分类标签（thread / jni-global / …）
    pub root_kind: String,
    pub shallow_bytes: u64,
    /// 深堆：仅由此嫌疑支配的对象浅堆之和（MAT Retained Heap 口径）
    pub retained_bytes: u64,
    /// 支配子树对象数（含自身）
    pub retained_objects: u64,
    /// retained / 可达堆浅堆 的百分比
    pub pct: f64,
    /// GC Root → … → 嫌疑对象的支配链（含两端）
    pub path: Vec<MatLeakPathNode>,
}

#[derive(Debug, Serialize)]
pub struct MatLeakSuspectsResult {
    pub summary: MatLeakSummary,
    pub suspects: Vec<MatLeakSuspect>,
    pub notes: Vec<String>,
}

// ==================== 规模保护阈值 ====================

// 支配树上限由 mat_cfg 按「分析内存预算」动态换算：峰值 ≈ R×4B×11 + E×8B×2，
// 16GB 参考机预算 7.7GB → 25M 可达 / 120M 边（约 3-4GB 支配树追加内存）；
// 默认随物理内存自适应、可在前端「分析预算」设置中覆盖。
/// 返回的嫌疑对象条数
const SUSPECT_LIMIT: usize = 25;

// ==================== 支配树（Lengauer-Tarjan） ====================

/// 计算可达对象图的最近支配者 idom（relabel 域；0 = 超根，1..=m-1 为可达对象）。
///
/// 内存策略（大堆友好）：
/// - DFS 正向遍历直接复用对象图**原始节点域** CSR（fwd_* + relabel/inv 过滤），
///   不额外物化一份 relabel 域正向边；
/// - semi 阶段与引用路径前缀 BFS 共用一份 relabel 域**反向** CSR（pred_*，
///   w 的前驱 = 引用 w 的对象，超根→GC Root 的前驱 0 已含）；
/// - 编号/半支配者/桶全部 u32 紧凑数组 + 链表化 bucket（无 Vec<Vec>）。
///
/// 算法遵循 Cambridge《The Lengauer-Tarjan Algorithm》(M. Richards) 第 9-11 节
/// simple EVAL/LINK 版（无路径压缩，正确性由论文保证）。DFS 多根起步：
/// 每个 GC Root 都作为超根的孩子编号。
fn lengauer_tarjan(
    m: usize,
    relabel: &[u32],
    inv: &[u32],
    fwd_head: &[u32],
    fwd_to: &[u32],
    fwd_next: &[u32],
    roots_rl: &[u32],
    pred_head: &[u32],
    pred_to: &[u32],
    pred_next: &[u32],
) -> Vec<u32> {
    // ---- 1) 迭代 DFS（多根），求 1-based 编号与 DFS 树父 ----
    let mut dfn_of = vec![0u32; m]; // rl -> 编号（0 = 未访问）
    let mut vertex = vec![0u32; m]; // 编号 -> rl
    let mut parent = vec![0u32; m]; // 编号 -> 父编号（根父 = 0）
    let mut next = 1u32;
    {
        let mut stack: Vec<(u32, u32)> = Vec::new(); // (rl, 其出边链表当前游标)
        for &r in roots_rl.iter() {
            let r = r as usize;
            if dfn_of[r] == 0 {
                dfn_of[r] = next;
                vertex[next as usize] = r as u32;
                parent[next as usize] = 0;
                next += 1;
                stack.push((r as u32, fwd_head[inv[r] as usize]));
            }
        }
        while let Some((rl, e0)) = stack.last().copied() {
            let mut e = e0;
            let mut advanced = false;
            while e != u32::MAX {
                let child_rl = relabel[fwd_to[e as usize] as usize];
                if child_rl != u32::MAX && dfn_of[child_rl as usize] == 0 {
                    let c = child_rl as usize;
                    dfn_of[c] = next;
                    vertex[next as usize] = c as u32;
                    parent[next as usize] = dfn_of[rl as usize];
                    next += 1;
                    stack.last_mut().unwrap().1 = fwd_next[e as usize];
                    stack.push((c as u32, fwd_head[inv[c] as usize]));
                    advanced = true;
                    break;
                }
                e = fwd_next[e as usize];
            }
            if !advanced {
                stack.pop();
            }
        }
    }
    let r_total = next - 1; // 可达对象数（relabel 域全可达，应等于 m - 1）
    debug_assert_eq!(r_total, (m - 1) as u32);

    // ---- 2) simple EVAL / LINK（编号域）----
    let mut semi: Vec<u32> = (0..m as u32).collect();
    let mut ancestor: Vec<u32> = vec![0; m];
    let mut idom: Vec<u32> = vec![0; m];
    let mut bucket_head: Vec<u32> = vec![0; m]; // bucket[semi 编号] 链表头
    let mut bucket_next: Vec<u32> = vec![0; m]; // 链表下一元素（编号）

    fn eval_simple(mut v: u32, ancestor: &[u32], semi: &[u32]) -> u32 {
        let mut a = ancestor[v as usize];
        while ancestor[a as usize] != 0 {
            if semi[v as usize] > semi[a as usize] {
                v = a;
            }
            a = ancestor[a as usize];
        }
        v
    }

    // ---- 3) 逆编号序算 semi / 结算 bucket ----
    for wnum in (2..=r_total).rev() {
        let w_rl = vertex[wnum as usize];
        // step2: semi[w]
        let mut e = pred_head[w_rl as usize];
        while e != u32::MAX {
            let p_rl = pred_to[e as usize] as usize;
            let pn = dfn_of[p_rl];
            if pn != 0 {
                let u = eval_simple(pn, &ancestor, &semi);
                if semi[wnum as usize] > semi[u as usize] {
                    semi[wnum as usize] = semi[u as usize];
                }
            }
            e = pred_next[e as usize];
        }
        // 入桶 + LINK(p, w)
        bucket_next[wnum as usize] = bucket_head[semi[wnum as usize] as usize];
        bucket_head[semi[wnum as usize] as usize] = wnum;
        let pnum = parent[wnum as usize];
        ancestor[wnum as usize] = pnum;
        // step3: 结算 p 的桶
        if pnum != 0 {
            let mut v = bucket_head[pnum as usize];
            bucket_head[pnum as usize] = 0;
            while v != 0 {
                let u = eval_simple(v, &ancestor, &semi);
                if semi[u as usize] < semi[v as usize] {
                    idom[v as usize] = u;
                } else {
                    idom[v as usize] = pnum;
                }
                v = bucket_next[v as usize];
            }
        }
    }
    // ---- 4) 终值 idom ----
    for wnum in 2..=r_total {
        let w = wnum as usize;
        if idom[w] != semi[w] {
            idom[w] = idom[idom[w] as usize];
        }
    }

    // ---- 映射回 relabel 域 ----
    let mut out = vec![0u32; m];
    for wnum in 1..=r_total {
        let rl = vertex[wnum as usize];
        let pn = idom[wnum as usize];
        out[rl as usize] = if pn == 0 { 0 } else { vertex[pn as usize] };
    }
    out
}


// ==================== 命令入口 ====================

/// 泄漏嫌疑分析（纯同步，调用方需 spawn_blocking 包裹）
pub fn mat_leak_suspects(path: &str) -> Result<ResData<MatLeakSuspectsResult>, String> {
    let started = std::time::Instant::now();
    let head = read_head(path, 64)?;
    let (_format, supported) = sniff_format(&head);
    if !supported {
        return Err(format!(
            "不支持的堆转储格式（{})，当前仅支持 JAVA PROFILE 1.0.x",
            String::from_utf8_lossy(&head)
        ));
    }
    // 当前预算换算出的上限（前端可随时改 plugins/mat.json，故每次调用现算）
    let lim = mat_cfg::current_limits();

    // 与引用追踪共用同一会话图（LRU 缓存）：同一 dump 不再各自全量解析、各存一份图。
    // 两道闸（上限均来自 mat_cfg 的当前预算）：
    // ① 建图闸：对象总数超「引用追踪节点上限」→ 预检直接截断（建图需物化完整会话图，
    //    泄漏与引用追踪同代价，故共用同一闸；会话已缓存时跳过预检走精确图统计）；
    // ② 支配树闸：图建好后按「可达数/边」单独判定——堆里垃圾多、可达少时
    //    至少返回精确的可达统计，而不是被全零截断。
    let session = if refs_has_session(path) {
        ensure_session(path)?
    } else {
        let cnt = catch_parse(|| count_heap_objects(path))?;
        if cnt > lim.refs_nodes as u64 {
            let elapsed_ms = started.elapsed().as_millis() as u64;
            let mut notes = Vec::new();
            notes.push(format!(
                "对象总数 {} 超过建图闸（预算 {} GB → 约 {} 节点 / {} 边；本机物理内存 {} GB）。预检直接跳过全量建图以降低内存峰值；可在「分析预算」设置中调高后重试",
                cnt, lim.budget_gb, lim.refs_nodes, lim.refs_edges, mat_cfg::physical_mem_gb()
            ));
            let summary = MatLeakSummary {
                total_nodes: cnt,
                total_shallow: 0,
                root_nodes: 0,
                edge_count: 0,
                skipped_edge_count: 0,
                reachable_nodes: 0,
                reachable_shallow: 0,
                unreachable_nodes: cnt,
                unreachable_shallow: 0,
                truncated: true,
                elapsed_ms,
            };
            return Ok(ResData {
                code: 200,
                msg: "ok".to_string(),
                data: MatLeakSuspectsResult {
                    summary,
                    suspects: Vec::new(),
                    notes,
                },
            });
        }
        ensure_session(path)?
    };
    let g = session_graph(&session);
    let n = g.nodes.len();
    let total_shallow: u64 = g.nodes.iter().map(|nd| nd.shallow).sum();
    let total_edges: usize = g.edge_to.len();
    let root_count = g.roots.len();

    let mut notes: Vec<String> = Vec::new();
    if g.field_mismatch > 0 {
        notes.push(format!(
            "{} 个实例的字段字节与类定义不符，未提取引用（多为继承链元数据缺失的类）",
            g.field_mismatch
        ));
    }
    if g.skipped > 0 {
        notes.push(format!("{} 条引用指向堆外对象，已忽略", g.skipped));
    }

    let mut summary = MatLeakSummary {
        total_nodes: n as u64,
        total_shallow,
        root_nodes: root_count as u64,
        edge_count: total_edges as u64,
        skipped_edge_count: g.skipped,
        reachable_nodes: 0,
        reachable_shallow: 0,
        unreachable_nodes: n as u64,
        unreachable_shallow: total_shallow,
        truncated: false,
        elapsed_ms: 0,
    };

    // ---- 可达性（复用会话缓存的 BFS，与引用追踪同口径；roots 为空时全不可达）----
    let reachable = session_reachable(&session);
    let mut reach_shallow: u64 = 0;
    let mut reach_cnt: usize = 0;
    let mut unreach_shallow: u64 = 0;
    let mut unreach_cnt: usize = 0;
    for i in 0..n {
        if reachable[i] {
            reach_cnt += 1;
            reach_shallow += g.nodes[i].shallow;
        } else {
            unreach_cnt += 1;
            unreach_shallow += g.nodes[i].shallow;
        }
    }
    summary.reachable_nodes = reach_cnt as u64;
    summary.reachable_shallow = reach_shallow;
    summary.unreachable_nodes = unreach_cnt as u64;
    summary.unreachable_shallow = unreach_shallow;

    // ---- 规模保护 ----
    let mut suspects: Vec<MatLeakSuspect> = Vec::new();
    if reach_cnt == 0 {
        notes.push("未发现任何 GC Roots，无法分析引用关系".to_string());
    } else if reach_cnt > lim.dom_nodes || total_edges > lim.dom_edges {
        summary.truncated = true;
        notes.push(format!(
            "可达对象 {}（边 {}）超过支配树闸（预算 {} GB → 约 {} 可达对象 / {} 边；本机物理内存 {} GB），已跳过支配树分析；对象图与可达性统计仍然有效。可在「分析预算」设置中调高后重试",
            reach_cnt, total_edges, lim.budget_gb, lim.dom_nodes, lim.dom_edges,
            mat_cfg::physical_mem_gb()
        ));
    } else {
        suspects = analyze_suspects(
            &g.nodes,
            &g.meta,
            &g.edge_head,
            &g.edge_to,
            &g.edge_next,
            &reachable,
            &g.roots,
            reach_cnt,
            reach_shallow,
        );
    }

    summary.elapsed_ms = started.elapsed().as_millis() as u64;
    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: MatLeakSuspectsResult { summary, suspects, notes },
    })
}

/// 在可达子图上做支配树 + Retained Heap，输出 Top 嫌疑。
///
/// 内存策略：只额外物化一份 relabel 域**反向**边（LT semi + 引用路径前缀 BFS 共用），
/// 正向遍历直接复用原始图 CSR（配合 relabel 过滤），数组全 u32 / 链表化，峰值可控。
fn analyze_suspects(
    nodes: &[Node],
    meta: &[ClassMeta],
    edge_head: &[u32],
    edge_to: &[u32],
    edge_next: &[u32],
    reachable: &[bool],
    roots: &[(u32, &'static str)],
    reach_cnt: usize,
    reach_shallow: u64,
) -> Vec<MatLeakSuspect> {
    // 可达节点重标号：0 = 超根（super root），1..=reach_cnt = 原节点
    let nn = nodes.len();
    let mut relabel: Vec<u32> = vec![u32::MAX; nn];
    let mut inv: Vec<u32> = Vec::with_capacity(reach_cnt + 1);
    inv.push(u32::MAX); // 0 = 超根
    for i in 0..nn {
        if reachable[i] {
            relabel[i] = inv.len() as u32;
            inv.push(i as u32);
        }
    }
    let m = reach_cnt + 1;

    // ---- relabel 域反向边（前驱表，头插 CSR）：
    //  a→b 存为 b 的前驱 a；含 超根→GC Root 的前驱 0。LT 与前缀 BFS 共用。
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
        let a = relabel[u];
        if a == u32::MAX {
            continue;
        }
        let mut e = edge_head[u];
        while e != u32::MAX {
            let v = edge_to[e as usize] as usize;
            let b = relabel[v];
            if b != u32::MAX && b != a {
                add_pred!(b, a);
            }
            e = edge_next[e as usize];
        }
    }

    // GC Roots（relabel 域）：LT DFS 起点 / 排除候选 / 前缀 BFS 终点
    let mut roots_rl: Vec<u32> = Vec::with_capacity(roots.len());
    let mut root_set: HashSet<u32> = HashSet::with_capacity(roots.len());
    for &(ri, _) in roots.iter() {
        let a = relabel[ri as usize];
        if a != u32::MAX && a != 0 {
            roots_rl.push(a);
            root_set.insert(a);
        }
    }

    let idom = lengauer_tarjan(
        m,
        &relabel,
        &inv,
        edge_head,
        edge_to,
        edge_next,
        &roots_rl,
        &pred_head,
        &pred_to,
        &pred_next,
    );
    if idom.len() != m {
        return Vec::new();
    }

    // ---- retained 自底向上：支配树孩子链表 → BFS 层序 → 逆序累加 ----
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

    // ---- 嫌疑候选：非 GC Root 按 retained 降序 ----
    struct Cand {
        rl: u32,
        retained: u64,
        objs: u64,
    }
    let mut cands: Vec<Cand> = Vec::new();
    for i in 1..m {
        if root_set.contains(&(i as u32)) {
            continue;
        }
        if ret_bytes[i] == 0 {
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

    // ---- 反向 BFS 补路径前缀 ----
    // 沿 preds（a ∈ pred[b] ⇔ a 引用 b，朝超根方向）从链顶找最近 GC Root。
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
                return None; // 保护：对象引用网过大时放弃补路径
            }
            if root_set.contains(&u) {
                let mut path = vec![u];
                let mut cur = u;
                while let Some(&p) = parent.get(&cur) {
                    path.push(p);
                    cur = p;
                }
                path.pop(); // 去掉末尾的 start
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

        // 支配链（idom 上溯）：[链顶 … suspect]
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

        // 链顶非单一 GC Root（多根共同支配，idom=超根）时补最近 GC Root 前缀
        let mut prefix: Vec<u32> = Vec::new();
        if !root_set.contains(&top) {
            if let Some(pr) = bfs_root_prefix(top, &pred_head, &pred_to, &pred_next, &root_set, m) {
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
        let root_kind = if !prefix.is_empty() {
            kind_of(prefix[0])
        } else {
            kind_of(top)
        };

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


#[cfg(test)]
mod tests {
    use super::*;

    // ==================== LT 对照：数据流迭代求 idom ====================

    /// 测试 harness：把「顶点域 0..n-1（0 为根）」前驱表映射到 LT 的
    /// relabel 域约定（0' = 超根不可编号，真实顶点 1..=n，旧顶点 v → v+1）后跑新 LT。
    /// 返回旧顶点域的 idom（旧根 0 的 idom = 0）。
    fn lt_via_preds(n: usize, preds: &[Vec<u32>]) -> Vec<u32> {
        let m = n + 1; // 含超根
        // 正向边（新域）：旧边 u→v ⇒ (u+1)→(v+1)；超根 0'→旧根 0(=1) 由 DFS 种子替代
        let mut fh: Vec<u32> = vec![u32::MAX; m];
        let mut ft: Vec<u32> = Vec::new();
        let mut fnx: Vec<u32> = Vec::new();
        for (v, ps) in preds.iter().enumerate() {
            let nv = v as u32 + 1;
            for &p in ps {
                let nu = p + 1;
                if nu != nv {
                    let e = ft.len() as u32;
                    ft.push(nv);
                    fnx.push(fh[nu as usize]);
                    fh[nu as usize] = e;
                }
            }
        }
        // 反向边（新域）：pred[v+1] = {u+1} ∪ (v==0 ? {0'} : ∅)
        let mut ph: Vec<u32> = vec![u32::MAX; m];
        let mut pt: Vec<u32> = Vec::new();
        let mut pnx: Vec<u32> = Vec::new();
        for (v, ps) in preds.iter().enumerate() {
            let nv = v as u32 + 1;
            if v == 0 {
                let e = pt.len() as u32;
                pt.push(0);
                pnx.push(ph[nv as usize]);
                ph[nv as usize] = e;
            }
            for &p in ps {
                let nu = p + 1;
                if nu != nv {
                    let e = pt.len() as u32;
                    pt.push(nu);
                    pnx.push(ph[nv as usize]);
                    ph[nv as usize] = e;
                }
            }
        }
        let relabel: Vec<u32> = (0..m as u32).collect();
        let inv: Vec<u32> = (0..m as u32).collect();
        let roots_rl = vec![1u32];
        let idom_new = lengauer_tarjan(m, &relabel, &inv, &fh, &ft, &fnx, &roots_rl, &ph, &pt, &pnx);
        // 映射回旧顶点域
        let mut out = vec![0u32; n];
        out[0] = 0;
        for v in 1..n {
            let p = idom_new[v + 1];
            out[v] = if p == 0 { 0 } else { p - 1 };
        }
        out
    }

    /// 数据流迭代求支配集，再由支配集推 idom（只用于小图对照）
    fn df_idom(n: usize, preds: &[Vec<u32>]) -> Vec<u32> {
        // dom[v][u] = u 支配 v
        let mut dom: Vec<Vec<bool>> = vec![vec![false; n]; n];
        for v in 1..n {
            dom[v].iter_mut().for_each(|b| *b = true);
        }
        dom[0][0] = true;
        loop {
            let mut changed = false;
            for v in 1..n {
                let mut nd = vec![true; n];
                if preds[v].is_empty() {
                    nd.iter_mut().for_each(|b| *b = false);
                } else {
                    for &p in preds[v].iter() {
                        for u in 0..n {
                            nd[u] &= dom[p as usize][u];
                        }
                    }
                }
                nd[v] = true;
                if nd != dom[v] {
                    dom[v] = nd;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let mut idom = vec![0u32; n];
        for v in 1..n {
            // idom[v] = dom[v]\{v} 中支配集最大者（支配链上最接近 v 的：支配集沿链向下递增）
            let mut best = usize::MAX;
            let mut best_sz = 0usize;
            for u in 0..n {
                if u != v && dom[v][u] {
                    let sz = dom[u].iter().filter(|b| **b).count();
                    if sz > best_sz {
                        best_sz = sz;
                        best = u;
                    }
                }
            }
            idom[v] = best as u32;
        }
        idom
    }

    #[test]
    fn lt_matches_dataflow_reference() {
        // 手写图：0 → {1,2} → 3（菱形）等
        let cases: Vec<(usize, Vec<(u32, u32)>)> = vec![
            // diamond
            (4, vec![(0, 1), (0, 2), (1, 3), (2, 3)]),
            // 链 + 交叉
            (5, vec![(0, 1), (1, 2), (0, 2), (2, 3), (2, 4), (3, 4)]),
            // 汇聚
            (6, vec![(0, 1), (0, 2), (1, 3), (2, 3), (3, 4), (0, 5), (5, 4)]),
        ];
        for (n, edges) in cases {
            let mut preds: Vec<Vec<u32>> = vec![Vec::new(); n];
            for (u, v) in edges.iter().copied() {
                preds[v as usize].push(u);
            }
            let idom = lt_via_preds(n, &preds);
            let want = df_idom(n, &preds);
            if idom != want {
                for i in 0..n {
                    if idom[i] != want[i] {
                        eprintln!("MISMATCH case={edges:?} v{i}: lt={} df={}", idom[i], want[i]);
                    }
                }
                panic!("graph n={n} edges={edges:?}");
            }
        }
    }

    #[test]
    fn lt_matches_dataflow_random() {
        // 伪随机：树保证连通 + 随机交叉边
        let mut seed = 0x9e3779b9u32;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for n in 2..14usize {
            for _ in 0..30 {
                let mut preds: Vec<Vec<u32>> = vec![Vec::new(); n];
                for v in 1..n {
                    let p = (rnd() as usize) % v;
                    preds[v].push(p as u32);
                }
                let extra = (rnd() as usize) % (n * 2);
                for _ in 0..extra {
                    let u = (rnd() as usize) % n;
                    let v = (rnd() as usize) % n;
                    if u != v {
                        preds[v].push(u as u32);
                    }
                }
                let idom = lt_via_preds(n, &preds);
                let want = df_idom(n, &preds);
                assert_eq!(idom, want, "random n={n}");
            }
        }
    }

    /// 大图冒烟：200 万顶点链，验证 CSR 化 LT 线性完成、编号/链表索引不溢出
    #[test]
    fn lt_large_chain_smoke() {
        let r = 2_000_000u32; // 真实顶点数
        let m = r as usize + 1; // 含超根
        let mut fh: Vec<u32> = vec![u32::MAX; m];
        let mut ft: Vec<u32> = Vec::with_capacity(r as usize);
        let mut fnx: Vec<u32> = Vec::with_capacity(r as usize);
        let mut ph: Vec<u32> = vec![u32::MAX; m];
        let mut pt: Vec<u32> = Vec::with_capacity(r as usize);
        let mut pnx: Vec<u32> = Vec::with_capacity(r as usize);
        // 1 → 2 → … → r（真实顶点 1..=r，超根为 0）
        for v in 1..r {
            let e = ft.len() as u32;
            ft.push(v + 1);
            fnx.push(fh[v as usize]);
            fh[v as usize] = e;
            let e2 = pt.len() as u32;
            pt.push(v);
            pnx.push(ph[(v + 1) as usize]);
            ph[(v + 1) as usize] = e2;
        }
        // 超根 0 → 真实根 1 的前驱
        let e = pt.len() as u32;
        pt.push(0);
        pnx.push(ph[1]);
        ph[1] = e;
        let relabel: Vec<u32> = (0..m as u32).collect();
        let inv: Vec<u32> = (0..m as u32).collect();
        let roots_rl = vec![1u32];
        let idom = lengauer_tarjan(m, &relabel, &inv, &fh, &ft, &fnx, &roots_rl, &ph, &pt, &pnx);
        assert_eq!(idom.len(), m);
        assert_eq!(idom[1], 0); // 真实根由超根支配
        for v in 2..=r as usize {
            assert_eq!(idom[v], (v - 1) as u32, "v={v}");
        }
    }

    // ==================== 合成 HPROF 端到端 ====================

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

    struct HprofBuilder {
        f: Vec<u8>,
        next_name: u64,
        next_serial: u32,
    }

    impl HprofBuilder {
        fn new() -> Self {
            let mut f = Vec::new();
            f.extend_from_slice(b"JAVA PROFILE 1.0.2\0");
            u4(&mut f, 8);
            u8(&mut f, 0);
            HprofBuilder { f, next_name: 0x1000, next_serial: 1 }
        }
        /// 注册类名（返回 name id）
        fn utf8(&mut self, text: &str) -> u64 {
            let nid = self.next_name;
            self.next_name += 1;
            let mut p = Vec::new();
            id(&mut p, nid);
            p.extend_from_slice(text.as_bytes());
            rec(&mut self.f, 0x01, &p);
            nid
        }
        /// LOAD_CLASS（返回 class obj id 映射用：调用方记住）
        fn load_class(&mut self, class_obj: u64, name_id: u64) {
            let mut p = Vec::new();
            u4(&mut p, self.next_serial);
            self.next_serial += 1;
            id(&mut p, class_obj);
            u4(&mut p, 0);
            id(&mut p, name_id);
            rec(&mut self.f, 0x02, &p);
        }
        /// 追加一个 HEAP_DUMP 记录
        fn heap(&mut self, sub: &[u8]) {
            rec(&mut self.f, 0x0C, sub);
        }
        fn into_bytes(self) -> Vec<u8> { self.f }
    }

    /// ClassDump 子记录
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
        id(&mut b, 0); // loader
        id(&mut b, 0); // signers
        id(&mut b, 0); // protection domain
        id(&mut b, 0); // reserved
        id(&mut b, 0); // reserved
        u4(&mut b, instance_size);
        u2(&mut b, 0); // constant pool
        u2(&mut b, static_refs.len() as u16);
        for &t in static_refs {
            id(&mut b, field_name_id);
            u1(&mut b, 0x02); // object
            id(&mut b, t);
        }
        u2(&mut b, inst_fields as u16);
        for _ in 0..inst_fields {
            id(&mut b, field_name_id);
            u1(&mut b, 0x02); // 全部用对象引用字段
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

    fn sub_prim_byte_array(obj: u64, n: usize) -> Vec<u8> {
        let mut b = Vec::new();
        u1(&mut b, 0x23);
        id(&mut b, obj);
        u4(&mut b, 0);
        u4(&mut b, n as u32);
        u1(&mut b, 0x08); // byte
        b.extend(std::iter::repeat(0u8).take(n));
        b
    }

    fn sub_root_system_class(obj: u64) -> Vec<u8> {
        let mut b = Vec::new();
        u1(&mut b, 0x05); // GC_ROOT_SYSTEM_CLASS
        id(&mut b, obj);
        b
    }

    /// 端到端：类 Cache(static→Holder 实例) → Holder extends Leaf(对象字段) → 2 个 Leaf 实例 + 一个无人引用的 1KB byte[]
    #[test]
    fn leak_suspects_synthetic_end_to_end() {
        let mut hb = HprofBuilder::new();
        let n_cache = hb.utf8("test/Cache");
        let n_holder = hb.utf8("test/Holder");
        let n_leaf = hb.utf8("test/Leaf");
        let field_name = hb.utf8("ref");
        hb.load_class(0x100, n_cache);
        hb.load_class(0x200, n_leaf);
        hb.load_class(0x300, n_holder);

        // 第一个 heap dump：GC root(system class) + 三个 ClassDump + 实例 + 数组
        let mut h1 = Vec::new();
        h1.extend_from_slice(&sub_root_system_class(0x100));
        // Leaf：instance_size 32，1 个对象字段
        h1.extend_from_slice(&sub_class(0x200, None, 32, &[], 1, field_name));
        // Holder extends Leaf：instance_size 56，1 个对象字段（数据序：Holder 字段在前）
        h1.extend_from_slice(&sub_class(0x300, Some(0x200), 56, &[], 1, field_name));
        // Cache：无实例，静态字段 → holder 实例 0x301
        h1.extend_from_slice(&sub_class(0x100, None, 0, &[0x301], 0, field_name));
        // holder 0x301（Holder 类）：字段数据 = [0x201(ref) | 0x202(Leaf 的 ref，null) ]
        let mut holder_fields = Vec::new();
        id(&mut holder_fields, 0x201); // Holder 自己的字段
        id(&mut holder_fields, 0x202); // 父类 Leaf 的字段（具体类字段在前）
        h1.extend_from_slice(&sub_instance(0x301, 0x300, &holder_fields));
        // leaf 0x201 / 0x202（Leaf 类）：对象字段为 null
        h1.extend_from_slice(&sub_instance(0x201, 0x200, &[0u8; 8]));
        h1.extend_from_slice(&sub_instance(0x202, 0x200, &[0u8; 8]));
        // 1KB byte[]：无引用 → 不可达
        h1.extend_from_slice(&sub_prim_byte_array(0x401, 1000));
        hb.heap(&h1);

        let dir = std::env::temp_dir().join("mat_leak_e2e.hprof");
        std::fs::write(&dir, hb.into_bytes()).unwrap();
        let res = mat_leak_suspects(dir.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_file(&dir);

        assert_eq!(res.code, 200);
        let s = &res.data.summary;
        // 节点：Cache/Leaf/Holder 3 个 class + holder + 2 leaf + byte[] = 7
        assert_eq!(s.total_nodes, 7);
        assert_eq!(s.root_nodes, 1);
        assert_eq!(s.edge_count, 3); // Cache→holder, holder→leaf201, holder→leaf202
        assert_eq!(s.skipped_edge_count, 0);
        assert!(!s.truncated);
        // 可达：Cache + holder + leaf201 + leaf202；不可达：byte[] + Leaf/Holder class 节点
        assert_eq!(s.reachable_nodes, 4);
        assert_eq!(s.unreachable_nodes, 3);
        // 浅堆：holder=56，leaf=32×2，byte[]=1016；Cache/class 节点 = 0
        assert_eq!(s.reachable_shallow, 56 + 32 * 2);
        assert_eq!(s.unreachable_shallow, 1016);

        let su = &res.data.suspects;
        assert!(!su.is_empty());
        // 第一大嫌疑：holder（retained = 56 + 32 + 32 = 120，3 个对象）
        let top = &su[0];
        assert_eq!(top.obj_id, "0x301");
        assert_eq!(top.class_name, "test/Holder");
        assert!(!top.is_class);
        assert_eq!(top.retained_bytes, 120);
        assert_eq!(top.retained_objects, 3);
        assert!((top.pct - 100.0).abs() < 1e-9);
        // 路径：Cache(root) → holder
        assert_eq!(top.path.len(), 2);
        assert!(top.path[0].is_root);
        assert_eq!(top.path[0].class_name, "test/Cache");
        assert_eq!(top.path[1].class_name, "test/Holder");
        assert_eq!(top.root_kind, "system-class");
        // 两个 leaf 实例排在后面（retained = 32）
        assert!(su.len() >= 3);
        for sp in su.iter().skip(1) {
            assert_eq!(sp.class_name, "test/Leaf");
            assert_eq!(sp.retained_bytes, 32);
        }
    }

    /// 真实 HotSpot dump 集成探针（手动跑：cargo test real_dump_probe -- --ignored --nocapture）
    /// 前提：mat-realcheck/leak_demo.hprof 已用 LeakDemo.java + JDK8 dumpHeap 生成。
    /// 期望：静态持有（CACHE 6.4MB + MAP + BIG 2MB）的 LeakDemo 类对象成为第一大嫌疑。
    #[test]
    #[ignore]
    fn real_dump_probe() {
        let p = "D:/ideaProject/BackendDevelopmentTools/mat-realcheck/leak_demo.hprof";
        let res = mat_leak_suspects(p).unwrap();
        let d = res.data;
        let s = &d.summary;
        println!(
            "nodes={} shallow={} roots={} edges={} skipped={} reachable={} ({}B) unreachable={} ({}B) truncated={} elapsed={}ms",
            s.total_nodes, s.total_shallow, s.root_nodes, s.edge_count, s.skipped_edge_count,
            s.reachable_nodes, s.reachable_shallow, s.unreachable_nodes, s.unreachable_shallow,
            s.truncated, s.elapsed_ms
        );
        for n in &d.notes {
            println!("note: {n}");
        }
        for sp in d.suspects.iter().take(8) {
            let tag = if sp.is_class { "[class]" } else { "" };
            let path_s: Vec<String> = sp
                .path
                .iter()
                .map(|p| format!("{}{}", p.class_name, if p.is_root { "*" } else { "" }))
                .collect();
            println!(
                "#{} {} {} shallow={} retained={} objs={} pct={:.1}% root={} path={}",
                sp.rank,
                tag,
                sp.obj_id,
                sp.shallow_bytes,
                sp.retained_bytes,
                sp.retained_objects,
                sp.pct,
                sp.root_kind,
                path_s.join(" -> ")
            );
        }
        // 断言：第一大嫌疑是静态持有者（类对象或 HashMap/ArrayList）
        let top = &d.suspects[0];
        assert!(top.retained_bytes > 5_000_000, "top retained too small: {}", top.retained_bytes);
        assert!(s.reachable_nodes > 1000);
    }
}
