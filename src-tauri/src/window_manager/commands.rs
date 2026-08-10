/*! # 窗口管理 Tauri 命令 */
use serde_json::json;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_RESTORE};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

use crate::entity::window::{WindowMeta, WindowRole};
use crate::response::response::Response;
use crate::window_manager::registry;

/// 创建新的编辑器窗口
#[tauri::command]
pub async fn create_editor_window(
    app: tauri::AppHandle,
    workspace_id: Option<String>,
    width: Option<f64>,
    height: Option<f64>,
    title: Option<String>,
) -> Response<WindowMeta> {
    println!("[create_editor_window] 命令被调用, workspace_id={:?}, width={:?}, height={:?}", workspace_id, width, height);
    let uuid = Uuid::new_v4().to_string();
    // 截取 uuid 前 8 位作为简短标识
    let short_id = &uuid[..8];
    let label = format!("editor-{}", short_id);
    let win_title = title.unwrap_or_else(|| format!("编辑器 - {}", short_id));
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
                100.0,
                100.0,
                w,
                h,
            );

            // 注册到全局注册表
            registry::register_window(meta.clone());

            // 监听窗口关闭事件，自动从注册表移除
            let label_clone = label.clone();
            let app_clone = app.clone();
            win.on_window_event(move |event| {
                if let tauri::WindowEvent::Destroyed = event {
                    registry::unregister_window(&label_clone);
                    // 通知其他窗口
                    let _ = app_clone.emit("window-destroyed", &label_clone);
                }
            });

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
                x: 0.0, y: 0.0, width: 0.0, height: 0.0,
                is_maximized: false,
                created_at: 0,
            }, &format!("窗口创建失败: {}", err))
        }
    }
}

/// 获取所有窗口列表
#[tauri::command]
pub async fn get_window_list() -> Response<Vec<WindowMeta>> {
    let windows = registry::get_all_windows();
    Response::new(200, windows, "获取窗口列表成功")
}

/// 关闭指定窗口
#[tauri::command]
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
#[tauri::command]
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
#[tauri::command]
pub async fn get_window_count() -> Response<usize> {
    let count = registry::get_window_count(true);
    Response::new(200, count, "获取窗口数量成功")
}

/// 更新窗口位置信息（前端调用，用于同步注册表）
#[tauri::command]
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
) -> Option<String> {
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
    .inner_size(width, height)
    .min_inner_size(400.0, 300.0)
    .visible(false)
    .build();

    match window {
        Ok(win) => {
            let meta = registry::create_window_meta(
                label.clone(),
                WindowRole::Editor,
                None,
                title,
                100.0,
                100.0,
                width,
                height,
            );
            registry::register_window(meta.clone());

            let label_clone = label.clone();
            let app_clone = app.clone();
            win.on_window_event(move |event| {
                if let tauri::WindowEvent::Destroyed = event {
                    registry::unregister_window(&label_clone);
                    let _ = app_clone.emit("window-destroyed", &label_clone);
                }
            });

            let _ = win.center();
            let _ = win.show();
            let _ = win.set_focus();
            let _ = app.emit("window-created", &meta);

            // 将布局配置写入 settings.json，供新窗口前端读取
            if let Ok(store) = app.store("settings.json") {
                let config_key = format!("window_layout_{}", label);
                let _ = store.set(
                    config_key,
                    json!({
                        "enabledPlugins": enabled_plugins,
                        "focusMode": focus_mode,
                        "presetId": preset_id
                    }),
                );
                let _ = store.save();
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
