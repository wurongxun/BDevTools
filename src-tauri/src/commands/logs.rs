//! 日志索引筛选 / 实时流日志 命令（原 commands.rs 对应分区的机械拆分）。
// 分片内命令可能被 feature 整体裁掉，故 glob import 允许未使用。
#[allow(unused_imports)]
use super::*;

// ==================== 日志工作台：服务端索引与筛选（logs 插件） ====================

/// 建立日志索引（级别计数 / 时间范围 / 时间桶）
#[cfg(feature = "plugin-logs")]
#[tauri::command]
pub async fn logs_index_build(
    file_path: String,
    encoding: Option<String>,
) -> Result<ResData<LogIndexMeta>, String> {
    log_index::log_index_build(file_path, encoding).await
}

/// 合并查询：**筛选 + 关键词一次完成**，同时返回渲染段与导航行号（§2.5）
#[cfg(feature = "plugin-logs")]
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn logs_query_lines(
    file_path: String,
    encoding: Option<String>,
    levels: Option<Vec<String>>,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    invert: Option<bool>,
    merge_gap: Option<u64>,
    max_segments: Option<u64>,
    keyword: Option<String>,
    mode: Option<String>,
    ignore_case: Option<bool>,
    max_matches: Option<u64>,
    task_id: Option<String>,
) -> Result<ResData<LogQueryResult>, String> {
    log_index::log_query_lines(
        file_path,
        encoding,
        levels,
        ts_from,
        ts_to,
        invert,
        merge_gap,
        max_segments,
        keyword,
        mode,
        ignore_case,
        max_matches,
        task_id,
    )
    .await
}

/// 取消正在进行的查询任务（P6）；前端凭 `task_id` 静默丢弃结果
#[cfg(feature = "plugin-logs")]
#[tauri::command]
pub fn logs_cancel(task_id: String) -> Result<ResData<bool>, String> {
    let hit = log_index::cancel_query(task_id);
    Ok(ResData {
        code: 200,
        msg: if hit {
            "已请求取消".to_string()
        } else {
            "任务不存在或已结束".to_string()
        },
        data: hit,
    })
}

/// 按筛选条件流式导出日志到目标文件
#[cfg(feature = "plugin-logs")]
#[tauri::command]
pub async fn logs_index_export(
    file_path: String,
    encoding: Option<String>,
    dest_path: String,
    levels: Option<Vec<String>>,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    invert: Option<bool>,
    with_line_number: Option<bool>,
    with_bom: Option<bool>,
) -> Result<ResData<LogExportResult>, String> {
    log_index::log_index_export(
        file_path,
        encoding,
        dest_path,
        levels,
        ts_from,
        ts_to,
        invert,
        with_line_number,
        with_bom,
    )
    .await
}

/// 释放指定文件的索引缓存
#[cfg(feature = "plugin-logs")]
#[tauri::command]
pub async fn logs_index_drop(file_path: String) -> Result<ResData<bool>, String> {
    log_index::log_index_drop(file_path).await
}

// ==================== 实时流日志 ====================

/// 启动实时日志流。
/// - WebSocket：由前端原生 `WebSocket` 直连，后端不参与（此处返回提示）。
/// - TCP：后端建立监听，按行推送 `logs-stream-line` 事件。
#[cfg(feature = "plugin-logs")]
#[tauri::command]
pub async fn logs_stream_start(
    app_handle: tauri::AppHandle,
    stream_id: String,
    protocol: String,
    host: Option<String>,
    port: Option<u16>,
    #[allow(unused_variables)] url: Option<String>,
) -> Response<String> {
    if protocol == "tcp" {
        let host = match host {
            Some(h) => h,
            None => return Response::new(400, String::new(), "缺少 host 参数"),
        };
        let port = match port {
            Some(p) => p,
            None => return Response::new(400, String::new(), "缺少 port 参数"),
        };
        // 业务层不依赖 Tauri：把 AppHandle::emit 包成闭包注入（同 terminal 的 Channel 范式）。
        let emit = {
            let app = app_handle.clone();
            move |event: &str, payload: serde_json::Value| app.emit(event, payload).is_ok()
        };
        log_stream::log_stream_start_tcp(emit, stream_id, host, port).await
    } else {
        // WebSocket 由前端处理，无需后端
        Response::new(200, String::new(), "WebSocket 流由前端原生处理")
    }
}

/// 停止指定实时流（TCP 后端任务 / WebSocket 前端已自行关闭）
#[cfg(feature = "plugin-logs")]
#[tauri::command]
pub async fn logs_stream_stop(stream_id: String) -> Response<bool> {
    log_stream::log_stream_stop(stream_id).await
}
