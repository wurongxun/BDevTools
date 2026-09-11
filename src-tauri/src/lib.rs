// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use tauri::Manager;
use tauri::{App, Emitter, Listener};
use tauri::WebviewWindowBuilder;
use tauri_plugin_window_state::{StateFlags, WindowExt};

mod app;
mod command;
mod core;
mod entity;
mod response;
mod utils;
mod sqlite;
mod window_manager;
mod workspace;
mod session;
mod database;
pub mod commands;

use crate::app::app_store::{app_open_file, app_settings, APP};
use crate::app::tray::create_tray;
use crate::commands::{
    about, close_window, connection_create, connection_delete, connection_edit,
    connection_list, count_file_lines, create_editor_window, create_folder,
    create_note_file, db_connect, db_connection_create, db_connection_delete,
    db_connection_list, db_connection_test, db_connection_update, db_delete_row,
    db_disconnect, db_execute_sql, db_export_csv, db_export_excel, db_export_sql_csv,
    db_export_sql_excel, db_import_csv, db_import_excel, db_insert_row, db_list_databases, db_list_tables,
    db_query_table_data, db_table_columns, db_table_indexes, db_table_schema,
    db_update_row, detect_file_encoding, draft_check, draft_delete, draft_load,
    draft_save, ensure_dir, file_manager_open_path, focus_window, get_app_dir,
    get_document_notebooks, get_file_size, get_note_list, get_path_childrenFoldersAndFiles,
    get_window_count, get_window_list, kv_delete, kv_entries, kv_get, kv_set,
    list_script_files,     local_list_dir, logs_index_build, logs_index_drop,
    logs_index_export, logs_index_query,
    loding_window_close, note_book_window, mat_hprof_header, mat_hprof_histogram,
    mat_leak_suspects,
    mat_analysis_limits,
    mat_refs_build, mat_refs_inspect, mat_refs_instances, mat_refs_release,
    mat_refs_search_classes,
    open_default_apps_settings, read_file_lines,
    read_note_file, register_file_association, remove_file, remove_folder,
    rename_script_file, save_note, search_log_index, search_log_jump_to, search_log_page,
    search_log_parallel, search_log_preview, session_clear, session_delete,
    session_delete_workspace, session_list, session_load, session_load_all,
    session_load_workspace, session_save, sftp_check, sftp_connect, sftp_disconnect,
    sftp_download, sftp_list_dir, sftp_mkdir, sftp_remove, sftp_rename, sftp_upload,
    logs_pull_remote,
    logs_stream_start, logs_stream_stop,
    ssh_check, ssh_connect, ssh_disconnect, ssh_resize_pty, ssh_write, up_file_name,
    up_folder_name, update_window_position_cmd, windows_close, windows_hide,
    windows_maximize, windows_minimize, workspace_create, workspace_delete,
    workspace_get, workspace_get_default,
    workspace_add_plugin_external_dir, workspace_get_plugin_dir,
    workspace_get_plugin_subdir, workspace_list, workspace_list_plugin_external_dirs,
    workspace_lock, workspace_lock_status, workspace_remove_plugin_external_dir,
    workspace_unlock, workspace_update,
    write_script_file,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 二次启动实例时聚焦主窗口（原实现误写在被丢弃的 builder 上，从未生效）
            let _ = app.get_webview_window("main").map(|w| w.set_focus());
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        // 全局窗口销毁监听：释放工作区锁并从注册表移除（覆盖所有窗口，含主窗口）
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                let label = window.label().to_string();
                crate::window_manager::registry::unregister_window(&label);
                let _ = window.app_handle().emit("window-destroyed", &label);
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_document_notebooks,
            file_manager_open_path,
            get_note_list,
            get_file_size,
            read_note_file,
            create_folder,
            ensure_dir,
            create_note_file,
            get_path_childrenFoldersAndFiles,
            list_script_files,
            rename_script_file,
            write_script_file,
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
            // 日志工作台：服务端索引与筛选（logs 插件）
            logs_index_build,
            logs_index_query,
            logs_index_export,
            logs_index_drop,
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
            logs_pull_remote,
            logs_stream_start,
            logs_stream_stop,
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
            // 工作区锁
            workspace_lock,
            workspace_unlock,
            workspace_lock_status,
            // 插件工作目录
            workspace_get_plugin_dir,
            workspace_get_plugin_subdir,
            // 外部脚本目录（用户级，跨工作区共享）
            workspace_list_plugin_external_dirs,
            workspace_add_plugin_external_dir,
            workspace_remove_plugin_external_dir,
            // 通用 KV 存储（替代 tauri-plugin-store）
            kv_get,
            kv_set,
            kv_delete,
            kv_entries,
            // Java 堆转储分析（mat 插件）
            mat_hprof_header,
            mat_hprof_histogram,
            mat_leak_suspects,
            mat_analysis_limits,
            mat_refs_build,
            mat_refs_release,
            mat_refs_search_classes,
            mat_refs_instances,
            mat_refs_inspect,
            // 会话管理（主键：工作区 ID）
            session_save,
            session_load,
            session_load_workspace,
            session_delete_workspace,
            session_load_all,
            session_list,
            session_delete,
            session_clear,
            // 草稿管理（未保存内容保护，按工作区隔离）
            draft_save,
            draft_check,
            draft_load,
            draft_delete,
            // 数据库连接管理
            db_connection_create,
            db_connection_delete,
            db_connection_update,
            db_connection_list,
            db_connection_test,
            db_connect,
            db_disconnect,
            // 数据库操作
            db_execute_sql,
            db_query_table_data,
            db_insert_row,
            db_update_row,
            db_delete_row,
            // 元数据查询
            db_list_databases,
            db_list_tables,
            db_table_columns,
            db_table_indexes,
            db_table_schema,
            // 导入导出
            db_export_csv,
            db_export_excel,
            db_export_sql_csv,
            db_export_sql_excel,
            db_import_csv,
            db_import_excel,
        ])
        .setup(move |app: &mut App| {
            // 初始化 app 数据目录与配置目录（纯路径，无 tauri 依赖）
            let home_dir = app.path().home_dir().unwrap_or_default();
            let config_dir = app.path().app_config_dir().unwrap_or_default();
            APP.lock().update_dirs(&home_dir, &config_dir);
            // 初始化工作区和会话目录
            crate::workspace::config::ensure_workspaces_dir();
            crate::session::snapshot::ensure_sessions_dir();
            crate::workspace::plugin_dir::ensure_plugin_data_dir();
            #[cfg(all(desktop))]
            {
                let handle = app.handle();
                create_tray(handle)?;
            }
            // 获取命令行参数，检查是否有文件关联打开的路径
            let args: Vec<String> = std::env::args().collect();
            let handle = app.handle().clone();
            // 首个非程序路径参数即待打开文件（原实现逐个探测扩展名但谓词恒真，等价于取 args[1]）
            if let Some(path) = args.get(1).cloned() {
                println!("打开文件 path {:?}", path);
                app_open_file(&path);
                let _ = handle.emit("file-opened-from-os", path);
            }
            let main_window = match WebviewWindowBuilder::new(
                app,
                "main".to_string(),
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("BackendDevTools")
            .decorations(false)
            // 无边框窗口必须启用阴影，否则 Windows 下 DWM 会在左/右/底边露出 1px 黑线
            .shadow(true)
            .inner_size(960.0, 640.0)
            .min_inner_size(640.0, 480.0)
            .center()
            .visible(false) // Tauri 2.0 Splash Screen: 初始隐藏，等待前端通知
            .build()
            {
                Ok(it) => it,
                Err(err) => return Err(Box::new(err)),
            };
            // 主窗口注册（阴影已在 builder 中启用）
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
            let _ = main_window.restore_state(StateFlags::all() ^ StateFlags::VISIBLE); // 恢复窗口状态，但不恢复可见性（由 openOnStartup 配置控制）
            // 注册主窗口到全局注册表
            {
                let meta = crate::window_manager::registry::create_window_meta(
                    "main".to_string(),
                    crate::entity::window::WindowRole::Main,
                    None,
                    "BackendDevTools".to_string(),
                    Some("main".to_string()),
                    0.0, 0.0, 960.0, 640.0,
                );
                crate::window_manager::registry::register_window(meta);
            }
            app_settings();

            // 读取主窗口预设，判断是否显示主窗口（默认显示；有配置且 openOnStartup=false 时不显示）
            // 直接从磁盘读取 settings.json，避免 store 插件内存缓存与前端保存不同步的问题
            let show_main_window = {
                let mut result = true; // 默认显示
                let settings = crate::app::app_store::read_settings();
                if let Some(main_preset) = settings.get("main_window_preset") {
                    if let Some(open_on_startup) =
                        main_preset.get("openOnStartup").and_then(|v| v.as_bool())
                    {
                        result = open_on_startup;
                    }
                }
                println!("[BackendDevTools] show_main_window = {}", result);
                result
            };
            // Tauri 2.0 Splash Screen: 监听前端启动完成事件
            let app_handle = app.handle().clone();
            let main_window_clone = main_window.clone();
            app_handle.listen("splash-screen-ready", move |_event| {
                println!("[SplashScreen] Frontend ready, showing main window");
                let _ = main_window_clone.show();
            });

            // 如果配置为立即显示，则直接显示（不等待 splash 事件）
            if show_main_window {
                main_window.show().unwrap();
            } else {
                // 确保主窗口保持隐藏（restore_state 可能已恢复可见性）
                let _ = main_window.hide();
            }

            // 启动时自动创建 openOnStartup=true 的预设窗口（Rust 层直接创建，无需等待前端加载）
            {
                let handle = app.handle().clone();
                let presets_val = crate::app::app_store::read_settings().get("window_presets").cloned();
                if let Some(presets) = presets_val.as_ref().and_then(|v| v.as_array()) {
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
                        // 预设绑定的工作区（可选；空字符串视为未绑定）
                        let workspace_id = preset.get("workspaceId")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .filter(|s| !s.is_empty());

                        crate::window_manager::commands::create_preset_window_from_preset(
                            &handle,
                            &preset_id,
                            &name,
                            width,
                            height,
                            enabled_plugins,
                            focus_mode,
                            workspace_id,
                        );
                    }
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
