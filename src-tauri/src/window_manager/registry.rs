/*! # 全局窗口注册表
管理所有窗口的元数据，窗口创建时注册，关闭时自动移除。
*/
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::collections::HashMap;
use chrono::Utc;

use crate::entity::window::{WindowMeta, WindowRole};

lazy_static! {
    /// 全局窗口注册表：label -> WindowMeta
    pub static ref WINDOW_REGISTRY: Mutex<HashMap<String, WindowMeta>> =
        Mutex::new(HashMap::new());
}

/// 注册窗口到全局注册表
pub fn register_window(meta: WindowMeta) {
    let mut registry = WINDOW_REGISTRY.lock();
    registry.insert(meta.label.clone(), meta);
}

/// 从全局注册表移除窗口
pub fn unregister_window(label: &str) {
    // 释放该窗口持有的所有工作区锁
    crate::workspace::lock::unlock_all(label);
    let mut registry = WINDOW_REGISTRY.lock();
    registry.remove(label);
}

/// 获取所有窗口元数据列表
pub fn get_all_windows() -> Vec<WindowMeta> {
    let registry = WINDOW_REGISTRY.lock();
    registry.values().cloned().collect()
}

/// 获取窗口数量（可排除主窗口）
pub fn get_window_count(exclude_main: bool) -> usize {
    let registry = WINDOW_REGISTRY.lock();
    if exclude_main {
        registry.values().filter(|w| w.role != WindowRole::Main).count()
    } else {
        registry.len()
    }
}

/// 更新窗口的位置和大小信息
pub fn update_window_position(label: &str, x: f64, y: f64, width: f64, height: f64, is_maximized: bool) {
    let mut registry = WINDOW_REGISTRY.lock();
    if let Some(meta) = registry.get_mut(label) {
        meta.x = x;
        meta.y = y;
        meta.width = width;
        meta.height = height;
        meta.is_maximized = is_maximized;
    }
}

/// 创建 WindowMeta 的辅助函数
pub fn create_window_meta(
    label: String,
    role: WindowRole,
    workspace_id: Option<String>,
    title: String,
    preset_id: Option<String>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> WindowMeta {
    WindowMeta {
        label,
        role,
        workspace_id,
        title,
        preset_id,
        x,
        y,
        width,
        height,
        is_maximized: false,
        created_at: Utc::now().timestamp_millis(),
    }
}
