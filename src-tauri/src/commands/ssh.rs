//! SSH / SFTP 远程连接与配置 命令（原 commands.rs 对应分区的机械拆分）。
// 分片内命令可能被 feature 整体裁掉，故 glob import 允许未使用。
#[allow(unused_imports)]
use super::*;

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
    passphrase: Option<String>,
    trust_host: Option<bool>,
    keepalive_interval: Option<u32>,
    term_type: Option<String>,
    options: Option<String>,
) -> Response<Option<String>> {
    remote_conn::ssh_connect(
        ctx_from_app(&app_handle), tab_id, host, port, username, password, auth_type,
        private_key_path, passphrase, trust_host, keepalive_interval, term_type, options,
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
    passphrase: Option<String>,
    trust_host: Option<bool>,
    options: Option<String>,
) -> Response<Option<String>> {
    remote_conn::sftp_connect(
        tab_id, host, port, username, password, auth_type, private_key_path, passphrase,
        trust_host, options,
    )
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
    resume: Option<bool>,
) -> Response<bool> {
    remote_conn::sftp_upload(
        ctx_from_app(&app_handle), session_id, local_path, remote_path, task_id, resume,
    ).await
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
    resume: Option<bool>,
) -> Response<bool> {
    remote_conn::sftp_download(
        ctx_from_app(&app_handle), session_id, remote_path, local_path, task_id, resume,
    ).await
}

/// 远端文件属性（totalSize 预取 / 权限展示 / 续传判定）
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn sftp_stat(session_id: String, path: String) -> Response<remote_conn::FileStatResp> {
    remote_conn::sftp_stat(session_id, path).await
}

/// 本地文件属性（拖放进来的路径不在列目录结果里）
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn local_stat(path: String) -> Response<remote_conn::FileStatResp> {
    remote_conn::local_stat(path).await
}

/// 修改远端权限
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn sftp_chmod(session_id: String, path: String, mode: u32) -> Response<bool> {
    remote_conn::sftp_chmod(session_id, path, mode).await
}

/// 请求取消 / 暂停一次传输
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn sftp_cancel_transfer(task_id: String) -> Response<bool> {
    remote_conn::sftp_cancel_transfer(task_id).await
}

/// 从远程服务器拉取日志文件到本地缓存，并返回本地路径
///
/// 复用现有 SFTP 能力：建立 SFTP 会话 → 下载到 config_dir/plugin_data/logs_remote → 断开。
/// 业务编排已下沉 `plugins::ssh::remote_conn::pull_remote_log`，命令层只做 Tauri → AppContext 适配。
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
    passphrase: Option<String>,
    trust_host: Option<bool>,
    options: Option<String>,
    remote_path: String,
) -> Response<remote_conn::LogsPullRemoteResp> {
    remote_conn::pull_remote_log(
        &ctx_from_app(&app_handle),
        host, port, username, password, auth_type, private_key_path, passphrase, trust_host,
        options, remote_path,
    ).await
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

// ==================== P2：exec / 端口转发 ====================

/// 执行单条远程命令（stdout / stderr 分流 + 退出码）
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_exec(
    target: String,
    command: String,
    timeout_ms: Option<u64>,
) -> Response<crate::plugins::ssh::exec::ExecResult> {
    crate::plugins::ssh::exec::ssh_exec(target, command, timeout_ms).await
}

/// 启动一条 -L / -R / -D 转发规则
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_tunnel_start(
    target: String,
    rule: String,
) -> Response<crate::plugins::ssh::tunnel::ForwardInfo> {
    crate::plugins::ssh::tunnel::ssh_tunnel_start(target, rule).await
}

/// 停止一条转发规则
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_tunnel_stop(id: String) -> Response<bool> {
    crate::plugins::ssh::tunnel::ssh_tunnel_stop(id).await
}

/// 列出运行中的转发任务（含连接数与累计流量）
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_tunnel_list() -> Response<Vec<crate::plugins::ssh::tunnel::ForwardInfo>> {
    crate::plugins::ssh::tunnel::ssh_tunnel_list().await
}

/// 停止全部转发任务（应用退出 / 切换工作区时用）
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_tunnel_stop_all() -> Response<bool> {
    crate::plugins::ssh::tunnel::ssh_tunnel_stop_all().await
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


// ==================== P3：会话保险库（凭据加密存储） ====================

/// 保险库状态：是否已创建主密码、本次进程是否解锁、元数据路径
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_vault_status() -> Response<crate::sqlite::vault::VaultStatus> {
    crate::sqlite::ssh_connection::vault_status().await
}

/// 设置主密码；migrate 默认 true，同时把已有明文凭据加密
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_vault_setup(password: String, migrate: Option<bool>) -> Response<usize> {
    crate::sqlite::ssh_connection::vault_enable(password, migrate.unwrap_or(true)).await
}

/// 解锁（密钥只存在进程内存，重启后需重新输入）
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_vault_unlock(password: String) -> Response<bool> {
    crate::sqlite::ssh_connection::vault_unlock(password).await
}

/// 立即锁定：读出凭据为空，编辑会话被 423 拦住
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_vault_lock() -> Response<bool> {
    crate::sqlite::ssh_connection::vault_lock().await
}

/// 补做迁移：把库里剩余的明文凭据加密
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_vault_encrypt_existing() -> Response<usize> {
    crate::sqlite::ssh_connection::vault_encrypt_existing().await
}

/// 取消保险库：先验主密码，再把密文还原为明文，最后删元数据
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_vault_disable(password: String) -> Response<usize> {
    crate::sqlite::ssh_connection::vault_disable(password).await
}

// ==================== P3：会话日志录制 ====================

/// 追加一批已加时间戳的日志行
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_log_append(name: String, text: String) -> Response<bool> {
    crate::plugins::ssh::session_log::ssh_log_append(name, text).await
}

/// 列出日志文件
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_log_list() -> Response<Vec<crate::plugins::ssh::session_log::LogFileInfo>> {
    crate::plugins::ssh::session_log::ssh_log_list().await
}

/// 读取日志尾部（默认 256KB）
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_log_read(name: String, max_bytes: Option<u64>) -> Response<String> {
    crate::plugins::ssh::session_log::ssh_log_read(name, max_bytes).await
}

/// 删除一个日志文件
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_log_delete(name: String) -> Response<bool> {
    crate::plugins::ssh::session_log::ssh_log_delete(name).await
}

/// 日志目录绝对路径（便于在系统文件管理器中打开）
#[cfg(feature = "plugin-ssh")]
#[tauri::command]
pub async fn ssh_log_dir() -> Response<String> {
    crate::plugins::ssh::session_log::ssh_log_dir().await
}
