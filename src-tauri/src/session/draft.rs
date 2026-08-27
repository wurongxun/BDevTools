/*! # 草稿管理
为脏文档的未保存内容提供工作区级草稿存储（路线1：内容级保护）。
目录结构：
  {app_dir}/drafts/{workspace_id}/{md5(file_path)}.draft   草稿内容
  {app_dir}/drafts/{workspace_id}/{md5(file_path)}.meta.json  指纹元数据

铁律：草稿只还原到编辑器内存缓冲区，永不自动写入目标文件磁盘。
*/
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::response::response::Response;

/// 草稿元数据（保存时记录目标文件指纹，恢复时用于检测外部修改）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DraftMeta {
    pub path: String,
    pub saved_at: i64,
    pub file_mtime: i64,
    pub file_size: u64,
    pub file_exists: bool,
}

/// 草稿检查结果（恢复前调用）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DraftCheckResult {
    pub has_draft: bool,
    /// 目标文件自草稿保存后是否被外部修改（无草稿时恒为 false）
    pub file_changed: bool,
    pub draft_saved_at: i64,
    pub file_mtime: i64,
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 读取文件指纹（mtime 秒 + size）；不存在或不可读返回 (0, 0, false)
fn file_fingerprint(path: &str) -> (i64, u64, bool) {
    match fs::metadata(path) {
        Ok(meta) => {
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            (mtime, meta.len(), true)
        }
        Err(_) => (0, 0, false),
    }
}

fn drafts_root() -> PathBuf {
    PathBuf::from(&crate::app::app_store::APP.lock().app_dir).join("drafts")
}

fn workspace_draft_dir(workspace_id: &str) -> PathBuf {
    drafts_root().join(workspace_id)
}

/// 文件路径 → 安全的草稿文件名（md5 hex）
fn hash_name(file_path: &str) -> String {
    format!("{:x}", md5::compute(file_path.as_bytes()))
}

fn draft_file_paths(workspace_id: &str, file_path: &str) -> (PathBuf, PathBuf) {
    let dir = workspace_draft_dir(workspace_id);
    let name = hash_name(file_path);
    (dir.join(format!("{}.draft", name)), dir.join(format!("{}.meta.json", name)))
}

fn read_meta(meta_path: &PathBuf) -> Option<DraftMeta> {
    let content = fs::read_to_string(meta_path).ok()?;
    serde_json::from_str(&content).ok()
}

// ==================== Tauri 命令 ====================

/// 保存草稿（同时记录目标文件当前指纹）
pub fn draft_save(
    workspace_id: String,
    file_path: String,
    content: String,
) -> Response<DraftMeta> {
    let dir = workspace_draft_dir(&workspace_id);
    if !dir.exists() {
        if let Err(e) = fs::create_dir_all(&dir) {
            return Response::new(500, DraftMeta {
                path: file_path, saved_at: 0, file_mtime: 0, file_size: 0, file_exists: false,
            }, &format!("创建草稿目录失败: {}", e));
        }
    }
    let (draft_path, meta_path) = draft_file_paths(&workspace_id, &file_path);

    // 未命名文档（untitled:xxx）没有目标文件，指纹置空
    let (mtime, size, exists) = if file_path.starts_with("untitled:") {
        (0, 0, false)
    } else {
        file_fingerprint(&file_path)
    };

    let meta = DraftMeta {
        path: file_path.clone(),
        saved_at: now_millis(),
        file_mtime: mtime,
        file_size: size,
        file_exists: exists,
    };

    if let Err(e) = fs::write(&draft_path, &content) {
        return Response::new(500, meta, &format!("写入草稿失败: {}", e));
    }
    if let Ok(json) = serde_json::to_string(&meta) {
        let _ = fs::write(&meta_path, json);
    }
    Response::new(200, meta, "草稿保存成功")
}

/// 检查草稿状态：是否存在草稿、目标文件是否被外部修改
pub fn draft_check(workspace_id: String, file_path: String) -> Response<DraftCheckResult> {
    let (draft_path, meta_path) = draft_file_paths(&workspace_id, &file_path);
    if !draft_path.exists() {
        return Response::new(200, DraftCheckResult {
            has_draft: false, file_changed: false, draft_saved_at: 0, file_mtime: 0,
        }, "无草稿");
    }

    let meta = read_meta(&meta_path);
    let (draft_saved_at, saved_mtime, saved_size, file_existed) = match &meta {
        Some(m) => (m.saved_at, m.file_mtime, m.file_size, m.file_exists),
        None => (0, 0, 0, false),
    };

    let (current_mtime, current_size, exists) = if file_path.starts_with("untitled:") {
        (0, 0, false)
    } else {
        file_fingerprint(&file_path)
    };

    // 外部修改判定：
    // - 保存时文件存在且现在指纹（mtime+size）变化 → 被外部修改
    // - 保存时文件存在但现已删除 → 视为变化（需用户决策）
    // - 保存时文件不存在且现在仍不存在（如 untitled）→ 未变化
    // - 保存时文件不存在但现在已经创建 → 视为变化
    let file_changed = if file_existed {
        current_mtime != saved_mtime || current_size != saved_size || !exists
    } else {
        exists
    };

    Response::new(200, DraftCheckResult {
        has_draft: true,
        file_changed,
        draft_saved_at,
        file_mtime: current_mtime,
    }, "ok")
}

/// 读取草稿内容
pub fn draft_load(workspace_id: String, file_path: String) -> Response<Option<String>> {
    let (draft_path, _) = draft_file_paths(&workspace_id, &file_path);
    let content = fs::read_to_string(&draft_path).ok();
    Response::new(200, content, "ok")
}

/// 删除指定文件的草稿（含元数据）
pub fn draft_delete(workspace_id: String, file_path: String) -> Response<bool> {
    let (draft_path, meta_path) = draft_file_paths(&workspace_id, &file_path);
    let _ = fs::remove_file(&draft_path);
    let _ = fs::remove_file(&meta_path);
    Response::new(200, true, "草稿已删除")
}

/// 清除工作区的全部草稿（删除工作区时调用）
pub fn clear_workspace_drafts(workspace_id: &str) {
    let dir = workspace_draft_dir(workspace_id);
    if dir.exists() {
        if let Err(e) = fs::remove_dir_all(&dir) {
            println!("[Draft] 清理工作区 {} 草稿失败: {}", workspace_id, e);
        } else {
            println!("[Draft] 已清理工作区 {} 草稿", workspace_id);
        }
    }
}
