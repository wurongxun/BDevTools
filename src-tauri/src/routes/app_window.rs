use tauri::{App, Context, Manager};
use window_shadows::set_shadow;
/**
 * 关闭加载窗口  显示主窗口
 * @param window
 */
#[tauri::command]
pub async fn loding_window_close(window: tauri::Window) {
    // Close splashscreen
    if let Some(splashscreen) = window.get_window("loding-window") {
        splashscreen.close().unwrap();
    }
    // Show main window
    window.get_window("main-window").unwrap().show().unwrap();
}
#[tauri::command]
pub async fn exit() {
    std::process::exit(0)
}
/**
 * 关于窗口
 * @param handle
 * @param window
 */
#[tauri::command]
pub async fn about(handle: tauri::AppHandle, window: tauri::Window) {
    let about_window = tauri::WindowBuilder::new(
        &handle,
        "about-window", /* the unique window label */
        tauri::WindowUrl::App("/Windows/About".into()),
    )
    .title("关于")
    .inner_size(720.0, 440.0)
    .decorations(false)
    .center()
    .resizable(false) //是否可调整大小
    .build()
    .unwrap();

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    set_shadow(&about_window, true).expect("Unsupported platform!");
}

#[tauri::command]
pub async fn windows_minimize(handle: tauri::AppHandle, label: String) {
    //获取窗口实例
    let window = handle.get_window(label.as_str()).unwrap();
    window.minimize().unwrap();
}

#[tauri::command]
pub async fn windows_hide(handle: tauri::AppHandle, label: String) {
    //获取窗口实例
    let window = handle.get_window(label.as_str()).unwrap();
    window.hide().unwrap();
}
#[tauri::command]
pub async fn windows_maximize(handle: tauri::AppHandle, label: String) {
    //获取窗口实例
    let window = handle.get_window(label.as_str()).unwrap();
    window.maximize().unwrap();
}
/**
 * 窗口close
 * @param handle
 * @param label
 */
#[tauri::command]
pub async fn windows_close(handle: tauri::AppHandle, label: String) {
    //获取窗口实例
    let window = handle.get_window(label.as_str()).unwrap();
    window.close().unwrap();
}
