//! KV 存储 / 会话 / 草稿 命令（原 commands.rs 对应分区的机械拆分）。
// 分片内命令可能被 feature 整体裁掉，故 glob import 允许未使用。
#[allow(unused_imports)]
use super::*;

// ==================== 通用 KV 存储（替代 tauri-plugin-store） ====================

/// 读取指定 key（不存在返回 null）
#[tauri::command]
pub fn kv_get(file: String, key: String) -> Response<Option<serde_json::Value>> {
    crate::app::kv_store::kv_get(file, key)
}

/// 写入指定 key（即时落盘）
#[tauri::command]
pub fn kv_set(file: String, key: String, value: serde_json::Value) -> Response<bool> {
    crate::app::kv_store::kv_set(file, key, value)
}

/// 删除指定 key（即时落盘）
#[tauri::command]
pub fn kv_delete(file: String, key: String) -> Response<bool> {
    crate::app::kv_store::kv_delete(file, key)
}

/// 读取全部键值对（返回 [key, value] 数组）
#[tauri::command]
pub fn kv_entries(file: String) -> Response<Vec<(String, serde_json::Value)>> {
    crate::app::kv_store::kv_entries(file)
}


// ==================== 会话管理 ====================

/// 保存会话快照
#[tauri::command]
pub async fn session_save(snapshot_data: SessionSnapshot) -> Response<bool> {
    crate::session::commands::session_save(snapshot_data).await
}

/// 加载指定窗口的会话快照
#[tauri::command]
pub async fn session_load(window_label: String) -> Response<Option<SessionSnapshot>> {
    crate::session::commands::session_load(window_label).await
}

/// 按工作区 ID 加载会话快照
#[tauri::command]
pub async fn session_load_workspace(workspace_id: String) -> Response<Option<SessionSnapshot>> {
    crate::session::commands::session_load_workspace(workspace_id).await
}

/// 按工作区 ID 删除会话快照
#[tauri::command]
pub async fn session_delete_workspace(workspace_id: String) -> Response<bool> {
    crate::session::commands::session_delete_workspace(workspace_id).await
}

/// 加载所有会话快照
#[tauri::command]
pub async fn session_load_all() -> Response<Vec<SessionSnapshot>> {
    crate::session::commands::session_load_all().await
}

/// 列出所有会话摘要
#[tauri::command]
pub async fn session_list() -> Response<Vec<SessionSummary>> {
    crate::session::commands::session_list().await
}

/// 删除指定窗口的会话快照
#[tauri::command]
pub async fn session_delete(window_label: String) -> Response<bool> {
    crate::session::commands::session_delete(window_label).await
}

/// 清除所有会话快照
#[tauri::command]
pub async fn session_clear() -> Response<bool> {
    crate::session::commands::session_clear().await
}

// ==================== 草稿管理 ====================

/// 保存草稿
#[tauri::command]
pub fn draft_save(
    workspace_id: String,
    file_path: String,
    content: String,
) -> Response<DraftMeta> {
    crate::session::draft::draft_save(workspace_id, file_path, content)
}

/// 检查草稿状态
#[tauri::command]
pub fn draft_check(workspace_id: String, file_path: String) -> Response<DraftCheckResult> {
    crate::session::draft::draft_check(workspace_id, file_path)
}

/// 读取草稿内容
#[tauri::command]
pub fn draft_load(workspace_id: String, file_path: String) -> Response<Option<String>> {
    crate::session::draft::draft_load(workspace_id, file_path)
}

/// 删除指定文件的草稿
#[tauri::command]
pub fn draft_delete(workspace_id: String, file_path: String) -> Response<bool> {
    crate::session::draft::draft_delete(workspace_id, file_path)
}
