//! 文件管理 / 大文件按行读取 / 文件关联 命令（原 commands.rs 对应分区的机械拆分）。
// 分片内命令可能被 feature 整体裁掉，故 glob import 允许未使用。
#[allow(unused_imports)]
use super::*;

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
    file_command::read_file_bytes(path)
        .await
        .map(tauri::ipc::Response::new)
}

#[cfg(all(test, feature = "plugin-files"))]
mod read_file_bytes_channel_tests {
    use tauri::ipc::{InvokeResponseBody, IpcResponse};

    /// 命令层必须把 Vec<u8> 包成裸字节通道；退化成 Json 会让前端收到数字数组。
    #[tokio::test]
    async fn uses_raw_ipc_body() {
        let dir = std::env::temp_dir().join("bdevtools_ipc_bytes");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.bin");
        std::fs::write(&path, b"probe-bytes").unwrap();

        let resp = super::read_file_bytes(path.to_string_lossy().to_string())
            .await
            .unwrap();
        match resp.body().unwrap() {
            InvokeResponseBody::Raw(v) => assert_eq!(v, b"probe-bytes".to_vec()),
            InvokeResponseBody::Json(s) => panic!("退化成 JSON 了: {}", s),
        }
    }
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
