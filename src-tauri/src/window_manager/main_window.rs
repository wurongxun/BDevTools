/*! # 主窗口工厂：创建 + 唤回的唯一入口
 *
 * 主窗口的创建**只能有这一份实现**：启动（`lib.rs` 的 setup）、托盘「显示窗口」、
 * 二次启动实例（single-instance）三条路都走这里，否则三处的窗口配置迟早各自漂移。
 *
 * ## 为什么托盘不能只写 `show()`
 *
 * 主窗口的 label 固定是 [`MAIN_WINDOW_LABEL`]（`"main"`）。用户点关闭按钮走的是
 * `WebviewWindow::close()` ⇒ 窗口被**销毁**：从 `app.webview_windows()` 里消失、句柄作废，
 * `lib.rs` 的全局 `Destroyed` 监听还会把它从窗口注册表摘掉。此时「遍历 `webview_windows()`
 * 找到 key == "main" 再 `show()`」永远匹配不到任何东西，点击托盘就成了**静默空操作**
 * —— 这就是「主窗口被关掉后托盘再也唤不回来」的根因。
 *
 * 所以在「已被销毁」这一支必须**重建**：[`build_main_window`] 造一个新的主窗口并重新注册，
 * 然后 show + 抢前台。只有「未被销毁（隐藏 / 最小化）」这一支才做 `show()`。
 *
 * 另：`webview_windows()` 是一份**快照**（`HashMap<String, WebviewWindow>`，值克隆自内部注册表），
 * 拿它遍历去 `show()` 会写在已失效的句柄上；按 label 现查（`get_webview_window`）才拿得到
 * 此刻真实的窗口。
 */
use tauri::{
    AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_window_state::{StateFlags, WindowExt};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_RESTORE};

use crate::entity::window::WindowRole;
use crate::window_manager::registry;

/// 主窗口 label。全项目**唯一出处**，不要再写字面量去猜 ——
/// 历史上托盘代码还判断过一个从来没人创建的 `"home"`，是条死条件。
pub const MAIN_WINDOW_LABEL: &str = "main";

/// 这次 `CloseRequested` 是否该被拦下来、改成「隐藏到托盘」（close-to-tray）。
///
/// **只拦主窗口**：它是常驻窗口（`openOnStartup=false` 时也靠托盘唤回），用户点关闭通常只想
/// 把它收起来，而不是结束整个应用；`editor-*` / `app-window-*` 等窗口照常真正关闭。
///
/// 纯函数（不读全局状态）以便直接跑断言。调用点见 `lib.rs` 的 `on_window_event`。
pub fn should_hide_instead_of_close(label: &str) -> bool {
    label == MAIN_WINDOW_LABEL
}

const MAIN_TITLE: &str = "BackendDevTools";
const MAIN_WIDTH: f64 = 960.0;
const MAIN_HEIGHT: f64 = 640.0;

/// 创建主窗口：建窗口 → 注册进全局窗口注册表 → 广播 `window-created`。
///
/// 窗口以 `visible(false)` 创建，可见性交给调用方决定（启动时看 `openOnStartup`，
/// 托盘唤回时由 [`show_main_window`] 立刻 `show()`）。
pub fn build_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    let window = WebviewWindowBuilder::new(
        app,
        MAIN_WINDOW_LABEL.to_string(),
        WebviewUrl::App("index.html".into()),
    )
    .title(MAIN_TITLE)
    .decorations(false)
    // 无边框窗口必须启用阴影，否则 Windows 下 DWM 会在左/右/底边露出 1px 黑线
    .shadow(true)
    .inner_size(MAIN_WIDTH, MAIN_HEIGHT)
    .min_inner_size(640.0, 480.0)
    .center()
    .visible(false)
    .build()?;

    // 恢复上次的尺寸/位置，但**不**恢复可见性（由上面的 `visible(false)` + 调用方决定）
    let _ = window.restore_state(StateFlags::all() ^ StateFlags::VISIBLE);

    // 注册：重建时覆盖被 `Destroyed` 监听删掉的旧条目（幂等）
    let meta = registry::create_window_meta(
        MAIN_WINDOW_LABEL.to_string(),
        WindowRole::Main,
        None,
        MAIN_TITLE.to_string(),
        Some(MAIN_WINDOW_LABEL.to_string()),
        0.0,
        0.0,
        MAIN_WIDTH,
        MAIN_HEIGHT,
    );
    registry::register_window(meta.clone());
    // 与编辑器窗口创建时同一事件：其它窗口据此刷新窗口列表
    let _ = app.emit("window-created", &meta);

    Ok(window)
}

/// 唤回主窗口：**存在就显示，不存在就重建**。
///
/// 返回值只用于日志判断，失败绝不 panic —— 托盘回调跑在主线程的事件循环里，
/// 在那里 panic 会把整个应用带走（旧实现里的 `value.show().unwrap()` 就有这个风险）。
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> bool {
    // 先按 label 现查（不能缓存句柄：主窗口可能已被销毁并重建过）
    if let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = win.show();
        let _ = win.unminimize();
        focus(&win);
        return true;
    }

    println!("[main_window] 主窗口已不存在（被销毁），重建中…");
    match build_main_window(app) {
        Ok(win) => {
            let _ = win.show();
            focus(&win);
            println!("[main_window] 主窗口已重建并显示");
            true
        }
        Err(err) => {
            eprintln!("[main_window] 主窗口重建失败: {}", err);
            false
        }
    }
}

/// 把窗口提到最前。
///
/// Windows 下只有 `set_focus()` 常常抢不到前台（前台锁定策略会拦），需要 Win32 兜底
/// —— 与 `window_manager::commands::focus_window` 同一口径。
fn focus<R: Runtime>(win: &WebviewWindow<R>) {
    let _ = win.set_focus();
    #[cfg(target_os = "windows")]
    {
        if let Ok(hwnd) = win.hwnd() {
            let raw = hwnd.0 as usize as isize;
            unsafe {
                let _ = ShowWindow(HWND(raw), SW_RESTORE);
                let _ = SetForegroundWindow(HWND(raw));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 主窗口必须被拦成「隐藏」，其它窗口一律照常关闭 ——
    /// 拦错窗口会变成「编辑器窗口关不掉」，漏拦主窗口则退回「关掉后托盘唤不回」的老 bug。
    #[test]
    fn only_main_window_hides_instead_of_closing() {
        assert!(should_hide_instead_of_close(MAIN_WINDOW_LABEL));
        assert!(!should_hide_instead_of_close("editor-1a2b3c4d"));
        assert!(!should_hide_instead_of_close("app-window-App-setting"));
        assert!(!should_hide_instead_of_close(""));
        // 历史上托盘误判过的臆造 label 不能被当成主窗口
        assert!(!should_hide_instead_of_close("home"));
    }
}
