/*! # SSH 端口转发：-L / -R / -D

任务表在**进程内存**（规则本身由前端持久化）。每个任务独占一条 SSH 会话，
避免与终端/sftp 的读循环争用同一把 Session 锁。

线程模型：
- 1 个 accept 线程（nonblocking，20ms 轮询停止标志）
- 每个连接 1 个 relay 线程（`relay::relay_pair` 单线程双向泵）
- 停止时置 `AtomicBool` ⇒ listener 与所有 relay 在 ≤20ms 内自行退出并释放 socket/channel

为什么全程 nonblocking：ssh2 的 `Stream::read` 在读取期间持有 `SessionInner` 锁，
blocking 模式会锁到对端来数据为止，导致 `channel_direct_tcpip` 和另一方向写全部卡住。
*/
use crate::options::parse_target;
use crate::relay::{relay_pair, RelayStats};
use crate::remote_conn::{connect_failure, establish_session, params_from_target, ConnectError};
use anyhow::{anyhow, Result};
use bdevtools_core::response::response::Response;
use serde::{Deserialize, Serialize};
use ssh2::{Listener, Session};
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

lazy_static::lazy_static! {
    static ref TASKS: Mutex<HashMap<String, Arc<ForwardTask>>> = Mutex::new(HashMap::new());
}

/// 前端定义的一条转发规则。`kind`: local(-L) / remote(-R) / dynamic(-D SOCKS5)
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ForwardRule {
    pub id: String,
    pub kind: String,
    /// local / dynamic: 本地监听地址；remote: 远端监听地址
    pub bind_host: String,
    pub bind_port: u16,
    /// local: 远端目标；remote: 本地目标；dynamic: 忽略（由 SOCKS 请求决定）
    pub target_host: String,
    pub target_port: u16,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ForwardInfo {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub bind_host: String,
    pub bind_port: u16,
    pub target_host: String,
    pub target_port: u16,
    /// running | stopped | error
    pub status: String,
    pub error: String,
    pub connections: u64,
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub started_at: u64,
}

struct ForwardTask {
    info: Arc<Mutex<ForwardInfo>>,
    stop: Arc<AtomicBool>,
    stats: Arc<RelayStats>,
    connections: Arc<AtomicU64>,
}

enum Backend {
    Local(TcpListener),
    Remote(Listener),
}

/// nonblocking 下 `channel_direct_tcpip` 可能先返回 EAGAIN，重试到成功或超时。
/// 不区分错误码是为了不依赖 libssh2 的内部常量；真错误最多多等 3 秒，用户基本无感。
fn open_direct(session: &Session, host: &str, port: u16) -> Result<ssh2::Channel> {
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut last = String::new();
    while Instant::now() < deadline {
        match session.channel_direct_tcpip(host, port, None) {
            Ok(ch) => return Ok(ch),
            Err(e) => {
                last = e.to_string();
                thread::sleep(Duration::from_millis(20));
            }
        }
    }
    Err(anyhow!(
        "建立到 {}:{} 的 direct-tcpip 通道失败: {}",
        host,
        port,
        last
    ))
}

/// SOCKS5 服务端握手（无鉴权、只支持 CONNECT），返回客户端想连的目标。
fn socks5_negotiate(tcp: &mut TcpStream) -> Result<(String, u16)> {
    let _ = tcp.set_nonblocking(false);
    tcp.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut greeting = [0u8; 2];
    tcp.read_exact(&mut greeting)?;
    if greeting[0] != 0x05 {
        return Err(anyhow!("不是 SOCKS5 客户端"));
    }
    let mut methods = vec![0u8; greeting[1] as usize];
    tcp.read_exact(&mut methods)?;
    tcp.write_all(&[0x05, 0x00])?;

    let mut head = [0u8; 4];
    tcp.read_exact(&mut head)?;
    if head[1] != 0x01 {
        // 只支持 CONNECT，UDP ASSOCIATE / BIND 明确拒绝
        let _ = tcp.write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
        return Err(anyhow!("SOCKS5 仅支持 CONNECT"));
    }
    let host = match head[3] {
        0x01 => {
            let mut ip = [0u8; 4];
            tcp.read_exact(&mut ip)?;
            format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
        }
        0x03 => {
            let mut n = [0u8; 1];
            tcp.read_exact(&mut n)?;
            let mut name = vec![0u8; n[0] as usize];
            tcp.read_exact(&mut name)?;
            String::from_utf8_lossy(&name).to_string()
        }
        0x04 => {
            let mut ip = [0u8; 16];
            tcp.read_exact(&mut ip)?;
            let parts: Vec<String> = ip
                .chunks(2)
                .map(|c| format!("{:02x}{:02x}", c[0], c[1]))
                .collect();
            parts.join(":")
        }
        _ => return Err(anyhow!("SOCKS5 地址类型非法")),
    };
    let mut port = [0u8; 2];
    tcp.read_exact(&mut port)?;
    Ok((host, u16::from_be_bytes(port)))
}

const SOCKS_OK: [u8; 10] = [0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
const SOCKS_FAIL: [u8; 10] = [0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0];

fn spawn_relay(
    mut tcp: TcpStream,
    channel: ssh2::Channel,
    socks_reply: Option<[u8; 10]>,
    task: &Arc<ForwardTask>,
) {
    task.connections.fetch_add(1, Ordering::Relaxed);
    if let Some(reply) = socks_reply {
        let _ = tcp.write_all(&reply);
        let _ = tcp.flush();
    }
    let stop = task.stop.clone();
    let stats = task.stats.clone();
    let connections = task.connections.clone();
    thread::spawn(move || {
        relay_pair(tcp, channel, stop, Some(stats));
        connections.fetch_sub(1, Ordering::Relaxed);
    });
}

/// accept 主循环。session 由本线程持有到退出；已建立的 channel 自带 `Arc<SessionInner>`，
/// 即使 accept 线程先退出，relay 仍能正常工作。
fn run_accept(
    session: Session,
    backend: Backend,
    rule: ForwardRule,
    task: Arc<ForwardTask>,
) {
    let _keepalive = session;
    let stop = task.stop.clone();
    let local_target = (
        if rule.target_host.is_empty() {
            "127.0.0.1".to_string()
        } else {
            rule.target_host.clone()
        },
        rule.target_port,
    );

    match backend {
        Backend::Local(listener) => {
            while !stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut tcp, _)) => {
                        if rule.kind == "dynamic" {
                            let target = match socks5_negotiate(&mut tcp) {
                                Ok(v) => v,
                                Err(_) => {
                                    let _ = tcp.write_all(&SOCKS_FAIL);
                                    let _ = tcp.shutdown(Shutdown::Both);
                                    continue;
                                }
                            };
                            let channel = match open_direct(&_keepalive, &target.0, target.1) {
                                Ok(c) => c,
                                Err(_) => {
                                    let _ = tcp.write_all(&SOCKS_FAIL);
                                    let _ = tcp.shutdown(Shutdown::Both);
                                    continue;
                                }
                            };
                            spawn_relay(tcp, channel, Some(SOCKS_OK), &task);
                        } else {
                            let channel = match open_direct(
                                &_keepalive,
                                &local_target.0,
                                local_target.1,
                            ) {
                                Ok(c) => c,
                                Err(_) => {
                                    let _ = tcp.shutdown(Shutdown::Both);
                                    continue;
                                }
                            };
                            spawn_relay(tcp, channel, None, &task);
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(e) if e.kind() == ErrorKind::Interrupted => {}
                    Err(e) => {
                        if let Ok(mut info) = task.info.lock() {
                            info.status = "error".to_string();
                            info.error = format!("本地监听异常: {}", e);
                        }
                        break;
                    }
                }
            }
        }
        Backend::Remote(mut listener) => {
            while !stop.load(Ordering::Relaxed) {
                // 非阻塞下「暂无连接」也是 Err(EAGAIN)，与真错误难以区分；
                // 统一重试，靠 stop 标志退出，避免一次误判把整条转发停掉。
                match listener.accept() {
                    Ok(mut channel) => match TcpStream::connect((
                        local_target.0.as_str(),
                        local_target.1,
                    )) {
                        Ok(tcp) => spawn_relay(tcp, channel, None, &task),
                        Err(_) => {
                            let _ = channel.close();
                        }
                    },
                    Err(_) => thread::sleep(Duration::from_millis(20)),
                }
            }
        }
    }
    if let Ok(mut info) = task.info.lock() {
        if info.status == "running" {
            info.status = if stop.load(Ordering::Relaxed) {
                "stopped".to_string()
            } else {
                "error".to_string()
            };
        }
    }
}

/// 启动一条转发规则。`target` 是 ConnectTarget JSON，`rule` 是 ForwardRule JSON。
pub async fn ssh_tunnel_start(target: String, rule: String) -> Response<ForwardInfo> {
    let t = match parse_target(&target) {
        Ok(t) => t,
        Err(e) => return Response::new(300, ForwardInfo::default(), &e.to_string()),
    };
    let mut rule: ForwardRule = match serde_json::from_str(rule.trim()) {
        Ok(r) => r,
        Err(e) => {
            return Response::new(
                300,
                ForwardInfo::default(),
                &format!("转发规则不是合法 JSON: {}", e),
            )
        }
    };
    if rule.id.is_empty() {
        rule.id = format!("fwd-{}", uuid::Uuid::new_v4());
    }
    if !matches!(rule.kind.as_str(), "local" | "remote" | "dynamic") {
        return Response::new(300, ForwardInfo::default(), "转发类型必须是 local / remote / dynamic");
    }
    if rule.bind_port == 0 {
        return Response::new(300, ForwardInfo::default(), "监听端口不能为 0");
    }
    if rule.kind != "dynamic" && rule.target_port == 0 {
        return Response::new(300, ForwardInfo::default(), "目标端口不能为 0");
    }
    if rule.bind_host.is_empty() {
        rule.bind_host = "127.0.0.1".to_string();
    }
    // 同 id 重复启动先停旧的，避免端口被占却返回成功
    stop_by_id(&rule.id);

    let params = params_from_target(t);
    let rule_for_thread = rule.clone();
    let rule_for_accept = rule.clone();
    let outcome = tokio::task::spawn_blocking(
        move || -> std::result::Result<(ForwardInfo, Session, Backend), ConnectError> {
            let mut effective = rule_for_thread.clone();
            let session = establish_session(&params)?;
            let backend = if effective.kind == "remote" {
                let (listener, bound) = session.channel_forward_listen(
                    effective.bind_port,
                    Some(&effective.bind_host),
                    None,
                )?;
                // 远端实际分配的端口可能与请求的不同，回填给 UI 展示
                if bound != 0 {
                    effective.bind_port = bound;
                }
                Backend::Remote(listener)
            } else {
                let listener = TcpListener::bind((
                    effective.bind_host.as_str(),
                    effective.bind_port,
                ))?;
                listener.set_nonblocking(true)?;
                effective.bind_port = listener.local_addr()?.port();
                Backend::Local(listener)
            };
            // 所有 channel 已就绪（或本地监听已建立），此后全程非阻塞
            session.set_blocking(false);
            let info = ForwardInfo {
                id: effective.id.clone(),
                kind: effective.kind.clone(),
                label: effective.label.clone(),
                bind_host: effective.bind_host.clone(),
                bind_port: effective.bind_port,
                target_host: effective.target_host.clone(),
                target_port: effective.target_port,
                status: "running".to_string(),
                error: String::new(),
                connections: 0,
                bytes_up: 0,
                bytes_down: 0,
                started_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            };
            Ok((info, session, backend))
        },
    )
    .await;

    match outcome {
        Ok(Ok((info, session, backend))) => {
            let task = Arc::new(ForwardTask {
                info: Arc::new(Mutex::new(info.clone())),
                stop: Arc::new(AtomicBool::new(false)),
                stats: Arc::new(RelayStats::default()),
                connections: Arc::new(AtomicU64::new(0)),
            });
            let task_for_thread = task.clone();
            thread::spawn(move || run_accept(session, backend, rule_for_accept, task_for_thread));
            TASKS.lock().unwrap().insert(info.id.clone(), task);
            Response::new(200, info, "转发已启动")
        }
        Ok(Err(e)) => connect_failure(e, "转发启动失败: "),
        Err(e) => Response::new(
            300,
            ForwardInfo::default(),
            &format!("转发启动异常: {}", e),
        ),
    }
}

fn stop_by_id(id: &str) {
    if let Some(task) = TASKS.lock().unwrap().remove(id) {
        task.stop.store(true, Ordering::Relaxed);
        if let Ok(mut info) = task.info.lock() {
            info.status = "stopped".to_string();
        }
    }
}

pub async fn ssh_tunnel_stop(id: String) -> Response<bool> {
    let existed = TASKS.lock().unwrap().contains_key(&id);
    stop_by_id(&id);
    if existed {
        Response::new(200, true, "转发已停止")
    } else {
        Response::new(300, false, "转发任务不存在")
    }
}

/// 列出运行中的转发任务，带实时连接数与累计流量。
pub async fn ssh_tunnel_list() -> Response<Vec<ForwardInfo>> {
    let tasks: Vec<Arc<ForwardTask>> = { TASKS.lock().unwrap().values().cloned().collect() };
    let mut out = Vec::with_capacity(tasks.len());
    for task in tasks {
        if let Ok(info) = task.info.lock() {
            let mut item = info.clone();
            item.connections = task.connections.load(Ordering::Relaxed);
            item.bytes_up = task.stats.bytes_up.load(Ordering::Relaxed);
            item.bytes_down = task.stats.bytes_down.load(Ordering::Relaxed);
            out.push(item);
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Response::new(200, out, "查询成功")
}

pub async fn ssh_tunnel_stop_all() -> Response<bool> {
    let ids: Vec<String> = { TASKS.lock().unwrap().keys().cloned().collect() };
    for id in ids {
        stop_by_id(&id);
    }
    Response::new(200, true, "已停止全部转发")
}
