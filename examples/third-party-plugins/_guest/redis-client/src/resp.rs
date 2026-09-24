// RESP 协议编解码（Redis Serialization Protocol）—— 完全在 WASM 里实现，字节收发走宿主的
// `net:` 授权下的原始 socket（见 crate::host::Conn，同步的 host_tcp_send / host_tcp_recv）。
//
// 解析结果统一映射为插件前端约定的 `Reply` JSON 形状：
//   { "t": "status"|"bulk"|"int"|"array"|"error", "v": <value> }
// 数组内层把 bulk/int/status 折叠为原始值（字符串/数字/嵌套数组），与前端 MockEngine 的约定一致，
// 前端 `unwrap()` / `RedisClient` 无需改动即可消费。

use serde_json::{json, Value};

use crate::host::Conn;

/// 解析后的 RESP 回复（尚未折叠为前端 Reply）。
#[derive(Debug, Clone)]
pub enum RespReply {
    Status(String),
    Bulk(Option<String>),
    Int(i64),
    Array(Option<Vec<RespReply>>),
    Error(String),
}

/// 将命令参数序列化为 RESP 请求字节（`*N` + `$len\r\narg\r\n` ...）。
pub fn encode_command(args: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("*{}\r\n", args.len()).as_bytes());
    for a in args {
        out.extend_from_slice(format!("${}\r\n", a.len()).as_bytes());
        out.extend_from_slice(a.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out
}

/// 发一条命令并读回一条完整回复（请求/响应模型，一问一答）。
pub fn send_cmd(conn: &mut Conn, args: &[String]) -> Result<RespReply, String> {
    conn.write_all(&encode_command(args))?;
    read_reply(conn)
}

/// 从连接读一条完整 RESP 回复。
///
/// 这里是**同步递归**（不是 async），所以数组嵌套直接递归即可，不需要 `Box::pin`。
pub fn read_reply(conn: &mut Conn) -> Result<RespReply, String> {
    match conn.read_byte()? {
        b'+' => Ok(RespReply::Status(conn.read_line()?)),
        b'-' => Ok(RespReply::Error(conn.read_line()?)),
        b':' => {
            let s = conn.read_line()?;
            let n = s.parse::<i64>().map_err(|_| format!("RESP 整数解析失败: {s}"))?;
            Ok(RespReply::Int(n))
        }
        b'$' => {
            let len = conn
                .read_line()?
                .parse::<i64>()
                .map_err(|_| "bulk 长度非整数".to_string())?;
            if len < 0 {
                Ok(RespReply::Bulk(None))
            } else {
                let n = len as usize;
                let body = conn.read_exact_n(n)?;
                let _ = conn.read_exact_n(2)?; // 末尾 CRLF
                // 按 UTF-8 宽松解码（二进制值会损失，客户端展示场景可接受）
                Ok(RespReply::Bulk(Some(String::from_utf8_lossy(&body).into_owned())))
            }
        }
        b'*' => {
            let count = conn
                .read_line()?
                .parse::<i64>()
                .map_err(|_| "array 长度非整数".to_string())?;
            if count < 0 {
                Ok(RespReply::Array(None))
            } else {
                let mut items = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    items.push(read_reply(conn)?);
                }
                Ok(RespReply::Array(Some(items)))
            }
        }
        other => Err(format!("未知 RESP 类型字节: {}", other as char)),
    }
}

impl RespReply {
    /// 顶层 Reply JSON（保留 {t, v} 结构）。
    pub fn to_reply_json(&self) -> Value {
        match self {
            RespReply::Status(s) => json!({ "t": "status", "v": s }),
            RespReply::Bulk(None) => json!({ "t": "bulk", "v": null }),
            RespReply::Bulk(Some(s)) => json!({ "t": "bulk", "v": s }),
            RespReply::Int(n) => json!({ "t": "int", "v": n }),
            RespReply::Error(s) => json!({ "t": "error", "v": s }),
            RespReply::Array(None) => json!({ "t": "array", "v": [] }),
            RespReply::Array(Some(items)) => json!({
                "t": "array",
                "v": items.iter().map(|x| x.flatten()).collect::<Vec<_>>()
            }),
        }
    }

    /// 数组内层折叠：bulk/status → 字符串，int → 数字，array → 嵌套数组。
    fn flatten(&self) -> Value {
        match self {
            RespReply::Status(s) => json!(s),
            RespReply::Bulk(None) => Value::Null,
            RespReply::Bulk(Some(s)) => json!(s),
            RespReply::Int(n) => json!(n),
            RespReply::Error(s) => json!(s),
            RespReply::Array(None) => json!([]),
            RespReply::Array(Some(items)) => {
                json!(items.iter().map(|x| x.flatten()).collect::<Vec<_>>())
            }
        }
    }
}
