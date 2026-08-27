/**
 * 插件工作目录管理服务
 * 封装插件工作目录相关的 Tauri invoke 调用
 */
import { invoke } from '@/api/rpc'
import type { ApiResponse } from '@/plugins/database/types'

export const workspaceDirService = {
  /** 获取插件在当前工作区的工作目录 */
  async getPluginDir(workspaceId: string, pluginId: string): Promise<string> {
    const res = await invoke<ApiResponse<string>>('workspace_get_plugin_dir', { workspaceId, pluginId })
    return res.code === 200 ? res.data : ''
  },

  /** 获取插件的子目录路径 */
  async getPluginSubDir(workspaceId: string, pluginId: string, subdir: string): Promise<string> {
    const res = await invoke<ApiResponse<string>>('workspace_get_plugin_subdir', { workspaceId, pluginId, subdir })
    return res.code === 200 ? res.data : ''
  },

  /** 获取全局插件目录（跨工作区共享） */
  async getGlobalPluginDir(pluginId: string): Promise<string> {
    const res = await invoke<ApiResponse<string>>('workspace_get_global_plugin_dir', { pluginId })
    return res.code === 200 ? res.data : ''
  },

  /** 设置插件自定义工作目录（传空字符串恢复默认，按工作区隔离） */
  async setCustomDir(workspaceId: string, pluginId: string, customPath: string): Promise<boolean> {
    const res = await invoke<ApiResponse<boolean>>('workspace_set_plugin_custom_dir', { workspaceId, pluginId, customPath })
    return res.code === 200
  },

  /** 获取插件自定义工作目录（null 表示使用默认，按工作区隔离） */
  async getCustomDir(workspaceId: string, pluginId: string): Promise<string | null> {
    const res = await invoke<ApiResponse<string | null>>('workspace_get_plugin_custom_dir', { workspaceId, pluginId })
    return res.code === 200 ? res.data : null
  },
}
