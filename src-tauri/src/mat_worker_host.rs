//! MAT sidecar 作业管理（父进程侧）。
//!
//! 子进程逻辑在 `bdevtools_plugin_mat::mat_worker`（纯 Rust，不含 tauri）；
//! 这里只管三件事：**起进程、读它的 NDJSON 进度、允许杀掉它**。
//!
//! ## 为什么用 `current_exe()` 自派发，而不是再打包一个 exe
//!
//! 多一个可执行文件就要多一份签名 / 安装清单 / 更新逻辑，而 sidecar 与主程序
//! **共享同一份二进制与同一套配置目录解析**（`app_dir` / `plugins/mat.json` 都靠
//! 「app 目录」定位）。自派发的代价只是 main 里一次 argv 判断。
//!
//! 风险是「用户参数撞名」：只认 `argv[1] == __mat_worker`（见 `is_worker_invocation`），
//! 且该分支在 Tauri Builder 之前返回 —— 不会建窗口、不会触发单实例插件。
//!
//! ## 事件
//!
//! 每行子进程输出原样转发到前端事件 `mat-worker`，附加 `job_id` / `path`。
//! 前端按 `ev` 分派：`stage` 更新进度、`done` 刷新索引与统计、`error` 提示、
//! `exit` 收尾（**无论成功失败都会发**，否则前端会一直转圈）。

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::response::resp_data::ResData;

/// 前端监听的事件名
pub const EVENT: &str = "mat-worker";
/// 同时保留的已结束作业数（只为让「切页面再回来」看得到上次结果）
const KEEP_FINISHED: usize = 8;

/// 作业快照（前端契约）
#[derive(Debug, Clone, Serialize)]
pub struct MatWorkerJob {
    pub job_id: String,
    pub path: String,
    /// 是否连支配树一起算
    pub dom: bool,
    /// 当前阶段：`starting` / `graph-meta` / `graph-edges` / `dom` / `dom-lt` / `dom-retained` / `done` / `error`
    pub phase: String,
    pub done: u64,
    pub total: Option<u64>,
    pub msg: String,
    /// 已收到 done / error / exit（终态）
    pub finished: bool,
    /// 成功（拿到 done）
    pub ok: bool,
    pub canceled: bool,
    pub error: Option<String>,
    pub nodes: Option<u32>,
    pub edges: Option<usize>,
    /// 支配树结果：`ready` / `unavailable` / `skipped`
    pub dom_state: Option<String>,
    pub pid: Option<u32>,
    pub elapsed_ms: u64,
}

#[derive(Default)]
struct State {
    phase: String,
    done: u64,
    total: Option<u64>,
    msg: String,
    finished: bool,
    ok: bool,
    canceled: bool,
    error: Option<String>,
    nodes: Option<u32>,
    edges: Option<usize>,
    dom_state: Option<String>,
    pid: Option<u32>,
    elapsed_ms: u64,
}

struct Job {
    id: String,
    path: String,
    dom: bool,
    started: std::time::Instant,
    /// `None` = 已被收尾线程取走（此时进程已结束，kill 无意义）
    child: Mutex<Option<Child>>,
    st: Mutex<State>,
}

impl Job {
    fn snapshot(&self) -> MatWorkerJob {
        let st = self.st.lock().unwrap();
        MatWorkerJob {
            job_id: self.id.clone(),
            path: self.path.clone(),
            dom: self.dom,
            phase: st.phase.clone(),
            done: st.done,
            total: st.total,
            msg: st.msg.clone(),
            finished: st.finished,
            ok: st.ok,
            canceled: st.canceled,
            error: st.error.clone(),
            nodes: st.nodes,
            edges: st.edges,
            dom_state: st.dom_state.clone(),
            pid: st.pid,
            elapsed_ms: if st.finished {
                st.elapsed_ms
            } else {
                self.started.elapsed().as_millis() as u64
            },
        }
    }

    /// 吸收一行子进程 JSON
    fn absorb(&self, v: &serde_json::Value) {
        let num = |k: &str| v.get(k).and_then(|x| x.as_u64());
        let text = |k: &str| v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string());
        let ev = v.get("ev").and_then(|x| x.as_str()).unwrap_or("");
        let mut st = self.st.lock().unwrap();
        match ev {
            "hello" => {
                st.pid = num("pid").map(|p| p as u32);
                st.phase = "starting".to_string();
                st.msg = "分析进程已启动".to_string();
            }
            "stage" => {
                if let Some(s) = text("stage") {
                    st.phase = s;
                }
                if let Some(d) = num("done") {
                    st.done = d;
                }
                st.total = num("total");
                if let Some(m) = text("msg") {
                    st.msg = m;
                }
            }
            "done" => {
                st.ok = true;
                st.finished = true;
                st.phase = "done".to_string();
                st.done = st.total.unwrap_or(st.done);
                st.nodes = num("nodes").map(|n| n as u32);
                st.edges = num("edges").map(|n| n as usize);
                st.dom_state = text("dom_state");
                st.elapsed_ms = num("ms").unwrap_or_else(|| self.started.elapsed().as_millis() as u64);
                st.msg = "分析完成".to_string();
            }
            "error" => {
                st.finished = true;
                st.phase = "error".to_string();
                st.error = text("message").or_else(|| Some("分析进程报错".to_string()));
                st.elapsed_ms = num("ms").unwrap_or_else(|| self.started.elapsed().as_millis() as u64);
            }
            _ => {}
        }
    }
}

lazy_static::lazy_static! {
    static ref JOBS: Mutex<HashMap<String, Arc<Job>>> = Mutex::new(HashMap::new());
    static ref SEQ: AtomicU64 = AtomicU64::new(0);
}

fn jobs() -> &'static Mutex<HashMap<String, Arc<Job>>> {
    &JOBS
}

/// 起一个分析作业。同一路径已在跑时**复用现有作业**（连点不该起一堆进程互相抢 CPU）。
pub fn start(app: &AppHandle, path: &str, dom: bool) -> Result<MatWorkerJob, String> {
    if path.trim().is_empty() {
        return Err("堆转储路径为空".to_string());
    }
    // 复用 / 腾位置
    {
        let mut map = jobs().lock().unwrap();
        if let Some(j) = map.values().find(|j| j.path == path && !j.st.lock().unwrap().finished) {
            return Ok(j.snapshot());
        }
        // 已结束的只留最近几条，其余清掉（快照已发过，前端不需要无限多的历史）
        let mut finished: Vec<(u64, String)> = map
            .iter()
            .filter(|(_, j)| j.st.lock().unwrap().finished)
            .map(|(k, j)| (j.started.elapsed().as_millis() as u64, k.clone()))
            .collect();
        finished.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, k) in finished.into_iter().skip(KEEP_FINISHED) {
            map.remove(&k);
        }
    }

    let exe = std::env::current_exe().map_err(|e| format!("无法定位应用可执行文件: {e}"))?;
    let mut cmd = Command::new(exe);
    cmd.arg(crate::plugins::mat::mat_worker::WORKER_ARG)
        .arg("--path")
        .arg(path)
        // stdin 给 null：子进程只从 argv 取参数，绝不能等着读输入
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if dom {
        cmd.arg("--dom");
    }
    // Windows：别让 debug 构建的子进程继承父控制台（Ctrl+C / 关窗口会连带杀掉分析）
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn().map_err(|e| format!("启动分析进程失败: {e}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let id = format!(
        "mw-{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let job = Arc::new(Job {
        id,
        path: path.to_string(),
        dom,
        started: std::time::Instant::now(),
        child: Mutex::new(Some(child)),
        st: Mutex::new(State {
            phase: "spawned".to_string(),
            msg: "正在启动分析进程".to_string(),
            ..Default::default()
        }),
    });
    let snap = job.snapshot();
    jobs().lock().unwrap().insert(snap.job_id.clone(), job.clone());

    // stderr 只做诊断：子进程 panic / 系统级错误会写这里，解析不了但报错时要有据可查
    let err_buf = Arc::new(Mutex::new(String::new()));
    let err_handle = {
        let buf = err_buf.clone();
        std::thread::spawn(move || {
            if let Some(e) = stderr {
                for line in BufReader::new(e).lines().map_while(Result::ok) {
                    let mut b = buf.lock().unwrap();
                    if b.len() < 8192 {
                        b.push_str(&line);
                        b.push('\n');
                    }
                }
            }
        })
    };

    let app2 = app.clone();
    let job2 = job.clone();
    std::thread::spawn(move || {
        if let Some(out) = stdout {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                // 非协议行（例如某天有人在 worker 里 print 调试）当日志转发，不丢弃
                let v = serde_json::from_str::<serde_json::Value>(line)
                    .unwrap_or_else(|_| serde_json::json!({ "ev": "log", "msg": line }));
                if v.get("ev").and_then(|x| x.as_str()).is_some() {
                    job2.absorb(&v);
                }
                forward(&app2, &job2, v);
            }
        }
        let _ = err_handle.join();
        // 收尾：stdout EOF 说明进程已结束（或已 kill），此时 wait 不会阻塞
        let code = job2
            .child
            .lock()
            .unwrap()
            .take()
            .and_then(|mut c| c.wait().ok())
            .and_then(|s| s.code());
        {
            let mut st = job2.st.lock().unwrap();
            st.finished = true;
            st.elapsed_ms = job2.started.elapsed().as_millis() as u64;
            if st.phase == "error" {
                // 子进程自己报了错，保留原文
            } else if st.ok {
                st.phase = "done".to_string();
            } else if st.canceled {
                st.phase = "canceled".to_string();
                st.error = Some("已取消".to_string());
            } else {
                // 没拿到 done 也没拿到 error：崩溃 / 被外部杀掉
                st.phase = "error".to_string();
                let tail = err_buf.lock().unwrap().trim().to_string();
                st.error = Some(if tail.is_empty() {
                    format!("分析进程异常退出（code={code:?}）")
                } else {
                    format!("分析进程异常退出（code={code:?}）：{tail}")
                });
            }
        }
        let (canceled, ok, err) = {
            let st = job2.st.lock().unwrap();
            (st.canceled, st.ok, st.error.clone())
        };
        forward(
            &app2,
            &job2,
            serde_json::json!({
                "ev": "exit",
                "code": code,
                "canceled": canceled,
                "ok": ok,
                "error": err,
            }),
        );
    });

    Ok(snap)
}

/// 取消：直接杀进程。
///
/// 不做「发信号让子进程自己退」—— 解析循环里没有安全的让出点（埋点只在 tick 里，
/// 而 tick 要判标志就得让每个循环都查一次原子量）。进程被杀时索引写入走的是
/// 「唯一临时名 → rename」，残留临时文件不影响一致性。
pub fn cancel(job_id: &str) -> bool {
    let job = match jobs().lock().unwrap().get(job_id).cloned() {
        Some(j) => j,
        None => return false,
    };
    {
        let mut st = job.st.lock().unwrap();
        if st.finished {
            return false;
        }
        st.canceled = true;
        st.msg = "正在取消…".to_string();
    }
    let mut guard = job.child.lock().unwrap();
    match guard.as_mut() {
        Some(c) => {
            let _ = c.kill();
            true
        }
        None => false,
    }
}

pub fn status(job_id: &str) -> Option<MatWorkerJob> {
    jobs().lock().unwrap().get(job_id).map(|j| j.snapshot())
}

/// 最近一次某 dump 的作业（页面重新挂载时用它恢复进度条）
pub fn status_for_path(path: &str) -> Option<MatWorkerJob> {
    jobs()
        .lock()
        .unwrap()
        .values()
        .filter(|j| j.path == path)
        .max_by_key(|j| j.started)
        .map(|j| j.snapshot())
}

/// 转发一行事件到前端（附加作业标识）
fn forward(app: &AppHandle, job: &Job, mut v: serde_json::Value) {
    if let Some(obj) = v.as_object_mut() {
        obj.insert("job_id".to_string(), serde_json::json!(job.id));
        obj.insert("path".to_string(), serde_json::json!(job.path));
        // `dom` = 本作业是否算支配树；**不要**和 `dom_state`（支配树结果）混用同一个键
        obj.insert("with_dom".to_string(), serde_json::json!(job.dom));
    }
    let _ = app.emit(EVENT, v);
}

/// main 侧分派：argv 命中 worker 标记时**直接跑子进程逻辑**，不初始化 Tauri。
/// 返回 `Some(exit_code)` 表示本次进程就是 sidecar。
pub fn try_run_as_worker() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if !crate::plugins::mat::mat_worker::is_worker_invocation(&args) {
        return None;
    }
    Some(crate::plugins::mat::mat_worker::run_args(&args))
}

// ==================== 命令入口 ====================

fn wrap(msg: &str, data: MatWorkerJob) -> Result<ResData<MatWorkerJob>, String> {
    Ok(ResData { code: 200, msg: msg.to_string(), data })
}

/// 启动后台分析进程（大 dump 不卡主窗口）
pub fn mat_worker_start(
    app: &AppHandle,
    path: &str,
    dom: bool,
) -> Result<ResData<MatWorkerJob>, String> {
    let job = start(app, path, dom)?;
    let msg = if job.finished { "作业已结束" } else { "分析进程已启动" };
    wrap(msg, job)
}

/// 取消后台分析（杀进程；已结束返回 false）
pub fn mat_worker_cancel(job_id: &str) -> Result<ResData<bool>, String> {
    let hit = cancel(job_id);
    Ok(ResData {
        code: 200,
        msg: if hit { "已请求取消".to_string() } else { "作业不存在或已结束".to_string() },
        data: hit,
    })
}

/// 查询作业状态（事件丢失时前端轮询兜底）
pub fn mat_worker_status(job_id: &str) -> Result<ResData<MatWorkerJob>, String> {
    let job = status(job_id).ok_or_else(|| "作业不存在（可能已过期）".to_string())?;
    wrap("ok", job)
}

/// 某 dump 最近一次作业（页面重挂载恢复进度条）
pub fn mat_worker_status_for_path(path: &str) -> Result<ResData<Option<MatWorkerJob>>, String> {
    Ok(ResData { code: 200, msg: "ok".to_string(), data: status_for_path(path) })
}
