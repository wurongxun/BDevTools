/**
 * 插件工作目录管理服务
 * 封装插件工作目录相关的 Tauri invoke 调用
 */
import { invoke } from '@/api/rpc'
import type { RpcResponse } from '@/api/rpc'

export const workspaceDirService = {
  /** 获取插件在当前工作区的工作目录 */
  async getPluginDir(workspaceId: string, pluginId: string): Promise<string> {
    const res = await invoke<RpcResponse<string>>('workspace_get_plugin_dir', { workspaceId, pluginId })
    return res.code === 200 ? res.data : ''
  },

  /** 获取插件的子目录路径 */
  async getPluginSubDir(workspaceId: string, pluginId: string, subdir: string): Promise<string> {
    const res = await invoke<RpcResponse<string>>('workspace_get_plugin_subdir', { workspaceId, pluginId, subdir })
    return res.code === 200 ? res.data : ''
  },

  /** 获取插件外部脚本目录列表（用户级，跨工作区共享） */
  async listExternalDirs(pluginId: string): Promise<string[]> {
    const res = await invoke<RpcResponse<string[]>>('workspace_list_plugin_external_dirs', { pluginId })
    return res.code === 200 ? res.data : []
  },

  /** 添加插件外部脚本目录（校验冲突后持久化） */
  async addExternalDir(workspaceId: string, pluginId: string, path: string): Promise<{ ok: boolean; msg?: string }> {
    const res = await invoke<RpcResponse<boolean>>('workspace_add_plugin_external_dir', { workspaceId, pluginId, path })
    return res.code === 200 ? { ok: true } : { ok: false, msg: res.msg }
  },

  /** 移除插件外部脚本目录（仅解除挂载，不删除内容） */
  async removeExternalDir(pluginId: string, path: string): Promise<boolean> {
    const res = await invoke<RpcResponse<boolean>>('workspace_remove_plugin_external_dir', { pluginId, path })
    return res.code === 200
  },
}
