/*! # app初始化相关
> app 工作目录初始化
> 配置初始化等

纯 Rust 实现，不依赖 tauri：
- `app_dir` 业务数据目录（workspace / session / plugin_data / 数据库 / 第三方插件）
- `config_dir` 配置目录（settings.json / openfile.json，对应前端 store 插件位置）
- 两者的分工按「卸载时该不该删」划分，见 [`resolve_user_data_root`] 的注释
依赖注入：`update_dirs` 由 lib.rs（框架边界）在启动时传入路径；
autostart 能力通过 [`AppContext`](crate::core::ctx::AppContext) 注入。
 */
use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde_json::json;
use std::{
    fs::{self, read_to_string, write},
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

// ==================== 业务数据根目录与迁移 ====================

/// 旧版业务数据目录名：`<home>/app`（历史遗留）
const LEGACY_APP_FOLDER: &str = "app";

/// 业务数据根目录名（= 产品名，见 `tauri.conf.json` 的 `productName`）
const USER_DATA_FOLDER: &str = "BDevTools";

/// 平台放用户数据的基目录（不含产品名那一层）
///
/// **Windows 必须是 `%APPDATA%`（Roaming）而不是 `%LOCALAPPDATA%`**：
/// `bundle.windows.nsis.installMode = "currentUser"` 时 NSIS 的 `$INSTDIR`
/// 正是 `%LOCALAPPDATA%\<产品名>` —— 卸载/升级时会被整棵删除。
#[cfg(target_os = "windows")]
fn os_data_base(home_dir: &Path) -> PathBuf {
    let _ = home_dir;
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir.join("AppData").join("Roaming"))
}

/// 平台放用户数据的基目录（不含产品名那一层）
#[cfg(target_os = "macos")]
fn os_data_base(home_dir: &Path) -> PathBuf {
    home_dir.join("Library").join("Application Support")
}

/// 平台放用户数据的基目录（不含产品名那一层）
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn os_data_base(home_dir: &Path) -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir.join(".local").join("share"))
}

/// 业务数据根目录（用户资产：workspace / session / 数据库 / 第三方插件）
///
/// 在 Windows 上要同时躲开 **installer 会删的两个地方**：
///
/// 1. tauri 的 `app_data_dir()` = `%LOCALAPPDATA%\<identifier>`
///    （本项目 identifier = `com.backend-development-tools.app`），卸载脚本在用户勾选
///    「删除应用数据」时会 `RmDir /r "$LOCALAPPDATA\${BUNDLEID}"`；
/// 2. NSIS 的安装目录 `$INSTDIR` —— `installMode = "currentUser"` 下它**正好是
///    `%LOCALAPPDATA%\<产品名>`**，升级/卸载时被删得更彻底。
///
/// （2026-09-16 真实事故：第一版把数据根定在 `%LOCALAPPDATA%\BDevTools`，
/// 本机该目录里躺着 `BDevTools.exe` 与 `uninstall.exe` —— 数据差点住进安装目录。）
///
/// 因此 Windows 走 **`%APPDATA%\BDevTools`**（Roaming）：既不在安装目录内，
/// 也不在 BUNDLEID 目录内，installer 无论如何都够不着。
pub fn resolve_user_data_root(home_dir: &Path) -> PathBuf {
    os_data_base(home_dir).join(USER_DATA_FOLDER)
}

/// 旧版业务数据目录 `<home>/app`
///
/// 除了目录搬迁本身，工作区配置里持久化过的「默认工作区文件夹」也可能还是这个旧值，
/// 需要一并清理（见 `workspace::config::migrate_default_workspace_folder`）。
pub fn legacy_app_dir(home_dir: &Path) -> PathBuf {
    home_dir.join(LEGACY_APP_FOLDER)
}

/// 目录搬迁的结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrateOutcome {
    /// 不需要迁移（没有旧目录，或新目录已存在）
    NotNeeded,
    /// 已迁移
    Migrated,
    /// 迁移失败 —— 调用方应当**回退使用旧目录**，绝不能让用户数据凭空消失
    Failed,
}

/// 递归复制目录（`rename` 跨盘会失败，用它兜底）
fn copy_dir_recursive(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let dst = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dst)?;
        } else {
            fs::copy(entry.path(), &dst)?;
        }
    }
    Ok(())
}

/// 把旧目录搬到新位置：**只在新位置不存在时动手**，绝不覆盖已有新数据
pub(crate) fn migrate_dir_once(from: &Path, to: &Path) -> MigrateOutcome {
    if !from.is_dir() {
        return MigrateOutcome::NotNeeded;
    }
    if to.exists() {
        return MigrateOutcome::NotNeeded;
    }
    if let Some(parent) = to.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::rename(from, to) {
        Ok(()) => MigrateOutcome::Migrated,
        Err(_) => match copy_dir_recursive(from, to) {
            Ok(()) => {
                let _ = fs::remove_dir_all(from);
                MigrateOutcome::Migrated
            }
            Err(e) => {
                println!("[App] 目录迁移失败 {:?} -> {:?}: {}", from, to, e);
                let _ = fs::remove_dir_all(to);
                MigrateOutcome::Failed
            }
        },
    }
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
    ///
    /// 顺带做一次目录迁移：旧的 `<home>/app` → [`resolve_user_data_root`]。
    /// **迁移失败时回退到旧目录**：宁可继续污染 HOME，也不能让用户数据看起来消失了。
    pub fn update_dirs(&mut self, home_dir: &Path, config_dir: &Path) {
        let legacy = legacy_app_dir(home_dir);
        let target = resolve_user_data_root(home_dir);
        if home_dir.as_os_str().is_empty() {
            // 拿不到真实 home 时不要拿相对路径开玩笑，直接用目标目录
            self.app_dir = target;
        } else {
            match migrate_dir_once(&legacy, &target) {
                MigrateOutcome::Failed => {
                    self.app_dir = legacy;
                    println!("[App] 业务数据迁移失败，继续使用旧目录 {:?}", self.app_dir);
                }
                outcome => {
                    if outcome == MigrateOutcome::Migrated {
                        println!("[App] 业务数据已迁移: {:?} -> {:?}", legacy, target);
                    }
                    self.app_dir = target;
                }
            }
        }
        self.config_dir = config_dir.to_path_buf();
        println!("update_app_dir {:?}", self.app_dir);
        println!("update_config_dir {:?}", self.config_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **「升级不丢第三方插件」唯一的结构性防线**
    ///
    /// NSIS 卸载脚本（`target/release/nsis/x64/installer.nsi`）在用户勾选「删除应用数据」时执行：
    /// `RmDir /r "$APPDATA\${BUNDLEID}"` 与 `RmDir /r "$LOCALAPPDATA\${BUNDLEID}"`。
    /// 一旦有人把用户资产放回 `<BUNDLEID>` 目录里，这条断言会立刻失败 —— 别删它。
    #[test]
    fn user_data_root_is_outside_uninstall_scope() {
        let home = Path::new("/home/tester");
        let root = resolve_user_data_root(home);
        let path = root.to_string_lossy().replace('\\', "/");
        assert!(
            !path.contains("com.backend-development-tools.app"),
            "业务数据目录不能落在 installer 会删除的 BUNDLEID 目录里: {}",
            path
        );
        assert!(path.contains(USER_DATA_FOLDER), "应带产品名那一层: {}", path);
    }

    /// Windows 上 NSIS 的 `installMode = "currentUser"` 会把 `$INSTDIR`
    /// 放在 `%LOCALAPPDATA%\<产品名>`，卸载/升级时整棵删除。
    /// 曾经把业务数据根目录定在这里，正好和 exe / uninstall.exe 挤在同一目录。
    #[test]
    fn user_data_root_is_not_the_install_dir() {
        let home = Path::new("/home/tester");
        let root = resolve_user_data_root(home);
        let path = root.to_string_lossy().replace('\\', "/");
        assert!(
            !path.contains("AppData/Local/BDevTools"),
            "业务数据目录不能落在 NSIS 安装目录 $INSTDIR 里: {}",
            path
        );
    }

    /// 迁移：有旧目录且新目录不存在时搬过去；重复调用与新目录已存在时都不能乱动
    #[test]
    fn migrate_dir_moves_once_and_never_overwrites() {
        let tmp = std::env::temp_dir().join(format!("bdt-appdir-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let from = tmp.join("app");
        let to = tmp.join("data");
        fs::create_dir_all(from.join("workspaces")).expect("造旧目录");
        fs::write(from.join("app.db"), "x").expect("写旧数据");

        assert_eq!(migrate_dir_once(&from, &to), MigrateOutcome::Migrated);
        assert!(to.join("app.db").is_file(), "数据应被搬走");
        assert!(to.join("workspaces").is_dir(), "子目录应被搬走");
        assert!(!from.exists(), "旧目录不应残留");

        // 重复调用：没有旧目录了
        assert_eq!(migrate_dir_once(&from, &to), MigrateOutcome::NotNeeded);

        // 新目录已存在时绝不覆盖（老用户的数据优先）
        let legacy_again = tmp.join("app2");
        fs::create_dir_all(&legacy_again).expect("再造旧目录");
        assert_eq!(migrate_dir_once(&legacy_again, &to), MigrateOutcome::NotNeeded);
        assert!(legacy_again.exists(), "不能动旧目录");

        let _ = fs::remove_dir_all(&tmp);
    }
}