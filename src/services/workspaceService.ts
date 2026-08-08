/**
 * 工作区管理服务
 * 封装工作区配置相关的 Tauri invoke 调用
 */
import { invoke } from '@tauri-apps/api/core'
import type { WorkspaceConfig, WorkspaceSummary } from '@/Types/workspace'
import type { ResponseData } from '@/entity/entitys'

export const workspaceService = {
  /** 列出所有工作区摘要 */
  async list(): Promise<WorkspaceSummary[]> {
    const res = await invoke<ResponseData<WorkspaceSummary[]>>('workspace_list')
    return res.code === 200 ? res.data : []
  },

  /** 获取指定工作区完整配置 */
  async get(id: string): Promise<WorkspaceConfig | null> {
    const res = await invoke<ResponseData<WorkspaceConfig | null>>('workspace_get', { id })
    return res.code === 200 ? res.data : null
  },

  /** 创建新工作区 */
  async create(name: string, folders: string[]): Promise<WorkspaceConfig | null> {
    const res = await invoke<ResponseData<WorkspaceConfig>>('workspace_create', { name, folders })
    return res.code === 200 ? res.data : null
  },

  /** 更新工作区配置 */
  async update(
    id: string,
    options: { name?: string; folders?: string[]; settings?: Record<string, any> }
  ): Promise<WorkspaceConfig | null> {
    const res = await invoke<ResponseData<WorkspaceConfig | null>>('workspace_update', {
      id,
      name: options.name ?? null,
      folders: options.folders ?? null,
      settings: options.settings ?? null
    })
    return res.code === 200 ? res.data : null
  },

  /** 删除工作区 */
  async delete(id: string): Promise<boolean> {
    const res = await invoke<ResponseData<boolean>>('workspace_delete', { id })
    return res.code === 200
  },

  /** 获取或创建默认工作区 */
  async getDefault(): Promise<WorkspaceConfig | null> {
    const res = await invoke<ResponseData<WorkspaceConfig>>('workspace_get_default')
    return res.code === 200 ? res.data : null
  }
}
