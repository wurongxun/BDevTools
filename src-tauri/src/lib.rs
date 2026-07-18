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

use crate::app::app_store::{app_autostart, app_settings,app_openFile};
use crate::app::tray::create_tray;
use crate::app_setup::setup::setup_windows;
use crate::command::app_window::{
    about, loding_window_close, note_book_window, windows_close, windows_hide, windows_maximize,
    windows_minimize,
};
use crate::command::file_command::{
    create_folder, create_note_file, get_note_list, get_path_childrenFoldersAndFiles,
    read_note_file, remove_file, remove_folder, save_note, up_file_name,
};
use crate::command::file_utils::{file_manager_open_path, get_document_notebooks, up_folder_name,detect_file_encoding};
use crate::command::file_association::{register_file_association, open_default_apps_settings};

use crate::command::search_log_parallel::{
    read_file_lines, count_file_lines, search_log_index, search_log_jump_to, search_log_page,
    search_log_parallel, search_log_preview,
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
        .invoke_handler(tauri::generate_handler![
            get_document_notebooks,
            file_manager_open_path,
            get_note_list,
            read_note_file,
            create_folder,
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
            open_default_apps_settings
        ])
        .setup(move |app: &mut App| {
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
            main_window.restore_state(StateFlags::all()); // will restore the window's state from disk
            app_settings(&app.handle());
            main_window.show().unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
