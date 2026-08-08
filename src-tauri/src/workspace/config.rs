/*! # 工作区配置管理
工作区配置以 JSON 文件形式存储在 {app_dir}/workspaces/ 目录下。
每个工作区一个文件：{workspace_id}.workspace.json
*/
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use chrono::Utc;
use uuid::Uuid;

use crate::entity::window::{WorkspaceConfig, WorkspaceSummary};

/// 获取工作区存储目录路径
fn get_workspaces_dir() -> PathBuf {
    let app_dir = PathBuf::from(&crate::app::app_store::APP.lock().app_dir);
    app_dir.join("workspaces")
}

/// 确保工作区目录存在
pub fn ensure_workspaces_dir() {
    let dir = get_workspaces_dir();
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
}

/// 获取工作区文件路径
fn get_workspace_file(id: &str) -> PathBuf {
    get_workspaces_dir().join(format!("{}.workspace.json", id))
}

/// 列出所有工作区摘要
pub fn list_workspaces() -> Vec<WorkspaceSummary> {
    let dir = get_workspaces_dir();
    if !dir.exists() {
        return Vec::new();
    }

    let mut summaries = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json")
                && path.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.ends_with(".workspace.json"))
            {
                match read_workspace_file(&path) {
                    Ok(config) => {
                        summaries.push(WorkspaceSummary {
                            id: config.id,
                            name: config.name,
                            folders: config.folders,
                            updated_at: config.updated_at,
                        });
                    }
                    Err(_) => continue,
                }
            }
        }
    }

    // 按更新时间降序排列
    summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    summaries
}

/// 获取指定工作区完整配置
pub fn get_workspace(id: &str) -> Option<WorkspaceConfig> {
    let path = get_workspace_file(id);
    read_workspace_file(&path).ok()
}

/// 创建工作区
pub fn create_workspace(name: String, folders: Vec<String>) -> WorkspaceConfig {
    let now = Utc::now().timestamp_millis();
    let config = WorkspaceConfig {
        id: Uuid::new_v4().to_string(),
        name,
        folders,
        settings: HashMap::new(),
        created_at: now,
        updated_at: now,
    };

    let _ = write_workspace_file(&config);
    config
}

/// 更新工作区配置
pub fn update_workspace(id: &str, name: Option<String>, folders: Option<Vec<String>>, settings: Option<HashMap<String, serde_json::Value>>) -> Option<WorkspaceConfig> {
    let mut config = get_workspace(id)?;

    if let Some(name) = name {
        config.name = name;
    }
    if let Some(folders) = folders {
        config.folders = folders;
    }
    if let Some(settings) = settings {
        config.settings = settings;
    }
    config.updated_at = Utc::now().timestamp_millis();

    let _ = write_workspace_file(&config);
    Some(config)
}

/// 删除工作区
pub fn delete_workspace(id: &str) -> bool {
    let path = get_workspace_file(id);
    if path.exists() {
        fs::remove_file(&path).is_ok()
    } else {
        false
    }
}

/// 获取或创建默认工作区
pub fn get_or_create_default_workspace() -> WorkspaceConfig {
    // 先查找是否已有默认工作区
    let workspaces = list_workspaces();
    if let Some(ws) = workspaces.first() {
        if let Some(config) = get_workspace(&ws.id) {
            return config;
        }
    }

    // 没有工作区，创建默认工作区（使用 app_dir 作为默认文件夹）
    let app_dir = crate::app::app_store::APP.lock().app_dir.to_string_lossy().to_string();

    create_workspace("默认工作区".to_string(), vec![app_dir])
}

/// 读取工作区 JSON 文件
fn read_workspace_file(path: &PathBuf) -> Result<WorkspaceConfig, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("解析 JSON 失败: {}", e))
}

/// 写入工作区 JSON 文件
fn write_workspace_file(config: &WorkspaceConfig) -> Result<(), String> {
    ensure_workspaces_dir();
    let path = get_workspace_file(&config.id);
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化 JSON 失败: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("写入文件失败: {}", e))
}
