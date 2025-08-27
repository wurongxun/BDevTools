use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, Runtime,
};

pub fn create_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let set_i = MenuItem::with_id(app, "set", "设置", true, None::<&str>)?;
    let show_i = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    // 分割线
    let menu = Menu::with_items(app, &[&show_i, &set_i, &quit_i])?;

    let _ = TrayIconBuilder::with_id("tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                for (key, value) in app.webview_windows() {
                    println!(" show {}",key);
                    if key == "main" || key == "home" {
                        value.show().unwrap();
                    }
                }
            }
            "set" => {
                println!("set");
            }
            "quit" => {
                app.exit(0);
            }
            // Add more events here
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let windows = tray.app_handle().webview_windows();
                for (key, value) in windows {
                    if key == "main" || key == "home" {
                        value.show().unwrap();
                        value.unminimize().unwrap();
                        value.set_focus().unwrap();
                    }
                }
            }
        })
        .build(app);

    Ok(())
}
