/*! # app初始化相关
> app 工作目录初始化
> 配置初始化等

纯 Rust 实现，不依赖 tauri：
- `app_dir` 业务数据目录（workspace/session/plugin_data/数据库等）
- `config_dir` 配置目录（settings.json / openfile.json，对应前端 store 插件位置）
依赖注入：`update_dirs` 由 lib.rs（框架边界）在启动时传入路径；
autostart 能力通过 [`AppContext`](crate::core::ctx::AppContext) 注入。
 */
use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde_json::json;
use std::{
    fs::{read_to_string, write},
    path::{Path, PathBuf},
};

use crate::utils::file_util::generate_md5_hash;

lazy_static! {
    pub static ref APP: Mutex<App> = Mutex::new(App::new());
}

// ==================== 配置读写（settings.json / openfile.json） ====================

/// settings.json 绝对路径（与前端 store 插件同一文件）
pub fn settings_path() -> PathBuf {
    APP.lock().config_dir.join("settings.json")
}

/// openfile.json 绝对路径（与前端 store 插件同一文件）
pub fn openfile_path() -> PathBuf {
    APP.lock().config_dir.join("openfile.json")
}

/// 读取 settings.json（不存在或损坏时返回空对象）
pub fn read_settings() -> serde_json::Value {
    read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// 写回 settings.json（保留原有内容，仅更新目标字段）
pub fn write_settings(settings: &serde_json::Value) {
    let _ = write(
        settings_path(),
        serde_json::to_string_pretty(settings).unwrap_or_default(),
    );
}

/**
* app “关联文件” 和 “打开方式” 打开文件时存储文件路径 key:文件路径md5 val:文件详细路径;
* 保存在 openfile.json 中，前端加载完成后请求全部数据 并打开
*/
pub fn app_open_file(file: &str) {
    let mut store = read_to_string(openfile_path())
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let md5 = generate_md5_hash(file);
    if let Some(obj) = store.as_object_mut() {
        obj.insert(md5, json!(file));
    }
    let _ = write(
        openfile_path(),
        serde_json::to_string_pretty(&store).unwrap_or_default(),
    );
}

/// 初始化 settings.json 默认值（仅补缺省 key，不覆盖已有值）
pub fn app_settings() {
    let mut settings = read_settings();
    let defaults: [(&str, serde_json::Value); 3] = [
        ("theme", json!({ "value": "light" })),
        ("auto_start", json!({ "value": false })),
        ("language", json!({ "value": "zh_CN" })),
    ];
    for (key, val) in defaults {
        if settings.get(key).is_none() {
            if let Some(obj) = settings.as_object_mut() {
                obj.insert(key.to_string(), val);
            }
        }
    }
    write_settings(&settings);
}

/// app配置map
pub struct App {
    /// 业务数据目录（workspace/session/plugin_data/数据库等）
    pub app_dir: PathBuf,
    /// 配置目录（settings.json / openfile.json 所在，对应 tauri 的 app_config_dir）
    pub config_dir: PathBuf,
}

impl App {
    pub fn new() -> App {
        App {
            app_dir: PathBuf::new(),
            config_dir: PathBuf::new(),
        }
    }
    /// 设置数据目录与配置目录（由 lib.rs 在启动时从 tauri 路径解析后传入）
    pub fn update_dirs(&mut self, home_dir: &Path, config_dir: &Path) {
        self.app_dir = home_dir.join("app");
        self.config_dir = config_dir.to_path_buf();
        println!("update_app_dir {:?}", self.app_dir);
        println!("update_config_dir {:?}", self.config_dir);
    }
}
