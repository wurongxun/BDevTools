//! # Tauri 命令统一门面
//!
//! 本文件是唯一的 `#[tauri::command]` 定义处（RPC 边界），
//! 也是 **唯一** 持有 `tauri` / `tauri_plugin_*` 依赖的业务代码。
//! 所有命令均为薄包装：仅做参数转发，业务逻辑保留在各模块中。
//! 将来更换 UI 框架时，只需重写本文件（及 lib.rs 的组装逻辑）。
//!
//! 命令名即暴露给前端的 invoke 名称，改名需同步前端。

use std::collections::HashMap;
// 仅 ssh 插件用（logs_pull_remote 取远端文件名）；裁掉 ssh 后必须一起裁，否则 unused import
#[cfg(feature = "plugin-ssh")]
use std::path::Path;

use serde::Serialize;

use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::{NotificationExt, PermissionState};
use tauri_plugin_opener::OpenerExt;

use crate::core::ctx::AppContext;
use crate::window_manager::app_window;
#[cfg(feature = "plugin-files")]
use crate::plugins::files::file_association;
#[cfg(feature = "plugin-files")]
use crate::plugins::files::file_command::{self, ScriptFileEntry};
#[cfg(feature = "plugin-files")]
use crate::plugins::files::file_utils;
#[cfg(feature = "plugin-logs")]
use crate::plugins::logs::log_index::{
    self, LogExportResult, LogIndexMeta, LogQueryResult,
};
#[cfg(feature = "plugin-logs")]
use crate::plugins::logs::log_stream;
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_hprof::{self, MatHeaderInfo, MatHistogramResult};
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_leak::{self, MatLeakSuspectsResult};
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_cfg::{self, MatLimitInfo};
// 目录已改名为 plugin_host/{host,wasm}.rs，但函数体一律用裸名 `plugin_host::` / `plugin_wasm::`
// 书写（与插件 id 语义对齐），故 import 时用别名把名字拉回原样，避免改动几百处调用点。
use crate::plugin_host::host as plugin_host;
use crate::plugin_host::wasm as plugin_wasm;
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_refs::{
    self, RefBuildInfo, RefInstances, RefObjectDetail, RefSearchResult,
};
#[cfg(feature = "plugin-ssh")]
use crate::plugins::ssh::remote_conn::{self, RemoteFileItem};
#[cfg(feature = "plugin-terminal")]
use crate::plugins::terminal::terminal;
// 行读取是**核心能力**（files 的 TextView 与 logs 都用），不随任何插件裁剪
use crate::utils::line_reader::{self, FileLinesResult};
#[cfg(feature = "plugin-database")]
use crate::plugins::database::connection::DbConnectionConfig;
#[cfg(feature = "plugin-database")]
use crate::plugins::database::import_export::{ExportResult, ImportResult};
#[cfg(feature = "plugin-database")]
use crate::plugins::database::metadata::{
    CatalogCapabilities, DatabaseIndexInfo, DatabaseObjectInfo, DatabaseInfo, DbAdminInfo, DbUserInfo,
    IndexInfo, KeyValueRow, TableInfo,
};
#[cfg(feature = "plugin-database")]
use crate::plugins::database::operations::{ColumnInfo, SqlResult, TableDataResult};
use crate::entity::session::{SessionSnapshot, SessionSummary};
use crate::entity::window::WindowMeta;
use crate::entity::workspace::{WorkspaceConfig, WorkspaceSummary};
use crate::response::resp_data::ResData;
use crate::response::response::Response;
use crate::session::draft::{DraftCheckResult, DraftMeta};
// SSH 连接配置（sqlite 表在核心，但只有 ssh 插件的命令用它）
#[cfg(feature = "plugin-ssh")]
use crate::sqlite::ssh_connection::SshSubMenu;

/// tauri → 业务 的唯一适配点：把 `tauri::AppHandle` 转换为平台无关的 [`AppContext`]。
/// 将来更换 UI 框架：仅重写此函数，业务模块零改动。
fn ctx_from_app(app: &tauri::AppHandle) -> AppContext {
    let data_dir = app.path().app_data_dir().unwrap_or_default();
    let config_dir = app.path().app_config_dir().unwrap_or_default();
    let document_dir = app.path().document_dir().ok();

    let emitter_app = app.clone();
    let opener_app = app.clone();
    let notify_app = app.clone();
    let autostart_app = app.clone();

    AppContext::new(data_dir)
        .with_config_dir(config_dir)
        .with_document_dir(document_dir)
        .with_emitter(move |event, payload| {
            let _ = emitter_app.emit(event, payload);
        })
        .with_opener(move |path| {
            opener_app
                .opener()
                .open_path(path, None::<String>)
                .map_err(|e| e.to_string())
        })
        .with_notifier(move |body| {
            let notification = notify_app.notification();
            if matches!(notification.permission_state(), Ok(PermissionState::Granted)) {
                let _ = notification.builder().body(body.to_string()).show();
            }
        })
        .with_autostart(move |enabled| {
            let manager = autostart_app.autolaunch();
            if enabled {
                let _ = manager.enable();
            } else {
                let _ = manager.disable();
            }
        })
}

// ==================== 文件管理 ====================

/// 打开目录（项目）。**核心能力**（`utils::fs`）：任何插件都可能要「打开所在目录」，
/// 不该被 files 插件的开关绑住。
#[tauri::command]
pub async fn file_manager_open_path(app_handle: tauri::AppHandle, path: String) -> ResData<String> {
    crate::utils::fs::open_path_in_file_manager(&ctx_from_app(&app_handle), path).await
}

/// 获取文档笔记本（目录树）
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn get_document_notebooks(
    app_handle: tauri::AppHandle,
    working_directory: Option<String>,
) -> ResData<String> {
    file_utils::get_document_notebooks(&ctx_from_app(&app_handle), working_directory).await
}

/// 更新文件夹名称
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn up_folder_name(
    path: String,
    up_folder_name: String,
) -> Result<ResData<String>, String> {
    file_utils::up_folder_name(path, up_folder_name).await
}

/// 检测文件编码
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn detect_file_encoding(path: String) -> Result<ResData<String>, String> {
    file_utils::detect_file_encoding(path).await
}

/// 确保目录存在（递归创建）。
/// **核心能力**（`utils::fs`）：database 等插件也要建目录，不能挂在 `plugin-files` 开关上。
#[tauri::command]
pub async fn ensure_dir(path: String) -> Result<ResData<String>, String> {
    crate::utils::fs::ensure_dir(path).await
}

/// 获取目录下笔记
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn get_note_list(path: String) -> Result<ResData<Vec<String>>, String> {
    file_command::get_note_list(path).await
}

/// 读取本地笔记文件（自动检测编码）
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn read_note_file(path: String) -> Result<ResData<String>, String> {
    file_command::read_note_file(path).await
}

/// 获取文件大小（字节），供大文件分诊
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn get_file_size(path: String) -> Result<ResData<u64>, String> {
    file_command::get_file_size(path).await
}

/// 读取文件原始字节，前端直接拿到 ArrayBuffer（图片查看用）。
/// 走裸字节通道而非 ResData，见 `file_command::read_file_bytes` 的说明。
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn read_file_bytes(path: String) -> Result<tauri::ipc::Response, String> {
    file_command::read_file_bytes(path).await
}

/// 创建本地文件夹。
/// **核心能力**（`utils::fs`）：见 `ensure_dir` 的说明 —— 纯 fs 操作，不随 files 插件裁剪。
#[tauri::command]
pub async fn create_folder(
    father_path: String,
    new_folder_name: String,
) -> Result<ResData<String>, String> {
    crate::utils::fs::create_folder(father_path, new_folder_name).await
}

/// 创建笔记文件
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn create_note_file(
    father_path: String,
    new_note_md_file_name: String,
) -> Result<ResData<String>, String> {
    file_command::create_note_file(father_path, new_note_md_file_name).await
}

/// 保存笔记
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn save_note(path: String, md_text: String) -> Result<ResData<String>, String> {
    file_command::save_note(path, md_text).await
}

/// 更新文件名
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn up_file_name(
    path: String,
    up_file_name: String,
) -> Result<ResData<String>, String> {
    file_command::up_file_name(path, up_file_name).await
}

/// 删除本地文件夹
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn remove_folder(path: String) -> Result<ResData<String>, String> {
    file_command::remove_folder(path).await
}

/// 删除本地文件
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn remove_file(path: String) -> Result<ResData<String>, String> {
    file_command::remove_file(path).await
}

/// 获取目录下所有文件及文件夹名称
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn get_path_childrenFoldersAndFiles(
    path: String,
) -> Result<ResData<Vec<String>>, String> {
    file_command::get_path_childrenFoldersAndFiles(path).await
}

/// 写入脚本文件（不存在则创建，存在则覆盖）
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn write_script_file(path: String, content: String) -> Result<ResData<String>, String> {
    file_command::write_script_file(path, content).await
}

/// 重命名脚本文件（保留原扩展名）
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn rename_script_file(
    path: String,
    new_name: String,
) -> Result<ResData<String>, String> {
    file_command::rename_script_file(path, new_name).await
}

/// 递归列出目录下所有文件与文件夹
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn list_script_files(path: String) -> Result<ResData<Vec<ScriptFileEntry>>, String> {
    file_command::list_script_files(path).await
}

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

// ==================== 大文件按行读取（保留） ====================

/// 按行范围读取文件内容
#[tauri::command]
pub async fn read_file_lines(
    file_path: String,
    start_line: Option<u64>,
    line_count: Option<u64>,
    encoding: Option<String>,
) -> Result<ResData<FileLinesResult>, String> {
    line_reader::read_file_lines(file_path, start_line, line_count, encoding).await
}

/// 快速统计文件总行数
#[tauri::command]
pub async fn count_file_lines(file_path: String) -> Result<ResData<u64>, String> {
    line_reader::count_file_lines(file_path).await
}

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

// ==================== Java 堆转储分析（mat 插件） ====================

/// 读取堆转储文件头：格式识别（HPROF / PHD / 未知）+ 元信息
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_hprof_header(path: String) -> Result<ResData<MatHeaderInfo>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_hprof::mat_hprof_header(&p))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 生成按类聚合的浅堆直方图（Shallow Heap）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_hprof_histogram(
    path: String,
    limit: Option<usize>,
) -> Result<ResData<MatHistogramResult>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_hprof::mat_hprof_histogram(&p, limit))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 分析预算 / 规模上限明细（前端「分析预算」设置弹窗，不依赖任何 dump）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_analysis_limits() -> Result<ResData<MatLimitInfo>, String> {
    tokio::task::spawn_blocking(mat_cfg::mat_analysis_limits)
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// Leak Suspects：支配树 + Retained Heap 的泄漏嫌疑分析（对象多时耗时数十秒级）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_leak_suspects(path: String) -> Result<ResData<MatLeakSuspectsResult>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_leak::mat_leak_suspects(&p))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：构建（或复用）对象图会话，返回图规模与可达性汇总
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_refs_build(path: String) -> Result<ResData<RefBuildInfo>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_refs::mat_refs_build(&p))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：释放某 dump 的会话缓存（关闭 dump 时调用）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_refs_release(path: String) -> Result<ResData<bool>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_refs::mat_refs_release(&p))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：类名搜索（大小写不敏感子串，按浅堆合计降序）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_refs_search_classes(
    path: String,
    query: String,
    limit: Option<usize>,
) -> Result<ResData<RefSearchResult>, String> {
    let (p, q) = (path.clone(), query.clone());
    tokio::task::spawn_blocking(move || {
        mat_refs::mat_refs_search_classes(&p, &q, limit.unwrap_or(40))
    })
    .await
    .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：取某类实例样本（按浅堆降序 Top N）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_refs_instances(
    path: String,
    class_name: String,
    limit: Option<usize>,
) -> Result<ResData<RefInstances>, String> {
    let (p, c) = (path.clone(), class_name.clone());
    tokio::task::spawn_blocking(move || {
        mat_refs::mat_refs_instances(&p, &c, limit.unwrap_or(300))
    })
    .await
    .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：单个对象详情（outgoing / incoming / Path to GC Roots）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_refs_inspect(
    path: String,
    obj_id: String,
) -> Result<ResData<RefObjectDetail>, String> {
    let (p, o) = (path.clone(), obj_id.clone());
    tokio::task::spawn_blocking(move || mat_refs::mat_refs_inspect(&p, &o))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

// ==================== 文件关联（Windows） ====================

/// 注册文件关联
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn register_file_association() -> ResData<String> {
    file_association::register_file_association().await
}

/// 打开系统默认应用设置
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn open_default_apps_settings() -> ResData<String> {
    file_association::open_default_apps_settings().await
}

// ==================== SSH / SFTP 远程连接 ====================

/// SSH 连接
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_connect(
    app_handle: tauri::AppHandle,
    tab_id: String,
    host: String,
    port: u16,
    username: String,
    password: String,
    auth_type: Option<String>,
    private_key_path: Option<String>,
) -> Response<Option<String>> {
    remote_conn::ssh_connect(
        ctx_from_app(&app_handle), tab_id, host, port, username, password, auth_type, private_key_path,
    )
    .await
}

/// SSH 写入（用户输入）
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_write(session_id: String, data: String) -> Response<bool> {
    remote_conn::ssh_write(session_id, data).await
}

/// SSH 断开连接
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_disconnect(session_id: String) -> Response<bool> {
    remote_conn::ssh_disconnect(session_id).await
}

/// SSH 连接状态检测
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_check(session_id: String) -> Response<bool> {
    remote_conn::ssh_check(session_id).await
}

/// SSH 调整 PTY 大小
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_resize_pty(session_id: String, cols: u32, rows: u32) -> Response<bool> {
    remote_conn::ssh_resize_pty(session_id, cols, rows).await
}

/// SFTP 连接
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn sftp_connect(
    tab_id: String,
    host: String,
    port: u16,
    username: String,
    password: String,
    auth_type: Option<String>,
    private_key_path: Option<String>,
) -> Response<Option<String>> {
    remote_conn::sftp_connect(tab_id, host, port, username, password, auth_type, private_key_path)
        .await
}

/// SFTP 断开连接
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn sftp_disconnect(session_id: String) -> Response<bool> {
    remote_conn::sftp_disconnect(session_id).await
}

/// SFTP 连接状态检测
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn sftp_check(session_id: String) -> Response<bool> {
    remote_conn::sftp_check(session_id).await
}

/// SFTP 列出目录
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn sftp_list_dir(session_id: String, path: String) -> Response<Vec<RemoteFileItem>> {
    remote_conn::sftp_list_dir(session_id, path).await
}

/// SFTP 创建目录
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn sftp_mkdir(session_id: String, path: String) -> Response<bool> {
    remote_conn::sftp_mkdir(session_id, path).await
}

/// SFTP 删除文件/目录
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn sftp_remove(session_id: String, path: String) -> Response<bool> {
    remote_conn::sftp_remove(session_id, path).await
}

/// SFTP 上传文件
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn sftp_upload(
    app_handle: tauri::AppHandle,
    session_id: String,
    local_path: String,
    remote_path: String,
    task_id: String,
) -> Response<bool> {
    remote_conn::sftp_upload(ctx_from_app(&app_handle), session_id, local_path, remote_path, task_id).await
}

/// SFTP 下载文件
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn sftp_download(
    app_handle: tauri::AppHandle,
    session_id: String,
    remote_path: String,
    local_path: String,
    task_id: String,
) -> Response<bool> {
    remote_conn::sftp_download(ctx_from_app(&app_handle), session_id, remote_path, local_path, task_id).await
}

/// 远程日志拉取结果
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsPullRemoteResp {
    pub local_path: String,
    pub size: u64,
}

/// 从远程服务器拉取日志文件到本地缓存，并返回本地路径
///
/// 复用现有 SFTP 能力：建立 SFTP 会话 → 下载到 config_dir/plugin_data/logs_remote → 断开。
/// 前端拿到本地缓存路径后，像打开本地文件一样走既有分块加载 / 索引 / 筛选 / 时间轴链路。
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn logs_pull_remote(
    app_handle: tauri::AppHandle,
    host: String,
    port: u16,
    username: String,
    password: String,
    auth_type: Option<String>,
    private_key_path: Option<String>,
    remote_path: String,
) -> Response<LogsPullRemoteResp> {
    let ctx = ctx_from_app(&app_handle);
    let conn = remote_conn::sftp_connect(
        "logs".to_string(),
        host.clone(),
        port,
        username.clone(),
        password.clone(),
        auth_type.clone(),
        private_key_path.clone(),
    )
    .await;
    let session_id = match conn.data {
        Some(sid) => sid,
        None => {
            return Response::new(
                conn.code,
                LogsPullRemoteResp { local_path: String::new(), size: 0 },
                conn.msg.as_str(),
            )
        }
    };

    let cache_dir = ctx.config_dir.join("plugin_data").join("logs_remote");
    let _ = std::fs::create_dir_all(&cache_dir);
    let raw_name = Path::new(&remote_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "remote.log".to_string());
    let safe_name: String = raw_name
        .chars()
        .map(|c| if c == '/' || c == '\\' || c == ':' || c == '?' || c == '*' { '_' } else { c })
        .collect();
    let local_path = cache_dir.join(safe_name).to_string_lossy().to_string();

    let dl = remote_conn::sftp_download(
        ctx.clone(),
        session_id.clone(),
        remote_path.clone(),
        local_path.clone(),
        "logs-pull".to_string(),
    )
    .await;
    let _ = remote_conn::sftp_disconnect(session_id).await;

    if dl.code != 200 {
        return Response::new(
            dl.code,
            LogsPullRemoteResp { local_path: String::new(), size: 0 },
            dl.msg.as_str(),
        );
    }
    let size = std::fs::metadata(&local_path).map(|m| m.len()).unwrap_or(0);
    Response::new(200, LogsPullRemoteResp { local_path, size }, "拉取成功")
}

/// 本地目录列出
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn local_list_dir(path: String) -> Response<Vec<RemoteFileItem>> {
    remote_conn::local_list_dir(path).await
}

/// SFTP 重命名文件/目录
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn sftp_rename(session_id: String, old_path: String, new_path: String) -> Response<bool> {
    remote_conn::sftp_rename(session_id, old_path, new_path).await
}

/// 获取应用数据目录路径
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn get_app_dir() -> Response<String> {
    remote_conn::get_app_dir().await
}

// ==================== SSH 连接配置（SQLite） ====================

/// 创建 SSH 连接配置
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn connection_create(map: SshSubMenu) -> Response<Option<bool>> {
    crate::sqlite::ssh_connection::connection_create(map).await
}

/// 删除 SSH 连接配置
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn connection_delete(id: String) -> Response<bool> {
    crate::sqlite::ssh_connection::connection_delete(id).await
}

/// 编辑 SSH 连接配置
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn connection_edit(map: SshSubMenu) -> Response<bool> {
    crate::sqlite::ssh_connection::connection_edit(map).await
}

/// 查询 SSH 连接配置列表
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn connection_list() -> Response<Vec<SshSubMenu>> {
    crate::sqlite::ssh_connection::connection_list().await
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

// ==================== 数据库连接配置 ====================

/// 创建数据库连接配置
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_connection_create(config: DbConnectionConfig) -> Response<Option<String>> {
    crate::plugins::database::connection::db_connection_create(config).await
}

/// 删除数据库连接配置
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_connection_delete(id: String) -> Response<bool> {
    crate::plugins::database::connection::db_connection_delete(id).await
}

/// 更新数据库连接配置
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_connection_update(config: DbConnectionConfig) -> Response<bool> {
    crate::plugins::database::connection::db_connection_update(config).await
}

/// 获取所有连接配置列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_connection_list() -> Response<Vec<DbConnectionConfig>> {
    crate::plugins::database::connection::db_connection_list().await
}

/// 测试数据库连接连通性
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_connection_test(config: DbConnectionConfig) -> Response<bool> {
    crate::plugins::database::connection::db_connection_test(config).await
}

/// 建立数据库连接（创建连接池）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_connect(config_id: String) -> Response<bool> {
    crate::plugins::database::connection::db_connect(config_id).await
}

/// 断开数据库连接
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_disconnect(config_id: String) -> Response<bool> {
    crate::plugins::database::connection::db_disconnect(config_id).await
}

// ==================== 数据库操作 ====================

/// 执行 SQL 语句（分页）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_execute_sql(
    config_id: String,
    sql: String,
    database: Option<String>,
    page_size: Option<u64>,
    offset: Option<u64>,
) -> Response<SqlResult> {
    crate::plugins::database::operations::db_execute_sql(config_id, sql, database, page_size, offset).await
}

/// 分页查询表数据
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_query_table_data(
    config_id: String,
    database: String,
    table: String,
    page: u64,
    page_size: u64,
    where_clause: Option<String>,
    order_by: Option<String>,
) -> Response<TableDataResult> {
    crate::plugins::database::operations::db_query_table_data(
        config_id, database, table, page, page_size, where_clause, order_by,
    )
    .await
}

/// 结果网格：批量写回（一个事务，逐条返回受影响行数）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_apply_edits(
    config_id: String,
    database: String,
    ops: Vec<crate::plugins::database::editing::EditOp>,
) -> Response<crate::plugins::database::editing::EditBatchResult> {
    crate::plugins::database::editing::db_apply_edits(config_id, database, ops).await
}

/// 结果网格：预览批量写将要执行的 SQL（与 `db_apply_edits` 同源，不落地）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_preview_edits(
    config_id: String,
    database: String,
    ops: Vec<crate::plugins::database::editing::EditOp>,
) -> Response<crate::plugins::database::editing::EditPreview> {
    crate::plugins::database::editing::db_preview_edits(config_id, database, ops).await
}

// ==================== 数据库元数据 ====================
/// 获取所有数据库列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_databases(config_id: String) -> Response<Vec<DatabaseInfo>> {
    crate::plugins::database::metadata::db_list_databases(config_id).await
}

/// 获取指定数据库的所有表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_tables(config_id: String, database: String) -> Response<Vec<TableInfo>> {
    crate::plugins::database::metadata::db_list_tables(config_id, database).await
}

/// 获取表的字段信息
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_table_columns(
    config_id: String,
    database: String,
    table: String,
) -> Response<Vec<ColumnInfo>> {
    crate::plugins::database::metadata::db_table_columns(config_id, database, table).await
}

/// 获取表的索引信息
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_table_indexes(
    config_id: String,
    database: String,
    table: String,
) -> Response<Vec<IndexInfo>> {
    crate::plugins::database::metadata::db_table_indexes(config_id, database, table).await
}

/// 获取建表 DDL
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_table_schema(
    config_id: String,
    database: String,
    table: String,
) -> Response<String> {
    crate::plugins::database::metadata::db_table_schema(config_id, database, table).await
}

// ---- 目录能力声明与新增对象（走 catalog 抽象，各库按自身能力实现）----

/// 查询当前连接支持哪些目录能力（前端据此决定渲染哪些树节点）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_catalog_capabilities(config_id: String) -> Response<CatalogCapabilities> {
    crate::plugins::database::metadata::db_catalog_capabilities(config_id).await
}

/// 获取指定库的视图列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_views(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    crate::plugins::database::metadata::db_list_views(config_id, database).await
}

/// 获取指定库的存储过程列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_procedures(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    crate::plugins::database::metadata::db_list_procedures(config_id, database).await
}

/// 获取指定库的函数列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_functions(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    crate::plugins::database::metadata::db_list_functions(config_id, database).await
}

/// 取某个对象的定义源码（视图 / 存储过程 / 函数 / 触发器）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_object_source(
    config_id: String,
    database: String,
    kind: String,
    name: String,
) -> Response<String> {
    crate::plugins::database::metadata::db_object_source(config_id, database, kind, name).await
}

/// 获取指定库的触发器列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_triggers(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    crate::plugins::database::metadata::db_list_triggers(config_id, database).await
}

/// 获取指定库的事件列表（仅 MySQL 支持）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_events(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    crate::plugins::database::metadata::db_list_events(config_id, database).await
}

/// 获取指定库的索引聚合
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_database_indexes(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseIndexInfo>> {
    crate::plugins::database::metadata::db_list_database_indexes(config_id, database).await
}

/// 获取实例用户列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_users(config_id: String) -> Response<Vec<DbUserInfo>> {
    crate::plugins::database::metadata::db_list_users(config_id).await
}

/// 获取管理员列表（各库用自身原生语义判定）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_admins(config_id: String) -> Response<Vec<DbAdminInfo>> {
    crate::plugins::database::metadata::db_list_admins(config_id).await
}

/// 获取运行状态（scope: session | global）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_show_status(config_id: String, scope: String) -> Response<Vec<KeyValueRow>> {
    crate::plugins::database::metadata::db_show_status(config_id, scope).await
}

/// 获取系统变量（scope: session | global）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_show_variables(config_id: String, scope: String) -> Response<Vec<KeyValueRow>> {
    crate::plugins::database::metadata::db_show_variables(config_id, scope).await
}

// ==================== 数据库导入导出 ====================

/// 导出表数据为 CSV
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_export_csv(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
) -> Response<ExportResult> {
    crate::plugins::database::import_export::db_export_csv(config_id, database, table, file_path).await
}

/// 导出表数据为 Excel
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_export_excel(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
) -> Response<ExportResult> {
    crate::plugins::database::import_export::db_export_excel(config_id, database, table, file_path).await
}

/// 按任意 SQL 语句导出结果为 CSV
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_export_sql_csv(
    config_id: String,
    sql: String,
    file_path: String,
) -> Response<ExportResult> {
    crate::plugins::database::import_export::db_export_sql_csv(config_id, sql, file_path).await
}

/// 按任意 SQL 语句导出结果为 Excel
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_export_sql_excel(
    config_id: String,
    sql: String,
    file_path: String,
) -> Response<ExportResult> {
    crate::plugins::database::import_export::db_export_sql_excel(config_id, sql, file_path).await
}

/// 从 CSV 导入数据到表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_import_csv(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
    has_header: bool,
) -> Response<ImportResult> {
    crate::plugins::database::import_export::db_import_csv(config_id, database, table, file_path, has_header)
        .await
}

/// 从 Excel 导入数据到表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_import_excel(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
) -> Response<ImportResult> {
    crate::plugins::database::import_export::db_import_excel(config_id, database, table, file_path).await
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
        log_stream::log_stream_start_tcp(app_handle, stream_id, host, port).await
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

// ==================== 第三方插件（安装 / 卸载 / 排序 / 调用预留） ====================

/// 列出已安装第三方插件（内置插件由前端提供，不在此列）
#[tauri::command]
pub fn plugin_list() -> Response<Vec<plugin_host::InstalledPlugin>> {
    plugin_host::plugin_list()
}

/// 安装插件：`source` 为插件目录或 .zip 包；同名插件覆盖安装
#[tauri::command]
pub async fn plugin_install(source: String) -> Response<plugin_host::InstalledPlugin> {
    match tokio::task::spawn_blocking(move || plugin_host::plugin_install(source)).await {
        Ok(r) => r,
        Err(e) => Response::new(
            500,
            plugin_host::InstalledPlugin::empty(),
            &format!("安装线程异常: {e}"),
        ),
    }
}

/// 卸载插件（removeData=true 时一并清理插件私有数据）
#[tauri::command]
pub async fn plugin_uninstall(id: String, remove_data: bool) -> Response<bool> {
    match tokio::task::spawn_blocking(move || plugin_host::plugin_uninstall(id, remove_data)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, false, &format!("卸载线程异常: {e}")),
    }
}

/// 启用 / 禁用插件（内置与第三方同一套：禁用集合持久化在 settings.json）
#[tauri::command]
pub fn plugin_set_enabled(id: String, enabled: bool) -> Response<bool> {
    plugin_host::plugin_set_enabled(id, enabled)
}

/// 被禁用的插件 ID 列表（内置 + 第三方统一集合；前端据此决定注册哪些插件）
#[tauri::command]
pub fn plugin_get_disabled() -> Response<Vec<String>> {
    plugin_host::plugin_get_disabled()
}

/// 读取统一排序（内置 + 第三方插件 ID 的全量顺序）
#[tauri::command]
pub fn plugin_get_order() -> Response<Vec<String>> {
    plugin_host::plugin_get_order()
}

/// 保存统一排序（内置 + 第三方插件 ID 的全量顺序）
#[tauri::command]
pub fn plugin_set_order(order: Vec<String>) -> Response<bool> {
    plugin_host::plugin_set_order(order)
}

/// 收紧某插件的页面能力（**只能减，不能加**；传空数组 = 恢复插件自己的声明）
///
/// 生效口径 = 清单 `pageCapabilities` − `denied`，下一次加载插件页时生效
/// （CSP 是按响应算的，已打开的页面要重新打开才拿得到新策略）。
#[tauri::command]
pub fn plugin_set_page_caps(id: String, denied: Vec<String>) -> Response<bool> {
    match plugin_host::set_denied_page_caps(&id, &denied) {
        Ok(()) => Response::new(200, true, "ok"),
        Err(e) => Response::new(500, false, &e),
    }
}

/// 调用插件后端逻辑（进 Extism WASM 沙箱）。
/// Extism 的调用是同步阻塞的，必须放 blocking 线程池，否则卡死 Tauri 的 async 运行时。
/// 插件未声明 wasm / 运行时未启用时返回 501 结构化错误。
#[tauri::command]
pub async fn plugin_call(id: String, func: String, input: String) -> Response<String> {
    match tokio::task::spawn_blocking(move || plugin_host::plugin_call(id, func, input)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, String::new(), &format!("插件调用线程异常: {e}")),
    }
}

/// 第三方插件 WASM 运行时信息（是否启用 / 资源限制 / 支持的权限声明形态 / 调试开关）
///
/// 会读 settings.json（拿出错快照开关），所以走 blocking 线程池而不是主线程。
#[tauri::command]
pub async fn plugin_runtime_info() -> Response<plugin_wasm::RuntimeInfo> {
    match tokio::task::spawn_blocking(plugin_host::plugin_runtime_info).await {
        Ok(r) => r,
        Err(e) => Response::new(
            500,
            plugin_wasm::runtime_info_placeholder(),
            &format!("读取运行时信息线程异常: {e}"),
        ),
    }
}

/// 某插件上一次调用的资源占用（进程内存增量 / fuel / 出错快照）
#[tauri::command]
pub fn plugin_last_stats(id: String) -> Response<Option<plugin_wasm::CallStats>> {
    plugin_host::plugin_last_stats(id)
}

/// 插件设置的当前值（已存的优先，没有则回落到清单 default）
#[tauri::command]
pub fn plugin_get_settings(id: String) -> Response<serde_json::Value> {
    Response::new(200, plugin_host::plugin_settings_of(&id), "ok")
}

/// 写入插件设置值（按清单定义校验类型与范围；只覆盖提交上来的键）
#[tauri::command]
pub fn plugin_set_settings(id: String, values: String) -> Response<bool> {
    match serde_json::from_str::<serde_json::Value>(&values) {
        Ok(v) => match plugin_host::plugin_set_settings(&id, &v) {
            Ok(()) => Response::new(200, true, "ok"),
            Err(e) => Response::new(500, false, &e),
        },
        Err(e) => Response::new(500, false, &format!("设置值不是合法 JSON: {e}")),
    }
}

/// 插件声明的 `fs` 作用域根目录（宿主文件对话框用它把用户选中的路径限制在声明范围内）
///
/// 只读查询，不进 blocking 线程池（解析清单 + 建一次能力上下文，量级很小）。
#[tauri::command]
pub fn plugin_fs_roots(id: String) -> Response<plugin_wasm::FsRoots> {
    match plugin_wasm::fs_roots(&id) {
        Ok(r) => Response::new(200, r, "ok"),
        Err(e) => Response::new(500, plugin_wasm::FsRoots::default(), &e),
    }
}

/// 设置「插件调用出错时导出线性内存快照」的调试开关（写 settings.json，需 blocking）
#[tauri::command]
pub async fn plugin_set_memdump(enabled: bool) -> Response<bool> {
    match tokio::task::spawn_blocking(move || plugin_host::plugin_set_memdump(enabled)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, false, &format!("更新出错快照开关线程异常: {e}")),
    }
}

// ==================== 本地交互式终端（terminal 插件） ====================
//
// 输出流经 Tauri v2 `Channel` 推送（IPC 层自带消息序号，前端按序投递，不会乱序/丢块）。
// 这里把 `Channel` 适配成业务模块的中性 `TermSink` 闭包，业务模块不感知任何 tauri 类型。

/// 打开一个本地 PTY 会话（跨平台 shell），输出经 `onEvent` 持续推送
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_open(
    options: terminal::TermOpenOptions,
    on_event: tauri::ipc::Channel<terminal::TermEvent>,
) -> Response<terminal::TermOpenResult> {
    let sink: terminal::TermSink = std::sync::Arc::new(move |ev| {
        if let Err(e) = on_event.send(ev) {
            log::debug!("[terminal] 推送事件失败（会话可能已关闭）: {e}");
        }
    });
    match tokio::task::spawn_blocking(move || terminal::terminal_open(options, sink)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, terminal::empty_open_result_default(), &format!("打开终端线程异常: {e}")),
    }
}

/// 向终端写入按键 / 粘贴内容
///
/// 注意 `async` 不是装饰：**Tauri 的同步命令跑在主线程**（就是 webview 的事件循环线程）上，
/// 而「写 PTY」在 shell 不读 stdin 时（正在跑长命令 / 分页器）会阻塞住，
/// 一旦阻塞主线程整个界面就无响应了（粘贴大段文本时最容易撞上）。
/// 凡是有可能阻塞的命令一律 `async` + `spawn_blocking`（与 mat_* 一致）。
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_write(session_id: String, data: String) -> Response<bool> {
    match tokio::task::spawn_blocking(move || terminal::terminal_write(session_id, data)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, false, &format!("写入终端线程异常: {e}")),
    }
}

/// 调整终端尺寸（内核据此向 shell 发 SIGWINCH / ConPTY 同步窗口大小）
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_resize(session_id: String, cols: u16, rows: u16) -> Response<bool> {
    match tokio::task::spawn_blocking(move || terminal::terminal_resize(session_id, cols, rows)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, false, &format!("调整终端尺寸线程异常: {e}")),
    }
}

/// 关闭会话（kill shell + 释放会话表 + 清理集成脚本临时目录）
///
/// 释放 PTY 时 Windows 上会走 `ClosePseudoConsole`，它会等 conhost 退出；
/// 该调用可能阻塞（见 `terminal::terminal_close` 的说明），绝不能落在主线程上。
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_close(session_id: String) -> Response<bool> {
    match tokio::task::spawn_blocking(move || terminal::terminal_close(session_id)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, false, &format!("关闭终端线程异常: {e}")),
    }
}

/// 会话是否存活（前端自检 / 崩溃恢复用）
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_alive(session_id: String) -> Response<bool> {
    match tokio::task::spawn_blocking(move || terminal::terminal_alive(session_id)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, false, &format!("查询终端状态线程异常: {e}")),
    }
}

/// 本机可用 shell 列表（「新建会话」下拉）
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_list_shells() -> Response<Vec<terminal::ShellInfo>> {
    match tokio::task::spawn_blocking(terminal::terminal_list_shells).await {
        Ok(r) => r,
        Err(e) => Response::new(500, Vec::new(), &format!("探测 shell 线程异常: {e}")),
    }
}

/// 终端默认启动信息（默认 shell / 默认 cwd / 平台）
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_defaults() -> Response<terminal::TermDefaults> {
    match tokio::task::spawn_blocking(terminal::terminal_defaults).await {
        Ok(r) => r,
        Err(e) => Response::new(500, terminal::empty_defaults(), &format!("读取终端默认值线程异常: {e}")),
    }
}
