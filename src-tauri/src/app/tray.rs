/*! # 系统托盘
 *
 * 托盘与菜单项都只是「唤回主窗口」的触发器，真正的逻辑在
 * [`crate::window_manager::main_window::show_main_window`]：
 * 存在就显示并聚焦，**已被关闭（销毁）就重建**。
 *
 * 旧实现在这里是「遍历 `app.webview_windows()`，key 等于 `"main"` 或 `"home"` 才 show」，
 * 两个问题：
 * 1. 主窗口关闭后已从窗口表里消失，遍历永远匹配不到 ⇒ 点击托盘静默失效；
 * 2. `"home"` 这个 label 全项目从没人创建过，是条死条件（真实 label 只有 `"main"`）。
 * 另外 `show()/unminimize()/set_focus()` 上的 `unwrap()` 一旦失败就会在主线程事件循环里
 * panic，把整个应用带走 —— 现在一律不 panic。
 */
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Runtime,
};

use crate::window_manager::main_window;

pub fn create_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let show_i = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

    let _ = TrayIconBuilder::with_id("tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                main_window::show_main_window(app);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                main_window::show_main_window(tray.app_handle());
            }
        })
        .build(app);

    Ok(())
}
