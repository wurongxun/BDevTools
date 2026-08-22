/**
 * 窗口/工作区/会话 类型定义
 * 与 Rust 后端 entity/window.rs 结构体一一对应（snake_case）
 */

/** 窗口角色 */
export type WindowRole = 'main' | 'editor' | 'auxiliary'

/** 窗口元数据 */
export interface WindowMeta {
  label: string
  role: WindowRole
  workspace_id: string | null
  title: string
  preset_id: string | null
  x: number
  y: number
  width: number
  height: number
  is_maximized: boolean
  created_at: number
}

/** 工作区配置 */
export interface WorkspaceConfig {
  id: string
  name: string
  folders: string[]
  settings: Record<string, any>
  created_at: number
  updated_at: number
}

/** 工作区摘要（列表用） */
export interface WorkspaceSummary {
  id: string
  name: string
  folders: string[]
  updated_at: number
}

/** 窗口状态 */
export interface WindowState {
  x: number
  y: number
  width: number
  height: number
  is_maximized: boolean
}

/** 打开文件信息（会话快照用） */
export interface OpenFileInfo {
  path: string
  is_dirty: boolean
  cursor_line: number
  cursor_col: number
  scroll_top: number
  /** 是否存在未保存内容草稿 */
  has_draft?: boolean
  /** 快照时目标文件 mtime 指纹（秒） */
  file_mtime?: number
  /** 快照时目标文件大小指纹 */
  file_size?: number
}

/** 草稿检查结果（恢复前指纹校验） */
export interface DraftCheckResult {
  has_draft: boolean
  file_changed: boolean
  draft_saved_at: number
  file_mtime: number
}

/** 草稿元数据（保存草稿时返回的指纹） */
export interface DraftMeta {
  path: string
  saved_at: number
  file_mtime: number
  file_size: number
  file_exists: boolean
}

/** 布局状态 */
export interface LayoutState {
  sidebar_width: number
  sidebar_collapsed: boolean
  active_plugin_id: string
}

/** 会话快照 */
export interface SessionSnapshot {
  id: string
  workspace_id: string | null
  window_label: string
  window_role: WindowRole
  window_state: WindowState
  open_files: OpenFileInfo[]
  active_file: string | null
  layout: LayoutState
  timestamp: number
}

/** 会话摘要（列表用） */
export interface SessionSummary {
  window_label: string
  window_role: WindowRole
  workspace_id: string | null
  open_file_count: number
  timestamp: number
}
