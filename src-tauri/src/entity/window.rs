/*! # 窗口/工作区/会话 数据模型 */
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// 工作区配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub id: String,
    pub name: String,
    pub folders: Vec<String>,
    pub settings: HashMap<String, serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 工作区列表项（摘要，不含 settings）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub folders: Vec<String>,
    pub updated_at: i64,
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

/// 打开文件的信息（用于会话快照）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpenFileInfo {
    pub path: String,
    pub is_dirty: bool,
    pub cursor_line: u32,
    pub cursor_col: u32,
    pub scroll_top: f64,
    /// 是否存在未保存内容的草稿（脏文档内容已写入 drafts 目录）
    #[serde(default)]
    pub has_draft: bool,
    /// 快照时目标文件的 mtime 指纹（秒，0 表示不存在/未记录）
    #[serde(default)]
    pub file_mtime: i64,
    /// 快照时目标文件的大小指纹（0 表示不存在/未记录）
    #[serde(default)]
    pub file_size: u64,
}

/// 布局状态
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayoutState {
    pub sidebar_width: f64,
    pub sidebar_collapsed: bool,
    pub active_plugin_id: String,
}

impl Default for LayoutState {
    fn default() -> Self {
        LayoutState {
            sidebar_width: 220.0,
            sidebar_collapsed: false,
            active_plugin_id: String::new(),
        }
    }
}

/// 会话快照
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub id: String,
    pub workspace_id: Option<String>,
    pub window_label: String,
    pub window_role: WindowRole,
    pub window_state: WindowState,
    pub open_files: Vec<OpenFileInfo>,
    pub active_file: Option<String>,
    pub layout: LayoutState,
    pub timestamp: i64,
}

/// 会话摘要（用于列表展示）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionSummary {
    pub window_label: String,
    pub window_role: WindowRole,
    pub workspace_id: Option<String>,
    pub open_file_count: usize,
    pub timestamp: i64,
}
