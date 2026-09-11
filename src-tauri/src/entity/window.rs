/*! # 窗口 数据模型 */
use serde::{Deserialize, Serialize};

/// 窗口角色
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WindowRole {
    Main,
    Editor,
    Auxiliary,
}

impl Default for WindowRole {
    fn default() -> Self {
        WindowRole::Main
    }
}

/// 窗口元数据（注册表条目）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowMeta {
    pub label: String,
    pub role: WindowRole,
    pub workspace_id: Option<String>,
    pub title: String,
    pub preset_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub is_maximized: bool,
    pub created_at: i64,
}

/// 窗口状态（位置、大小）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowState {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub is_maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        WindowState {
            x: 100.0,
            y: 100.0,
            width: 960.0,
            height: 640.0,
            is_maximized: false,
        }
    }
}
