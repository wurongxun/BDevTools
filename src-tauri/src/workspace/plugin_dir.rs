/*! # 插件工作目录管理
为每个插件在当前工作区下提供独立的默认工作目录：
  {app_dir}/plugin_data/{workspace_id}/{plugin_id}/[subdirs...]
同时支持用户级（跨工作区共享）的外部脚本目录挂载列表，
外部目录路径存储在 config_dir/plugins/<plugin_id>.json 的 externalDirs 字段中，
仅解除挂载不删除内容（历史数据在 settings.json 的 plugin_external_dirs.*，首次读取时自动迁移）。
*/
use std::fs;
use std::path::{Path, PathBuf};
use crate::response::response::Response;

/// 获取 app 根目录
fn app_dir() -> PathBuf {
    PathBuf::from(&crate::app::app_store::APP.lock().app_dir)
}

/// settings.json 绝对路径（复用 app_store 的统一入口，与前端 store 插件同一文件）
fn settings_path() -> PathBuf {
    crate::app::app_store::settings_path()
}

/// 读取 settings.json（不存在或损坏时返回空对象）
fn read_settings() -> serde_json::Value {
    fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// 获取插件数据根目录
fn plugin_data_root() -> PathBuf {
    app_dir().join("plugin_data")
}

/// 确保插件数据根目录存在
pub fn ensure_plugin_data_dir() {
    let dir = plugin_data_root();
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
}

/// 获取指定工作区下指定插件的目录
fn get_plugin_dir_path(workspace_id: &str, plugin_id: &str) -> PathBuf {
    plugin_data_root().join(workspace_id).join(plugin_id)
}

/// 确保目录存在，不存在则创建
fn ensure_dir(path: &PathBuf) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }
    Ok(())
}

// ==================== 外部脚本目录（用户级） ====================
// 存储位置：config_dir/plugins/<plugin_id>.json 的 externalDirs 字段（符合 KV 插件命名空间约定）
// 历史数据在 settings.json 的 plugin_external_dirs.<plugin_id>，首次读取时自动迁移。

/// 插件配置文件绝对路径（复用 kv_store 的命名空间校验，防目录穿越）
fn plugin_config_path(plugin_id: &str) -> Result<PathBuf, String> {
    crate::app::kv_store::kv_file_path(&format!("plugins/{}.json", plugin_id))
}

/// 读取插件配置文件（不存在或损坏时返回空对象）
fn read_plugin_config(path: &Path) -> serde_json::Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// 写回插件配置文件（确保 plugins 目录存在）
fn write_plugin_config(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
    }
    let content = serde_json::to_string_pretty(value)
        .map_err(|e| format!("序列化失败: {}", e))?;
    fs::write(path, content).map_err(|e| format!("写入失败: {}", e))
}

/// 一次性迁移：把 settings.json 中旧 key plugin_external_dirs.<id> 迁移到 plugins/<id>.json
fn migrate_legacy_external_dirs(plugin_id: &str) {
    let mut settings = read_settings();
    let key = format!("plugin_external_dirs.{}", plugin_id);
    if settings.get(&key).is_none() {
        return;
    }
    // 迁移数组到新位置（externalDirs 字段）
    let dirs: Vec<String> = settings
        .get(&key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s: &str| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if !dirs.is_empty() {
        set_external_dirs(plugin_id, &dirs);
    }
    // 从 settings.json 删除旧 key（保留其他 key）
    if let Some(obj) = settings.as_object_mut() {
        obj.remove(&key);
        let _ = fs::write(
            settings_path(),
            serde_json::to_string_pretty(&settings).unwrap_or_default(),
        );
    }
    println!(
        "[PluginDir] 已将 plugin_external_dirs.{} 迁移至 plugins/{}.json（{} 条）",
        plugin_id,
        plugin_id,
        dirs.len()
    );
}

/// 读取插件外部脚本目录列表（用户级，跨工作区共享）
fn get_external_dirs(plugin_id: &str) -> Vec<String> {
    migrate_legacy_external_dirs(plugin_id);
    let path = match plugin_config_path(plugin_id) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    read_plugin_config(&path)
        .get("externalDirs")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s: &str| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// 写入插件外部脚本目录列表（保留插件配置文件其他字段）
fn set_external_dirs(plugin_id: &str, dirs: &[String]) {
    let path = match plugin_config_path(plugin_id) {
        Ok(p) => p,
        Err(e) => {
            println!("[PluginDir] 非法插件配置路径: {}", e);
            return;
        }
    };
    let mut config = read_plugin_config(&path);
    if let Some(obj) = config.as_object_mut() {
        if dirs.is_empty() {
            obj.remove("externalDirs");
        } else {
            obj.insert("externalDirs".to_string(), serde_json::json!(dirs));
        }
    }
    if let Err(e) = write_plugin_config(&path, &config) {
        println!("[PluginDir] 写入插件配置失败: {}", e);
    }
}

/// 判断两个路径是否互为父子包含（规范化后按组件比较，避免 /a/foo 误判为 /a/foo2 的子路径）
fn paths_related(a: &Path, b: &Path) -> bool {
    let ca = fs::canonicalize(a).unwrap_or_else(|_| a.to_path_buf());
    let cb = fs::canonicalize(b).unwrap_or_else(|_| b.to_path_buf());
    ca.starts_with(&cb) || cb.starts_with(&ca)
}

// ==================== Tauri 命令 ====================

/// 获取插件在当前工作区的工作目录（自动创建，仅默认路径）
//#[command]
pub fn workspace_get_plugin_dir(
    workspace_id: String,
    plugin_id: String,
) -> Response<String> {
    let dir = get_plugin_dir_path(&workspace_id, &plugin_id);
    if let Err(e) = ensure_dir(&dir) {
        return Response::<String>::new(500, String::new(), &e);
    }
    Response::new(200, dir.to_string_lossy().to_string(), "ok")
}

/// 获取插件的子目录路径（自动创建）
//#[command]
pub fn workspace_get_plugin_subdir(
    workspace_id: String,
    plugin_id: String,
    subdir: String,
) -> Response<String> {
    let base = get_plugin_dir_path(&workspace_id, &plugin_id);
    let dir = base.join(&subdir);
    if let Err(e) = ensure_dir(&dir) {
        return Response::<String>::new(500, String::new(), &e);
    }
    Response::new(200, dir.to_string_lossy().to_string(), "ok")
}

/// 获取插件外部脚本目录列表（用户级，跨工作区共享）
//#[command]
pub fn workspace_list_plugin_external_dirs(plugin_id: String) -> Response<Vec<String>> {
    let dirs = get_external_dirs(&plugin_id);
    Response::new(200, dirs, "ok")
}

/// 添加插件外部脚本目录（用户级，跨工作区共享）
/// 校验：目录存在、不重复、与默认脚本目录及已有外部目录无父子包含关系。
//#[command]
pub fn workspace_add_plugin_external_dir(
    workspace_id: String,
    plugin_id: String,
    path: String,
) -> Response<bool> {
    let dir = PathBuf::from(&path);
    if !dir.exists() || !dir.is_dir() {
        return Response::new(500, false, "目录不存在或不可访问");
    }
    let mut dirs = get_external_dirs(&plugin_id);
    if dirs.iter().any(|d| d == &path) {
        return Response::new(200, true, "该目录已在列表中");
    }
    // 与默认脚本目录冲突检查
    let default_scripts = get_plugin_dir_path(&workspace_id, &plugin_id).join("scripts");
    if paths_related(&dir, &default_scripts) {
        return Response::new(500, false, "该目录与应用默认脚本目录冲突");
    }
    // 与已有外部目录冲突检查
    for d in &dirs {
        if paths_related(&dir, &PathBuf::from(d)) {
            return Response::new(500, false, "该目录与已挂载目录存在包含关系");
        }
    }
    dirs.push(path);
    set_external_dirs(&plugin_id, &dirs);
    Response::new(200, true, "添加成功")
}

/// 移除插件外部脚本目录（仅解除挂载，不删除目录内容）
//#[command]
pub fn workspace_remove_plugin_external_dir(
    plugin_id: String,
    path: String,
) -> Response<bool> {
    let mut dirs = get_external_dirs(&plugin_id);
    let before = dirs.len();
    dirs.retain(|d| d != &path);
    if dirs.len() == before {
        return Response::new(200, false, "该目录未在列表中");
    }
    set_external_dirs(&plugin_id, &dirs);
    Response::new(200, true, "已移除")
}

/// 删除指定工作区的全部插件数据目录（删除工作区时调用，避免孤儿数据）
pub fn delete_plugin_data_dir(workspace_id: &str) {
    let dir = get_plugin_dir_path(workspace_id, "");
    if dir.exists() {
        if let Err(e) = fs::remove_dir_all(&dir) {
            println!("[PluginDir] 清理工作区 {} 插件数据失败: {}", workspace_id, e);
        } else {
            println!("[PluginDir] 已清理工作区 {} 插件数据: {}", workspace_id, dir.display());
        }
    }
}
