// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use serde_json::json;
use tauri::AppHandle;
use tauri::Manager;
use tauri::{App, Emitter};
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_store::StoreExt;
use tauri_plugin_window_state::{StateFlags, WindowExt};

mod app;
mod app_setup;
mod command;
mod entity;
mod response;
mod system_menu;
mod utils;
mod sqlite;
mod window_manager;
mod workspace;
mod session;

use crate::app::app_store::{app_autostart, app_settings,app_openFile, APP};
use crate::app::tray::create_tray;
use crate::app_setup::setup::setup_windows;
use crate::command::app_window::{
    about, loding_window_close, note_book_window, windows_close, windows_hide, windows_maximize,
    windows_minimize,
};
use crate::command::file_command::{
    create_folder, create_note_file, ensure_dir, get_note_list, get_path_childrenFoldersAndFiles,
    read_note_file, remove_file, remove_folder, save_note, up_file_name,
};
use crate::command::file_utils::{file_manager_open_path, get_document_notebooks, up_folder_name,detect_file_encoding};
use crate::command::file_association::{register_file_association, open_default_apps_settings};
use crate::command::remote_conn::{
    ssh_connect, ssh_write, ssh_disconnect, ssh_resize_pty, ssh_check,
    sftp_connect, sftp_disconnect, sftp_list_dir, sftp_mkdir, sftp_remove,
    sftp_upload, sftp_download, local_list_dir, sftp_rename, get_app_dir, sftp_check,
};

use crate::command::search_log_parallel::{
    read_file_lines, count_file_lines, search_log_index, search_log_jump_to, search_log_page,
    search_log_parallel, search_log_preview,
};
use crate::sqlite::ssh_connection::{
    connection_create, connection_delete, connection_edit, connection_list,
};
use crate::window_manager::commands::{
    create_editor_window, get_window_list, close_window, focus_window,
    get_window_count, update_window_position_cmd,
};
use crate::workspace::commands::{
    workspace_list, workspace_get, workspace_create, workspace_update,
    workspace_delete, workspace_get_default,
};
use crate::session::commands::{
    session_save, session_load, session_load_all, session_list,
    session_delete, session_clear,
};


struct Payload {
    args: Vec<String>,
    cwd: String,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {}))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            get_document_notebooks,
            file_manager_open_path,
            get_note_list,
            read_note_file,
            create_folder,
            ensure_dir,
            create_note_file,
            get_path_childrenFoldersAndFiles,
            save_note,
            up_file_name,
            up_folder_name,
            remove_folder,
            remove_file,
            loding_window_close,
            about,
            note_book_window,
            windows_hide,
            windows_minimize,
            windows_maximize,
            windows_close,
            search_log_index,
            search_log_page,
            search_log_parallel,
            search_log_jump_to,
            search_log_preview,
            read_file_lines,
            count_file_lines,
            detect_file_encoding,
            register_file_association,
            open_default_apps_settings,
            ssh_connect,
            ssh_write,
            ssh_disconnect,
            ssh_resize_pty,
            ssh_check,
            sftp_connect,
            sftp_disconnect,
            sftp_check,
            sftp_list_dir,
            sftp_mkdir,
            sftp_remove,
            sftp_upload,
            sftp_download,
            local_list_dir,
            sftp_rename,
            get_app_dir,
            connection_create,
            connection_delete,
            connection_edit,
            connection_list,
            // 窗口管理
            create_editor_window,
            get_window_list,
            close_window,
            focus_window,
            get_window_count,
            update_window_position_cmd,
            // 工作区管理
            workspace_list,
            workspace_get,
            workspace_create,
            workspace_update,
            workspace_delete,
            workspace_get_default,
            // 会话管理
            session_save,
            session_load,
            session_load_all,
            session_list,
            session_delete,
            session_clear,
        ])
        .setup(move |app: &mut App| {
            // 初始化 app_dir
            APP.lock().update_app_dir(app.handle());
            // 初始化工作区和会话目录
            crate::workspace::config::ensure_workspaces_dir();
            crate::session::snapshot::ensure_sessions_dir();
            #[cfg(all(desktop))]
            {
                let handle = app.handle();
                create_tray(handle)?;
                //app_autostart(handle);
            }
            let mut builder = tauri::Builder::default();
            #[cfg(desktop)]
            {
                builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
                    let _ = app.get_webview_window("main")
                        .expect("no main window")
                        .set_focus();
                }));
            }
            // 获取命令行参数，检查是否有文件关联打开的路径
            let args: Vec<String> = std::env::args().collect();
            let handle = app.handle().clone();
            if args.len() > 1 {
                let file_path = args[1..]
                    .iter()
                    .find(|arg| {
                        let ext = std::path::Path::new(arg)
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("");
                        //全部返回
                        true
                        //matches!(ext.to_lowercase().as_str(), "md" | "png" | "jpg" | "jpeg" | "pdf")
                    })
                    .cloned();

                if let Some(path) = file_path {
                    println!("打开文件 path {:?}", path);
                    app_openFile(&path, &app.handle());
                    let _ = handle.emit("file-opened-from-os", path);

                }
            }
            let mut main_window = match WebviewWindowBuilder::new(
                app,
                "main".to_string(),
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("MdViewer")
            .decorations(false)
            .inner_size(960.0, 640.0)
            .min_inner_size(640.0, 480.0)
            .center()
            .visible(false)
            // .menu(system_menu(&context))//窗口菜单
            .build()
            {
                Ok(it) => it,
                Err(err) => return Err(Box::new(err)),
            };
            //窗口阴影功能
            // #[cfg(any(target_os = "windows", target_os = "macos"))]
            // set_shadow(&main_window, true).expect("Unsupported platform!");

            // let mut loding_window = match WebviewWindowBuilder::new(
            //     app,
            //     "loding-window".to_string(),
            //     tauri::WebviewUrl::App("/Windows/Loding".into()),
            // )
            // .title("Loding")
            // .decorations(false)
            // .inner_size(400.0, 200.0)
            // .center()
            // .visible(true)
            // .build()
            // {
            //     Ok(it) => it,
            //     Err(err) => return Err(Box::new(err)),
            // };
            main_window.restore_state(StateFlags::all() ^ StateFlags::VISIBLE); // 恢复窗口状态，但不恢复可见性（由 openOnStartup 配置控制）
            // 注册主窗口到全局注册表
            {
                let meta = crate::window_manager::registry::create_window_meta(
                    "main".to_string(),
                    crate::entity::window::WindowRole::Main,
                    None,
                    "MdViewer".to_string(),
                    0.0, 0.0, 960.0, 640.0,
                );
                crate::window_manager::registry::register_window(meta);
            }
            app_settings(&app.handle());

            // 读取主窗口预设，判断是否显示主窗口（默认显示；有配置且 openOnStartup=false 时不显示）
            // 直接从磁盘读取 settings.json，避免 store 插件内存缓存与前端保存不同步的问题
            let show_main_window = {
                let mut result = true; // 默认显示
                if let Ok(config_dir) = app.path().app_data_dir() {
                    let settings_path = config_dir.join("settings.json");
                    if settings_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&settings_path) {
                            if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(main_preset) = settings.get("main_window_preset") {
                                    if let Some(open_on_startup) = main_preset.get("openOnStartup").and_then(|v| v.as_bool()) {
                                        result = open_on_startup;
                                    }
                                }
                            }
                        }
                    }
                }
                println!("[MdViewer] show_main_window = {}", result);
                result
            };
            if show_main_window {
                main_window.show().unwrap();
            } else {
                // 确保主窗口保持隐藏（restore_state 可能已恢复可见性）
                let _ = main_window.hide();
            }

            // 启动时自动创建 openOnStartup=true 的预设窗口（Rust 层直接创建，无需等待前端加载）
            {
                let handle = app.handle().clone();
                if let Ok(store) = handle.store("settings.json") {
                    if let Some(presets_val) = store.get("window_presets") {
                        if let Some(presets) = presets_val.as_array() {
                            for preset in presets {
                                let open_on_startup = preset.get("openOnStartup")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                if !open_on_startup {
                                    continue;
                                }
                                let preset_id = preset.get("id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let name = preset.get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let width = preset.get("width")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(960.0);
                                let height = preset.get("height")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(640.0);
                                let focus_mode = preset.get("focusMode")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                let enabled_plugins = preset.get("enabledPlugins")
                                    .and_then(|v| v.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|item| item.as_str().map(String::from))
                                            .collect::<Vec<String>>()
                                    })
                                    .unwrap_or_default();

                                crate::window_manager::commands::create_preset_window_from_preset(
                                    &handle,
                                    &preset_id,
                                    &name,
                                    width,
                                    height,
                                    enabled_plugins,
                                    focus_mode,
                                );
                            }
                        }
                    }
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
