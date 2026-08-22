/*! # 工作区管理 Tauri 命令 */
use std::collections::HashMap;

use crate::entity::window::{WorkspaceConfig, WorkspaceSummary};
use crate::response::response::Response;
use crate::workspace::config;

/// 列出所有工作区摘要
#[tauri::command]
pub async fn workspace_list() -> Response<Vec<WorkspaceSummary>> {
    let list = config::list_workspaces();
    Response::new(200, list, "获取工作区列表成功")
}

/// 获取指定工作区完整配置
#[tauri::command]
pub async fn workspace_get(id: String) -> Response<Option<WorkspaceConfig>> {
    let config = config::get_workspace(&id);
    Response::new(200, config, "获取工作区配置成功")
}

/// 创建新工作区
#[tauri::command]
pub async fn workspace_create(name: String, folders: Vec<String>) -> Response<WorkspaceConfig> {
    let config = config::create_workspace(name, folders);
    Response::new(200, config, "工作区创建成功")
}

/// 更新工作区配置
#[tauri::command]
pub async fn workspace_update(
    id: String,
    name: Option<String>,
    folders: Option<Vec<String>>,
    settings: Option<HashMap<String, serde_json::Value>>,
) -> Response<Option<WorkspaceConfig>> {
    let config = config::update_workspace(&id, name, folders, settings);
    if config.is_some() {
        Response::new(200, config, "工作区更新成功")
    } else {
        Response::new(404, None, "工作区不存在")
    }
}

/// 删除工作区（同时清理该工作区的插件数据目录）
#[tauri::command]
pub async fn workspace_delete(id: String) -> Response<bool> {
    let ok = config::delete_workspace(&id);
    if ok {
        // 同步清理插件数据，避免孤儿目录残留（含临时工作区不保存场景）
        crate::workspace::plugin_dir::delete_plugin_data_dir(&id);
        // 同步清理会话快照与未保存草稿（与工作区同生命周期）
        crate::session::snapshot::delete_snapshot_by_workspace(&id);
        crate::session::draft::clear_workspace_drafts(&id);
        Response::new(200, true, "工作区已删除")
    } else {
        Response::new(404, false, "工作区不存在")
    }
}

/// 获取或创建默认工作区
#[tauri::command]
pub async fn workspace_get_default() -> Response<WorkspaceConfig> {
    let config = config::get_or_create_default_workspace();
    Response::new(200, config, "获取默认工作区成功")
}
