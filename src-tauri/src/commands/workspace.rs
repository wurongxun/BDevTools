//! 工作区 / 锁 / 插件工作目录 命令（原 commands.rs 对应分区的机械拆分）。
// 分片内命令可能被 feature 整体裁掉，故 glob import 允许未使用。
#[allow(unused_imports)]
use super::*;

// ==================== 工作区管理 ====================

/// 列出所有工作区摘要
#[tauri::command]
pub async fn workspace_list() -> Response<Vec<WorkspaceSummary>> {
    crate::workspace::commands::workspace_list().await
}

/// 获取指定工作区完整配置
#[tauri::command]
pub async fn workspace_get(id: String) -> Response<Option<WorkspaceConfig>> {
    crate::workspace::commands::workspace_get(id).await
}

/// 创建新工作区
#[tauri::command]
pub async fn workspace_create(name: String, folders: Vec<String>) -> Response<WorkspaceConfig> {
    crate::workspace::commands::workspace_create(name, folders).await
}

/// 更新工作区配置
#[tauri::command]
pub async fn workspace_update(
    id: String,
    name: Option<String>,
    folders: Option<Vec<String>>,
    settings: Option<HashMap<String, serde_json::Value>>,
) -> Response<Option<WorkspaceConfig>> {
    crate::workspace::commands::workspace_update(id, name, folders, settings).await
}

/// 删除工作区
#[tauri::command]
pub async fn workspace_delete(id: String) -> Response<bool> {
    crate::workspace::commands::workspace_delete(id).await
}

/// 获取或创建默认工作区
#[tauri::command]
pub async fn workspace_get_default() -> Response<WorkspaceConfig> {
    crate::workspace::commands::workspace_get_default().await
}

// ==================== 工作区锁 ====================

/// 尝试锁定工作区
#[tauri::command]
pub async fn workspace_lock(workspace_id: String, window_label: String) -> Response<bool> {
    crate::workspace::lock::workspace_lock(workspace_id, window_label).await
}

/// 释放工作区锁
#[tauri::command]
pub async fn workspace_unlock(workspace_id: String, window_label: String) -> Response<bool> {
    crate::workspace::lock::workspace_unlock(workspace_id, window_label).await
}

/// 查询工作区锁定状态
#[tauri::command]
pub async fn workspace_lock_status(workspace_id: String) -> Response<Option<String>> {
    crate::workspace::lock::workspace_lock_status(workspace_id).await
}

// ==================== 插件工作目录 ====================

/// 获取插件在当前工作区的工作目录（自动创建）
#[tauri::command]
pub fn workspace_get_plugin_dir(
    workspace_id: String,
    plugin_id: String,
) -> Response<String> {
    crate::workspace::plugin_dir::workspace_get_plugin_dir(workspace_id, plugin_id)
}

/// 获取插件的子目录路径（自动创建）
#[tauri::command]
pub fn workspace_get_plugin_subdir(
    workspace_id: String,
    plugin_id: String,
    subdir: String,
) -> Response<String> {
    crate::workspace::plugin_dir::workspace_get_plugin_subdir(workspace_id, plugin_id, subdir)
}

/// 获取插件外部脚本目录列表（用户级，跨工作区共享）
#[tauri::command]
pub fn workspace_list_plugin_external_dirs(plugin_id: String) -> Response<Vec<String>> {
    crate::workspace::plugin_dir::workspace_list_plugin_external_dirs(plugin_id)
}

/// 添加插件外部脚本目录（用户级，跨工作区共享）
#[tauri::command]
pub fn workspace_add_plugin_external_dir(
    workspace_id: String,
    plugin_id: String,
    path: String,
) -> Response<bool> {
    crate::workspace::plugin_dir::workspace_add_plugin_external_dir(workspace_id, plugin_id, path)
}

/// 移除插件外部脚本目录（仅解除挂载，不删除目录内容）
#[tauri::command]
pub fn workspace_remove_plugin_external_dir(
    plugin_id: String,
    path: String,
) -> Response<bool> {
    crate::workspace::plugin_dir::workspace_remove_plugin_external_dir(plugin_id, path)
}
