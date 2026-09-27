use tauri::Manager;

/**
 * 关闭加载窗口  显示主窗口
 * @param window
 */
pub async fn loding_window_close(window: tauri::Window) {
    // Close splashscreen
    if let Some(splashscreen) = window.get_webview_window("loding-window") {
        splashscreen.close().unwrap();
    }
    // Show main window
    window
        .get_webview_window("main-window")
        .unwrap()
        .show()
        .unwrap();
}

/**
 * 关于窗口
 * @param handle
 */
pub async fn about(handle: tauri::AppHandle, _window: tauri::Window) {
    tauri::WebviewWindowBuilder::new(
        &handle,
        "about-window", /* the unique window label */
        tauri::WebviewUrl::App("/Windows/About".into()),
    )
    .title("关于")
    .inner_size(720.0, 440.0)
    .decorations(false)
    .center()
    .resizable(false) //是否可调整大小
    .build()
    .unwrap();
}

/**
 * note_book_window窗口
 * @param handle
 */
pub async fn note_book_window(handle: tauri::AppHandle, _window: tauri::Window) {
    let note_book_window = tauri::WebviewWindowBuilder::new(
        &handle,
        "note_book", /* the unique window label */
        tauri::WebviewUrl::App("#/MainWindows/NoteBook".into()),
    )
    .title("NoteBook")
    .inner_size(720.0, 640.0)
    .min_inner_size(640.0, 480.0)
    .decorations(false)
    .resizable(true) //是否可调整大小
    .visible(false)
    .center()
    .build()
    .unwrap();
    note_book_window.show().unwrap();
}

/**
 * 最小化
 * @param handle
 * @param window
 */
pub fn windows_minimize(handle: tauri::AppHandle, label: String) {
    //获取窗口实例
    let window = handle.get_webview_window(label.as_str()).unwrap();
    window.minimize().unwrap();
}

/**
 * 隐藏
 * @param handle
 * @param window
 */
pub fn windows_hide(handle: tauri::AppHandle, label: String) {
    //获取窗口实例
    let window = handle.get_webview_window(label.as_str()).unwrap();
    window.hide().unwrap();
}

/**
 * 最大化maximize
 * @param handle
 * @param window
 */
pub fn windows_maximize(handle: tauri::AppHandle, label: String) {
    //获取窗口实例
    let window = handle.get_webview_window(label.as_str()).unwrap();
    window.maximize().unwrap();
}

/**
 * 窗口close
 * @param handle
 * @param label
 */
pub fn windows_close(handle: tauri::AppHandle, label: String) {
    //获取窗口实例
    let window = handle.get_webview_window(label.as_str()).unwrap();
    window.close().unwrap();
}
