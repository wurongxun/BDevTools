//! sidecar 分析进程（`<app> __mat_worker --path <dump> [--dom]`）。
//!
//! ## 为什么子进程只做「离线编译」，不管查询
//!
//! 主窗口卡的根因是一次性的重活：**解析整份 dump + 建对象图 + 跑 Lengauer-Tarjan**，
//! 在一份 2GB / 千万对象的堆上是分钟级；而所有后续查询本身都是毫秒级的分页 top-N。
//!
//! 让子进程把这段重活「编译」成索引（复用 P3-2 的 `.bdtidx` / `.bdtidx.dom`），
//! 父进程之后 `mat_index::load_graph` 命中即秒回 —— **26 条查询命令一行都不用改**，
//! 也不必为一个插件引入 RPC 层与「哪些命令走远端」的路由表。
//!
//! 代价是「结果要落盘才能共享」。这恰好是既有索引机制在做的事，所以代价≈0；
//! 反过来说，如果将来要支持「不落盘的临时分析」，才需要真的加 IPC 查询通道。
//!
//! ## 协议
//!
//! stdout 每行一个 JSON（NDJSON），父进程逐行读：
//!
//! ```json
//! {"ev":"hello","pid":1234,"path":"...","dom":true}
//! {"ev":"stage","stage":"graph-meta","done":0,"total":null,"msg":"..."}
//! {"ev":"done","ms":1200,"nodes":49730,"edges":69124,"dom_state":"ready"}
//! {"ev":"error","message":"..."}
//! ```
//!
//! 只用 stdout 传数据、stderr 传「不该被解析的日志」，两者都由父进程收。
//! 取消 = 父进程直接 `kill`：进程被终止时 `.part` 解压文件与索引临时文件都不影响
//! 一致性（写入一律「唯一临时名 → rename」，见 `mat_source` / `mat_index`）。

use std::io::Write;

/// 子进程身份标记（argv[1]）
pub const WORKER_ARG: &str = "__mat_worker";

/// argv 是否为 worker 调用。**必须在 Tauri 初始化之前判定**，否则会真的去建窗口。
pub fn is_worker_invocation(args: &[String]) -> bool {
    args.get(1).map(|s| s == WORKER_ARG).unwrap_or(false)
}

/// 解析出的任务参数
struct Task {
    path: String,
    dom: bool,
}

fn parse_args(args: &[String]) -> Result<Task, String> {
    let mut path = None;
    let mut dom = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--path" => {
                path = args.get(i + 1).cloned();
                i += 2;
            }
            "--dom" => {
                dom = true;
                i += 1;
            }
            other => return Err(format!("未知参数: {other}")),
        }
    }
    Ok(Task {
        path: path.ok_or_else(|| "缺少 --path".to_string())?,
        dom,
    })
}

/// 写一行 NDJSON 并**立即 flush**：父进程是阻塞读，缓冲住就等于没有进度。
fn emit(v: serde_json::Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

/// 把 `mat_progress` 的事件转成协议行
fn stage_event(p: crate::mat_progress::Progress) {
    emit(serde_json::json!({
        "ev": "stage",
        "stage": p.stage,
        "done": p.done,
        "total": p.total,
        "msg": p.msg,
    }));
}

/// 子进程入口。返回退出码（0 成功 / 1 业务错误 / 2 参数错误）。
///
/// 这个函数**不依赖 tauri**：mat crate 是纯业务层，宿主只负责把 argv 转进来。
pub fn run_args(args: &[String]) -> i32 {
    let task = match parse_args(args) {
        Ok(t) => t,
        Err(e) => {
            emit(serde_json::json!({ "ev": "error", "message": e }));
            return 2;
        }
    };

    emit(serde_json::json!({
        "ev": "hello",
        "pid": std::process::id(),
        "path": task.path,
        "dom": task.dom,
    }));
    crate::mat_progress::set_sink(Box::new(stage_event));

    let started = std::time::Instant::now();
    // 解析器对未知子记录（ART 变体）会 panic，与主进程同样用 catch_parse 兜住：
    // 子进程崩溃也要给父进程一条 error，而不是只留一个非零退出码。
    let result = crate::mat_hprof::catch_parse(|| -> Result<(u32, usize, &'static str), String> {
        let session = crate::mat_refs::ensure_session(&task.path)?;
        let nodes = crate::mat_refs::session_graph(&session).nodes.len() as u32;
        let edges = crate::mat_refs::session_graph(&session).edge_to.len();
        if !task.dom {
            return Ok((nodes, edges, "skipped"));
        }
        crate::mat_progress::stage("dom", None, "准备支配树");
        match crate::mat_dom::ensure_dom(&session)? {
            crate::mat_dom::DomReady::Ready(_) => Ok((nodes, edges, "ready")),
            // 规模超闸 / 无 GC Roots 属于「正常但没结果」，不是失败：照常退出 0，
            // 父进程按 `dom:"unavailable"` 展示原因（用户在 UI 里还能调预算重试）。
            crate::mat_dom::DomReady::Unavailable(_) => Ok((nodes, edges, "unavailable")),
        }
    });

    crate::mat_progress::clear_sink();
    let ms = started.elapsed().as_millis() as u64;
    match result {
        Ok((nodes, edges, dom)) => {
            emit(serde_json::json!({
                "ev": "done",
                "ms": ms,
                "nodes": nodes,
                "edges": edges,
                // 字段名带 `_state`：父进程转发时还会加一个 `dom: bool`（本作业是否算支配树），
                // 同名会让「支配树结果」被布尔值覆盖
                "dom_state": dom,
            }));
            0
        }
        Err(message) => {
            emit(serde_json::json!({ "ev": "error", "message": message, "ms": ms }));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn recognizes_worker_invocation() {
        assert!(is_worker_invocation(&args(&["app.exe", WORKER_ARG])));
        // 正常启动（无参数 / 普通参数）绝不能进 worker 分支
        assert!(!is_worker_invocation(&args(&["app.exe"])));
        assert!(!is_worker_invocation(&args(&["app.exe", "--help"])));
        // 位置固定在第 2 个参数：别处出现同名字符串不算
        assert!(!is_worker_invocation(&args(&["app.exe", "--path", WORKER_ARG])));
    }

    #[test]
    fn parse_args_requires_path() {
        // argv[0] 是 exe，argv[1] 才是 worker 标记 —— 参数从 argv[2] 开始解析
        assert!(parse_args(&args(&["app.exe", WORKER_ARG])).is_err());
        let t = parse_args(&args(&["app.exe", WORKER_ARG, "--path", "a.hprof"]))
            .ok()
            .unwrap();
        assert_eq!(t.path, "a.hprof");
        assert!(!t.dom);
        let t = parse_args(&args(&["app.exe", WORKER_ARG, "--dom", "--path", "b.hprof"]))
            .ok()
            .unwrap();
        assert!(t.dom);
        assert!(parse_args(&args(&["app.exe", WORKER_ARG, "--bogus"])).is_err());
    }

    /// 端到端：真跑一遍子进程逻辑（同进程内直调 `run_args`），断言事件序列。
    /// 依赖真实 dump ⇒ 走 `BDT_MAT_SAMPLE` 注入，未设置则 skip。
    #[test]
    fn run_emits_hello_stage_done() {
        let Some(sample) = crate::test_support::sample_dump() else {
            return;
        };
        // `run_args` 直接写 stdout，测试里不解析它，只断言退出码与「没 panic」。
        // 事件内容由父进程侧（宿主）解析，这里验证的是「跑通 + 进度汇能装上又不炸」。
        let code = run_args(&args(&["app.exe", WORKER_ARG, "--path", &sample]));
        assert_eq!(code, 0, "worker 应在 {sample} 上成功退出");
        assert!(!crate::mat_progress::enabled(), "收尾必须卸掉进度汇");
    }
}
