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

/// 保存笔记。
/// 新增的 5 个 Option 全是可选的：草稿备份 / 数据库脚本等旧调用点不传时行为不变。
/// `expected_mtime_ms` 与 `expected_size` 由前端从 `read_file_meta` 带回，用于阻止并发覆盖。
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn save_note(
    path: String,
    md_text: String,
    encoding: Option<String>,
    eol: Option<String>,
    bom: Option<String>,
    expected_mtime_ms: Option<i64>,
    expected_size: Option<u64>,
) -> Result<ResData<String>, String> {
    file_meta::save_note(
        path,
        md_text,
        file_meta::SaveMeta {
            encoding,
            eol,
            bom,
            expected_mtime_ms,
            expected_size,
        },
    )
    .await
}

/// 读取文件元数据（编码 / EOL / BOM / mtime / size），供编辑器按原值回写。
/// 失败时 `data: null`，前端回退 UTF-8 + LF，不阻断打开。
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn read_file_meta(path: String) -> Result<ResData<Option<FileMeta>>, String> {
    Ok(match file_meta::read_file_meta(&path).await {
        Ok(m) => ResData {
            code: 200,
            msg: "success".to_string(),
            data: Some(m),
        },
        Err(e) => ResData {
            code: 500,
            msg: e,
            data: None,
        },
    })
}

/// 移动文件/文件夹（文件树拖拽用）。目标已存在或把目录拖进自己子目录时拒绝，避免覆盖。
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn move_file(old_path: String, new_path: String) -> Result<ResData<String>, String> {
    file_command::move_file(old_path, new_path).await
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

// ==================== 跨文件搜索与批量替换（files 自建，不依赖 plugin-logs） ====================
// TextView 的单文件查询也走这里，避免 files 前端隐式耦合 `plugin-logs` feature。
// 所有命令都只做参数归一化 + 转发，业务逻辑全在 `plugins/files/file_search.rs`。

/// 跨目录搜索：glob 过滤、可取消、有上限；结果带编码/EOL/mtime，直接作为替换基线。
#[cfg(feature = "plugin-files")]
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn files_search_in_files(
    root_path: String,
    keyword: String,
    mode: Option<String>,
    ignore_case: Option<bool>,
    include: Option<String>,
    exclude: Option<String>,
    max_file_size: Option<u64>,
    max_files: Option<usize>,
    max_matches: Option<usize>,
    task_id: Option<String>,
) -> Result<ResData<file_search::SearchOutcome>, String> {
    file_search::search_in_files(file_search::SearchRequest {
        root_path,
        keyword,
        mode: mode.unwrap_or_else(|| "text".to_string()),
        ignore_case: ignore_case.unwrap_or(true),
        include: include.unwrap_or_default(),
        exclude: exclude.unwrap_or_default(),
        max_file_size: max_file_size.unwrap_or(DEFAULT_MAX_FILE_SIZE),
        max_files: max_files.unwrap_or(DEFAULT_MAX_FILES),
        max_matches: max_matches.unwrap_or(DEFAULT_MAX_MATCHES),
        task_id,
    })
    .await
}

/// 单文件按行查询。返回 `lines / matched / matches_truncated`，与旧 logs 契约同名。
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn files_query_lines(
    file_path: String,
    keyword: String,
    mode: Option<String>,
    ignore_case: Option<bool>,
    max_matches: Option<usize>,
    task_id: Option<String>,
) -> Result<ResData<file_search::LineQueryOutcome>, String> {
    file_search::query_lines(file_search::LineQueryRequest {
        file_path,
        keyword,
        mode: mode.unwrap_or_else(|| "text".to_string()),
        ignore_case: ignore_case.unwrap_or(true),
        max_matches: max_matches.unwrap_or(DEFAULT_MAX_MATCHES),
        task_id,
    })
    .await
}

/// 批量替换。`dry_run` 缺省为 true（不落盘）；应用时逐文件备份 + 原子写 + 409 检测。
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn files_replace_in_files(
    keyword: String,
    replacement: String,
    mode: Option<String>,
    ignore_case: Option<bool>,
    targets: Vec<ReplaceTarget>,
    dry_run: Option<bool>,
    task_id: Option<String>,
) -> Result<ResData<file_search::ReplaceOutcome>, String> {
    file_search::replace_in_files(file_search::ReplaceRequest {
        keyword,
        replacement,
        mode: mode.unwrap_or_else(|| "text".to_string()),
        ignore_case: ignore_case.unwrap_or(true),
        targets,
        dry_run: dry_run.unwrap_or(true),
        task_id,
    })
    .await
}

/// 撤销批量替换：从 `.bak` 原样恢复字节。
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn files_replace_undo(
    entries: Vec<UndoEntry>,
) -> Result<ResData<file_search::UndoOutcome>, String> {
    file_search::undo_replace(entries).await
}

/// 取消正在进行的搜索/替换任务；前端凭 task_id 静默丢弃过期结果。
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub fn files_search_cancel(task_id: String) -> Result<ResData<bool>, String> {
    Ok(ResData {
        code: 200,
        msg: "已请求取消".to_string(),
        data: file_search::cancel_task(task_id),
    })
}

/// 编码 / EOL / BOM 显式转换（M4 / E4+E5 遗留）。
/// 只有用户在对话框里确认后才调用：先整份备份、再走唯一写回通道，
/// 415（目标编码无法表示）/ 409（外部已改）都保证原文件不变且不留孤儿备份。
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn files_convert_encoding_eol(
    path: String,
    target_encoding: String,
    target_eol: String,
    target_bom: String,
    expected_mtime_ms: Option<i64>,
    expected_size: Option<u64>,
) -> Result<ResData<file_convert::ConvertOutcome>, String> {
    file_convert::convert_encoding_eol(file_convert::ConvertRequest {
        path,
        target_encoding,
        target_eol,
        target_bom,
        expected_mtime_ms,
        expected_size,
    })
    .await
}

/// 撤销清单探测：重启后只把备份仍存在的条目恢复成「可撤销」，避免假可用状态。
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn files_undo_probe(
    entries: Vec<UndoEntry>,
) -> Result<ResData<Vec<UndoEntry>>, String> {
    Ok(ResData {
        code: 200,
        msg: "success".to_string(),
        data: file_search::probe_undo_entries(entries).await,
    })
}

/// 主动丢弃撤销清单（用户点「不再撤销」）：把对应 `.bak` 一起删掉，不让磁盘堆垃圾。
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn files_undo_discard(
    entries: Vec<UndoEntry>,
) -> Result<ResData<Vec<String>>, String> {
    Ok(ResData {
        code: 200,
        msg: "success".to_string(),
        data: file_search::discard_undo_entries(entries).await,
    })
}

/// 大文件（>1MB）行级补丁保存（M4 / E7）。
/// 前端只提交脏行区间，宿主读一次原文件、应用补丁后仍走 `file_meta::save_note`：
/// 编码 / EOL / BOM / 原子写 / 409 / 415 的保护与普通文件保存完全一致。
#[cfg(feature = "plugin-files")]
#[tauri::command]
pub async fn files_apply_line_edits(
    path: String,
    edits: Vec<LineEdit>,
    encoding: Option<String>,
    eol: Option<String>,
    bom: Option<String>,
    expected_mtime_ms: Option<i64>,
    expected_size: Option<u64>,
) -> Result<ResData<file_large::LineEditOutcome>, String> {
    file_large::apply_line_edits_to_file(
        path,
        edits,
        file_meta::SaveMeta {
            encoding,
            eol,
            bom,
            expected_mtime_ms,
            expected_size,
        },
    )
    .await
}
