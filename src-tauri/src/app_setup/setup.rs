#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
  )]
use tauri::utils::assets::EmbeddedAssets;
use tauri::window::WindowBuilder;
use tauri::{App, Context, Manager};
use window_vibrancy::{apply_blur, apply_vibrancy, NSVisualEffectMaterial};


use crate::system_menu::system_menus::system_menu;
//窗口
pub fn setup_windows(
    app: &mut App,
    context: Context<EmbeddedAssets>,
) -> Result<(), Box<dyn std::error::Error + Send>> {
    let mut window = match WindowBuilder::new(
        app,
        "main-window".to_string(),
        tauri::WindowUrl::App("index.html".into()),
    ) .title("NoteBook")
    .inner_size(1280.0, 720.0)
    .position(200.0, 100.0)
    .visible(false)
    .menu(system_menu(&context))
    .build()
    {
        Ok(it) => it,
        Err(err) => return Err(Box::new(err)),
    };
    let win = app.get_window("main").unwrap();

    #[cfg(target_os = "macos")]
    apply_vibrancy(&win, NSVisualEffectMaterial::HudWindow, None, None)
      .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

    #[cfg(target_os = "windows")]
    apply_blur(&win, Some((18, 18, 18, 125)))
      .expect("Unsupported platform! 'apply_blur' is only supported on Windows");

    let window_ = window.clone();
    window.on_menu_event(move |event| match event.menu_item_id() {
        "quit" => {
            std::process::exit(0);
        }
        "close" => {
            window_.close().unwrap();
        }
        _ => {}
    });
    
    Ok(())
}