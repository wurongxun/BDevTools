/*! # 会话管理 Tauri 命令 */
use crate::entity::window::{SessionSnapshot, SessionSummary};
use crate::response::response::Response;
use crate::session::snapshot;

/// 保存会话快照
#[tauri::command]
pub async fn session_save(snapshot_data: SessionSnapshot) -> Response<bool> {
    match snapshot::save_snapshot(&snapshot_data) {
        Ok(_) => Response::new(200, true, "会话快照保存成功"),
        Err(e) => Response::new(500, false, &format!("会话快照保存失败: {}", e)),
    }
}

/// 加载指定窗口的会话快照（兼容旧键：窗口 label）
#[tauri::command]
pub async fn session_load(window_label: String) -> Response<Option<SessionSnapshot>> {
    let snapshot = snapshot::load_snapshot(&window_label);
    Response::new(200, snapshot, "获取会话快照成功")
}

/// 按工作区 ID 加载会话快照（多窗口体系下的主入口）
#[tauri::command]
pub async fn session_load_workspace(workspace_id: String) -> Response<Option<SessionSnapshot>> {
    let snapshot = snapshot::load_snapshot_by_workspace(&workspace_id);
    Response::new(200, snapshot, "获取会话快照成功")
}

/// 按工作区 ID 删除会话快照（删除工作区/丢弃临时工作区时调用）
#[tauri::command]
pub async fn session_delete_workspace(workspace_id: String) -> Response<bool> {
    let ok = snapshot::delete_snapshot_by_workspace(&workspace_id);
    Response::new(200, ok, "会话快照已删除")
}

/// 加载所有会话快照
#[tauri::command]
pub async fn session_load_all() -> Response<Vec<SessionSnapshot>> {
    let snapshots = snapshot::load_all_snapshots();
    Response::new(200, snapshots, "获取所有会话快照成功")
}

/// 列出所有会话摘要
#[tauri::command]
pub async fn session_list() -> Response<Vec<SessionSummary>> {
    let list = snapshot::list_sessions();
    Response::new(200, list, "获取会话列表成功")
}

/// 删除指定窗口的会话快照
#[tauri::command]
pub async fn session_delete(window_label: String) -> Response<bool> {
    let ok = snapshot::delete_snapshot(&window_label);
    Response::new(200, ok, "会话快照已删除")
}

/// 清除所有会话快照
#[tauri::command]
pub async fn session_clear() -> Response<bool> {
    let ok = snapshot::clear_all_snapshots();
    Response::new(200, ok, "所有会话快照已清除")
}
