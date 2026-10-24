// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use tauri::Manager;
use tauri::{App, Emitter, Listener};

mod app;
mod plugin_host;
mod plugins;
mod core;
mod entity;
mod response;
mod utils;
mod sqlite;
mod window_manager;
mod workspace;
mod session;
mod immersive_shortcut;
pub mod commands;

use crate::app::app_store::{app_open_file, app_settings, APP};
use crate::app::tray::create_tray;
use crate::commands::{
    about, close_window, create_editor_window, create_folder, draft_check, draft_delete,
    file_manager_open_path,
    count_file_lines, read_file_lines,
    draft_load, draft_save, ensure_dir, focus_window, get_window_count, get_window_list,
    kv_delete, kv_entries, kv_get, kv_set, loding_window_close, note_book_window, plugin_call,
    plugin_fs_roots, plugin_get_disabled, plugin_get_order, plugin_get_settings,
    plugin_install, plugin_last_stats, plugin_set_settings, plugin_list, plugin_runtime_info,
    plugin_set_enabled, plugin_set_memdump, plugin_set_order, plugin_uninstall,
    plugin_set_page_caps, session_clear, session_delete, session_delete_workspace,
    session_list, session_load, session_load_all, session_load_workspace, session_save,
    update_window_position_cmd, windows_close, windows_hide, windows_maximize,
    windows_minimize, workspace_create, workspace_delete, workspace_get, workspace_get_default,
    workspace_add_plugin_external_dir, workspace_get_plugin_dir, workspace_get_plugin_subdir,
    workspace_list, workspace_list_plugin_external_dirs, workspace_lock, workspace_lock_status,
    workspace_remove_plugin_external_dir, workspace_unlock, workspace_update,
};
#[cfg(feature = "plugin-database")]
use crate::commands::{
    db_catalog_capabilities, db_connect, db_connection_create, db_connection_delete,
    db_connection_list, db_connection_test, db_connection_update, db_apply_edits,
    db_disconnect, db_execute_sql, db_export_csv, db_export_excel, db_export_sql_csv,
    db_export_sql_excel, db_import_csv, db_import_excel, db_list_admins, db_list_database_indexes,
    db_list_databases, db_list_events, db_list_functions, db_list_procedures, db_list_tables,
    db_list_triggers, db_list_users, db_list_views, db_object_source, db_query_table_data,
    db_show_status, db_show_variables, db_table_columns, db_table_indexes, db_table_schema,
    db_preview_edits,
};
#[cfg(feature = "plugin-files")]
use crate::commands::{
    create_note_file, detect_file_encoding, get_document_notebooks,
    get_file_size, get_note_list, get_path_childrenFoldersAndFiles, list_script_files,
    open_default_apps_settings,
    read_note_file, read_file_bytes, register_file_association, remove_file, remove_folder,
    rename_script_file, save_note, up_file_name, up_folder_name, write_script_file,
};
#[cfg(feature = "plugin-logs")]
use crate::commands::{
    logs_index_build, logs_index_drop, logs_index_export, logs_cancel, logs_query_lines,
    logs_stream_start, logs_stream_stop,
};
#[cfg(feature = "plugin-mat")]
use crate::commands::{
    mat_hprof_header, mat_hprof_histogram, mat_leak_suspects, mat_analysis_limits,
    mat_refs_build, mat_refs_inspect, mat_refs_instances, mat_refs_release,
    mat_refs_search_classes,
};
#[cfg(feature = "plugin-ssh")]
use crate::commands::{
    connection_create, connection_delete, connection_edit, connection_list, get_app_dir,
    local_list_dir, sftp_check, sftp_connect, sftp_disconnect, sftp_download, sftp_list_dir,
    sftp_mkdir, sftp_remove, sftp_rename, sftp_upload, logs_pull_remote, ssh_check,
    ssh_connect, ssh_disconnect, ssh_resize_pty, ssh_write,
};
#[cfg(feature = "plugin-terminal")]
use crate::commands::{
    terminal_alive, terminal_close, terminal_defaults, terminal_list_shells, terminal_open,
    terminal_resize, terminal_write,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 沉浸式快捷键状态（所有窗口共享一份，详见 immersive_shortcut.rs）
    let immersive_state = std::sync::Arc::new(std::sync::Mutex::new(
        immersive_shortcut::ImmersiveState::default(),
    ));
    // builder 上的焦点监听闭包要用（原变量留给 setup 闭包）
    let immersive_state_focus = immersive_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 二次启动实例时唤回主窗口（原实现误写在被丢弃的 builder 上，从未生效）。
            // 走与托盘同一个入口：主窗口若已被关闭，这里会**重建**它。
            crate::window_manager::main_window::show_main_window(app);
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // 全局窗口销毁监听：释放工作区锁并从注册表移除（覆盖所有窗口，含主窗口）
        .on_window_event(move |window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 主窗口：关闭 = 隐藏到托盘（close-to-tray）。只有进程退出（托盘「退出」→
                // `app.exit`）才真的关掉它 —— `app.exit` 走 `ExitRequested`，不经过
                // `CloseRequested`，所以这里拦得住用户的关闭、拦不住退出。
                // 其余窗口（editor-* / app-window-*）照常关闭。
                if crate::window_manager::main_window::should_hide_instead_of_close(window.label()) {
                    api.prevent_close();
                    let _ = window.hide();
                }
                return;
            }
            if let tauri::WindowEvent::Destroyed = event {
                let label = window.label().to_string();
                crate::window_manager::registry::unregister_window(&label);
                let _ = window.app_handle().emit("window-destroyed", &label);
                // 清掉该窗口的沉浸/焦点标记，避免带着沉浸式被关掉的窗口把 Esc 永久占住
                immersive_shortcut::handle_window_destroyed(
                    &window.app_handle(),
                    &label,
                    &immersive_state_focus,
                );
            } else if let tauri::WindowEvent::Focused(focused) = event {
                // 焦点变化驱动沉浸式快捷键的注册/注销：本应用有焦点时才拦截 F11，
                // 避免后台运行时抢走其它程序的 F11（详见 immersive_shortcut.rs）
                immersive_shortcut::handle_focus(window, *focused, &immersive_state_focus);
            }
        })
        .invoke_handler(tauri::generate_handler![
            
    #[cfg(feature = "plugin-files")]
    get_document_notebooks,
            
            
    #[cfg(feature = "plugin-files")]
    get_note_list,
            
    #[cfg(feature = "plugin-files")]
    get_file_size,
            
    #[cfg(feature = "plugin-files")]
    read_note_file,
            
    #[cfg(feature = "plugin-files")]
    read_file_bytes,
            create_folder,
            ensure_dir,
            
    #[cfg(feature = "plugin-files")]
    create_note_file,
    #[cfg(feature = "plugin-files")]
    get_path_childrenFoldersAndFiles,
    
    #[cfg(feature = "plugin-files")]
    list_script_files,
            
    #[cfg(feature = "plugin-files")]
    rename_script_file,
            
    #[cfg(feature = "plugin-files")]
    write_script_file,
            
    #[cfg(feature = "plugin-files")]
    save_note,
            
    #[cfg(feature = "plugin-files")]
    up_file_name,
            
    #[cfg(feature = "plugin-files")]
    up_folder_name,
            
    #[cfg(feature = "plugin-files")]
    remove_folder,
            
    #[cfg(feature = "plugin-files")]
    remove_file,
            loding_window_close,
            file_manager_open_path,
            count_file_lines, read_file_lines,
            about,
            note_book_window,
            windows_hide,
            windows_minimize,
            windows_maximize,
            windows_close,
            
                
                
    #[cfg(feature = "plugin-files")]
    detect_file_encoding,
            // 日志工作台：服务端索引与筛选（logs 插件）
            
    #[cfg(feature = "plugin-logs")]
    logs_index_build,
            
    #[cfg(feature = "plugin-logs")]
    logs_query_lines,
            
    #[cfg(feature = "plugin-logs")]
    logs_cancel,
            
    #[cfg(feature = "plugin-logs")]
    logs_index_export,
            
    #[cfg(feature = "plugin-logs")]
    logs_index_drop,
            
    #[cfg(feature = "plugin-files")]
    register_file_association,
            
    #[cfg(feature = "plugin-files")]
    open_default_apps_settings,
            
    #[cfg(feature = "plugin-ssh")]
    ssh_connect,
            
    #[cfg(feature = "plugin-ssh")]
    ssh_write,
            
    #[cfg(feature = "plugin-ssh")]
    ssh_disconnect,
            
    #[cfg(feature = "plugin-ssh")]
    ssh_resize_pty,
            
    #[cfg(feature = "plugin-ssh")]
    ssh_check,
            
    #[cfg(feature = "plugin-ssh")]
    sftp_connect,
            
    #[cfg(feature = "plugin-ssh")]
    sftp_disconnect,
            
    #[cfg(feature = "plugin-ssh")]
    sftp_check,
            
    #[cfg(feature = "plugin-ssh")]
    sftp_list_dir,
            
    #[cfg(feature = "plugin-ssh")]
    sftp_mkdir,
            
    #[cfg(feature = "plugin-ssh")]
    sftp_remove,
            
    #[cfg(feature = "plugin-ssh")]
    sftp_upload,
            
    #[cfg(feature = "plugin-ssh")]
    sftp_download,
            
    #[cfg(feature = "plugin-ssh")]
    logs_pull_remote,
            
    #[cfg(feature = "plugin-logs")]
    logs_stream_start,
            
    #[cfg(feature = "plugin-logs")]
    logs_stream_stop,
            
    #[cfg(feature = "plugin-ssh")]
    local_list_dir,
            
    #[cfg(feature = "plugin-ssh")]
    sftp_rename,
            
    #[cfg(feature = "plugin-ssh")]
    get_app_dir,
            
    #[cfg(feature = "plugin-ssh")]
    connection_create,
            
    #[cfg(feature = "plugin-ssh")]
    connection_delete,
            
    #[cfg(feature = "plugin-ssh")]
    connection_edit,
            
    #[cfg(feature = "plugin-ssh")]
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
            
    #[cfg(feature = "plugin-mat")]
    mat_hprof_header,
            
    #[cfg(feature = "plugin-mat")]
    mat_hprof_histogram,
            
    #[cfg(feature = "plugin-mat")]
    mat_leak_suspects,
            
    #[cfg(feature = "plugin-mat")]
    mat_analysis_limits,
            
    #[cfg(feature = "plugin-mat")]
    mat_refs_build,
            
    #[cfg(feature = "plugin-mat")]
    mat_refs_release,
            
    #[cfg(feature = "plugin-mat")]
    mat_refs_search_classes,
            
    #[cfg(feature = "plugin-mat")]
    mat_refs_instances,
            
    #[cfg(feature = "plugin-mat")]
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
            
    #[cfg(feature = "plugin-database")]
    db_connection_create,
            
    #[cfg(feature = "plugin-database")]
    db_connection_delete,
            
    #[cfg(feature = "plugin-database")]
    db_connection_update,
            
    #[cfg(feature = "plugin-database")]
    db_connection_list,
            
    #[cfg(feature = "plugin-database")]
    db_connection_test,
            
    #[cfg(feature = "plugin-database")]
    db_connect,
            
    #[cfg(feature = "plugin-database")]
    db_disconnect,
            // 数据库操作
            
    #[cfg(feature = "plugin-database")]
    db_execute_sql,
            
    #[cfg(feature = "plugin-database")]
    db_query_table_data,
            
            // 结果网格编辑：批量写 + 预览（同一份 SQL 生成逻辑）
            
    #[cfg(feature = "plugin-database")]
    db_apply_edits,
            
    #[cfg(feature = "plugin-database")]
    db_preview_edits,
            // 元数据查询
            
    #[cfg(feature = "plugin-database")]
    db_list_databases,
            
    #[cfg(feature = "plugin-database")]
    db_list_tables,
            
    #[cfg(feature = "plugin-database")]
    db_table_columns,
            
    #[cfg(feature = "plugin-database")]
    db_table_indexes,
            
    #[cfg(feature = "plugin-database")]
    db_table_schema,
            // 目录能力声明与对象浏览（走 catalog 抽象，各库按自身能力实现）
            
    #[cfg(feature = "plugin-database")]
    db_catalog_capabilities,
            
    #[cfg(feature = "plugin-database")]
    db_list_views,
            
    #[cfg(feature = "plugin-database")]
    db_list_procedures,
            
    #[cfg(feature = "plugin-database")]
    db_list_functions,
            
    #[cfg(feature = "plugin-database")]
    db_object_source,
            
    #[cfg(feature = "plugin-database")]
    db_list_triggers,
            
    #[cfg(feature = "plugin-database")]
    db_list_events,
            
    #[cfg(feature = "plugin-database")]
    db_list_database_indexes,
            
    #[cfg(feature = "plugin-database")]
    db_list_users,
            
    #[cfg(feature = "plugin-database")]
    db_list_admins,
            
    #[cfg(feature = "plugin-database")]
    db_show_status,
            
    #[cfg(feature = "plugin-database")]
    db_show_variables,
            // 导入导出
            
    #[cfg(feature = "plugin-database")]
    db_export_csv,
            
    #[cfg(feature = "plugin-database")]
    db_export_excel,
            
    #[cfg(feature = "plugin-database")]
    db_export_sql_csv,
            
    #[cfg(feature = "plugin-database")]
    db_export_sql_excel,
            
    #[cfg(feature = "plugin-database")]
    db_import_csv,
            
    #[cfg(feature = "plugin-database")]
    db_import_excel,
            // 第三方插件（安装 / 卸载 / 排序 / 调用）
            plugin_list,
            plugin_install,
            plugin_uninstall,
            plugin_set_enabled,
            plugin_get_disabled,
            plugin_get_order,
            plugin_set_order,
            plugin_set_page_caps,
            plugin_call,
            plugin_runtime_info,
            plugin_last_stats,
            plugin_fs_roots,
            plugin_get_settings,
            plugin_set_settings,
            plugin_set_memdump,
            // 本地交互式终端（terminal 插件）：PTY 会话 + Channel 流式输出
            
    #[cfg(feature = "plugin-terminal")]
    terminal_open,
            
    #[cfg(feature = "plugin-terminal")]
    terminal_write,
            
    #[cfg(feature = "plugin-terminal")]
    terminal_resize,
            
    #[cfg(feature = "plugin-terminal")]
    terminal_close,
            
    #[cfg(feature = "plugin-terminal")]
    terminal_alive,
            
    #[cfg(feature = "plugin-terminal")]
    terminal_list_shells,
            
    #[cfg(feature = "plugin-terminal")]
    terminal_defaults,
        ])
        // 第三方插件页面协议：plugin://localhost/<plugin_id>/<包内相对路径>
        // host 必须固定为 localhost：Windows/Android 下 wry 会把它改写成 http://plugin.localhost/*，
        // 插件 ID 走路径首段才能跨平台一致（见 plugin_host::split_plugin_uri）。
        .register_uri_scheme_protocol("plugin", |_ctx, request| {
            let uri = request.uri();
            let (plugin_id, rel) = crate::plugin_host::host::split_plugin_uri(
                uri.host().unwrap_or_default(),
                uri.path(),
            );
            // Range：`<audio>` / `<video>` 拖动进度条会发 `Range: bytes=...`。之前一律 200 全量返回
            // → 大文件整份进内存且无法 seek（包内 `.data` 与私有数据目录里的音视频都受影响）。
            let range = request
                .headers()
                .get("range")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            let (status, mime, body, content_range) =
                match crate::plugin_host::host::read_plugin_asset_response(
                    &plugin_id,
                    &rel,
                    range.as_deref(),
                ) {
                    Ok(r) => (r.status, r.mime, r.body, r.content_range),
                    Err(e) => {
                        println!("[plugin://] 拒绝 /{}/{}: {}", plugin_id, rel, e);
                        (404u16, "text/plain; charset=utf-8", Vec::new(), None)
                    }
                };
            let mut builder = tauri::http::Response::builder()
                .status(status)
                .header("Content-Type", mime)
                // 插件包会被覆盖安装（升级）：禁掉缓存，否则页面可能仍跑旧文件
                .header("Cache-Control", "no-store")
                // 同源资源声明（声明了 isolate 的页面会带 COEP: require-corp，
                // 子资源按同源放行，这里显式声明一次更稳）
                .header("Cross-Origin-Resource-Policy", "same-origin")
                // 声明支持分段：浏览器看到它才会对媒体资源发 Range
                .header("Accept-Ranges", "bytes");

            // 206 / 416 必须带 Content-Range，否则浏览器不认这段响应
            if let Some(cr) = &content_range {
                builder = builder.header("Content-Range", cr.as_str());
            }

            // 跨源子资源（图标 / 图片 / WASM 等）需要 CORS 头才允许**宿主页面**
            // 把它用作 CSS `mask-image`。mask 要采样像素，Chromium 强制要求 CORS，
            // 否则 `Access-Control-Allow-Origin` 缺失 → 资源整体加载失败（连 `<img>` 之外的
            // 着色方式都没法用，第三方插件图标也就无法随主题变色）。
            // 只对**非 HTML** 资源放开：HTML 文档保持无 ACAO，隔离插件的页面安全边界不被削弱。
            if !mime.starts_with("text/html") {
                builder = builder.header("Access-Control-Allow-Origin", "*");
            }

            // CSP 只在 **HTML 文档**响应上拼：子资源响应里的 CSP 头浏览器会忽略，
            // 没必要为每个资源都去读一次清单。策略由该插件清单的 pageCapabilities 决定，
            // 未声明任何能力 = 与历史逐字一致的最严 CSP（见 plugin_host::page_policy）。
            if mime.starts_with("text/html") {
                let policy = crate::plugin_host::host::page_policy(&plugin_id);
                builder = builder.header("Content-Security-Policy", policy.csp);
                if policy.isolate {
                    // 跨源隔离：只有**顶层窗口**能靠这两个头生效（iframe 里加了也没用，
                    // 因为父页面自己没隔离）→ 解锁 SharedArrayBuffer，多线程 WASM 引擎才跑得起来
                    builder = builder
                        .header("Cross-Origin-Opener-Policy", "same-origin")
                        .header("Cross-Origin-Embedder-Policy", "require-corp");
                }
            }
            builder
                .body(body)
                .unwrap_or_else(|_| {
                    tauri::http::Response::builder()
                        .status(500)
                        .body(Vec::<u8>::new())
                        .unwrap()
                })
        })
        .setup(move |app: &mut App| {
            // 初始化 app 数据目录与配置目录（纯路径，无 tauri 依赖）
            let home_dir = app.path().home_dir().unwrap_or_default();
            let config_dir = app.path().app_config_dir().unwrap_or_default();
            // 会顺带把旧的 `<home>/app` 迁到卸载够不着的规范位置
            APP.lock().update_dirs(&home_dir, &config_dir);
            // 第三方插件存储一次性迁移：config_dir 会被卸载脚本删，必须落到 app_dir
            crate::plugin_host::host::migrate_third_party_storage();
            // 内置插件默认状态（首次运行只开 files，其余默认关闭；幂等，写 settings.json）
            crate::plugin_host::host::ensure_plugin_defaults();
            // 初始化工作区和会话目录
            crate::workspace::config::ensure_workspaces_dir();
            // 默认工作区文件夹一次性清理：早期版本把 app_dir（含旧版 <home>/app）本身当默认工作区，
            // 既会把内部资产（app.db / sessions / plugin_data …）暴露进用户文件树，
            // 也会在目录搬迁后留下指向旧位置的悬空路径。清掉后由用户在 files 侧边栏自选目录
            crate::workspace::config::migrate_default_workspace_folder(&home_dir);
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
            // 主窗口的**唯一创建点**：配置集中在 window_manager::main_window，
            // 启动 / 托盘唤回 / 二次启动三条路共用同一份（含注册进窗口注册表）。
            // 初始隐藏，等待前端 splash-screen-ready 通知。
            let main_window = match crate::window_manager::main_window::build_main_window(app.handle())
            {
                Ok(it) => it,
                Err(err) => return Err(Box::new(err)),
            };

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
            app_settings();

            // 沉浸式快捷键：F11 / Esc 走 OS 层热键，穿透第三方插件沙箱 iframe 的事件边界。
            // 启动时先对齐一次（主窗口假定已聚焦，便于 F11 立即可用）；之后由
            // 「窗口焦点变化」与「前端上报 immersive-state」两个入口驱动注册/注销。
            immersive_shortcut::reconcile(app.handle(), &mut *immersive_state.lock().unwrap());
            {
                let apph = app.handle().clone();
                let st = immersive_state.clone();
                // 注意：Tauri 2 的 listen 没有 payload 泛型，回调收到的是 `tauri::Event`
                app.listen("immersive-state", move |event| {
                    immersive_shortcut::handle_immersive_state(&apph, event.payload(), &st);
                });
            }

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
            // 注意：**按 label 现查**窗口，绝不缓存 `main_window` 句柄 —— 主窗口可能已被
            // 关闭并重建，缓存下来的 WebviewWindow 会指向作废的窗口（show 静默失败）。
            let app_handle = app.handle().clone();
            let app_for_splash = app_handle.clone();
            app_handle.listen("splash-screen-ready", move |_event| {
                println!("[SplashScreen] Frontend ready, showing main window");
                if let Some(win) = app_for_splash
                    .get_webview_window(crate::window_manager::main_window::MAIN_WINDOW_LABEL)
                {
                    let _ = win.show();
                }
            });

            // 如果配置为立即显示，则直接显示（不等待 splash 事件）
            if show_main_window {
                let _ = main_window.show();
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
