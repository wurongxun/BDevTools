use tauri::{CustomMenuItem, Menu, MenuItem, Submenu, Context, WindowMenuEvent};
use tauri::utils::assets::EmbeddedAssets;
use tauri::api::dialog::message;
pub fn system_menu(context: &Context<EmbeddedAssets>)->Menu{
    // accelerator()函数的参数判断标准：https://github.com/tauri-apps/tao/blob/dev/src/keyboard.rs
    // accelerator()文档：https://docs.rs/tao/latest/tao/accelerator/
    //Menu::new().add_submenu(file).add_submenu(edit);
    // 应用名称
    let name = &context.package_info().name;
    // tauri::Menu::os_default(name)
    // 应用主菜单
    let app_menu: Submenu = Submenu::new(
        "help",
        // MenuItem::About 为原生菜单
        Menu::new()
        .add_item(CustomMenuItem::new("update".to_string(), "更新"))
        //.add_native_item(MenuItem::About(name, ())),
    );

    // 文件菜单（自定义菜单）
    let file_menu = Submenu::new(
        "File",
        Menu::new()
            .add_item(CustomMenuItem::new("new_file".to_string(), "New File"))
            .add_item(CustomMenuItem::new("edit_file".to_string(), "Edit File"))
            .add_item(CustomMenuItem::new("open_file".to_string(), "打开文件...").accelerator("ctrl+o"))
            .add_item(CustomMenuItem::new("quit".to_string(), "退出").accelerator("ctrl+q")),
    );
    // 编辑菜单（自定义菜单）
    let edit_menu = Submenu::new(
        "Edit",
        Menu::new()
            .add_native_item(MenuItem::Cut)
            .add_native_item(MenuItem::Copy)
            .add_native_item(MenuItem::Paste)
            .add_native_item(MenuItem::SelectAll)
            .add_item(CustomMenuItem::new("undo".to_string(), "打开"))
            .add_item(CustomMenuItem::new("redo".to_string(), "恢复")),
    );

    Menu::new()
        .add_submenu(file_menu)
        .add_submenu(edit_menu)
        .add_submenu(app_menu)
}

/**
 * 应用菜单处理事件
 */
pub fn handler(event: WindowMenuEvent) {
    // 菜单所属的窗口
    let win = Some(event.window());
    // 匹配菜单 id
    match event.menu_item_id() {
        "new_file" => {
            // debug 信息（终端输出）
            dbg!("new file");
        }
        "edit_file" => {
            // 发送信息到菜单所属窗口（弹窗形式）
            message(win, "Eidt File", "TODO");
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