#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::menu::{Menu, MenuItem, Submenu};
use tauri::utils::assets::EmbeddedAssets;
use tauri::{App, Context, Manager};
use tauri::{WebviewUrl, WebviewWindowBuilder};
use window_vibrancy::{apply_blur, apply_vibrancy, NSVisualEffectMaterial};

use crate::system_menu::system_menus::create_menu;
//窗口
pub fn setup_windows(app: &mut App) -> Result<(), Box<dyn std::error::Error + Send>> {
    let handle = app.handle();
    let world_item = MenuItem::with_id(handle, "world", "World", true, None::<&str>).unwrap();
    let save_menu_item = MenuItem::new(handle, "Save", true, None::<&str>).unwrap();
    let menu = Menu::with_items(
        handle,
        &[&Submenu::with_items(
            handle,
            "File",
            true,
            &[&save_menu_item, &world_item],
        ).unwrap()],
    ).unwrap();
    let mut webview_window = match WebviewWindowBuilder::new(
        app,
        "main-window".to_string(),
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("NoteBook")
    .inner_size(1280.0, 720.0)
    .position(200.0, 100.0)
    .visible(false)
    .menu(menu)
    .build()
    {
        Ok(it) => it,
        Err(err) => return Err(Box::new(err)),
    };
    let win = app.get_webview_window("main").unwrap();

    #[cfg(target_os = "macos")]
    apply_vibrancy(&win, NSVisualEffectMaterial::HudWindow, None, None)
        .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

    webview_window.on_menu_event(
        move |window: &tauri::Window, event: tauri::menu::MenuEvent| {
            if event.id == save_menu_item.id() {
                // save menu item
            }
        },
    );

    Ok(())
}
