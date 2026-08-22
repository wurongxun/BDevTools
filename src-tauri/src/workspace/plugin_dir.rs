/*! # 插件工作目录管理
为每个插件在当前工作区下提供独立的工作目录。
目录结构：
  {app_dir}/plugin_data/{workspace_id}/{plugin_id}/[subdirs...]
  {app_dir}/plugin_data/_global/{plugin_id}/[subdirs...]
自定义路径存储在 settings.json 中。
*/
use std::fs;
use std::path::PathBuf;
use tauri::command;
use tauri::Manager;
use tauri_plugin_store::StoreExt;
use crate::response::response::Response;

/// 获取 app 根目录
fn app_dir() -> PathBuf {
    PathBuf::from(&crate::app::app_store::APP.lock().app_dir)
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

/// 获取全局（跨工作区）插件目录
fn get_global_plugin_dir_path(plugin_id: &str) -> PathBuf {
    plugin_data_root().join("_global").join(plugin_id)
}

/// 确保目录存在，不存在则创建
fn ensure_dir(path: &PathBuf) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }
    Ok(())
}

/// 从 settings.json 读取插件自定义目录（按工作区隔离）
/// 优先读取 `plugin_dir.{workspace_id}.{plugin_id}`，
/// 兼容旧版全局 key `plugin_dir.{plugin_id}` 作为迁移回退。
fn get_custom_dir(app: &tauri::AppHandle, workspace_id: &str, plugin_id: &str) -> Option<String> {
    let store = app.store("settings.json").ok()?;
    // 新：按工作区隔离的 key
    let scoped_key = format!("plugin_dir.{}.{}", workspace_id, plugin_id);
    if let Some(v) = store.get(&scoped_key) {
        return v.as_str().map(|s: &str| s.to_string());
    }
    // 旧：全局 key（历史数据迁移回退，读取但不回写）
    let legacy_key = format!("plugin_dir.{}", plugin_id);
    store.get(&legacy_key)
        .and_then(|v: serde_json::Value| v.as_str().map(|s: &str| s.to_string()))
}

/// 向 settings.json 写入插件自定义目录（按工作区隔离）
fn set_custom_dir(app: &tauri::AppHandle, workspace_id: &str, plugin_id: &str, path: &str) {
    if let Ok(store) = app.store("settings.json") {
        let key = format!("plugin_dir.{}.{}", workspace_id, plugin_id);
        if path.is_empty() {
            let _ = store.delete(&key);
        } else {
            store.set(key, serde_json::json!(path));
        }
        let _ = store.save();
    }
}

// ==================== Tauri 命令 ====================

/// 获取插件在当前工作区的工作目录（自动创建）
#[command]
pub fn workspace_get_plugin_dir(
    app: tauri::AppHandle,
    workspace_id: String,
    plugin_id: String,
) -> Response<String> {
    // 优先使用自定义路径（按工作区隔离）
    if let Some(custom) = get_custom_dir(&app, &workspace_id, &plugin_id) {
        if !custom.is_empty() {
            let path = PathBuf::from(&custom);
            if let Err(e) = ensure_dir(&path) {
                return Response::<String>::new(500, String::new(), &e);
            }
            return Response::new(200, custom, "ok");
        }
    }

    // 默认路径
    let dir = get_plugin_dir_path(&workspace_id, &plugin_id);
    if let Err(e) = ensure_dir(&dir) {
        return Response::<String>::new(500, String::new(), &e);
    }
    Response::new(200, dir.to_string_lossy().to_string(), "ok")
}

/// 获取插件的子目录路径（自动创建）
#[command]
pub fn workspace_get_plugin_subdir(
    app: tauri::AppHandle,
    workspace_id: String,
    plugin_id: String,
    subdir: String,
) -> Response<String> {
    // 优先使用自定义路径（按工作区隔离）
    let base = if let Some(custom) = get_custom_dir(&app, &workspace_id, &plugin_id) {
        if !custom.is_empty() {
            PathBuf::from(&custom)
        } else {
            get_plugin_dir_path(&workspace_id, &plugin_id)
        }
    } else {
        get_plugin_dir_path(&workspace_id, &plugin_id)
    };

    let dir = base.join(&subdir);
    if let Err(e) = ensure_dir(&dir) {
        return Response::<String>::new(500, String::new(), &e);
    }
    Response::new(200, dir.to_string_lossy().to_string(), "ok")
}

/// 获取全局插件目录（跨工作区共享）
#[command]
pub fn workspace_get_global_plugin_dir(
    plugin_id: String,
) -> Response<String> {
    let dir = get_global_plugin_dir_path(&plugin_id);
    if let Err(e) = ensure_dir(&dir) {
        return Response::<String>::new(500, String::new(), &e);
    }
    Response::new(200, dir.to_string_lossy().to_string(), "ok")
}

/// 设置插件自定义工作目录（传空字符串则恢复默认，按工作区隔离）
#[command]
pub fn workspace_set_plugin_custom_dir(
    app: tauri::AppHandle,
    workspace_id: String,
    plugin_id: String,
    custom_path: String,
) -> Response<bool> {
    // 验证路径有效性（非空时检查是否可访问）
    if !custom_path.is_empty() {
        let path = PathBuf::from(&custom_path);
        if let Err(e) = ensure_dir(&path) {
            return Response::new(500, false, &e);
        }
    }
    set_custom_dir(&app, &workspace_id, &plugin_id, &custom_path);
    Response::new(200, true, "设置成功")
}

/// 获取插件自定义工作目录（None 表示使用默认，按工作区隔离）
#[command]
pub fn workspace_get_plugin_custom_dir(
    app: tauri::AppHandle,
    workspace_id: String,
    plugin_id: String,
) -> Response<Option<String>> {
    let custom = get_custom_dir(&app, &workspace_id, &plugin_id);
    Response::new(200, custom, "ok")
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
