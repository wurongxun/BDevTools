//! 沉浸式模式快捷键：在 Tauri 层（而非前端 DOM）拦截 F11 / Esc，
//! 以穿透第三方插件沙箱 iframe 的事件边界。
//!
//! 背景：第三方插件以 `sandbox="allow-scripts"` 的跨源 iframe 渲染，其内部的原生
//! `keydown` / `mousemove` **不会冒泡到宿主 window**，导致在插件里编辑时按 F11、Esc、
//! 或鼠标移到窗口顶部「退出条」都毫无反应（被卡在沉浸式里出不去）。DOM 层监听对此无能为力，
//! 必须在 OS / WebView 这一层拦截。
//!
//! 关键约束：Tauri 的 global-shortcut 注册的是**系统级**热键。为避免本应用处于后台时
//! 抢走系统 F11（会破坏浏览器等其它程序的 F11），仅在「本应用有窗口获得焦点」时才注册；
//! Esc 进一步仅在「有窗口处于沉浸式」时才注册，避免常态下吞掉 Esc（关弹窗 / 退出全屏等）。

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, Window};
// macOS 专用的 Control+Command+F 只在 mac 下注册，Windows/Linux 下引了就是 unused import
#[cfg(target_os = "macos")]
use tauri_plugin_global_shortcut::Modifiers;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

/// 沉浸式快捷键的共享状态（所有窗口共用一份，由 Arc<Mutex> 保护）。
pub(crate) struct ImmersiveState {
    /// 当前获得焦点的本应用窗口 label（非空即表示本应用有焦点）。
    focused_labels: HashSet<String>,
    /// 处于沉浸式模式的窗口 label（用集合按 label 去重：
    /// 同窗口重复上报同一状态不会把计数叠歪，窗口关闭时也能精确移除）。
    immersive_labels: HashSet<String>,
    /// F11（及 macOS 的 Ctrl+Cmd+F）是否已注册。
    f11_registered: bool,
    /// Esc 是否已注册。
    esc_registered: bool,
}

impl Default for ImmersiveState {
    fn default() -> Self {
        // 两个集合都从**空**开始：只有真的收到 Focused(true) 才认为窗口拿到了焦点。
        // 这样本应用从未获得过焦点时不会注册任何系统热键 ——
        // 宁可让用户先点一下窗口再按 F11，也绝不能在后台偷偷抢走系统 F11。
        Self {
            focused_labels: HashSet::new(),
            immersive_labels: HashSet::new(),
            f11_registered: false,
            esc_registered: false,
        }
    }
}

fn on_toggle(app: &AppHandle, _sc: &Shortcut, event: ShortcutEvent) {
    if event.state == ShortcutState::Pressed {
        let _ = app.emit("immersive-shortcut-toggle", ());
    }
}

fn on_exit(app: &AppHandle, _sc: &Shortcut, event: ShortcutEvent) {
    if event.state == ShortcutState::Pressed {
        let _ = app.emit("immersive-shortcut-exit", ());
    }
}

/// 根据当前状态决定应注册 / 注销哪些全局快捷键。
pub(crate) fn reconcile(app: &AppHandle, st: &mut ImmersiveState) {
    let gs = app.global_shortcut();
    // 有窗口获得焦点 ⇒ 可以拦 F11；进一步「且有窗口处于沉浸式」才拦 Esc
    let focused = !st.focused_labels.is_empty();
    let want_f11 = focused;
    let want_esc = focused && !st.immersive_labels.is_empty();

    if want_f11 && !st.f11_registered {
        let _ = gs.on_shortcut(Shortcut::new(None, Code::F11), on_toggle);
        #[cfg(target_os = "macos")]
        let _ = gs.on_shortcut(
            Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SUPER), Code::KeyF),
            on_toggle,
        );
        st.f11_registered = true;
    } else if !want_f11 && st.f11_registered {
        let _ = gs.unregister(Shortcut::new(None, Code::F11));
        #[cfg(target_os = "macos")]
        let _ = gs.unregister(Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::SUPER),
            Code::KeyF,
        ));
        st.f11_registered = false;
    }

    if want_esc && !st.esc_registered {
        let _ = gs.on_shortcut(Shortcut::new(None, Code::Escape), on_exit);
        st.esc_registered = true;
    } else if !want_esc && st.esc_registered {
        let _ = gs.unregister(Shortcut::new(None, Code::Escape));
        st.esc_registered = false;
    }
}

/// 窗口焦点变化：把该窗口移入 / 移出焦点集合，并重新对齐快捷键。
pub(crate) fn handle_focus(window: &Window, focused: bool, state: &Arc<Mutex<ImmersiveState>>) {
    let mut st = state.lock().unwrap();
    let label = window.label().to_string();
    if focused {
        st.focused_labels.insert(label);
    } else {
        st.focused_labels.remove(&label);
    }
    reconcile(&window.app_handle(), &mut st);
}

/// 窗口销毁：清掉它留在集合里的痕迹。
///
/// 少了这一步，一个「带着沉浸式被关掉」的窗口会把自己的 label 永久留在集合里 ——
/// 于是 Esc 会一直被系统层拦截，用户在**其它窗口**里按 Esc 全部失效（连弹窗都关不掉）。
pub(crate) fn handle_window_destroyed(
    app: &AppHandle,
    label: &str,
    state: &Arc<Mutex<ImmersiveState>>,
) {
    let mut st = state.lock().unwrap();
    let was_focused = st.focused_labels.remove(label);
    let was_immersive = st.immersive_labels.remove(label);
    if was_focused || was_immersive {
        reconcile(app, &mut st);
    }
}

/// 前端在「进入 / 退出沉浸式」时上报：更新沉浸式窗口集合并重新对齐快捷键。
pub(crate) fn handle_immersive_state(
    app: &AppHandle,
    payload: &str,
    state: &Arc<Mutex<ImmersiveState>>,
) {
    // Tauri 2 的 listen 回调给的是原始 payload **字符串**（不是 Value），解析失败直接忽略：
    // 宁可不注册 Esc（用户还能用退出条按钮），也不要因为一条畸形消息把状态搞乱。
    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
        return;
    };
    let active = value
        .get("active")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let label = value.get("label").and_then(|v| v.as_str()).map(String::from);

    let mut st = state.lock().unwrap();
    if let Some(label) = label {
        if active {
            st.immersive_labels.insert(label);
        } else {
            st.immersive_labels.remove(&label);
        }
    }
    reconcile(app, &mut st);
}
