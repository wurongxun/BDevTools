/*! # 通用 KV 存储（替代 tauri-plugin-store）
config_dir 下白名单 JSON 文件的键值读写，前端经 rpc 调用。
每次写操作直接落盘，无内存缓存，避免前端（插件缓存）与后端（直读磁盘）双轨不同步的问题。
数据格式与原 store 插件生成的 JSON 完全一致（值为前端传入的任意 JSON）。

## 文件命名空间（可扩展）
- 应用核心文件：`config_dir` 根的 settings.json / openfile.json / pdf_store.json
- 插件数据文件：`config_dir/plugins/<plugin_id>.json`（单层限定，防目录穿越）
新增插件无需改本文件，前端 `load('plugins/<plugin_id>.json')` 即可使用。
*/
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::response::response::Response;

/// 应用核心文件（config_dir 根）
const CORE_FILES: [&str; 3] = ["settings.json", "openfile.json", "pdf_store.json"];
/// 插件数据文件前缀（config_dir/plugins/<plugin_id>.json）
const PLUGIN_PREFIX: &str = "plugins/";

/// 解析存储文件为绝对路径：
/// - 核心文件：限定 config_dir 根下指定文件名
/// - 插件文件：限定 `plugins/<plugin_id>.json` 单层路径，禁止 `/`、`\`、`..`（防目录穿越）
fn kv_file_path(file: &str) -> Result<PathBuf, String> {
    let config_dir = crate::app::app_store::APP.lock().config_dir.clone();
    if CORE_FILES.contains(&file) {
        return Ok(config_dir.join(file));
    }
    if let Some(rest) = file.strip_prefix(PLUGIN_PREFIX) {
        let valid = !rest.is_empty()
            && rest.ends_with(".json")
            && !rest.contains('/')
            && !rest.contains('\\')
            && !rest.contains("..");
        if valid {
            return Ok(config_dir.join(file));
        }
    }
    Err(format!("不允许访问的存储文件: {}", file))
}

/// 读取整个 JSON 文件（不存在或损坏时返回空对象）
fn read_kv_file(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// 写回整个 JSON 文件（确保配置目录存在）
fn write_kv_file(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
    }
    let content =
        serde_json::to_string_pretty(value).map_err(|e| format!("序列化失败: {}", e))?;
    fs::write(path, content).map_err(|e| format!("写入失败: {}", e))
}

/// 读取指定 key（不存在返回 null）
pub fn kv_get(file: String, key: String) -> Response<Option<Value>> {
    let path = match kv_file_path(&file) {
        Ok(p) => p,
        Err(e) => return Response::new(500, None, &e),
    };
    let value = read_kv_file(&path).get(&key).cloned();
    Response::new(200, value, "ok")
}

/// 写入指定 key（即时落盘；值为前端传入的任意 JSON，保持原 store 插件的数据格式）
pub fn kv_set(file: String, key: String, value: Value) -> Response<bool> {
    let path = match kv_file_path(&file) {
        Ok(p) => p,
        Err(e) => return Response::new(500, false, &e),
    };
    let mut root = read_kv_file(&path);
    if let Some(obj) = root.as_object_mut() {
        obj.insert(key, value);
    }
    match write_kv_file(&path, &root) {
        Ok(()) => Response::new(200, true, "ok"),
        Err(e) => Response::new(500, false, &e),
    }
}

/// 删除指定 key（即时落盘；key 不存在也视为成功）
pub fn kv_delete(file: String, key: String) -> Response<bool> {
    let path = match kv_file_path(&file) {
        Ok(p) => p,
        Err(e) => return Response::new(500, false, &e),
    };
    let mut root = read_kv_file(&path);
    let removed = root
        .as_object_mut()
        .map(|obj| obj.remove(&key).is_some())
        .unwrap_or(false);
    match write_kv_file(&path, &root) {
        Ok(()) => Response::new(200, removed, "ok"),
        Err(e) => Response::new(500, false, &e),
    }
}

/// 读取全部键值对（与 store 插件 entries() 一致，返回 [key, value] 数组）
pub fn kv_entries(file: String) -> Response<Vec<(String, Value)>> {
    let path = match kv_file_path(&file) {
        Ok(p) => p,
        Err(e) => return Response::new(500, Vec::new(), &e),
    };
    let root = read_kv_file(&path);
    let entries = root
        .as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    Response::new(200, entries, "ok")
}
