#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app_setup;
mod entity;
mod response;
mod routes;
mod system_menu;
mod system_tray;
mod system_window_event;
mod utils;

use system_menu::system_menus::{handler, system_menu};
use system_tray::system_trays::{system_tray, system_tray_event};

use system_window_event::system_window_events::system_on_window_event;
use tauri::utils::assets::EmbeddedAssets;
use tauri::window::WindowBuilder;
use tauri::{App, Context, Manager, Runtime};
use window_vibrancy::{apply_blur, apply_vibrancy, NSVisualEffectMaterial};

use window_shadows::set_shadow;

use crate::app_setup::setup::setup_windows;
use crate::routes::app_window::{loding_window_close,about,windows_hide,windows_minimize,windows_maximize,windows_close};
use crate::routes::file_utils::get_document_notebooks;
use crate::routes::note_book::{
    create_note_file, create_notebook, get_note_list, read_note_file,
    remove_note, remove_notebook, save_note,
};
struct Payload {
    args: Vec<String>,
    cwd: String,
}
fn main() {
    //单例启动
    #[cfg(target_os = "windows")]
    make_sure_single_case();
    let context = tauri::generate_context!();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_document_notebooks,
            get_note_list,
            read_note_file,
            create_notebook,
            create_note_file,
            save_note,
            remove_notebook,
            remove_note,
            loding_window_close,
            about,
            windows_hide,
            windows_minimize,
            windows_maximize,
            windows_close
        ])
        .setup(move |app: &mut App| {
            let mut main_window = match WindowBuilder::new(
                app,
                "main-window".to_string(),
                tauri::WindowUrl::App("index.html".into()),
            )
            .title("MdNote")
            .decorations(false)
            .inner_size(1280.0, 720.0)
            .center()
            .visible(false)
            // .menu(system_menu(&context))//窗口菜单
            .build()
            {
                Ok(it) => it,
                Err(err) => return Err(Box::new(err)),
            };
            //窗口阴影功能
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            set_shadow(&main_window, true).expect("Unsupported platform!");

            let mut loding_window = match WindowBuilder::new(
                app,
                "loding-window".to_string(),
                tauri::WindowUrl::App("/Windows/Loding".into()),
            )
            .title("Loding")
            .decorations(false)
            .inner_size(400.0, 200.0)
            .center()
            .visible(true)
            .build()
            {
                Ok(it) => it,
                Err(err) => return Err(Box::new(err)),
            };
            //窗口阴影功能
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            set_shadow(&loding_window, true).expect("Unsupported platform!");

            // let win = app.get_window("main-window").unwrap();

            // #[cfg(target_os = "macos")]
            // apply_vibrancy(&win, NSVisualEffectMaterial::HudWindow, None, None)
            //   .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

            // #[cfg(target_os = "windows")]
            // apply_blur(&win, Some((18, 18, 18, 125)))
            //   .expect("Unsupported platform! 'apply_blur' is only supported on Windows");

            // let window_ = window.clone();
            // //窗口菜单
            // window.on_menu_event(move |event| match event.menu_item_id() {
            //     "quit" => {
            //         std::process::exit(0);
            //     }
            //     "close" => {
            //         window_.close().unwrap();
            //     }
            //     _ => {}
            // });
            //set_window_shadow(app);
            Ok(())
        })
        //.on_window_event(|event| system_on_window_event(event))//窗口
        //.menu(system_menu(&context))//菜单 全局
        //.on_menu_event(handler)//菜单事件 全局
        .system_tray(system_tray())
        .on_system_tray_event(|app, event| system_tray_event(app, event))
        .run(tauri::generate_context!())
        .expect("tauri程序运行失败");
}

#[cfg(target_os = "windows")]
use windows::{
    w,
    Win32::{
        Foundation::{GetLastError, WIN32_ERROR},
        Globalization::GetUserDefaultUILanguage,
        System::Threading::{CreateMutexW, OpenMutexW, SYNCHRONIZATION_ACCESS_RIGHTS},
        UI::WindowsAndMessaging::{MessageBoxW, MB_OK},
    },
};
/// make sure app running at single-case
#[cfg(target_os = "windows")]
pub fn make_sure_single_case() {
    unsafe {
        let _ = OpenMutexW(
            SYNCHRONIZATION_ACCESS_RIGHTS(0),
            true,
            w!("link.xsa.bs@fuyoo"),
        );
        let WIN32_ERROR(code) = GetLastError();
        if code == 2 {
            // generate lock
            let _ = CreateMutexW(None, true, w!("link.xsa.bs@fuyoo"));
        } else {
            // get local language
            let lang = GetUserDefaultUILanguage();
            // Adapt to Chinese and english
            if lang == 2052u16 {
                MessageBoxW(
                    None,
                    w!("Rd 正在运行中, 请不要重复运行！"),
                    w!("提示"),
                    MB_OK,
                );
            } else {
                MessageBoxW(
                    None,
                    w!("Rd is Running, Please do not run it again!"),
                    w!("Tips"),
                    MB_OK,
                );
            }
            std::process::exit(0);
        }
    }
}
