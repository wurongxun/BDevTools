//System Tray 系统托盘菜单



use  tauri::{CustomMenuItem, SystemTray, SystemTrayMenu,SystemTrayMenuItem,AppHandle, Wry, Manager, GlobalWindowEvent, SystemTrayEvent};
pub  fn system_tray()->SystemTray{
    // 这里 `"quit".to_string()` 定义菜单项 ID，第二个参数是菜单项标签。
    let quit = CustomMenuItem::new("quit".to_string(), "关闭应用");
    let hide = CustomMenuItem::new("hide".to_string(), "最小化");
    let about = CustomMenuItem::new("about".to_string(), "关于应用");
    let show = CustomMenuItem::new("show".to_string(), "显示");
    let tray_menu = SystemTrayMenu::new()
    //.add_item(about)
    //.add_native_item(SystemTrayMenuItem::Separator)
    .add_item(show)
    .add_native_item(SystemTrayMenuItem::Separator)
    .add_item(hide)
    //设置横线
    .add_native_item(SystemTrayMenuItem::Separator)
    .add_item(quit);
    SystemTray::new().with_menu(tray_menu)
}

//托盘事件
pub fn system_tray_event(app: &AppHandle<Wry>, e: SystemTrayEvent) { //托盘事件
    match e {
        
        SystemTrayEvent::LeftClick { //左键事件
            position: _,
            size: _,
            ..
        } => { //显示窗口
            println!("系统托盘左键点击");
            let window = app.get_window("main-window").unwrap();
            window.show().unwrap();
            window.set_focus().unwrap();
        }
        SystemTrayEvent::RightClick {
            position: _,
            size: _,
            ..
          } => {
            println!("系统托盘收到右键点击");
        }
        SystemTrayEvent::DoubleClick {
            position: _,
            size: _,
            ..
          } => {
            println!("系统托盘收到双击");
            //显示窗口并聚焦
            let window = app.get_window("main-window").unwrap();
            window.show().unwrap();
            //将窗口移到前面并聚焦
            window.set_focus().unwrap();
        }
        SystemTrayEvent::MenuItemClick { id, .. } => {
            match id.as_str() { 
                
                //托盘菜单点击事件
                "quit" => { 
                    dbg!("退出程序");
                    //退出程序
                    std::process::exit(0);
                }
                "hide" => { 
                    dbg!("隐藏窗口");
                    //隐藏窗口
                    let window = app.get_window("main-window").unwrap(); //获取窗口实例
                    window.hide().unwrap();
                }
                "show" => { 
                    dbg!("显示窗口并聚焦");
                    //显示窗口并聚焦
                    let window = app.get_window("main-window").unwrap();
                    window.show().unwrap();
                    //将窗口移到前面并聚焦
                    window.set_focus().unwrap();
                }
                _ => {}
            }
        },
        _ => {}
    }
}

// fn window_event(e: GlobalWindowEvent) { //窗口事件
//     match e.event() {
//         tauri::WindowEvent::Focused(focused) => { //聚焦事件
//             if !focused { //不聚焦时
//                 e.window().hide().unwrap(); //隐藏窗口
//             }
//         }
//         _ => {}
//     }
// }
