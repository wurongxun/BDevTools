/**
 * 窗口管理服务
 * 封装所有窗口管理相关的 Tauri invoke 调用
 */
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { WindowMeta } from '@/Types/workspace'
import type { ResponseData } from '@/entity/entitys'

export const windowService = {
  /** 创建新的编辑器窗口 */
  async createEditorWindow(workspaceId?: string): Promise<WindowMeta | null> {
    const res = await invoke<ResponseData<WindowMeta>>('create_editor_window', {
      workspaceId: workspaceId ?? null
    })
    return res.code === 200 ? res.data : null
  },

  /** 获取所有窗口列表 */
  async getWindowList(): Promise<WindowMeta[]> {
    const res = await invoke<ResponseData<WindowMeta[]>>('get_window_list')
    return res.code === 200 ? res.data : []
  },

  /** 关闭指定窗口 */
  async closeWindow(label: string): Promise<boolean> {
    const res = await invoke<ResponseData<boolean>>('close_window', { label })
    return res.code === 200
  },

  /** 聚焦指定窗口 */
  async focusWindow(label: string): Promise<boolean> {
    const res = await invoke<ResponseData<boolean>>('focus_window', { label })
    return res.code === 200
  },

  /** 获取编辑器窗口数量（不含主窗口） */
  async getWindowCount(): Promise<number> {
    const res = await invoke<ResponseData<number>>('get_window_count')
    return res.code === 200 ? res.data : 0
  },

  /** 更新窗口位置信息到注册表 */
  async updateWindowPosition(
    label: string,
    x: number, y: number,
    width: number, height: number,
    isMaximized: boolean
  ): Promise<void> {
    await invoke('update_window_position_cmd', {
      label, x, y, width, height, isMaximized
    })
  },

  /** 监听窗口创建事件 */
  onWindowCreated(callback: (meta: WindowMeta) => void) {
    return listen<WindowMeta>('window-created', (event) => {
      callback(event.payload)
    })
  },

  /** 监听窗口销毁事件 */
  onWindowDestroyed(callback: (label: string) => void) {
    return listen<string>('window-destroyed', (event) => {
      callback(event.payload)
    })
  }
}
