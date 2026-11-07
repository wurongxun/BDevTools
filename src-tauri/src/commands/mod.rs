//! # Tauri 命令统一门面
//!
//! 本模块（mod.rs + 同目录分片）是唯一的 `#[tauri::command]` 定义处（RPC 边界），
//! 也是 **唯一** 持有 `tauri` / `tauri_plugin_*` 依赖的业务代码。
//! 所有命令均为薄包装：仅做参数转发与 Tauri 类型适配，业务逻辑保留在各模块中。
//! mod.rs 负责共享 import、`ctx_from_app` 适配与 `pub use` 重导出裸名；
//! 分片文件一律 `use super::*;`，函数级 `#[cfg]` 逐个保留。
//! 将来更换 UI 框架时，只需重写本模块（及 lib.rs 的组装逻辑）。
//!
//! 命令名即暴露给前端的 invoke 名称，改名需同步前端。

use std::collections::HashMap;

use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::{NotificationExt, PermissionState};
use tauri_plugin_opener::OpenerExt;

use crate::core::ctx::AppContext;
use crate::window_manager::app_window;
#[cfg(feature = "plugin-files")]
use crate::plugins::files::file_association;
#[cfg(feature = "plugin-files")]
use crate::plugins::files::file_command::{self, ScriptFileEntry};
#[cfg(feature = "plugin-files")]
use crate::plugins::files::file_utils;
#[cfg(feature = "plugin-logs")]
use crate::plugins::logs::log_index::{
    self, LogExportResult, LogIndexMeta, LogQueryResult,
};
#[cfg(feature = "plugin-logs")]
use crate::plugins::logs::log_stream;
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_hprof::{self, MatHeaderInfo, MatHistogramResult};
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_leak::{self, MatLeakSuspectsResult};
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_dom;
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_oql;
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_reports;
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_index;
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_source;
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_compare;
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_values;
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_cfg::{self, MatLimitInfo};
#[cfg(feature = "plugin-mat")]
use crate::mat_worker_host::MatWorkerJob;
// 目录已改名为 plugin_host/{host,wasm}.rs，但函数体一律用裸名 `plugin_host::` / `plugin_wasm::`
// 书写（与插件 id 语义对齐），故 import 时用别名把名字拉回原样，避免改动几百处调用点。
use crate::plugin_host::host as plugin_host;
use crate::plugin_host::wasm as plugin_wasm;
#[cfg(feature = "plugin-mat")]
use crate::plugins::mat::mat_refs::{
    self, RefBuildInfo, RefInstances, RefObjectDetail, RefSearchResult,
};
#[cfg(feature = "plugin-ssh")]
use crate::plugins::ssh::remote_conn::{self, RemoteFileItem};
#[cfg(feature = "plugin-terminal")]
use crate::plugins::terminal::terminal;
// 行读取是**核心能力**（files 的 TextView 与 logs 都用），不随任何插件裁剪
use crate::utils::line_reader::{self, FileLinesResult};
#[cfg(feature = "plugin-database")]
use crate::plugins::database::connection::DbConnectionConfig;
#[cfg(feature = "plugin-database")]
use crate::plugins::database::import_export::{ExportResult, ImportResult};
#[cfg(feature = "plugin-database")]
use crate::plugins::database::metadata::{
    CatalogCapabilities, DatabaseIndexInfo, DatabaseObjectInfo, DatabaseInfo, DbAdminInfo, DbUserInfo,
    IndexInfo, KeyValueRow, TableInfo,
};
#[cfg(feature = "plugin-database")]
use crate::plugins::database::operations::{ColumnInfo, SqlResult, TableDataResult};
use crate::entity::session::{SessionSnapshot, SessionSummary};
use crate::entity::window::WindowMeta;
use crate::entity::workspace::{WorkspaceConfig, WorkspaceSummary};
use crate::response::resp_data::ResData;
use crate::response::response::Response;
use crate::session::draft::{DraftCheckResult, DraftMeta};
// SSH 连接配置（sqlite 表在核心，但只有 ssh 插件的命令用它）
#[cfg(feature = "plugin-ssh")]
use crate::sqlite::ssh_connection::SshSubMenu;

/// tauri → 业务 的唯一适配点：把 `tauri::AppHandle` 转换为平台无关的 [`AppContext`]。
/// 将来更换 UI 框架：仅重写此函数，业务模块零改动。
fn ctx_from_app(app: &tauri::AppHandle) -> AppContext {
    let data_dir = app.path().app_data_dir().unwrap_or_default();
    let config_dir = app.path().app_config_dir().unwrap_or_default();
    let document_dir = app.path().document_dir().ok();

    let emitter_app = app.clone();
    let opener_app = app.clone();
    let notify_app = app.clone();
    let autostart_app = app.clone();

    AppContext::new(data_dir)
        .with_config_dir(config_dir)
        .with_document_dir(document_dir)
        .with_emitter(move |event, payload| {
            let _ = emitter_app.emit(event, payload);
        })
        .with_opener(move |path| {
            opener_app
                .opener()
                .open_path(path, None::<String>)
                .map_err(|e| e.to_string())
        })
        .with_notifier(move |body| {
            let notification = notify_app.notification();
            if matches!(notification.permission_state(), Ok(PermissionState::Granted)) {
                let _ = notification.builder().body(body.to_string()).show();
            }
        })
        .with_autostart(move |enabled| {
            let manager = autostart_app.autolaunch();
            if enabled {
                let _ = manager.enable();
            } else {
                let _ = manager.disable();
            }
        })
}

// ===== 命令分片：按现有分区机械拆分，命令名 / cfg / 函数体一律未改 =====
mod files;
mod window;
mod logs;
mod mat;
mod ssh;
mod workspace;
mod kv_session;
mod database;
mod plugin;
mod terminal_cmd;

// `lib.rs` 仍写 `use crate::commands::{...}` 拿裸名，验证脚本也只解析它的 cfg 块。
// 纯插件分片的导出要跟 lib.rs 的 use 块同 cfg，否则裁剪组合下会报 unused import。
// 命名坑：分片不能叫 terminal.rs —— mod.rs 里已有业务别名 `use ...::terminal::terminal;`，
// 模块名撞车会让 `terminal::TermEvent` 之类全部解析到分片（E0255 + 一串 E0425）。
pub use self::files::*;
pub use self::window::*;
#[cfg(feature = "plugin-logs")]
pub use self::logs::*;
#[cfg(feature = "plugin-mat")]
pub use self::mat::*;
#[cfg(feature = "plugin-ssh")]
pub use self::ssh::*;
pub use self::workspace::*;
pub use self::kv_session::*;
pub use self::database::*;
pub use self::plugin::*;
#[cfg(feature = "plugin-terminal")]
pub use self::terminal_cmd::*;
