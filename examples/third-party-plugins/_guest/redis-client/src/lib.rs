//! redis-client 第三方插件的 **WASM 后端**（Extism guest）。
//!
//! 前端（沙箱 iframe）只能 `postMessage`，宿主把 `call` 转发进这里；本模块实现 Redis 的 RESP
//! 线协议与 P2 运维分析，字节收发走宿主 `net:` 授权下的原始 socket（见 `host.rs`）。
//! 连接按「键」持久在宿主侧、跨 `plugin_call` 有效；本 guest 每次调用都是新实例、**无状态**。
//!
//! 统一返回信封（页面/传输层据此判断）：
//!   `{"ok":true,"message":"ok","data":<任意>}` / `{"ok":false,"message":"<错误>"}`
//! `data` 的形状与前端 `types.ts` / `MockEngine` 一致，前端的 `Reply` / `unwrap` 无需改动。

mod analyze;
mod host;
mod resp;

use extism_pdk::*;
use serde::Deserialize;
use serde_json::{json, Value};

use host::Conn;
use resp::{send_cmd, RespReply};

/// 页面发来的请求（`plugin_call` 已对 input 做过一次 JSON 序列化，故这里收到的是 JSON 文本）。
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Req {
    #[serde(default)]
    action: String,
    #[serde(default)]
    conn_id: String,
    #[serde(default)]
    host: String,
    #[serde(default)]
    port: u16,
    #[serde(default)]
    password: String,
    #[serde(default)]
    db: i64,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    count: i64,
    #[serde(default)]
    sub: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    sample_limit: i64,
}

/// 宿主侧连接键：同一插件内的命名空间，用前端的 connId 区分不同连接。
fn conn_key(req: &Req) -> String {
    format!("redis:{}", req.conn_id)
}

fn ok(data: Value) -> String {
    json!({ "ok": true, "message": "ok", "data": data }).to_string()
}

fn fail(msg: &str) -> String {
    json!({ "ok": false, "message": msg }).to_string()
}

/// 清单声明 `"init": "init"` → 宿主在每次新建实例后先调用它。
#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    host::log("redis-client 后端：实例已创建");
    Ok(r#"{"ok":true}"#.to_string())
}

/// 页面所有请求都打到这个函数（`plugin_call` 的 func = "handle"）。
#[plugin_fn]
pub fn handle(input: String) -> FnResult<String> {
    let req: Req = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => return Ok(fail(&format!("请求解析失败: {e}"))),
    };
    Ok(dispatch(&req))
}

fn dispatch(req: &Req) -> String {
    match req.action.as_str() {
        "connect" => match do_connect(req) {
            Ok(v) => ok(v),
            Err(e) => fail(&e),
        },
        "exec" => match do_exec(req) {
            Ok(v) => ok(v),
            Err(e) => fail(&e),
        },
        "disconnect" => {
            host::tcp_close(&conn_key(req));
            ok(json!({ "ok": true }))
        }
        // P2 运维分析：值形状与前端一致（bulk(object) / array）
        "analyze_memory" => with_conn(req, |c| analyze::analyze_memory(c, req.sample_limit)),
        "slowlog" => with_conn(req, |c| analyze::slowlog(c, req.count)),
        "cluster" => with_conn(req, |c| analyze::cluster(c)),
        "replication" => with_conn(req, |c| analyze::replication(c)),
        "sentinel" => with_conn(req, |c| analyze::sentinel(c, &req.sub, &req.name)),
        other => fail(&format!("未知 action: {other}")),
    }
}

/// 在一条已建立的连接上跑一段分析（缺 connId / 出错都折叠成 `ok:false`）。
fn with_conn<F>(req: &Req, f: F) -> String
where
    F: FnOnce(&mut Conn) -> Result<Value, String>,
{
    if req.conn_id.is_empty() {
        return fail("缺少 connId");
    }
    let mut conn = Conn::new(conn_key(req));
    match f(&mut conn) {
        Ok(v) => ok(v),
        Err(e) => fail(&e),
    }
}

/// 建立连接：`host_tcp_connect` 之后按需 AUTH / SELECT。
fn do_connect(req: &Req) -> Result<Value, String> {
    if req.conn_id.is_empty() {
        return Err("缺少 connId".to_string());
    }
    if req.host.is_empty() {
        return Err("缺少 host".to_string());
    }
    let key = conn_key(req);
    // 同键重连是幂等的（宿主侧会关掉旧连接）
    host::tcp_connect(&key, &req.host, req.port)?;
    let mut conn = Conn::new(key.clone());

    if !req.password.is_empty() {
        if let RespReply::Error(e) = send_cmd(&mut conn, &["AUTH".to_string(), req.password.clone()])? {
            host::tcp_close(&key);
            return Err(format!("AUTH 失败: {e}"));
        }
    }
    if req.db != 0 {
        if let RespReply::Error(e) = send_cmd(&mut conn, &["SELECT".to_string(), req.db.to_string()])? {
            host::tcp_close(&key);
            return Err(format!("SELECT 失败: {e}"));
        }
    }
    Ok(json!({ "ok": true, "mode": "tcp", "id": req.conn_id }))
}

/// 执行任意命令：发一条、读一条，返回插件前端约定的 `Reply`。
fn do_exec(req: &Req) -> Result<Value, String> {
    if req.conn_id.is_empty() {
        return Err("缺少 connId".to_string());
    }
    if req.args.is_empty() {
        return Err("缺少 args".to_string());
    }
    let mut conn = Conn::new(conn_key(req));
    let reply = send_cmd(&mut conn, &req.args)?;
    Ok(reply.to_reply_json())
}
