// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use serde_json::json;
use tauri::App;
use tauri::AppHandle;
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_store::StoreExt;
use tauri_plugin_window_state::{StateFlags, WindowExt};

mod app;
mod app_setup;
mod entity;
mod response;
mod routes;
mod system_menu;
mod utils;

use crate::app::app::{app_autostart, app_settings};
use crate::app::tray::create_tray;
use crate::app_setup::setup::setup_windows;
use crate::routes::app_window::{
    about, loding_window_close, note_book_window, windows_close, windows_hide, windows_maximize,
    windows_minimize,
};
use crate::routes::file_utils::{file_manager_open_path, get_document_notebooks, up_folder_name};
use crate::routes::note_book::{
    create_folder, create_note_file, get_note_list, get_path_childrenFoldersAndFiles,
    read_note_file, remove_file, remove_folder, save_note, up_file_name,
};
struct Payload {
    args: Vec<String>,
    cwd: String,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            windows_close
        ])
        .setup(move |app: &mut App| {
            #[cfg(all(desktop))]
            {
                let handle = app.handle();
                create_tray(handle)?;
                //app_autostart(handle);
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
