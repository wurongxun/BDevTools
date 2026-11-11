/*! # 工作区配置管理
工作区配置以 JSON 文件形式存储在 {app_dir}/workspaces/ 目录下。
每个工作区一个文件：{workspace_id}.workspace.json
*/
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use uuid::Uuid;

use crate::entity::workspace::{WorkspaceConfig, WorkspaceSummary};

// 默认工作区**刻意不预置任何文件夹**（folders 为空）。
//
// 历史上有两种做法都踩了坑：
// 1. 直接用 app_dir **本身**当 folder —— 等于把应用数据家目录当成项目根，`app.db` / `sessions` /
//    `drafts` / `plugin_data` / `plugins_ext` / `workspaces` 这些内部资产会一起出现在用户文件树里
//    （可被误删，且 `workspaces` 目录自引用）。
// 2. 用 `{app_dir}/workspace` 子目录 —— 虽然隔离了内部资产，但等于替用户决定了一个「文档该放哪」，
//    而这个目录不是用户选的：备份/迁移时用户找不到它，卸载清理也会连带删掉。
//
// 现在的约定：**目录由用户自己选**。folders 为空时 files 插件侧边栏显示「打开文件 / 打开文件夹」
// 引导入口，用户选了什么就用什么 —— 没有目录就不假装有。

/// 获取工作区存储目录路径
fn get_workspaces_dir() -> PathBuf {
    let app_dir = PathBuf::from(&crate::app::app_store::APP.lock().app_dir);
    app_dir.join("workspaces")
}

/// 路径比较键：忽略分隔符方向与大小写（Windows 下 `C:\a` 与 `c:/a` 视为同一路径）
fn path_key(p: &str) -> String {
    p.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// 一次性迁移：清掉仍指向「历史默认目录」的文件夹
///
/// 需要清理的历史值有两种形态（都曾被持久化到磁盘，目录搬迁时不会自动跟着走）：
/// 1. 旧版 app_dir `<home>/app` —— 目录本身早已迁到 `%APPDATA%\BDevTools`，但工作区配置与
///    `settings.json` 里的**路径字符串**还是旧值，于是文件树会指向一个空壳/残留目录；
/// 2. app_dir 本身 —— 默认工作区曾直接把应用数据家目录当项目根（内部资产全暴露）。
///
/// 清理而非替换：这些路径都不是用户主动选的，替换成另一个「替用户决定」的目录只是把问题搬个家。
/// 清掉之后 files 插件走空状态，由侧边栏引导用户自己选目录。
///
/// 由启动 setup 调用（早于前端读取工作区与 settings），幂等 —— 清理后不再命中。
pub fn migrate_default_workspace_folder(home_dir: &Path) {
    let app_dir = PathBuf::from(&crate::app::app_store::APP.lock().app_dir);
    let legacy: Vec<String> = [
        crate::app::app_store::legacy_app_dir(home_dir),
        app_dir.clone(),
        // 上一版曾把默认工作区指向过这里，一并清掉（该目录若已存在且放着用户文件，不受影响 ——
        // 这里只改配置里的路径字符串，不动磁盘）
        app_dir.join("workspace"),
    ]
    .iter()
    .map(|p| path_key(&p.to_string_lossy()))
    .collect();

    // 1) 工作区配置里的 folders
    for ws in list_workspaces() {
        let Some(mut config) = get_workspace(&ws.id) else { continue };
        let before = config.folders.len();
        config.folders.retain(|f| !legacy.contains(&path_key(f)));
        if config.folders.len() != before {
            // 刻意不更新 updated_at：迁移不是用户操作，不该改变工作区排序
            let _ = write_workspace_file(&config);
        }
    }

    // 2) 前端文件树工作目录（settings.json —— App.vue 启动时读取并设为文件树根节点）
    //    置空而不是删键：保留 { value } 结构，前端 `mdDir?.value` 读到的仍是 falsy，行为一致。
    let mut settings = crate::app::app_store::read_settings();
    let mut dirty = false;
    if let Some(obj) = settings.as_object_mut() {
        if let Some(entry) = obj.get_mut("md_working_directory") {
            let cur = entry
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if legacy.contains(&path_key(&cur)) {
                *entry = serde_json::json!({ "value": "" });
                dirty = true;
            }
        }
    }
    if dirty {
        crate::app::app_store::write_settings(&settings);
    }
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

    // 没有工作区：创建一个空的默认工作区（folders 为空 —— 目录由用户在 files 侧边栏自己选，
    // 不预置任何目录，理由见文件顶部注释）
    create_workspace("默认工作区".to_string(), Vec::new())
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
