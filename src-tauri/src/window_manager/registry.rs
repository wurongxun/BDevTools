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

/// 在**给定的窗口列表里**找 `preset_id` 对应的已打开窗口。
///
/// `preset_id` 为空串或 `"main"` 一律视为「不是预设窗口」：主窗口的 `preset_id` 也是
/// `"main"`，但它由 `tauri.conf.json` 声明、永远只有一个，不参与预设唯一性判定；工具菜单里
/// 「新建窗口」建的窗口 `preset_id` 是 `None`，同样不属于任何预设。
///
/// 抽成纯函数是为了能在不碰全局注册表的前提下测试 —— `WINDOW_REGISTRY` 是进程级
/// `lazy_static`，用例共用同一份状态，直接测会互相干扰。
pub fn pick_window_by_preset(windows: &[WindowMeta], preset_id: &str) -> Option<WindowMeta> {
    if preset_id.is_empty() || preset_id == "main" {
        return None;
    }
    windows
        .iter()
        .find(|w| w.preset_id.as_deref() == Some(preset_id))
        .cloned()
}

/// 按 `preset_id` 查已打开的窗口。
///
/// 这是「一个预设同时只开一个窗口」这条不变量的判据，被 `create_editor_window` 与启动阶段的
/// `create_preset_window_from_preset` 共用（见两处的唯一性检查）。
pub fn find_window_by_preset(preset_id: &str) -> Option<WindowMeta> {
    pick_window_by_preset(&get_all_windows(), preset_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(label: &str, preset: Option<&str>) -> WindowMeta {
        create_window_meta(
            label.to_string(),
            WindowRole::Editor,
            None,
            label.to_string(),
            preset.map(|s| s.to_string()),
            0.0,
            0.0,
            800.0,
            600.0,
        )
    }

    #[test]
    fn pick_window_by_preset_matches_by_preset_id() {
        let list = vec![win("editor-a", Some("p1")), win("editor-b", Some("p2"))];
        assert_eq!(
            pick_window_by_preset(&list, "p1").map(|w| w.label),
            Some("editor-a".to_string())
        );
        assert_eq!(
            pick_window_by_preset(&list, "p2").map(|w| w.label),
            Some("editor-b".to_string())
        );
    }

    #[test]
    fn pick_window_by_preset_returns_none_for_unopened() {
        let list = vec![win("editor-a", Some("p1"))];
        assert!(pick_window_by_preset(&list, "p9").is_none());
    }

    /// 主窗口 / 无预设窗口不能被当成「该预设已打开」：否则点主窗口的菜单项会去聚焦一个
    /// 不相干的窗口，而且那个预设的窗口永远打不开。
    #[test]
    fn pick_window_by_preset_ignores_main_empty_and_unbound() {
        let list = vec![win("main", Some("main")), win("editor-x", None)];
        assert!(pick_window_by_preset(&list, "main").is_none());
        assert!(pick_window_by_preset(&list, "").is_none());
        assert!(pick_window_by_preset(&list, "editor-x").is_none());
        assert!(!list.is_empty(), "列表本身不该为空");
    }
}
