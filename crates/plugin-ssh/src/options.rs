/*! # P2 高级连接选项

会话的「代理 / 跳板机 / 压缩 / 算法 / 2FA」配置以一个 JSON 字符串贯穿前后端：
SQLite 存一列 `advanced_options`，命令层原样透传给业务层，只有本模块负责解析。
这样新增选项不必反复改 `ALTER TABLE` 和命令形参。

安全边界：代理与跳板机只影响「怎么连到服务器」，**不改变主机密钥校验口径** ——
每一跳都按逻辑主机名（而非回环桥端口）校验 known_hosts，见 `remote_conn.rs::establish_hop`。
*/
use anyhow::{anyhow, Result};
use base64::Engine;
use serde::Deserialize;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::time::Duration;

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// 代理类型：`socks5` / `socks4` / `http`（CONNECT）。空串 / none 表示直连。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyConfig {
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

/// 一级跳板机。凭据与目标机相互独立；口令只存在内存与本地 SQLite，不进日志。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct JumpHost {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub auth_type: String,
    pub private_key_path: String,
    pub passphrase: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ConnectOptions {
    pub proxy: Option<ProxyConfig>,
    pub jump_hosts: Vec<JumpHost>,
    /// 握手前 `set_compress(true)`，对慢链路的交互有明显收益
    pub compress: bool,
    /// 算法偏好，逗号分隔（libssh2 `session_method_pref` 语法）；留空＝用库默认
    pub ciphers: String,
    pub macs: String,
    pub kex: String,
    pub host_key_algo: String,
    /// 密码之后继续 keyboard-interactive（TOTP / 短信验证码等 2FA）
    pub keyboard_interactive: bool,
    /// 一次性验证码；为空时后端返回 411 让前端弹框后带码重连
    pub mfa_code: String,
    /// 在 shell 通道上请求 ssh-agent 转发
    pub agent_forward: bool,
    /// 写入远端 `DISPLAY`。ssh2 0.9.6 未封装 X11 通道请求，
    /// 因此这里配合手工 `-L 6010:localhost:6010` 规则使用（见路线图）。
    pub x11_display: String,
}

/// 老数据 / 空串 / 坏 JSON 一律回落默认配置，不能让一个坏字段把连接彻底堵死。
pub fn parse_options(raw: Option<&str>) -> ConnectOptions {
    let Some(raw) = raw else {
        return ConnectOptions::default();
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return ConnectOptions::default();
    }
    serde_json::from_str(trimmed).unwrap_or_default()
}

impl ProxyConfig {
    pub fn enabled(&self) -> bool {
        !matches!(self.kind.as_str(), "" | "none" | "off") && !self.host.is_empty() && self.port > 0
    }
}

/// 按配置连出一条通往 `target_host:target_port` 的裸 TCP 流。
/// 只应用于**第一跳**；后续跳板机之间的流量走 SSH direct-tcpip，不再经过代理。
pub fn open_tcp(
    target_host: &str,
    target_port: u16,
    proxy: Option<&ProxyConfig>,
) -> Result<TcpStream> {
    let (socket_host, socket_port) = match proxy.filter(|p| p.enabled()) {
        Some(p) => (p.host.as_str(), p.port),
        None => (target_host, target_port),
    };
    let tcp = TcpStream::connect((socket_host, socket_port))
        .map_err(|e| anyhow!("连接 {}:{} 失败: {}", socket_host, socket_port, e))?;
    tcp.set_read_timeout(Some(Duration::from_secs(10)))?;
    tcp.set_write_timeout(Some(Duration::from_secs(10)))?;
    tcp.set_nodelay(true)?;

    match proxy.filter(|p| p.enabled()) {
        None => Ok(tcp),
        Some(p) => {
            let mut tcp = tcp;
            match p.kind.as_str() {
                "http" => http_connect(&mut tcp, p, target_host, target_port)?,
                "socks5" => socks5_connect(&mut tcp, p, target_host, target_port)?,
                "socks4" => socks4_connect(&mut tcp, p, target_host, target_port)?,
                other => return Err(anyhow!("不支持的代理类型: {}", other)),
            }
            Ok(tcp)
        }
    }
}

fn read_exact_or<R: Read>(r: &mut R, buf: &mut [u8], what: &str) -> Result<()> {
    r.read_exact(buf).map_err(|e| anyhow!("代理 {} 响应不完整: {}", what, e))
}

fn http_connect(
    tcp: &mut TcpStream,
    proxy: &ProxyConfig,
    host: &str,
    port: u16,
) -> Result<()> {
    let mut req = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Connection: Keep-Alive\r\n",
        host, port, host, port
    );
    if !proxy.username.is_empty() {
        let cred = format!("{}:{}", proxy.username, proxy.password);
        req.push_str(&format!(
            "Proxy-Authorization: Basic {}\r\n",
            B64.encode(cred.as_bytes())
        ));
    }
    req.push_str("\r\n");
    tcp.write_all(req.as_bytes())?;
    tcp.flush()?;

    // 逐字节读状态行：CONNECT 成功后的第一个字节就是隧道数据，绝不能多读。
    let mut line = Vec::with_capacity(128);
    let mut byte = [0u8; 1];
    loop {
        read_exact_or(tcp, &mut byte, "HTTP CONNECT")?;
        if byte[0] == b'\n' {
            break;
        }
        if byte[0] != b'\r' {
            line.push(byte[0]);
        }
        if line.len() > 1024 {
            return Err(anyhow!("代理返回的状态行过长"));
        }
    }
    let status = String::from_utf8_lossy(&line);
    if !status.contains(" 200 ") && !status.starts_with("HTTP/1.1 200") {
        return Err(anyhow!("代理 CONNECT 被拒绝: {}", status));
    }
    // 吃掉剩余响应头直到空行。逐字节，CONNECT 成功后的第一个数据字节不能被预读。
    read_headers_until_blank(tcp)
}

/// 状态行之后继续读到空行（\r\n）。逐字节，读到 \n 时检查该行是否为空。
fn read_headers_until_blank(tcp: &mut TcpStream) -> Result<()> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        read_exact_or(tcp, &mut byte, "HTTP CONNECT 响应头")?;
        match byte[0] {
            b'\n' => {
                if line.is_empty() || line == b"\r" {
                    return Ok(());
                }
                line.clear();
            }
            c => line.push(c),
        }
        if line.len() > 4096 {
            return Err(anyhow!("代理响应头过长"));
        }
    }
}

/// SOCKS5（RFC 1928）：无鉴权或用户名/密码（RFC 1929）。
fn socks5_connect(
    tcp: &mut TcpStream,
    proxy: &ProxyConfig,
    host: &str,
    port: u16,
) -> Result<()> {
    let use_auth = !proxy.username.is_empty();
    let greeting: &[u8] = if use_auth {
        &[0x05, 0x02, 0x00, 0x02]
    } else {
        &[0x05, 0x01, 0x00]
    };
    tcp.write_all(greeting)?;
    let mut chosen = [0u8; 2];
    read_exact_or(tcp, &mut chosen, "SOCKS5 方法选择")?;
    if chosen[0] != 0x05 {
        return Err(anyhow!("不是 SOCKS5 代理"));
    }
    match chosen[1] {
        0x00 => {}
        0x02 if use_auth => {
            let u = proxy.username.as_bytes();
            let p = proxy.password.as_bytes();
            if u.len() > 255 || p.len() > 255 {
                return Err(anyhow!("SOCKS5 用户名或密码过长"));
            }
            let mut auth = vec![0x01, u.len() as u8];
            auth.extend_from_slice(u);
            auth.push(p.len() as u8);
            auth.extend_from_slice(p);
            tcp.write_all(&auth)?;
            let mut reply = [0u8; 2];
            read_exact_or(tcp, &mut reply, "SOCKS5 鉴权")?;
            if reply[1] != 0x00 {
                return Err(anyhow!("SOCKS5 代理鉴权失败"));
            }
        }
        0xff => return Err(anyhow!("SOCKS5 代理拒绝了所有认证方式")),
        _ => return Err(anyhow!("SOCKS5 代理要求不支持的认证方式")),
    }

    let mut req = vec![0x05, 0x01, 0x00];
    match host.parse::<Ipv4Addr>() {
        Ok(ip) => {
            req.push(0x01);
            req.extend_from_slice(&ip.octets());
        }
        Err(_) => {
            if host.len() > 255 {
                return Err(anyhow!("目标主机名过长"));
            }
            req.push(0x03);
            req.push(host.len() as u8);
            req.extend_from_slice(host.as_bytes());
        }
    }
    req.extend_from_slice(&port.to_be_bytes());
    tcp.write_all(&req)?;

    let mut head = [0u8; 4];
    read_exact_or(tcp, &mut head, "SOCKS5 连接回复")?;
    let addr_len = match head[3] {
        0x01 => 4,
        0x04 => 16,
        0x03 => {
            let mut n = [0u8; 1];
            read_exact_or(tcp, &mut n, "SOCKS5 域名长度")?;
            // 长度字节本身已经读过，这里只剩余域名 + 2 字节端口
            n[0] as usize
        }
        _ => return Err(anyhow!("SOCKS5 回复地址类型非法")),
    };
    let mut rest = vec![0u8; addr_len + 2];
    read_exact_or(tcp, &mut rest, "SOCKS5 绑定地址")?;
    if head[1] != 0x00 {
        return Err(anyhow!("SOCKS5 连接失败，REP={:#04x}", head[1]));
    }
    Ok(())
}

/// SOCKS4 / SOCKS4a（域名走 0.0.0.x 约定）。
/// P2 新命令（exec / 端口转发）的统一连接目标。
/// 前端把会话配置序列化成这个 camelCase 对象；命令层只透传 JSON 字符串，
/// 避免每条命令重复十几个形参（也无法在 Rust 侧给 15 个参数做默认值）。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ConnectTarget {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub auth_type: String,
    pub private_key_path: String,
    pub passphrase: String,
    pub trust_host: bool,
    pub keepalive_interval: u32,
    pub term_type: String,
    /// 高级选项：允许前端传对象，也允许直接传 SQLite 里的 JSON 字符串
    pub advanced_options: Option<serde_json::Value>,
    /// 一次性 2FA 验证码，优先级高于 `advanced_options.mfaCode`
    pub mfa_code: String,
}

pub fn parse_target(raw: &str) -> Result<ConnectTarget> {
    let t: ConnectTarget = serde_json::from_str(raw.trim())
        .map_err(|e| anyhow!("连接配置不是合法 JSON: {}", e))?;
    if t.host.trim().is_empty() || t.username.trim().is_empty() {
        return Err(anyhow!("连接配置缺少主机地址或用户名"));
    }
    Ok(t)
}

fn socks4_connect(
    tcp: &mut TcpStream,
    proxy: &ProxyConfig,
    host: &str,
    port: u16,
) -> Result<()> {
    let mut req = vec![0x04, 0x01];
    req.extend_from_slice(&port.to_be_bytes());
    match host.parse::<Ipv4Addr>() {
        Ok(ip) => req.extend_from_slice(&ip.octets()),
        Err(_) => req.extend_from_slice(&[0, 0, 0, 1]),
    }
    req.extend_from_slice(proxy.username.as_bytes());
    req.push(0);
    if host.parse::<Ipv4Addr>().is_err() {
        req.extend_from_slice(host.as_bytes());
        req.push(0);
    }
    tcp.write_all(&req)?;
    let mut reply = [0u8; 8];
    read_exact_or(tcp, &mut reply, "SOCKS4 回复")?;
    if reply[1] != 0x5a {
        return Err(anyhow!("SOCKS4 连接被拒绝，code={:#04x}", reply[1]));
    }
    Ok(())
}
