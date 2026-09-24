/*! # 窗口管理 Tauri 命令 */
use serde_json::json;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_RESTORE};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

use crate::entity::window::{WindowMeta, WindowRole};
use crate::response::response::Response;
use crate::window_manager::registry;

/// 创建新的编辑器窗口
pub async fn create_editor_window(
    app: tauri::AppHandle,
    workspace_id: Option<String>,
    width: Option<f64>,
    height: Option<f64>,
    title: Option<String>,
    preset_id: Option<String>,
) -> Response<WindowMeta> {
    println!("[create_editor_window] 命令被调用, workspace_id={:?}, width={:?}, height={:?}, preset_id={:?}", workspace_id, width, height, preset_id);

    // 工作区锁检查：如果指定了 workspace_id 且已被其他窗口占用，拒绝创建
    if let Some(ref ws_id) = workspace_id {
        if let Some(holder) = crate::workspace::lock::get_lock_holder(ws_id) {
            println!("[create_editor_window] 工作区 {} 已被窗口 {} 锁定，拒绝创建", ws_id, holder);
            return Response::new(
                300,
                WindowMeta {
                    label: String::new(),
                    role: WindowRole::Editor,
                    workspace_id: None,
                    title: String::new(),
                    preset_id: None,
                    x: 0.0, y: 0.0, width: 0.0, height: 0.0,
                    is_maximized: false,
                    created_at: 0,
                },
                &format!("该工作区已被窗口 [{}] 占用，无法在同一工作区打开多个窗口", holder),
            );
        }
    }

    // 预设窗口唯一性：同一预设同时只允许存在一个窗口。
    //
    // 命中时**不创建**，把已开窗口的 meta 带回去（code 409）让前端聚焦。前端窗口菜单点击时
    // 也会先查一次 `get_window_list`，但那条路走 IPC、失败时会直接落到创建分支上，于是能绕过
    // 「一预设一窗口」——出现两个窗口共用同一预设，而它们的插件清单各改各的、还会互相顶掉。
    // 这里在进程内查注册表，是唯一可靠的判据；前端那次查询只是省一次创建往返。
    if let Some(ref pid) = preset_id {
        if let Some(existing) = registry::find_window_by_preset(pid) {
            println!(
                "[create_editor_window] 预设 {} 已有窗口 {}，不重复创建",
                pid, existing.label
            );
            return Response::new(409, existing, "该预设的窗口已打开，已切换到该窗口");
        }
    }

    let uuid = Uuid::new_v4().to_string();
    // 截取 uuid 前 8 位作为简短标识（仅用于窗口 label，用户不可见）
    let short_id = &uuid[..8];
    let label = format!("editor-{}", short_id);
    // 窗口标题用序号，后续前端会更新为工作区名称
    let editor_count = registry::get_window_count(true) + 1;
    let win_title = title.unwrap_or_else(|| format!("编辑器 {}", editor_count));
    let w = width.unwrap_or(960.0);
    let h = height.unwrap_or(640.0);
    println!("[create_editor_window] 准备创建窗口: label={}, title={}, size={}x{}", label, win_title, w, h);

    // 创建 WebviewWindow
    let window = WebviewWindowBuilder::new(
        &app,
        label.clone(),
        WebviewUrl::App("index.html".into()),
    )
    .title(&win_title)
    .decorations(false)
    // 无边框窗口必须启用阴影，否则 Windows 下边缘露出 1px 黑线
    .shadow(true)
    .inner_size(w, h)
    .min_inner_size(400.0, 300.0)
    .visible(false)
    .build();

    match window {
        Ok(win) => {
            // 创建窗口元数据
            let meta = registry::create_window_meta(
                label.clone(),
                WindowRole::Editor,
                workspace_id,
                win_title,
                preset_id,
                100.0,
                100.0,
                w,
                h,
            );

            // 注册到全局注册表（窗口销毁时由 lib.rs 的全局监听自动移除并释放工作区锁）
            registry::register_window(meta.clone());

            // 居中显示
            let _ = win.center();
            let _ = win.show();
            let _ = win.set_focus();

            // 通知所有窗口新窗口已创建
            let _ = app.emit("window-created", &meta);

            Response::new(200, meta, "编辑器窗口创建成功")
        }
        Err(err) => {
            println!("[create_editor_window] 窗口创建失败: {}", err);
            Response::new(500, WindowMeta {
                label: String::new(),
                role: WindowRole::Editor,
                workspace_id: None,
                title: String::new(),
                preset_id: None,
                x: 0.0, y: 0.0, width: 0.0, height: 0.0,
                is_maximized: false,
                created_at: 0,
            }, &format!("窗口创建失败: {}", err))
        }
    }
}

/// 获取所有窗口列表
pub async fn get_window_list() -> Response<Vec<WindowMeta>> {
    let windows = registry::get_all_windows();
    Response::new(200, windows, "获取窗口列表成功")
}

/// 关闭指定窗口
pub async fn close_window(app: tauri::AppHandle, label: String) -> Response<bool> {
    let window = app.get_webview_window(&label);
    match window {
        Some(win) => {
            let _ = win.close();
            Response::new(200, true, "窗口已关闭")
        }
        None => Response::new(404, false, &format!("窗口 {} 不存在", label)),
    }
}

/// 聚焦指定窗口（通用，使用 Win32 SetForegroundWindow 确保到最前）
pub async fn focus_window(app: tauri::AppHandle, label: String) -> Response<bool> {
    let window = app.get_webview_window(&label);
    match window {
        Some(win) => {
            let _ = win.show();
            #[cfg(target_os = "windows")]
            {
                // Windows 下使用 SetForegroundWindow 强制到最前
                if let Ok(hwnd) = win.hwnd() {
                    unsafe {
                        let raw = hwnd.0 as usize as isize;
                        let _ = ShowWindow(HWND(raw), SW_RESTORE);
                        let _ = SetForegroundWindow(HWND(raw));
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = win.set_focus();
            }
            Response::new(200, true, "窗口已聚焦")
        }
        None => Response::new(404, false, &format!("窗口 {} 不存在", label)),
    }
}

/// 获取窗口数量（不含主窗口）
pub async fn get_window_count() -> Response<usize> {
    let count = registry::get_window_count(true);
    Response::new(200, count, "获取窗口数量成功")
}

/// 更新窗口位置信息（前端调用，用于同步注册表）
pub async fn update_window_position_cmd(
    label: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    is_maximized: bool,
) -> Response<bool> {
    registry::update_window_position(&label, x, y, width, height, is_maximized);
    Response::new(200, true, "窗口位置已更新")
}

/// 根据预设配置创建编辑器窗口（setup 阶段调用，非 Tauri 命令）
/// 创建窗口后将布局配置写入 settings.json，供前端 initEditorWindow 读取
pub fn create_preset_window_from_preset(
    app: &tauri::AppHandle,
    preset_id: &str,
    name: &str,
    width: f64,
    height: f64,
    enabled_plugins: Vec<String>,
    focus_mode: bool,
    workspace_id: Option<String>,
) -> Option<String> {
    // 绑定工作区的锁检查：同一工作区同时只能在一个窗口中打开（多个预设可绑同一工作区）
    if let Some(ref ws_id) = workspace_id {
        if let Some(holder) = crate::workspace::lock::get_lock_holder(ws_id) {
            println!(
                "[Rust] 预设 [{}] 绑定的工作区 {} 已被窗口 {} 打开，跳过创建",
                name, ws_id, holder
            );
            return None;
        }
    }

    // 同一不变量（判据见 registry::find_window_by_preset）：settings.json 可被手工编辑，
    // 若两个预设共用同一个 id，这里会开出两个「同一个预设」的窗口。
    if let Some(existing) = registry::find_window_by_preset(preset_id) {
        println!(
            "[Rust] 预设 {} 已有窗口 {}，跳过创建",
            preset_id, existing.label
        );
        return None;
    }

    let uuid = Uuid::new_v4().to_string();
    let short_id = &uuid[..8];
    let label = format!("editor-{}", short_id);
    let title = if name.is_empty() {
        format!("编辑器 - {}", short_id)
    } else {
        name.to_string()
    };

    let window = WebviewWindowBuilder::new(
        app,
        label.clone(),
        WebviewUrl::App("index.html".into()),
    )
    .title(&title)
    .decorations(false)
    .shadow(true)
    .inner_size(width, height)
    .min_inner_size(400.0, 300.0)
    .visible(false)
    .build();

    match window {
        Ok(win) => {
            let meta = registry::create_window_meta(
                label.clone(),
                WindowRole::Editor,
                workspace_id.clone(),
                title,
                Some(preset_id.to_string()),
                100.0,
                100.0,
                width,
                height,
            );
            registry::register_window(meta.clone());

            let _ = win.center();
            let _ = win.show();
            let _ = win.set_focus();
            let _ = app.emit("window-created", &meta);

            // 将布局配置写入 settings.json，供新窗口前端读取（含工作区绑定）
            {
                let mut settings = crate::app::app_store::read_settings();
                if let Some(obj) = settings.as_object_mut() {
                    let config_key = format!("window_layout_{}", label);
                    obj.insert(
                        config_key,
                        json!({
                            "enabledPlugins": enabled_plugins,
                            "focusMode": focus_mode,
                            "presetId": preset_id,
                            "workspaceId": workspace_id.unwrap_or_default()
                        }),
                    );
                }
                crate::app::app_store::write_settings(&settings);
            }

            println!("[Rust] 预设窗口已创建: {} ({})", meta.title, label);
            Some(label)
        }
        Err(err) => {
            println!("[Rust] 预设窗口创建失败: {}", err);
            None
        }
    }
}
