//! # Tauri 命令统一门面
//!
//! 本文件是唯一的 `#[tauri::command]` 定义处（RPC 边界），
//! 也是 **唯一** 持有 `tauri` / `tauri_plugin_*` 依赖的业务代码。
//! 所有命令均为薄包装：仅做参数转发，业务逻辑保留在各模块中。
//! 将来更换 UI 框架时，只需重写本文件（及 lib.rs 的组装逻辑）。
//!
//! 命令名即暴露给前端的 invoke 名称，改名需同步前端。

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::{NotificationExt, PermissionState};
use tauri_plugin_opener::OpenerExt;

use crate::core::ctx::AppContext;
use crate::command::app_window;
use crate::command::file_association;
use crate::command::file_command::{self, ScriptFileEntry};
use crate::command::file_utils;
use crate::command::log_index::{
    self, LogExportResult, LogIndexMeta, LogQueryResult,
};
use crate::command::log_stream;
use crate::command::mat_hprof::{self, MatHeaderInfo, MatHistogramResult};
use crate::command::mat_leak::{self, MatLeakSuspectsResult};
use crate::command::mat_cfg::{self, MatLimitInfo};
use crate::command::mat_refs::{
    self, RefBuildInfo, RefInstances, RefObjectDetail, RefSearchResult,
};
use crate::command::remote_conn::{self, RemoteFileItem};
use crate::command::search_log_parallel::{
    self, FileLinesResult, LogMatch, LogSearchResult, PageResult, SearchIndex, TextPreviewBlock,
};
use crate::database::connection::DbConnectionConfig;
use crate::database::import_export::{ExportResult, ImportResult};
use crate::database::metadata::{DatabaseInfo, IndexInfo, TableInfo};
use crate::database::operations::{ColumnInfo, SqlResult, TableDataResult};
use crate::entity::session::{SessionSnapshot, SessionSummary};
use crate::entity::window::WindowMeta;
use crate::entity::workspace::{WorkspaceConfig, WorkspaceSummary};
use crate::response::resp_data::ResData;
use crate::response::response::Response;
use crate::session::draft::{DraftCheckResult, DraftMeta};
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

/// 打开目录（项目）
#[tauri::command]
pub async fn file_manager_open_path(app_handle: tauri::AppHandle, path: String) -> ResData<String> {
    file_utils::file_manager_open_path(&ctx_from_app(&app_handle), path).await
}

/// 获取文档笔记本（目录树）
#[tauri::command]
pub async fn get_document_notebooks(
    app_handle: tauri::AppHandle,
    working_directory: Option<String>,
) -> ResData<String> {
    file_utils::get_document_notebooks(&ctx_from_app(&app_handle), working_directory).await
}

/// 更新文件夹名称
#[tauri::command]
pub async fn up_folder_name(
    path: String,
    up_folder_name: String,
) -> Result<ResData<String>, String> {
    file_utils::up_folder_name(path, up_folder_name).await
}

/// 检测文件编码
#[tauri::command]
pub async fn detect_file_encoding(path: String) -> Result<ResData<String>, String> {
    file_utils::detect_file_encoding(path).await
}

/// 确保目录存在（递归创建）
#[tauri::command]
pub async fn ensure_dir(path: String) -> Result<ResData<String>, String> {
    file_command::ensure_dir(path).await
}

/// 获取目录下笔记
#[tauri::command]
pub async fn get_note_list(path: String) -> Result<ResData<Vec<String>>, String> {
    file_command::get_note_list(path).await
}

/// 读取本地笔记文件（自动检测编码）
#[tauri::command]
pub async fn read_note_file(path: String) -> Result<ResData<String>, String> {
    file_command::read_note_file(path).await
}

/// 获取文件大小（字节），供大文件分诊
#[tauri::command]
pub async fn get_file_size(path: String) -> Result<ResData<u64>, String> {
    file_command::get_file_size(path).await
}

/// 创建本地文件夹
#[tauri::command]
pub async fn create_folder(
    father_path: String,
    new_folder_name: String,
) -> Result<ResData<String>, String> {
    file_command::create_folder(father_path, new_folder_name).await
}

/// 创建笔记文件
#[tauri::command]
pub async fn create_note_file(
    father_path: String,
    new_note_md_file_name: String,
) -> Result<ResData<String>, String> {
    file_command::create_note_file(father_path, new_note_md_file_name).await
}

/// 保存笔记
#[tauri::command]
pub async fn save_note(path: String, md_text: String) -> Result<ResData<String>, String> {
    file_command::save_note(path, md_text).await
}

/// 更新文件名
#[tauri::command]
pub async fn up_file_name(
    path: String,
    up_file_name: String,
) -> Result<ResData<String>, String> {
    file_command::up_file_name(path, up_file_name).await
}

/// 删除本地文件夹
#[tauri::command]
pub async fn remove_folder(path: String) -> Result<ResData<String>, String> {
    file_command::remove_folder(path).await
}

/// 删除本地文件
#[tauri::command]
pub async fn remove_file(path: String) -> Result<ResData<String>, String> {
    file_command::remove_file(path).await
}

/// 获取目录下所有文件及文件夹名称
#[tauri::command]
pub async fn get_path_childrenFoldersAndFiles(
    path: String,
) -> Result<ResData<Vec<String>>, String> {
    file_command::get_path_childrenFoldersAndFiles(path).await
}

/// 写入脚本文件（不存在则创建，存在则覆盖）
#[tauri::command]
pub async fn write_script_file(path: String, content: String) -> Result<ResData<String>, String> {
    file_command::write_script_file(path, content).await
}

/// 重命名脚本文件（保留原扩展名）
#[tauri::command]
pub async fn rename_script_file(
    path: String,
    new_name: String,
) -> Result<ResData<String>, String> {
    file_command::rename_script_file(path, new_name).await
}

/// 递归列出目录下所有文件与文件夹
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

// ==================== 大日志检索 ====================

/// 流式搜索（只存行号，低内存）
#[tauri::command]
pub async fn search_log_parallel(
    file_path: String,
    keyword: String,
    max_results: Option<u64>,
) -> Result<ResData<LogSearchResult>, String> {
    search_log_parallel::search_log_parallel(file_path, keyword, max_results).await
}

/// 建立搜索索引（单线程流式，低内存）
#[tauri::command]
pub async fn search_log_index(
    file_path: String,
    keyword: String,
    encoding: Option<String>,
) -> Result<ResData<SearchIndex>, String> {
    search_log_parallel::search_log_index(file_path, keyword, encoding).await
}

/// 分页获取搜索结果
#[tauri::command]
pub async fn search_log_page(
    search_id: String,
    page: u64,
    page_size: Option<u64>,
) -> Result<ResData<PageResult>, String> {
    search_log_parallel::search_log_page(search_id, page, page_size).await
}

/// 跳转到第 N 条搜索结果
#[tauri::command]
pub async fn search_log_jump_to(
    search_id: String,
    target_index: u64,
    context_count: Option<u64>,
) -> Result<ResData<Vec<LogMatch>>, String> {
    search_log_parallel::search_log_jump_to(search_id, target_index, context_count).await
}

/// 文本预览（流式读取，低内存）
#[tauri::command]
pub async fn search_log_preview(
    search_id: String,
    target_index: u64,
    context_lines: Option<u64>,
    encoding: Option<String>,
) -> Result<ResData<TextPreviewBlock>, String> {
    search_log_parallel::search_log_preview(search_id, target_index, context_lines, encoding).await
}

/// 按行范围读取文件内容
#[tauri::command]
pub async fn read_file_lines(
    file_path: String,
    start_line: Option<u64>,
    line_count: Option<u64>,
    encoding: Option<String>,
) -> Result<ResData<FileLinesResult>, String> {
    search_log_parallel::read_file_lines(file_path, start_line, line_count, encoding).await
}

/// 快速统计文件总行数
#[tauri::command]
pub async fn count_file_lines(file_path: String) -> Result<ResData<u64>, String> {
    search_log_parallel::count_file_lines(file_path).await
}

// ==================== 日志工作台：服务端索引与筛选（logs 插件） ====================

/// 建立日志索引（级别计数 / 时间范围 / 时间桶）
#[tauri::command]
pub async fn logs_index_build(
    file_path: String,
    encoding: Option<String>,
) -> Result<ResData<LogIndexMeta>, String> {
    log_index::log_index_build(file_path, encoding).await
}

/// 按级别与时间范围筛选，返回命中行的连续段
#[tauri::command]
pub async fn logs_index_query(
    file_path: String,
    encoding: Option<String>,
    levels: Option<Vec<String>>,
    ts_from: Option<i64>,
    ts_to: Option<i64>,
    invert: Option<bool>,
    merge_gap: Option<u64>,
    max_segments: Option<u64>,
) -> Result<ResData<LogQueryResult>, String> {
    log_index::log_index_query(
        file_path,
        encoding,
        levels,
        ts_from,
        ts_to,
        invert,
        merge_gap,
        max_segments,
    )
    .await
}

/// 按筛选条件流式导出日志到目标文件
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
#[tauri::command]
pub async fn logs_index_drop(file_path: String) -> Result<ResData<bool>, String> {
    log_index::log_index_drop(file_path).await
}

// ==================== Java 堆转储分析（mat 插件） ====================

/// 读取堆转储文件头：格式识别（HPROF / PHD / 未知）+ 元信息
#[tauri::command]
pub async fn mat_hprof_header(path: String) -> Result<ResData<MatHeaderInfo>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_hprof::mat_hprof_header(&p))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 生成按类聚合的浅堆直方图（Shallow Heap）
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
#[tauri::command]
pub async fn mat_analysis_limits() -> Result<ResData<MatLimitInfo>, String> {
    tokio::task::spawn_blocking(mat_cfg::mat_analysis_limits)
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// Leak Suspects：支配树 + Retained Heap 的泄漏嫌疑分析（对象多时耗时数十秒级）
#[tauri::command]
pub async fn mat_leak_suspects(path: String) -> Result<ResData<MatLeakSuspectsResult>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_leak::mat_leak_suspects(&p))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：构建（或复用）对象图会话，返回图规模与可达性汇总
#[tauri::command]
pub async fn mat_refs_build(path: String) -> Result<ResData<RefBuildInfo>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_refs::mat_refs_build(&p))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：释放某 dump 的会话缓存（关闭 dump 时调用）
#[tauri::command]
pub async fn mat_refs_release(path: String) -> Result<ResData<bool>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_refs::mat_refs_release(&p))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：类名搜索（大小写不敏感子串，按浅堆合计降序）
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
#[tauri::command]
pub async fn register_file_association() -> ResData<String> {
    file_association::register_file_association().await
}

/// 打开系统默认应用设置
#[tauri::command]
pub async fn open_default_apps_settings() -> ResData<String> {
    file_association::open_default_apps_settings().await
}

// ==================== SSH / SFTP 远程连接 ====================

/// SSH 连接
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
#[tauri::command]
pub async fn ssh_write(session_id: String, data: String) -> Response<bool> {
    remote_conn::ssh_write(session_id, data).await
}

/// SSH 断开连接
#[tauri::command]
pub async fn ssh_disconnect(session_id: String) -> Response<bool> {
    remote_conn::ssh_disconnect(session_id).await
}

/// SSH 连接状态检测
#[tauri::command]
pub async fn ssh_check(session_id: String) -> Response<bool> {
    remote_conn::ssh_check(session_id).await
}

/// SSH 调整 PTY 大小
#[tauri::command]
pub async fn ssh_resize_pty(session_id: String, cols: u32, rows: u32) -> Response<bool> {
    remote_conn::ssh_resize_pty(session_id, cols, rows).await
}

/// SFTP 连接
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
#[tauri::command]
pub async fn sftp_disconnect(session_id: String) -> Response<bool> {
    remote_conn::sftp_disconnect(session_id).await
}

/// SFTP 连接状态检测
#[tauri::command]
pub async fn sftp_check(session_id: String) -> Response<bool> {
    remote_conn::sftp_check(session_id).await
}

/// SFTP 列出目录
#[tauri::command]
pub async fn sftp_list_dir(session_id: String, path: String) -> Response<Vec<RemoteFileItem>> {
    remote_conn::sftp_list_dir(session_id, path).await
}

/// SFTP 创建目录
#[tauri::command]
pub async fn sftp_mkdir(session_id: String, path: String) -> Response<bool> {
    remote_conn::sftp_mkdir(session_id, path).await
}

/// SFTP 删除文件/目录
#[tauri::command]
pub async fn sftp_remove(session_id: String, path: String) -> Response<bool> {
    remote_conn::sftp_remove(session_id, path).await
}

/// SFTP 上传文件
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
#[tauri::command]
pub async fn local_list_dir(path: String) -> Response<Vec<RemoteFileItem>> {
    remote_conn::local_list_dir(path).await
}

/// SFTP 重命名文件/目录
#[tauri::command]
pub async fn sftp_rename(session_id: String, old_path: String, new_path: String) -> Response<bool> {
    remote_conn::sftp_rename(session_id, old_path, new_path).await
}

/// 获取应用数据目录路径
#[tauri::command]
pub async fn get_app_dir() -> Response<String> {
    remote_conn::get_app_dir().await
}

// ==================== SSH 连接配置（SQLite） ====================

/// 创建 SSH 连接配置
#[tauri::command]
pub async fn connection_create(map: SshSubMenu) -> Response<Option<bool>> {
    crate::sqlite::ssh_connection::connection_create(map).await
}

/// 删除 SSH 连接配置
#[tauri::command]
pub async fn connection_delete(id: String) -> Response<bool> {
    crate::sqlite::ssh_connection::connection_delete(id).await
}

/// 编辑 SSH 连接配置
#[tauri::command]
pub async fn connection_edit(map: SshSubMenu) -> Response<bool> {
    crate::sqlite::ssh_connection::connection_edit(map).await
}

/// 查询 SSH 连接配置列表
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
#[tauri::command]
pub async fn db_connection_create(config: DbConnectionConfig) -> Response<Option<String>> {
    crate::database::connection::db_connection_create(config).await
}

/// 删除数据库连接配置
#[tauri::command]
pub async fn db_connection_delete(id: String) -> Response<bool> {
    crate::database::connection::db_connection_delete(id).await
}

/// 更新数据库连接配置
#[tauri::command]
pub async fn db_connection_update(config: DbConnectionConfig) -> Response<bool> {
    crate::database::connection::db_connection_update(config).await
}

/// 获取所有连接配置列表
#[tauri::command]
pub async fn db_connection_list() -> Response<Vec<DbConnectionConfig>> {
    crate::database::connection::db_connection_list().await
}

/// 测试数据库连接连通性
#[tauri::command]
pub async fn db_connection_test(config: DbConnectionConfig) -> Response<bool> {
    crate::database::connection::db_connection_test(config).await
}

/// 建立数据库连接（创建连接池）
#[tauri::command]
pub async fn db_connect(config_id: String) -> Response<bool> {
    crate::database::connection::db_connect(config_id).await
}

/// 断开数据库连接
#[tauri::command]
pub async fn db_disconnect(config_id: String) -> Response<bool> {
    crate::database::connection::db_disconnect(config_id).await
}

// ==================== 数据库操作 ====================

/// 执行 SQL 语句（分页）
#[tauri::command]
pub async fn db_execute_sql(
    config_id: String,
    sql: String,
    database: Option<String>,
    page_size: Option<u64>,
    offset: Option<u64>,
) -> Response<SqlResult> {
    crate::database::operations::db_execute_sql(config_id, sql, database, page_size, offset).await
}

/// 分页查询表数据
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
    crate::database::operations::db_query_table_data(
        config_id, database, table, page, page_size, where_clause, order_by,
    )
    .await
}

/// 插入单行数据
#[tauri::command]
pub async fn db_insert_row(
    config_id: String,
    database: String,
    table: String,
    data: serde_json::Value,
) -> Response<bool> {
    crate::database::operations::db_insert_row(config_id, database, table, data).await
}

/// 更新单行数据
#[tauri::command]
pub async fn db_update_row(
    config_id: String,
    database: String,
    table: String,
    primary_key: String,
    primary_value: serde_json::Value,
    data: serde_json::Value,
) -> Response<bool> {
    crate::database::operations::db_update_row(
        config_id, database, table, primary_key, primary_value, data,
    )
    .await
}

/// 删除单行数据
#[tauri::command]
pub async fn db_delete_row(
    config_id: String,
    database: String,
    table: String,
    primary_key: String,
    primary_value: serde_json::Value,
) -> Response<bool> {
    crate::database::operations::db_delete_row(config_id, database, table, primary_key, primary_value)
        .await
}

// ==================== 数据库元数据 ====================

/// 获取所有数据库列表
#[tauri::command]
pub async fn db_list_databases(config_id: String) -> Response<Vec<DatabaseInfo>> {
    crate::database::metadata::db_list_databases(config_id).await
}

/// 获取指定数据库的所有表
#[tauri::command]
pub async fn db_list_tables(config_id: String, database: String) -> Response<Vec<TableInfo>> {
    crate::database::metadata::db_list_tables(config_id, database).await
}

/// 获取表的字段信息
#[tauri::command]
pub async fn db_table_columns(
    config_id: String,
    database: String,
    table: String,
) -> Response<Vec<ColumnInfo>> {
    crate::database::metadata::db_table_columns(config_id, database, table).await
}

/// 获取表的索引信息
#[tauri::command]
pub async fn db_table_indexes(
    config_id: String,
    database: String,
    table: String,
) -> Response<Vec<IndexInfo>> {
    crate::database::metadata::db_table_indexes(config_id, database, table).await
}

/// 获取建表 DDL
#[tauri::command]
pub async fn db_table_schema(
    config_id: String,
    database: String,
    table: String,
) -> Response<String> {
    crate::database::metadata::db_table_schema(config_id, database, table).await
}

// ==================== 数据库导入导出 ====================

/// 导出表数据为 CSV
#[tauri::command]
pub async fn db_export_csv(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
) -> Response<ExportResult> {
    crate::database::import_export::db_export_csv(config_id, database, table, file_path).await
}

/// 导出表数据为 Excel
#[tauri::command]
pub async fn db_export_excel(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
) -> Response<ExportResult> {
    crate::database::import_export::db_export_excel(config_id, database, table, file_path).await
}

/// 按任意 SQL 语句导出结果为 CSV
#[tauri::command]
pub async fn db_export_sql_csv(
    config_id: String,
    sql: String,
    file_path: String,
) -> Response<ExportResult> {
    crate::database::import_export::db_export_sql_csv(config_id, sql, file_path).await
}

/// 按任意 SQL 语句导出结果为 Excel
#[tauri::command]
pub async fn db_export_sql_excel(
    config_id: String,
    sql: String,
    file_path: String,
) -> Response<ExportResult> {
    crate::database::import_export::db_export_sql_excel(config_id, sql, file_path).await
}

/// 从 CSV 导入数据到表
#[tauri::command]
pub async fn db_import_csv(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
    has_header: bool,
) -> Response<ImportResult> {
    crate::database::import_export::db_import_csv(config_id, database, table, file_path, has_header)
        .await
}

/// 从 Excel 导入数据到表
#[tauri::command]
pub async fn db_import_excel(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
) -> Response<ImportResult> {
    crate::database::import_export::db_import_excel(config_id, database, table, file_path).await
}

// ==================== 实时流日志 ====================

/// 启动实时日志流。
/// - WebSocket：由前端原生 `WebSocket` 直连，后端不参与（此处返回提示）。
/// - TCP：后端建立监听，按行推送 `logs-stream-line` 事件。
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
#[tauri::command]
pub async fn logs_stream_stop(stream_id: String) -> Response<bool> {
    log_stream::log_stream_stop(stream_id).await
}
