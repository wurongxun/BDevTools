use tauri::{GlobalWindowEvent, WindowEvent::CloseRequested};

/**
 * 窗口
 */
pub fn system_on_window_event(event: GlobalWindowEvent){
    match event.event() {
        // tauri::WindowEvent::Resized(_) => todo!(),
        // tauri::WindowEvent::Moved(_) => todo!(),
        // tauri::WindowEvent::CloseRequested { label, signal_tx, api } => {
        //     //关闭到时候拦截
        //     println!("CloseRequested窗口关闭了");
        // },
        // tauri::WindowEvent::Destroyed => todo!(),
        // tauri::WindowEvent::Focused(_) => todo!(),
        // tauri::WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size } => todo!(),
        _ => {},
    }

}