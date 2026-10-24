// 宿主能力封装：网络访问（net: 权限，原始 socket 出口）+ 日志。
//
// 本 crate 是 redis-client 插件的 Extism **guest** 后端：它自己实现 Redis 的 RESP 线协议，
// 但 WASM 里没有 socket —— 字节收发由宿主经 `host_tcp_*` 提供（见 host 的 plugin_wasm.rs）。
// 连接按「键」持久在宿主侧（跨 `plugin_call` 有效），所以这里只需要把键带过来即可，
// guest 自身是无状态的（每次调用都是新实例）。

use extism_pdk::*;

#[host_fn]
extern "ExtismHost" {
    /// `(key, host, port)` → `{"ok":true,"peer":"host:port"}`；权限不通过时整次调用失败
    fn host_tcp_connect(key: String, host: String, port: i64) -> String;
    /// 往连接写字节（协议由本插件自己拼）
    fn host_tcp_send(key: String, data: Vec<u8>);
    /// 从连接读一块（可能不足 max_len；空块 = 对端关闭）
    fn host_tcp_recv(key: String, max_len: i64) -> Vec<u8>;
    /// 关闭连接（幂等）
    fn host_tcp_close(key: String);
    /// 写宿主日志（需 log 权限）
    fn host_log(msg: String);
}

/// 单次从宿主拉取的字节数（≤ 宿主的 MAX_TCP_CHUNK = 1MB）
const CHUNK: i64 = 64 * 1024;

/// 写一行宿主日志（失败静默：日志不该影响主流程）
pub fn log(msg: &str) {
    let _ = unsafe { host_log(msg.to_string()) };
}

/// 建立连接（同键重连会关掉宿主侧旧连接）
pub fn tcp_connect(key: &str, host: &str, port: u16) -> Result<(), String> {
    let raw = unsafe { host_tcp_connect(key.to_string(), host.to_string(), port as i64) }
        .map_err(|e| format!("连接 {host}:{port} 失败: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&raw).map_err(|_| format!("宿主返回无法解析: {raw}"))?;
    if v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false) {
        Ok(())
    } else {
        Err(format!("宿主拒绝连接: {raw}"))
    }
}

/// 关闭连接（幂等，出错也不抛）
pub fn tcp_close(key: &str) {
    let _ = unsafe { host_tcp_close(key.to_string()) };
}

/// 带缓冲的连接读取器：把宿主的「按块收字节」拼成「按 RESP 语义读」。
///
/// 每次 `plugin_call` 都会 `new` 一个：因为每条命令都是「发一条、完整读一条回复」，
/// 调用开始时缓冲必为空，所以不需要跨调用保留任何状态。
pub struct Conn {
    key: String,
    buf: Vec<u8>,
    pos: usize,
}

impl Conn {
    pub fn new(key: String) -> Self {
        Self { key, buf: Vec::new(), pos: 0 }
    }

    /// 从宿主再拉一块；EOF（空块）视为连接关闭
    fn fill(&mut self) -> Result<(), String> {
        let chunk = unsafe { host_tcp_recv(self.key.clone(), CHUNK) }
            .map_err(|e| format!("读取失败: {e}"))?;
        if chunk.is_empty() {
            return Err("连接被对端关闭（EOF）".to_string());
        }
        // 丢弃已消费前缀，避免缓冲区随命令数无界增长
        if self.pos > 0 {
            self.buf.drain(0..self.pos);
            self.pos = 0;
        }
        self.buf.extend_from_slice(&chunk);
        Ok(())
    }

    /// 读一个字节（RESP 的类型前缀）
    pub fn read_byte(&mut self) -> Result<u8, String> {
        if self.pos >= self.buf.len() {
            self.fill()?;
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Ok(b)
    }

    /// 读一行（到 `\n`，剥掉结尾 `\r`）
    pub fn read_line(&mut self) -> Result<String, String> {
        loop {
            if let Some(i) = self.buf[self.pos..].iter().position(|&b| b == b'\n') {
                let mut end = self.pos + i;
                if end > self.pos && self.buf[end - 1] == b'\r' {
                    end -= 1;
                }
                let s = String::from_utf8(self.buf[self.pos..end].to_vec())
                    .map_err(|_| "RESP 行非 UTF-8".to_string())?;
                self.pos += i + 1;
                return Ok(s);
            }
            self.fill()?;
        }
    }

    /// 读满 n 个字节（bulk 主体 / 末尾 CRLF 用）
    pub fn read_exact_n(&mut self, n: usize) -> Result<Vec<u8>, String> {
        while self.buf.len() - self.pos < n {
            self.fill()?;
        }
        let out = self.buf[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(out)
    }

    /// 把整块字节写到连接
    pub fn write_all(&self, data: &[u8]) -> Result<(), String> {
        unsafe { host_tcp_send(self.key.clone(), data.to_vec()) }
            .map_err(|e| format!("写入失败: {e}"))
    }
}
