/*! # 通用 KV 存储（替代 tauri-plugin-store）
config_dir 下白名单 JSON 文件的键值读写，前端经 rpc 调用。
每次写操作直接落盘，无内存缓存，避免前端（插件缓存）与后端（直读磁盘）双轨不同步的问题。
数据格式与原 store 插件生成的 JSON 完全一致（值为前端传入的任意 JSON）。

## 文件命名空间（可扩展）
- 应用核心文件：`config_dir` 根的 settings.json / openfile.json / pdf_store.json
- 内置插件数据文件：`config_dir/plugins/<plugin_id>.json`（单层限定，防目录穿越）
- 第三方插件数据文件：`app_dir/plugins_ext_kv/<plugin_id>.json`
  （与内置 `plugins/` 隔离；`plugins_ext/` 已被插件包目录占用，故另起 `plugins_ext_kv/`）
  ❗**落在 `app_dir` 而非 `config_dir`**：第三方插件数据是用户资产，而 config_dir 会被
  NSIS 卸载脚本 `RmDir /r "$APPDATA\${BUNDLEID}"` 整棵删掉（卸载/重装即丢插件）
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
/// 第三方插件数据文件前缀（app_dir/plugins_ext_kv/<plugin_id>.json）
const THIRD_PARTY_PREFIX: &str = "plugins_ext_kv/";

/// 解析存储文件为绝对路径：
/// - 核心文件：限定 config_dir 根下指定文件名
/// - 插件文件：限定 `plugins/<plugin_id>.json` 或 `plugins_ext_kv/<plugin_id>.json`
///   单层路径，禁止 `/`、\、`..`（防目录穿越）
/// 其他模块（如 workspace/plugin_dir）读写插件配置时复用此校验，保证路径安全与约定一致。
pub fn kv_file_path(file: &str) -> Result<PathBuf, String> {
    let (config_dir, app_dir) = {
        let app = crate::app::app_store::APP.lock();
        (app.config_dir.clone(), app.app_dir.clone())
    };
    kv_file_path_in(&config_dir, &app_dir, file)
}

/// 纯函数版路径解析（不碰全局单例，便于单测）
///
/// 前缀 → 根目录的映射即「卸载时该不该删」的策略：
/// 核心文件与内置插件数据属于应用配置（config_dir，随卸载清除可接受）；
/// **第三方插件数据属于用户资产，必须落在 app_dir**。
fn kv_file_path_in(config_dir: &Path, app_dir: &Path, file: &str) -> Result<PathBuf, String> {
    if CORE_FILES.contains(&file) {
        return Ok(config_dir.join(file));
    }
    for (prefix, root) in [(PLUGIN_PREFIX, config_dir), (THIRD_PARTY_PREFIX, app_dir)] {
        if let Some(rest) = file.strip_prefix(prefix) {
            let valid = !rest.is_empty()
                && rest.ends_with(".json")
                && !rest.contains('/')
                && !rest.contains('\\')
                && !rest.contains("..");
            if valid {
                return Ok(root.join(prefix).join(rest));
            }
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

/// 直读某 key（供后端业务模块内部使用，如 MAT 插件读 `analysisBudgetGb` 覆盖项）。
/// 文件不存在 / 损坏 / 路径校验失败均返回 None，调用方按默认值处理。
pub fn kv_read_value(file: &str, key: &str) -> Option<Value> {
    let path = kv_file_path(file).ok()?;
    read_kv_file(&path).get(key).cloned()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// 第三方插件的 KV 必须落在 `app_dir`（用户资产），不能落在 `config_dir`（会被卸载删）
    #[test]
    fn third_party_kv_lives_in_app_dir() {
        let cfg = Path::new("/cfg");
        let app = Path::new("/data/bdt");

        let third = kv_file_path_in(cfg, app, "plugins_ext_kv/demo.json").expect("应放行");
        assert!(third.starts_with(app), "第三方 KV 应在 app_dir 下: {:?}", third);
        assert!(!third.starts_with(cfg), "不能落在 config_dir: {:?}", third);

        // 应用配置与内置插件数据仍按原样留在 config_dir（随卸载清除是可接受的）
        let core = kv_file_path_in(cfg, app, "settings.json").expect("应放行");
        assert!(core.starts_with(cfg), "核心配置仍在 config_dir: {:?}", core);
        let builtin = kv_file_path_in(cfg, app, "plugins/logs.json").expect("应放行");
        assert!(builtin.starts_with(cfg), "内置插件数据仍在 config_dir: {:?}", builtin);

        // 目录穿越防护没有被这次改动削弱
        assert!(kv_file_path_in(cfg, app, "plugins_ext_kv/../settings.json").is_err());
        assert!(kv_file_path_in(cfg, app, "plugins_ext_kv/evil").is_err());
    }
}