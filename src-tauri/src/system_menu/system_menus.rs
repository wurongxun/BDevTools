use tauri::image::Image;
use tauri::menu::MenuBuilder;
use tauri::menu::{IconMenuItem, IconMenuItemBuilder, Menu, MenuEvent, MenuItem, Submenu};
use tauri::{AppHandle, Result, Window, Wry};
pub fn create_menu(app: &AppHandle) -> Result<Menu<Wry>> {
    let hello_item = MenuItem::with_id(app, "hello", "Hello", true, None::<&str>)?;
    let world_item = MenuItem::with_id(app, "world", "World", true, None::<&str>)?;
    // let icon_item = IconMenuItem::with_id(
    //     app,
    //     "icon",
    //     "Icon",
    //     true,
    //     Some(Image::from_path("../../icons/icon.png").unwrap()),
    //     None::<&str>,
    // )?;
    let icon_image = Image::from_path("../../icons/icon.png").unwrap();

    let icon_item = IconMenuItemBuilder::new("icon")
        .icon(icon_image)
        .build(app)?;

    let first_submenu = Submenu::with_items(
        app,
        "first_submenu",
        true,
        &[&hello_item, &world_item, &icon_item],
    )?;

    let menu = Menu::with_id_and_items(app, "menu_id", &[&first_submenu])?;
    Ok(menu)
}

/**
 *
 * 应用菜单处理事件
 */
pub fn handler1(app_handle: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    //let e=event.id().as_ref();
    match event.id().as_ref() {
        "new_file" => {
            dbg!("new file");
        }
        "edit_file" => {
            dbg!("edit_file");
        }
        "undo" => {
            dbg!("undo");
        }
        "redo" => {
            dbg!("redo");
        }
        "quit" => {
            std::process::exit(0);
        }
        _ => {}
    }
}

// pub fn handler(event: tauri::menu::MenuEvent) {
//     // 菜单所属的窗口
//     let win = Some(event.window());
//     // 匹配菜单 id
//     match event.menu_item_id() {
//         "new_file" => {
//             // debug 信息（终端输出）
//             dbg!("new file");
//         }
//         "edit_file" => {
//             // 发送信息到菜单所属窗口（弹窗形式）
//             message(win, "Eidt File", "TODO");
//         }
//         "undo" => {
//             dbg!("undo");
//         }
//         "redo" => {
//             dbg!("redo");
//         }
//         "quit" => {
//             std::process::exit(0);
//         }
//         _ => {}
//     }
// }
// fn message(window: Option<Window>, title: &str, content: &str) {
//     if let Some(win) = window {
//         win.emit(
//             "menu_event",
//             format!("{{\"title\":\"{}\",\"content\":\"{}\"}}", title, content),
//         )
//         .unwrap();
//     }
// }
