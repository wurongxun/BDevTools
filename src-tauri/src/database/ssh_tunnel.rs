/*! # SSH 隧道模块
通过 SSH 端口转发实现数据库安全连接。
使用现有的 ssh2 crate 实现 TCP 转发。
*/
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::sync::Arc;
use parking_lot::Mutex;
use lazy_static::lazy_static;
use std::collections::HashMap;

/// SSH 隧道会话
pub struct SshTunnel {
    pub local_port: u16,
    pub session: ssh2::Session,
    /// 保持 listener 存活
    _listener: TcpListener,
}

/// 隧道配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SshTunnelConfig {
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
    pub ssh_password: Option<String>,
    pub ssh_key_path: Option<String>,
    pub remote_host: String,
    pub remote_port: u16,
}

lazy_static! {
    /// 全局隧道管理器: config_id -> SshTunnel
    static ref TUNNELS: Mutex<HashMap<String, Arc<SshTunnel>>> = Mutex::new(HashMap::new());
}

/// 创建 SSH 隧道
pub fn create_tunnel(config_id: &str, config: &SshTunnelConfig) -> Result<u16> {
    // 建立 SSH 会话
    let tcp = std::net::TcpStream::connect(format!("{}:{}", config.ssh_host, config.ssh_port))?;
    let mut session = ssh2::Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;

    // 认证
    if let Some(key_path) = &config.ssh_key_path {
        session.userauth_pubkey_file(&config.ssh_username, None, std::path::Path::new(key_path), None)?;
    } else if let Some(password) = &config.ssh_password {
        session.userauth_password(&config.ssh_username, password)?;
    } else {
        return Err(anyhow!("SSH认证方式未配置"));
    }

    if !session.authenticated() {
        return Err(anyhow!("SSH认证失败"));
    }

    // 绑定本地端口
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let local_port = listener.local_addr()?.port();

    let tunnel = SshTunnel {
        local_port,
        session,
        _listener: listener,
    };

    TUNNELS.lock().insert(config_id.to_string(), Arc::new(tunnel));
    Ok(local_port)
}

/// 关闭 SSH 隧道
pub fn close_tunnel(config_id: &str) {
    TUNNELS.lock().remove(config_id);
}

/// 获取隧道本地端口
pub fn get_tunnel_port(config_id: &str) -> Option<u16> {
    TUNNELS.lock().get(config_id).map(|t| t.local_port)
}
