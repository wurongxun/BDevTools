//! # 实时流日志 —— 后端 TCP 监听
//!
//! 设计取舍：实时流分两类协议
//! - **WebSocket**：由前端用浏览器原生 `WebSocket` 直连，无需后端参与（避免新增 WS 依赖）。
//! - **TCP**：后端用 tokio 监听，按行切分后通过 Tauri 全局事件推送给前端。
//!
//! 本模块只负责 TCP 部分。事件：
//! - `logs-stream-line`：每行 `{ streamId, line }`
//! - `logs-stream-status`：状态变化 `{ streamId, status, message }`（`live`/`closed`/`error`）
//!
//! 前端以 `LogSource.id` 作为 `stream_id` 透传，便于事件路由到正确的源。

use std::collections::HashMap;

use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;

use bdevtools_core::response::response::Response;

/// 每行日志事件载荷
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamLinePayload {
    pub stream_id: String,
    pub line: String,
}

/// 流状态变化事件载荷
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamStatusPayload {
    pub stream_id: String,
    pub status: String,
    pub message: Option<String>,
}

type StreamHandle = tokio::task::JoinHandle<()>;

lazy_static! {
    static ref STREAMS: Mutex<HashMap<String, StreamHandle>> = Mutex::new(HashMap::new());
}

/// 启动一个 TCP 实时日志流。
/// - `emit`：事件推送出口，由 `commands.rs` 把 `tauri::AppHandle::emit` 包成闭包注入；
///   返回 false 表示事件总线不可用，任务自行结束。业务层不依赖 Tauri。
/// - `stream_id`：前端传入的源 ID（与 `LogSource.id` 对齐）
/// - `host` / `port`：TCP 目标
pub async fn log_stream_start_tcp<F>(
    emit: F,
    stream_id: String,
    host: String,
    port: u16,
) -> Response<String>
where
    F: Fn(&str, Value) -> bool + Send + Sync + Clone + 'static,
{
    // 同名旧流先停掉，避免重复
    stop_one(&stream_id);

    let emit_status_sink = emit.clone();
    let sid = stream_id.clone();
    let handle = tokio::spawn(async move {
        let emit_status = |status: &str, msg: Option<String>| {
            let payload = serde_json::to_value(StreamStatusPayload {
                stream_id: sid.clone(),
                status: status.to_string(),
                message: msg,
            })
            .unwrap_or_default();
            emit_status_sink("logs-stream-status", payload)
        };

        let connect = TcpStream::connect((host.as_str(), port)).await;
        let stream = match connect {
            Ok(s) => s,
            Err(e) => {
                emit_status("error", Some(format!("连接失败: {e}")));
                return;
            }
        };
        emit_status("live", None);

        let mut lines = BufReader::new(stream).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let payload = serde_json::to_value(StreamLinePayload {
                        stream_id: sid.clone(),
                        line,
                    })
                    .unwrap_or_default();
                    if !emit("logs-stream-line", payload) {
                        // 事件总线不可用，结束任务
                        break;
                    }
                }
                Ok(None) => {
                    emit_status("closed", Some("连接已关闭".to_string()));
                    break;
                }
                Err(e) => {
                    emit_status("error", Some(format!("读取失败: {e}")));
                    break;
                }
            }
        }
    });

    STREAMS.lock().insert(stream_id.clone(), handle);
    Response::new(200, stream_id, "")
}

/// 停止指定流（若存在），返回是否真的停止了一个
pub async fn log_stream_stop(stream_id: String) -> Response<bool> {
    let removed = stop_one(&stream_id);
    Response::new(200, removed, "")
}

/// 停止全部流（工作区切换 / 退出清理用）
pub fn log_stream_stop_all() {
    let mut map = STREAMS.lock();
    for (_, h) in map.drain() {
        h.abort();
    }
}

/// 停止单个流（若存在）
fn stop_one(stream_id: &str) -> bool {
    let mut map = STREAMS.lock();
    if let Some(h) = map.remove(stream_id) {
        h.abort();
        true
    } else {
        false
    }
}
