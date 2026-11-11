/*! # 远程连接管理模块 - SSH终端 + SFTP文件操作

连接建立层（代理 / 跳板机链 / 主机密钥 / 认证 / 2FA）位于本文件前半段，
`crate::exec` 与 `crate::tunnel` 通过 `pub(crate)` 复用 `establish_session`，
保证「终端、SFTP、exec、端口转发」四条链路的安全口径不分叉。
*/
use anyhow::Result;
use base64::Engine;
use bdevtools_core::ctx::AppContext;
use bdevtools_core::response::response::Response;
use crate::options::{open_tcp, parse_options, ConnectOptions, JumpHost};
use crate::relay::relay_pair;
use serde::{Deserialize, Serialize};
use ssh2::{
    CheckResult, HashType, KeyboardInteractivePrompt, KnownHostFileKind, MethodType, Prompt,
    Session,
};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// 终端输出编码器：SSH 吐的是任意字节，跨块的多字节字符必须在前端按会话编码流式解码。
/// 这里只做 base64 透传，绝不 `from_utf8_lossy`（那会把半个汉字变成 U+FFFD）。
const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// 主机密钥类业务错误码：前端据此弹「信任主机」确认框 / 直接拒绝
pub const HOST_KEY_UNKNOWN: usize = 409;
pub const HOST_KEY_MISMATCH: usize = 410;
/// 服务器要求 keyboard-interactive 挑战（2FA），前端弹框收集验证码后带码重连
pub const MFA_REQUIRED: usize = 411;

// 全局会话管理
lazy_static::lazy_static! {
    static ref SSH_SESSIONS: Mutex<HashMap<String, Arc<Mutex<SshSessionInner>>>> = Mutex::new(HashMap::new());
    static ref SFTP_SESSIONS: Mutex<HashMap<String, Arc<Session>>> = Mutex::new(HashMap::new());
}

/// SSH 会话内部状态
struct SshSessionInner {
    session: Session,
    channel: Option<ssh2::Channel>,
    /// 读线程退出标志：disconnect / EOF / 读错误都会置位，防止读线程永久存活
    closed: bool,
    /// "manual" | "eof" | "error"，随 `ssh-closed` 事件回前端
    reason: &'static str,
    keepalive_interval: u32,
}

/// 建立连接所需的全部参数（终端 / SFTP / exec / 端口转发 / 日志拉取共用）
#[derive(Clone)]
pub(crate) struct ConnectParams {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) auth_type: String,
    pub(crate) private_key_path: String,
    pub(crate) passphrase: String,
    pub(crate) trust_host: bool,
    pub(crate) keepalive_interval: u32,
    /// P2 高级选项（代理 / 跳板机 / 压缩 / 算法 / 2FA），JSON 字符串解析而来
    pub(crate) options: ConnectOptions,
}

impl ConnectParams {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        host: String,
        port: u16,
        username: String,
        password: String,
        auth_type: Option<String>,
        private_key_path: Option<String>,
        passphrase: Option<String>,
        trust_host: Option<bool>,
        keepalive_interval: Option<u32>,
        options: Option<String>,
    ) -> Self {
        Self {
            host,
            port,
            username,
            password,
            auth_type: auth_type.unwrap_or_else(|| "password".to_string()),
            private_key_path: private_key_path.unwrap_or_default(),
            passphrase: passphrase.unwrap_or_default(),
            trust_host: trust_host.unwrap_or(false),
            keepalive_interval: keepalive_interval.unwrap_or(30),
            options: parse_options(options.as_deref()),
        }
    }
}

/// 把 P2 新命令的 JSON 目标转成内部连接参数；`mfaCode` 覆盖高级选项里的静态值。
pub(crate) fn params_from_target(t: crate::options::ConnectTarget) -> ConnectParams {
    let raw = match &t.advanced_options {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(v) => Some(v.to_string()),
        None => None,
    };
    let mut p = ConnectParams::new(
        t.host,
        if t.port == 0 { 22 } else { t.port },
        t.username,
        t.password,
        Some(t.auth_type),
        Some(t.private_key_path),
        Some(t.passphrase),
        Some(t.trust_host),
        Some(if t.keepalive_interval == 0 {
            30
        } else {
            t.keepalive_interval
        }),
        raw,
    );
    if !t.mfa_code.is_empty() {
        p.options.mfa_code = t.mfa_code;
    }
    p
}

/// 链路中的一跳。`logical_*` 用于 known_hosts 校验（必须是真实主机名，不能是回环桥端口）；
/// `socket_*` 是这一跳实际 TCP 要连的地址。
#[derive(Clone)]
struct HopSpec {
    logical_host: String,
    logical_port: u16,
    socket_host: String,
    socket_port: u16,
    username: String,
    password: String,
    auth_type: String,
    private_key_path: String,
    passphrase: String,
}

impl HopSpec {
    fn target(p: &ConnectParams, socket_host: String, socket_port: u16) -> Self {
        Self {
            logical_host: p.host.clone(),
            logical_port: p.port,
            socket_host,
            socket_port,
            username: p.username.clone(),
            password: p.password.clone(),
            auth_type: p.auth_type.clone(),
            private_key_path: p.private_key_path.clone(),
            passphrase: p.passphrase.clone(),
        }
    }

    fn jump(j: &JumpHost, socket_host: String, socket_port: u16) -> Self {
        Self {
            logical_host: j.host.clone(),
            logical_port: if j.port == 0 { 22 } else { j.port },
            socket_host,
            socket_port,
            username: j.username.clone(),
            password: j.password.clone(),
            auth_type: if j.auth_type.is_empty() {
                "password".to_string()
            } else {
                j.auth_type.clone()
            },
            private_key_path: j.private_key_path.clone(),
            passphrase: j.passphrase.clone(),
        }
    }
}

/// keyboard-interactive 应答器：收集服务器挑战文本；有验证码时按挑战数回填。
struct MfaPrompter {
    code: String,
    prompts: Arc<Mutex<Vec<String>>>,
}

impl KeyboardInteractivePrompt for MfaPrompter {
    fn prompt<'a>(
        &mut self,
        _username: &str,
        _instructions: &str,
        prompts: &[Prompt<'a>],
    ) -> Vec<String> {
        if let Ok(mut collected) = self.prompts.lock() {
            for p in prompts {
                collected.push(p.text.to_string());
            }
        }
        vec![self.code.clone(); prompts.len()]
    }
}

/// 连接失败分类：主机密钥错误必须让前端区分「首次连接可信任」与「疑似中间人攻击」
pub(crate) enum ConnectError {
    UnknownHost(String),
    HostKeyMismatch(String),
    /// 需要 2FA 验证码，携带服务器下发的挑战提示
    MfaRequired(String),
    Other(anyhow::Error),
}

impl From<anyhow::Error> for ConnectError {
    fn from(e: anyhow::Error) -> Self {
        ConnectError::Other(e)
    }
}

impl From<std::io::Error> for ConnectError {
    fn from(e: std::io::Error) -> Self {
        ConnectError::Other(e.into())
    }
}

impl From<ssh2::Error> for ConnectError {
    fn from(e: ssh2::Error) -> Self {
        ConnectError::Other(e.into())
    }
}

/// known_hosts 放 `app_dir`（用户资产），不放会被卸载脚本删掉的 `config_dir`。
fn known_hosts_path() -> PathBuf {
    let app_dir = bdevtools_core::app::app_store::APP.lock().app_dir.clone();
    app_dir.join("ssh").join("known_hosts")
}

/// 用户已有的 OpenSSH 记录（有则复用，不必让用户二次信任）
fn user_known_hosts_path() -> Option<PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;
    Some(PathBuf::from(home).join(".ssh").join("known_hosts"))
}

/// SHA256 指纹（base64，去掉尾部 '='，与 OpenSSH `ssh-keygen -lf` 展示习惯一致）
fn host_fingerprint(session: &Session) -> String {
    match session.host_key_hash(HashType::Sha256) {
        Some(bytes) => format!("SHA256:{}", B64.encode(bytes).trim_end_matches('=')),
        None => "SHA256:未知".to_string(),
    }
}

/// OpenSSH 对非 22 端口的 known_hosts 条目写成 `[host]:port`，add 与 check 必须用同一个 pattern
fn host_pattern(host: &str, port: u16) -> String {
    if port == 22 {
        host.to_string()
    } else {
        format!("[{}]:{}", host, port)
    }
}

/// 握手前应用算法偏好与压缩（对链路上的每一跳一致生效）。
fn apply_transport_prefs(
    session: &Session,
    p: &ConnectParams,
) -> std::result::Result<(), ConnectError> {
    if p.options.compress {
        session.set_compress(true);
    }
    let prefs = [
        (MethodType::CryptCs, p.options.ciphers.trim()),
        (MethodType::CryptSc, p.options.ciphers.trim()),
        (MethodType::MacCs, p.options.macs.trim()),
        (MethodType::MacSc, p.options.macs.trim()),
        (MethodType::Kex, p.options.kex.trim()),
        (MethodType::HostKey, p.options.host_key_algo.trim()),
    ];
    for (kind, value) in prefs {
        if !value.is_empty() {
            session.method_pref(kind, value)?;
        }
    }
    Ok(())
}

/// 主机密钥校验。**逻辑主机名**参与 pattern，跳板机回环桥端口绝不能写进 known_hosts。
fn check_host_key(
    session: &Session,
    logical_host: &str,
    logical_port: u16,
    p: &ConnectParams,
) -> std::result::Result<(), ConnectError> {
    let fp = host_fingerprint(session);
    let (key, key_type) = session
        .host_key()
        .ok_or_else(|| anyhow::anyhow!("服务器未提供主机密钥"))?;
    let pattern = host_pattern(logical_host, logical_port);
    let mut hosts = session.known_hosts()?;
    let kh_path = known_hosts_path();
    if let Some(parent) = kh_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if kh_path.exists() {
        let _ = hosts.read_file(&kh_path, KnownHostFileKind::OpenSSH);
    }
    if let Some(user_kh) = user_known_hosts_path() {
        if user_kh.exists() {
            let _ = hosts.read_file(&user_kh, KnownHostFileKind::OpenSSH);
        }
    }
    match hosts.check(&pattern, key) {
        CheckResult::Match => Ok(()),
        CheckResult::NotFound => {
            if p.trust_host {
                hosts.add(&pattern, key, "BDevTools", key_type.into())?;
                hosts.write_file(&kh_path, KnownHostFileKind::OpenSSH)?;
                Ok(())
            } else {
                Err(ConnectError::UnknownHost(format!("{} ({})", fp, pattern)))
            }
        }
        // 密钥变更是明确的 MITM 信号，绝不允许「再点一次信任」绕过
        CheckResult::Mismatch | CheckResult::Failure => Err(ConnectError::HostKeyMismatch(
            format!("{} ({})", fp, pattern),
        )),
    }
}

/// 单跳认证。密码/私钥/Agent 为主认证，配置允许时再补 keyboard-interactive（2FA）。
fn authenticate(
    session: &Session,
    hop: &HopSpec,
    p: &ConnectParams,
    prompts: &Arc<Mutex<Vec<String>>>,
) -> std::result::Result<(), ConnectError> {
    let need_ki = p.options.keyboard_interactive || hop.auth_type == "keyboard-interactive";
    let primary = match hop.auth_type.as_str() {
        "agent" => session.userauth_agent(&hop.username),
        "pubkey" if !hop.private_key_path.is_empty() => {
            let pass = if hop.passphrase.is_empty() {
                None
            } else {
                Some(hop.passphrase.as_str())
            };
            session.userauth_pubkey_file(
                &hop.username,
                None,
                Path::new(&hop.private_key_path),
                pass,
            )
        }
        "keyboard-interactive" => Ok(()),
        _ => session.userauth_password(&hop.username, &hop.password),
    };
    if let Err(e) = primary {
        // 主认证失败时，只有明确开启 2FA 才值得再试一次（否则直接把真实错误抛出去）
        if !need_ki {
            return Err(ConnectError::Other(e.into()));
        }
    }
    if !session.authenticated() && need_ki {
        let mut prompter = MfaPrompter {
            code: p.options.mfa_code.clone(),
            prompts: prompts.clone(),
        };
        let _ = session.userauth_keyboard_interactive(&hop.username, &mut prompter);
    }
    if !session.authenticated() {
        let collected = prompts.lock().map(|g| g.clone()).unwrap_or_default();
        if p.options.mfa_code.is_empty() && !collected.is_empty() {
            return Err(ConnectError::MfaRequired(collected.join(" / ")));
        }
        return Err(ConnectError::Other(anyhow::anyhow!(
            "认证失败，请检查用户名和{}",
            match hop.auth_type.as_str() {
                "agent" => "SSH Agent 中的密钥",
                "pubkey" => "私钥与私钥口令",
                "keyboard-interactive" => "验证码",
                _ => "密码",
            }
        )));
    }
    Ok(())
}

/// 一跳：TCP → 握手 → **主机密钥校验** → keepalive → 认证。
/// 校验必须在任何凭据发出之前，否则等于把密码交给一个未验证身份的对端。
fn establish_hop(
    tcp: TcpStream,
    hop: &HopSpec,
    p: &ConnectParams,
    prompts: &Arc<Mutex<Vec<String>>>,
) -> std::result::Result<Session, ConnectError> {
    let mut session = Session::new()?;
    apply_transport_prefs(&session, p)?;
    session.set_tcp_stream(tcp);
    session.set_timeout(15_000);
    session.handshake()?;
    check_host_key(&session, &hop.logical_host, hop.logical_port, p)?;
    if p.keepalive_interval > 0 {
        session.set_keepalive(true, p.keepalive_interval);
    }
    authenticate(&session, hop, p, prompts)?;
    Ok(session)
}

/// 把跳板机的 `direct-tcpip` 通道桥到一条本地回环 socket 上。
///
/// ssh2 0.9.6 的 `Session::set_tcp_stream` 在 Windows 只接受 `AsRawSocket`，
/// **不能**把 `Channel` 直接当传输喂给下一级 Session。这里起一个一次性本地监听，
/// 由桥线程把回环 socket 与 channel 双向中继，等价于 OpenSSH 的 ProxyCommand。
/// `Channel` 内部持有 `Arc<SessionInner>`，会话会随桥线程存活到链路断开。
fn spawn_bridge(
    session: Session,
    channel: ssh2::Channel,
) -> std::result::Result<u16, ConnectError> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    listener.set_nonblocking(true)?;
    // relay_pair 依赖非阻塞读；建桥时 channel 已经创建完成，此刻切换是安全的
    session.set_blocking(false);
    let stop = Arc::new(AtomicBool::new(false));
    thread::spawn(move || {
        let _keepalive = session;
        let deadline = Instant::now() + Duration::from_secs(15);
        let tcp = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return,
            }
        };
        drop(listener);
        relay_pair(tcp, channel, stop, None);
    });
    Ok(port)
}

/// 建立一条完整链路：可选代理 → N 级跳板机 → 目标机。
/// 所有调用方（终端 / SFTP / exec / 端口转发 / 日志拉取）都必须经此入口。
pub(crate) fn establish_session(
    p: &ConnectParams,
) -> std::result::Result<Session, ConnectError> {
    let prompts = Arc::new(Mutex::new(Vec::<String>::new()));
    let jumps = &p.options.jump_hosts;
    let total = jumps.len();
    let proxy = p.options.proxy.as_ref();

    let first_host = if total > 0 {
        jumps[0].host.clone()
    } else {
        p.host.clone()
    };
    let first_port = if total > 0 {
        if jumps[0].port == 0 { 22 } else { jumps[0].port }
    } else {
        p.port
    };
    let mut socket_host = first_host.clone();
    let mut socket_port = first_port;

    for i in 0..=total {
        let is_target = i == total;
        let hop = if is_target {
            HopSpec::target(p, socket_host.clone(), socket_port)
        } else {
            HopSpec::jump(&jumps[i], socket_host.clone(), socket_port)
        };

        // 只有第一跳可能经过代理；后续跳走的是回环桥，已是 SSH 隧道内部
        let tcp = if i == 0 {
            open_tcp(&hop.logical_host, hop.logical_port, proxy)?
        } else {
            open_tcp(&hop.socket_host, hop.socket_port, None)?
        };
        let session = establish_hop(tcp, &hop, p, &prompts)?;
        if is_target {
            return Ok(session);
        }

        let (next_host, next_port) = if i + 1 < total {
            let j = &jumps[i + 1];
            (j.host.clone(), if j.port == 0 { 22 } else { j.port })
        } else {
            (p.host.clone(), p.port)
        };
        // blocking 模式下发 direct-tcpip 请求，成功后由 spawn_bridge 切非阻塞
        let channel = session.channel_direct_tcpip(&next_host, next_port, None)?;
        socket_port = spawn_bridge(session, channel)?;
        socket_host = "127.0.0.1".to_string();
    }
    Err(ConnectError::Other(anyhow::anyhow!("跳板机链路建立失败")))
}

/// 把连接错误翻译成前端可判别的 Response（code + 面向用户的中文消息）
pub(crate) fn connect_failure<T>(e: ConnectError, prefix: &str) -> Response<T>
where
    T: Default + Serialize,
{
    match e {
        ConnectError::MfaRequired(challenge) => Response::new(
            MFA_REQUIRED,
            T::default(),
            &format!("{}{}服务器要求进行二次验证，请输入验证码后重试。", prefix, {
                if challenge.is_empty() {
                    String::new()
                } else {
                    format!("挑战内容：{}。", challenge)
                }
            }),
        ),
        ConnectError::UnknownHost(fp) => Response::new(
            HOST_KEY_UNKNOWN,
            T::default(),
            &format!(
                "{}首次连接到该主机，尚未保存主机密钥。指纹：{}。请确认指纹无误后选择「信任并连接」。",
                prefix, fp
            ),
        ),
        ConnectError::HostKeyMismatch(fp) => Response::new(
            HOST_KEY_MISMATCH,
            T::default(),
            &format!(
                "{}主机密钥与已知记录不一致，可能遭遇中间人攻击，已拒绝连接。指纹：{}",
                prefix, fp
            ),
        ),
        ConnectError::Other(err) => Response::new(300, T::default(), &format!("{}{}", prefix, err)),
    }
}

/// SSH 文件项
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFileItem {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified_time: String,
    pub permissions: String,
}

// ===================== SSH 终端操作 =====================

/// SSH 连接（交互式 PTY shell）
#[allow(clippy::too_many_arguments)]
pub async fn ssh_connect(
    ctx: AppContext,
    tab_id: String,
    host: String,
    port: u16,
    username: String,
    password: String,
    auth_type: Option<String>,
    private_key_path: Option<String>,
    passphrase: Option<String>,
    trust_host: Option<bool>,
    keepalive_interval: Option<u32>,
    term_type: Option<String>,
    options: Option<String>,
) -> Response<Option<String>> {
    let session_id = format!("ssh-{}", uuid::Uuid::new_v4());
    let params = ConnectParams::new(
        host,
        port,
        username,
        password,
        auth_type,
        private_key_path,
        passphrase,
        trust_host,
        keepalive_interval,
        options,
    );
    // channel 级请求必须在 shell() 之前，且这两个值要单独捕获进 spawn_blocking 闭包
    let agent_forward = params.options.agent_forward;
    let x11_display = params.options.x11_display.clone();
    let term = term_type
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "xterm-256color".to_string());

    let result = tokio::task::spawn_blocking({
        let session_id = session_id.clone();
        let ctx = ctx.clone();
        move || -> std::result::Result<String, ConnectError> {
            let session = establish_session(&params)?;

            let mut channel = session.channel_session()?;
            if agent_forward {
                let _ = channel.request_auth_agent_forwarding();
            }
            // ssh2 0.9.6 未封装 X11 通道请求；这里只把 DISPLAY 传给远端，
            // 实际 X11 流量由用户配置的 -L 6010:localhost:6010 规则承载。
            if !x11_display.is_empty() {
                let _ = channel.setenv("DISPLAY", &x11_display);
            }
            channel.request_pty(&term, None, Some((120, 40, 0, 0)))?;
            channel.shell()?;
            // 非阻塞：读循环用 WouldBlock 表示「暂时没数据」，避免卡住整个会话锁
            session.set_blocking(false);

            let inner = Arc::new(Mutex::new(SshSessionInner {
                session,
                channel: Some(channel),
                closed: false,
                reason: "",
                keepalive_interval: params.keepalive_interval,
            }));
            SSH_SESSIONS
                .lock()
                .unwrap()
                .insert(session_id.clone(), inner.clone());

            let inner_read = inner.clone();
            let ctx_read = ctx.clone();
            let sid_read = session_id.clone();
            let tab_read = tab_id.clone();
            thread::spawn(move || {
                let mut buf = [0u8; 4096];
                let mut last_keepalive = Instant::now();
                let mut reason: &'static str = "eof";
                loop {
                    let mut chunk: Option<Vec<u8>> = None;
                    let mut should_exit = false;
                    {
                        let mut guard = inner_read.lock().unwrap();
                        if guard.closed {
                            reason = guard.reason;
                            should_exit = true;
                        } else {
                            if guard.keepalive_interval > 0
                                && last_keepalive.elapsed()
                                    >= Duration::from_secs(guard.keepalive_interval as u64)
                            {
                                last_keepalive = Instant::now();
                                let _ = guard.session.keepalive_send();
                            }
                            match guard.channel.as_mut() {
                                Some(ch) => match ch.read(&mut buf) {
                                    Ok(0) => {
                                        guard.closed = true;
                                        guard.reason = "eof";
                                        reason = "eof";
                                        should_exit = true;
                                    }
                                    Ok(n) => chunk = Some(buf[..n].to_vec()),
                                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                                    Err(_) => {
                                        guard.closed = true;
                                        guard.reason = "error";
                                        reason = "error";
                                        should_exit = true;
                                    }
                                },
                                None => {
                                    guard.closed = true;
                                    guard.reason = "eof";
                                    reason = "eof";
                                    should_exit = true;
                                }
                            }
                        }
                    }
                    if should_exit {
                        break;
                    }
                    if let Some(bytes) = chunk {
                        // 原始字节 base64 透传；前端按会话编码用 TextDecoder(stream) 解码
                        ctx_read.emit(
                            "ssh-output",
                            serde_json::json!({
                                "tabId": tab_read,
                                "data": B64.encode(&bytes),
                                "format": "base64"
                            }),
                        );
                    }
                    thread::sleep(Duration::from_millis(20));
                }
                ctx_read.emit(
                    "ssh-closed",
                    serde_json::json!({
                        "tabId": tab_read,
                        "sessionId": sid_read,
                        "reason": reason
                    }),
                );
            });

            Ok(session_id)
        }
    })
    .await;

    match result {
        Ok(Ok(sid)) => Response::new(200, Some(sid), "连接成功"),
        Ok(Err(e)) => connect_failure(e, "连接失败: "),
        Err(e) => Response::new(300, None, &format!("连接异常: {}", e)),
    }
}

/// SSH 写入（用户输入）。锁 + socket 写可能阻塞，一律 spawn_blocking。
pub async fn ssh_write(session_id: String, data: String) -> Response<bool> {
    let result = tokio::task::spawn_blocking(move || {
        let inner = { SSH_SESSIONS.lock().unwrap().get(&session_id).cloned() };
        let Some(inner) = inner else {
            return Response::new(300, false, "会话不存在");
        };
        let mut guard = inner.lock().unwrap();
        if let Some(ref mut ch) = guard.channel {
            match ch.write_all(data.as_bytes()) {
                Ok(_) => {
                    let _ = ch.flush();
                    Response::new(200, true, "写入成功")
                }
                Err(e) => Response::new(300, false, &format!("写入失败: {}", e)),
            }
        } else {
            Response::new(300, false, "通道不存在")
        }
    })
    .await;
    result.unwrap_or_else(|e| Response::new(300, false, &format!("写入异常: {}", e)))
}

/// SSH 断开连接。
///
/// 早期实现只从 HashMap remove，读线程持有自己的 Arc 克隆且 channel 永远为 Some，
/// 退出条件永不成立 ⇒ 每断开一次泄漏一个 OS 线程并继续 emit。这里显式置位 +
/// 发 EOF + close，让读线程在下一轮（≤20ms）自行退出并推 `ssh-closed`。
pub async fn ssh_disconnect(session_id: String) -> Response<bool> {
    let result = tokio::task::spawn_blocking(move || {
        let inner = { SSH_SESSIONS.lock().unwrap().remove(&session_id) };
        let Some(inner) = inner else { return false };
        let mut guard = inner.lock().unwrap();
        guard.closed = true;
        guard.reason = "manual";
        if let Some(mut ch) = guard.channel.take() {
            let _ = ch.send_eof();
            let _ = ch.close();
        }
        true
    })
    .await
    .unwrap_or(false);
    if result {
        Response::new(200, true, "已断开")
    } else {
        Response::new(300, false, "会话不存在")
    }
}

/// SSH 连接状态检测
pub async fn ssh_check(session_id: String) -> Response<bool> {
    let result = tokio::task::spawn_blocking(move || {
        let inner = { SSH_SESSIONS.lock().unwrap().get(&session_id).cloned() };
        let Some(inner) = inner else {
            return Response::new(300, false, "会话不存在");
        };
        let guard = inner.lock().unwrap();
        let alive = !guard.closed
            && guard.channel.is_some()
            && guard.session.authenticated();
        Response::new(
            200,
            alive,
            if alive { "连接正常" } else { "连接已断开" },
        )
    })
    .await;
    result.unwrap_or_else(|e| Response::new(300, false, &format!("检测异常: {}", e)))
}

/// SSH 调整 PTY 大小
pub async fn ssh_resize_pty(session_id: String, cols: u32, rows: u32) -> Response<bool> {
    let result = tokio::task::spawn_blocking(move || {
        let inner = { SSH_SESSIONS.lock().unwrap().get(&session_id).cloned() };
        let Some(inner) = inner else {
            return Response::new(300, false, "会话不存在");
        };
        let mut guard = inner.lock().unwrap();
        if let Some(ref mut ch) = guard.channel {
            let _ = ch.request_pty_size(cols, rows, Some(0), Some(0));
            Response::new(200, true, "调整成功")
        } else {
            Response::new(300, false, "通道不存在")
        }
    })
    .await;
    result.unwrap_or_else(|e| Response::new(300, false, &format!("调整异常: {}", e)))
}

// ===================== SFTP 文件操作 =====================

/// SFTP 连接
pub async fn sftp_connect(
    _tab_id: String,
    host: String,
    port: u16,
    username: String,
    password: String,
    auth_type: Option<String>,
    private_key_path: Option<String>,
    passphrase: Option<String>,
    trust_host: Option<bool>,
    options: Option<String>,
) -> Response<Option<String>> {
    let session_id = format!("sftp-{}", uuid::Uuid::new_v4());
    // SFTP 与终端共用同一套握手/主机密钥/认证逻辑，避免「终端能连、文件管理却被 MITM」的分叉
    let params = ConnectParams::new(
        host,
        port,
        username,
        password,
        auth_type,
        private_key_path,
        passphrase,
        trust_host,
        Some(30),
        options,
    );

    let result = tokio::task::spawn_blocking(
        move || -> std::result::Result<String, ConnectError> {
            let session = establish_session(&params)?;
            SFTP_SESSIONS
                .lock()
                .unwrap()
                .insert(session_id.clone(), Arc::new(session));
            Ok(session_id)
        },
    )
    .await;

    match result {
        Ok(Ok(sid)) => Response::new(200, Some(sid), "SFTP 连接成功"),
        Ok(Err(e)) => connect_failure(e, "连接失败: "),
        Err(e) => Response::new(300, None, &format!("连接异常: {}", e)),
    }
}

/// SFTP 断开连接
pub async fn sftp_disconnect(session_id: String) -> Response<bool> {
    let mut sessions = SFTP_SESSIONS.lock().unwrap();
    sessions.remove(&session_id);
    Response::new(200, true, "已断开")
}

/// SFTP 连接状态检测
pub async fn sftp_check(session_id: String) -> Response<bool> {
    let sessions = SFTP_SESSIONS.lock().unwrap();
    match sessions.get(&session_id) {
        Some(sess) => {
            // 尝试创建 SFTP 句柄来检测连接是否存活
            match sess.sftp() {
                Ok(_sftp) => Response::new(200, true, "连接正常"),
                Err(_) => Response::new(300, false, "连接已断开"),
            }
        }
        None => Response::new(300, false, "会话不存在"),
    }
}

/// SFTP 列出目录
pub async fn sftp_list_dir(session_id: String, path: String) -> Response<Vec<RemoteFileItem>> {
    let sess = {
        let sessions = SFTP_SESSIONS.lock().unwrap();
        sessions.get(&session_id).cloned()
    };
    if let Some(sess) = sess {
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<RemoteFileItem>> {
            let sftp = sess.sftp()?;
            let entries = sftp.readdir(Path::new(&path))?;

            let mut files = Vec::new();
            for entry in entries {
                let (path_buf, stat) = entry;
                let name = path_buf
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let file_path = path_buf.to_string_lossy().replace('\\', "/");
                let is_dir = stat.is_dir();
                let size = stat.size.unwrap_or(0);

                // mtime 是 UTC 秒（Unix 时间戳）：from_timestamp 得到的是 DateTime<Utc>，
                // 直接 format 会显示 UTC 时间（东八区少 8 小时），必须转到本机时区再格式化。
                let modified_time = stat
                    .mtime
                    .map(|t| {
                        chrono::DateTime::from_timestamp(t as i64, 0)
                            .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();

                // 去掉文件类型位，只留 rwx 位；与 sftp_stat / chmod 的口径保持一致
                let permissions = stat.perm.map(|p| format!("{:o}", p & 0o7777)).unwrap_or_default();

                files.push(RemoteFileItem {
                    name,
                    path: file_path,
                    is_directory: is_dir,
                    size,
                    modified_time,
                    permissions,
                });
            }

            // 排序：目录在前，文件在后
            files.sort_by(|a, b| {
                if a.is_directory != b.is_directory {
                    b.is_directory.cmp(&a.is_directory)
                } else {
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                }
            });

            Ok(files)
        })
        .await;

        match result {
            Ok(Ok(files)) => Response::new(200, files, "查询成功"),
            Ok(Err(e)) => Response::new(300, Vec::new(), &format!("列出文件失败: {}", e)),
            Err(e) => Response::new(300, Vec::new(), &format!("执行异常: {}", e)),
        }
    } else {
        Response::new(300, Vec::new(), "会话不存在")
    }
}

/// SFTP 创建目录
pub async fn sftp_mkdir(session_id: String, path: String) -> Response<bool> {
    let sess = {
        let sessions = SFTP_SESSIONS.lock().unwrap();
        sessions.get(&session_id).cloned()
    };
    if let Some(sess) = sess {
        let result = tokio::task::spawn_blocking(move || -> Result<bool> {
            let sftp = sess.sftp()?;
            sftp.mkdir(Path::new(&path), 0o755)?;
            Ok(true)
        })
        .await;

        match result {
            Ok(Ok(_)) => Response::new(200, true, "创建成功"),
            Ok(Err(e)) => Response::new(300, false, &format!("创建失败: {}", e)),
            Err(e) => Response::new(300, false, &format!("执行异常: {}", e)),
        }
    } else {
        Response::new(300, false, "会话不存在")
    }
}

/// SFTP 删除文件/目录
pub async fn sftp_remove(session_id: String, path: String) -> Response<bool> {
    let sess = {
        let sessions = SFTP_SESSIONS.lock().unwrap();
        sessions.get(&session_id).cloned()
    };
    if let Some(sess) = sess {
        let result = tokio::task::spawn_blocking(move || -> Result<bool> {
            let sftp = sess.sftp()?;
            let p = Path::new(&path);

            if let Ok(stat) = sftp.stat(p) {
                if stat.is_dir() {
                    sftp.rmdir(p)?;
                } else {
                    sftp.unlink(p)?;
                }
                Ok(true)
            } else {
                Err(anyhow::anyhow!("文件不存在"))
            }
        })
        .await;

        match result {
            Ok(Ok(_)) => Response::new(200, true, "删除成功"),
            Ok(Err(e)) => Response::new(300, false, &format!("删除失败: {}", e)),
            Err(e) => Response::new(300, false, &format!("执行异常: {}", e)),
        }
    } else {
        Response::new(300, false, "会话不存在")
    }
}

/// 一次传输的收口：会话查找 + spawn_blocking + 错误码分类。
/// `StopReason::Cancelled` 映射成 499，前端据此把任务标为「已取消 / 已暂停」而不是失败。
async fn transfer_response<F>(session_id: String, task_id: String, f: F) -> Response<bool>
where
    F: FnOnce(Arc<Session>) -> Result<crate::transfer::StopReason> + Send + 'static,
{
    let sess = {
        let sessions = SFTP_SESSIONS.lock().unwrap();
        sessions.get(&session_id).cloned()
    };
    let Some(sess) = sess else {
        return Response::new(300, false, "会话不存在");
    };
    let result = tokio::task::spawn_blocking(move || f(sess)).await;
    // 无论成功 / 取消 / 出错都要清标志：taskId 重试时复用，且不能让 CANCELLED 无限增长
    crate::transfer::clear_cancel(&task_id);
    match result {
        Ok(Ok(crate::transfer::StopReason::Done)) => Response::new(200, true, "传输完成"),
        Ok(Ok(crate::transfer::StopReason::Cancelled)) => {
            Response::new(crate::transfer::TRANSFER_CANCELLED, false, "传输已取消")
        }
        Ok(Err(e)) => Response::new(300, false, &format!("传输失败: {}", e)),
        Err(e) => Response::new(300, false, &format!("执行异常: {}", e)),
    }
}

/// 文件属性（大小 / 权限）。传输队列入队前用它预取 totalSize，
/// 断点续传与权限展示也复用同一份结果，避免多套 stat 逻辑。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStatResp {
    pub exists: bool,
    pub size: u64,
    pub permissions: String,
    pub is_directory: bool,
}

impl FileStatResp {
    fn empty() -> Self {
        Self { exists: false, size: 0, permissions: String::new(), is_directory: false }
    }
}

/// 远端文件属性
pub async fn sftp_stat(session_id: String, path: String) -> Response<FileStatResp> {
    let sess = {
        let sessions = SFTP_SESSIONS.lock().unwrap();
        sessions.get(&session_id).cloned()
    };
    let Some(sess) = sess else {
        return Response::new(300, FileStatResp::empty(), "会话不存在");
    };
    let result = tokio::task::spawn_blocking(move || -> Result<FileStatResp> {
        let sftp = sess.sftp()?;
        match sftp.stat(Path::new(&path)) {
            Ok(st) => Ok(FileStatResp {
                exists: true,
                size: st.size.unwrap_or(0),
                permissions: st.perm.map(|p| format!("{:o}", p & 0o7777)).unwrap_or_default(),
                is_directory: st.is_dir(),
            }),
            Err(_) => Ok(FileStatResp::empty()),
        }
    })
    .await;
    match result {
        Ok(Ok(v)) => Response::new(200, v, "查询成功"),
        Ok(Err(e)) => Response::new(300, FileStatResp::empty(), &format!("查询失败: {}", e)),
        Err(e) => Response::new(300, FileStatResp::empty(), &format!("执行异常: {}", e)),
    }
}

/// 本地文件属性（拖放进来的文件不在列目录结果里，需要单独取大小）
pub async fn local_stat(path: String) -> Response<FileStatResp> {
    let result = tokio::task::spawn_blocking(move || -> Result<FileStatResp> {
        match std::fs::metadata(&path) {
            Ok(m) => Ok(FileStatResp {
                exists: true,
                size: m.len(),
                permissions: String::new(),
                is_directory: m.is_dir(),
            }),
            Err(_) => Ok(FileStatResp::empty()),
        }
    })
    .await;
    match result {
        Ok(Ok(v)) => Response::new(200, v, "查询成功"),
        Ok(Err(e)) => Response::new(300, FileStatResp::empty(), &format!("查询失败: {}", e)),
        Err(e) => Response::new(300, FileStatResp::empty(), &format!("执行异常: {}", e)),
    }
}

/// 修改远端权限。mode 为八进制数值（前端传 parseInt(str, 8)）。
pub async fn sftp_chmod(session_id: String, path: String, mode: u32) -> Response<bool> {
    let sess = {
        let sessions = SFTP_SESSIONS.lock().unwrap();
        sessions.get(&session_id).cloned()
    };
    let Some(sess) = sess else {
        return Response::new(300, false, "会话不存在");
    };
    if mode > 0o7777 {
        return Response::new(300, false, "权限值超出 0~7777");
    }
    let result = tokio::task::spawn_blocking(move || -> Result<bool> {
        let sftp = sess.sftp()?;
        crate::transfer::apply_chmod(&sftp, Path::new(&path), mode)?;
        Ok(true)
    })
    .await;
    match result {
        Ok(Ok(_)) => Response::new(200, true, "权限已修改"),
        Ok(Err(e)) => Response::new(300, false, &format!("修改权限失败: {}", e)),
        Err(e) => Response::new(300, false, &format!("执行异常: {}", e)),
    }
}

/// 请求取消一次传输（协作式，传输循环每 64KB 检查一次）
pub async fn sftp_cancel_transfer(task_id: String) -> Response<bool> {
    crate::transfer::sftp_cancel_transfer(task_id).await
}

/// SFTP 上传文件（P1-C：支持断点续传与协作式取消）
pub async fn sftp_upload(
    ctx: AppContext,
    session_id: String,
    local_path: String,
    remote_path: String,
    task_id: String,
    resume: Option<bool>,
) -> Response<bool> {
    transfer_response(session_id, task_id.clone(), move |sess| {
        crate::transfer::upload(&ctx, sess, &local_path, &remote_path, &task_id, resume.unwrap_or(false))
    })
    .await
}

/// SFTP 下载文件（P1-C：支持断点续传与协作式取消）
pub async fn sftp_download(
    ctx: AppContext,
    session_id: String,
    remote_path: String,
    local_path: String,
    task_id: String,
    resume: Option<bool>,
) -> Response<bool> {
    transfer_response(session_id, task_id.clone(), move |sess| {
        crate::transfer::download(&ctx, sess, &remote_path, &local_path, &task_id, resume.unwrap_or(false))
    })
    .await
}

/// 本地目录列出
pub async fn local_list_dir(path: String) -> Response<Vec<RemoteFileItem>> {
    let result = tokio::task::spawn_blocking(move || -> Result<Vec<RemoteFileItem>> {
        let dir_path = if path.is_empty() {
            std::env::current_dir()?
        } else {
            Path::new(&path).to_path_buf()
        };

        let mut files = Vec::new();
        for entry in std::fs::read_dir(&dir_path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let name = entry.file_name().to_string_lossy().to_string();
            let file_path = entry.path().to_string_lossy().to_string();
            let is_dir = metadata.is_dir();
            let size = metadata.len();

            // 同 sftp_list_dir：SystemTime 是 UTC 绝对时刻，需转本机时区，否则显示成 UTC
            let modified_time = metadata
                .modified()
                .ok()
                .and_then(|t| {
                    let duration = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                    chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0)
                        .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
                })
                .unwrap_or_default();

            files.push(RemoteFileItem {
                name,
                path: file_path,
                is_directory: is_dir,
                size,
                modified_time,
                permissions: String::new(),
            });
        }

        files.sort_by(|a, b| {
            if a.is_directory != b.is_directory {
                b.is_directory.cmp(&a.is_directory)
            } else {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        });

        Ok(files)
    })
    .await;

    match result {
        Ok(Ok(files)) => Response::new(200, files, "查询成功"),
        Ok(Err(e)) => Response::new(300, Vec::new(), &format!("列出文件失败: {}", e)),
        Err(e) => Response::new(300, Vec::new(), &format!("执行异常: {}", e)),
    }
}

/// SFTP 重命名文件/目录
pub async fn sftp_rename(session_id: String, old_path: String, new_path: String) -> Response<bool> {
    let sess = {
        let sessions = SFTP_SESSIONS.lock().unwrap();
        sessions.get(&session_id).cloned()
    };
    if let Some(sess) = sess {
        let result = tokio::task::spawn_blocking(move || -> Result<bool> {
            let sftp = sess.sftp()?;
            sftp.rename(Path::new(&old_path), Path::new(&new_path), None)?;
            Ok(true)
        })
        .await;

        match result {
            Ok(Ok(_)) => Response::new(200, true, "重命名成功"),
            Ok(Err(e)) => Response::new(300, false, &format!("重命名失败: {}", e)),
            Err(e) => Response::new(300, false, &format!("执行异常: {}", e)),
        }
    } else {
        Response::new(300, false, "会话不存在")
    }
}

/// 获取应用数据目录路径
pub async fn get_app_dir() -> Response<String> {
    let app_dir = &bdevtools_core::app::app_store::APP.lock().app_dir;
    Response::new(200, app_dir.to_string_lossy().to_string(), "成功")
}

/// 把字节/秒格式化成 "12.5 KB/s" / "3.2 MB/s" / "856 B/s"
pub(crate) fn format_speed(bps: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    if bps >= GB {
        format!("{:.2} GB/s", bps / GB)
    } else if bps >= MB {
        format!("{:.2} MB/s", bps / MB)
    } else if bps >= KB {
        format!("{:.1} KB/s", bps / KB)
    } else {
        format!("{:.0} B/s", bps)
    }
}

// ===================== 远程日志拉取 =====================

/// 远程日志拉取结果（命令层 `logs_pull_remote` 的返回体）
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsPullRemoteResp {
    pub local_path: String,
    pub size: u64,
}

/// 建立 SFTP 会话 → 下载到 config_dir/plugin_data/logs_remote → 断开。
/// 纯业务编排，不依赖 Tauri；`AppContext` 由命令层的 `ctx_from_app` 注入。
pub async fn pull_remote_log(
    ctx: &AppContext,
    host: String,
    port: u16,
    username: String,
    password: String,
    auth_type: Option<String>,
    private_key_path: Option<String>,
    passphrase: Option<String>,
    trust_host: Option<bool>,
    options: Option<String>,
    remote_path: String,
) -> Response<LogsPullRemoteResp> {
    let conn = sftp_connect(
        "logs".to_string(),
        host.clone(),
        port,
        username.clone(),
        password.clone(),
        auth_type.clone(),
        private_key_path.clone(),
        passphrase,
        trust_host,
        options,
    )
    .await;
    let session_id = match conn.data {
        Some(sid) => sid,
        None => {
            return Response::new(
                conn.code,
                LogsPullRemoteResp { local_path: String::new(), size: 0 },
                conn.msg.as_str(),
            )
        }
    };

    let cache_dir = ctx.config_dir.join("plugin_data").join("logs_remote");
    let _ = std::fs::create_dir_all(&cache_dir);
    let raw_name = Path::new(&remote_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "remote.log".to_string());
    let safe_name: String = raw_name
        .chars()
        .map(|c| if c == '/' || c == '\\' || c == ':' || c == '?' || c == '*' { '_' } else { c })
        .collect();
    let local_path = cache_dir.join(safe_name).to_string_lossy().to_string();

    let dl = sftp_download(
        ctx.clone(),
        session_id.clone(),
        remote_path.clone(),
        local_path.clone(),
        "logs-pull".to_string(),
        None,
    )
    .await;
    let _ = sftp_disconnect(session_id).await;

    if dl.code != 200 {
        return Response::new(
            dl.code,
            LogsPullRemoteResp { local_path: String::new(), size: 0 },
            dl.msg.as_str(),
        );
    }
    let size = std::fs::metadata(&local_path).map(|m| m.len()).unwrap_or(0);
    Response::new(200, LogsPullRemoteResp { local_path, size }, "拉取成功")
}
