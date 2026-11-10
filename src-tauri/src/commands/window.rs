//! 窗口控制与窗口管理 命令（原 commands.rs 对应分区的机械拆分）。
// 分片内命令可能被 feature 整体裁掉，故 glob import 允许未使用。
#[allow(unused_imports)]
use super::*;

// ==================== 窗口控制（Tauri 专属） ====================

/// 关闭加载窗口，显示主窗口
#[tauri::command]
pub async fn loding_window_close(window: tauri::Window) {
    app_window::loding_window_close(window).await
}

/// 关于窗口
#[tauri::command]
pub async fn about(handle: tauri::AppHandle, window: tauri::Window) {
    app_window::about(handle, window).await
}

/// 笔记本窗口
#[tauri::command]
pub async fn note_book_window(handle: tauri::AppHandle, window: tauri::Window) {
    app_window::note_book_window(handle, window).await
}

/// 最小化窗口
#[tauri::command]
pub fn windows_minimize(handle: tauri::AppHandle, label: String) {
    app_window::windows_minimize(handle, label)
}

/// 隐藏窗口
#[tauri::command]
pub fn windows_hide(handle: tauri::AppHandle, label: String) {
    app_window::windows_hide(handle, label)
}

/// 最大化窗口
#[tauri::command]
pub fn windows_maximize(handle: tauri::AppHandle, label: String) {
    app_window::windows_maximize(handle, label)
}

/// 关闭窗口
#[tauri::command]
pub fn windows_close(handle: tauri::AppHandle, label: String) {
    app_window::windows_close(handle, label)
}

/// 退出应用（与托盘「退出」同一入口）
///
/// 主窗口的 `CloseRequested` 被拦成「隐藏到托盘」，所以 window.close() 不能用来退出；
/// `app.exit` 走 `ExitRequested`，不经过 `CloseRequested`，能真正结束进程。
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

// ==================== 窗口管理 ====================

/// 创建新的编辑器窗口
#[tauri::command]
pub async fn create_editor_window(
    app: tauri::AppHandle,
    workspace_id: Option<String>,
    width: Option<f64>,
    height: Option<f64>,
    title: Option<String>,
    preset_id: Option<String>,
) -> Response<WindowMeta> {
    crate::window_manager::commands::create_editor_window(
        app, workspace_id, width, height, title, preset_id,
    )
    .await
}

/// 获取所有窗口列表
#[tauri::command]
pub async fn get_window_list() -> Response<Vec<WindowMeta>> {
    crate::window_manager::commands::get_window_list().await
}

/// 关闭指定窗口
#[tauri::command]
pub async fn close_window(app: tauri::AppHandle, label: String) -> Response<bool> {
    crate::window_manager::commands::close_window(app, label).await
}

/// 聚焦指定窗口
#[tauri::command]
pub async fn focus_window(app: tauri::AppHandle, label: String) -> Response<bool> {
    crate::window_manager::commands::focus_window(app, label).await
}

/// 获取窗口数量（不含主窗口）
#[tauri::command]
pub async fn get_window_count() -> Response<usize> {
    crate::window_manager::commands::get_window_count().await
}

/// 更新窗口位置信息
#[tauri::command]
pub async fn update_window_position_cmd(
    label: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    is_maximized: bool,
) -> Response<bool> {
    crate::window_manager::commands::update_window_position_cmd(
        label, x, y, width, height, is_maximized,
    )
    .await
}
