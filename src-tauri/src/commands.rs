
use tauri::command;
use std::collections::HashMap;
use crate::response::response::Response;
//workspace
use crate::entity::window::{WorkspaceConfig, WorkspaceSummary};
use crate::workspace::commands;
use crate::workspace::plugin_dir;
#[tauri::command]
pub async fn workspace_list() -> Response<Vec<WorkspaceSummary>> {
    commands::workspace_list().await
}
/// 获取指定工作区完整配置
#[tauri::command]
pub async fn workspace_get(id: String) -> Response<Option<WorkspaceConfig>> {
   commands::workspace_get(id).await
}
/// 创建新工作区
#[tauri::command]
pub async fn workspace_create(name: String, folders: Vec<String>) -> Response<WorkspaceConfig> {
    commands::workspace_create(name, folders).await
}
/// 更新工作区配置
#[tauri::command]
pub async fn workspace_update(
    id: String,
    name: Option<String>,
    folders: Option<Vec<String>>,
    settings: Option<HashMap<String, serde_json::Value>>,
) -> Response<Option<WorkspaceConfig>> {
    commands::workspace_update(id, name, folders, settings).await
}
/// 删除工作区（同时清理该工作区的插件数据目录）
#[tauri::command]
pub async fn workspace_delete(id: String) -> Response<bool> {
    commands::workspace_delete(id).await
}
/// 获取或创建默认工作区
#[tauri::command]
pub async fn workspace_get_default() -> Response<WorkspaceConfig> {
    commands::workspace_get_default().await
}

/// 获取插件在当前工作区的工作目录（自动创建）
#[command]
pub fn workspace_get_plugin_dir(
    app: tauri::AppHandle,
    workspace_id: String,
    plugin_id: String,
) -> Response<String> {
   plugin_dir::workspace_get_plugin_dir(app, workspace_id, plugin_id)
}

/// 获取插件的子目录路径（自动创建）
#[command]
pub fn workspace_get_plugin_subdir(
    app: tauri::AppHandle,
    workspace_id: String,
    plugin_id: String,
    subdir: String,
) -> Response<String> {
    plugin_dir::workspace_get_plugin_subdir(app, workspace_id, plugin_id, subdir)   
}

/// 获取全局插件目录（跨工作区共享）
#[command]
pub fn workspace_get_global_plugin_dir(
    plugin_id: String,
) -> Response<String> {
    plugin_dir::workspace_get_global_plugin_dir(plugin_id)
}

/// 设置插件自定义工作目录（传空字符串则恢复默认，按工作区隔离）
#[command]
pub fn workspace_set_plugin_custom_dir(
    app: tauri::AppHandle,
    workspace_id: String,
    plugin_id: String,
    custom_path: String,
) -> Response<bool> {
    plugin_dir::workspace_set_plugin_custom_dir(app, workspace_id, plugin_id, custom_path)  
}

/// 获取插件自定义工作目录（None 表示使用默认，按工作区隔离）
#[command]
pub fn workspace_get_plugin_custom_dir(
    app: tauri::AppHandle,
    workspace_id: String,
    plugin_id: String,
) -> Response<Option<String>> {
    plugin_dir::workspace_get_plugin_custom_dir(app, workspace_id, plugin_id)
}