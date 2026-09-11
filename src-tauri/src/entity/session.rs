/*! # 会话 数据模型（工作区/窗口 UI 状态快照） */
use serde::{Deserialize, Serialize};

use super::window::{WindowRole, WindowState};

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
