/**
 * 工作区管理服务
 * 封装工作区配置相关的 Tauri invoke 调用
 */
import { invoke } from '@/api/rpc'
// 「本窗口是谁」的判定只有一处：`core/windowPresets` 已在多窗口语义下统一裁决
// （主窗口 `main`、预设窗口、辅助窗口分别该返回什么），这里直接复用，别再抄一份。
import { currentWindowLabel } from '@/core/windowPresets'
import type { WorkspaceConfig, WorkspaceSummary } from '@/Types/workspace'
import type { ResponseData } from '@/entity/entitys'

/**
 * 持有者是否为**别的**窗口（true＝被别人占用）。
 *
 * 工作区锁解决的是「同一工作区不允许**多窗口**同时打开」，所以「本窗口自己持有」从来不算
 * 占用：页面刷新 / 重载时 Rust 进程没退，锁表仍在内存里且持有者就是自己，此时必须放行，
 * 否则刷新后会把自己刚在用的工作区判成「已被窗口 [X] 占用」并弹出选择框。
 */
export function isOtherWindow(holder?: string | null): boolean {
  // 取不到 label（非桌面环境）时退化为空串，与任何持有者都不相等 ⇒ 一律按「被占用」处理，
  // 这是最保守的口径：宁可提示一次，也不要让两个窗口同时打开同一个工作区。
  const me = currentWindowLabel()
  return !!holder && holder !== me
}

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
  },

  /** 尝试锁定工作区（防止多窗口同时打开同一工作区） */
  async lock(workspaceId: string, windowLabel: string): Promise<{ locked: boolean; holder?: string }> {
    const res = await invoke<ResponseData<boolean>>('workspace_lock', {
      workspaceId,
      windowLabel,
    })
    if (res.code === 200) {
      return { locked: true }
    }
    // 提取占用者窗口标签
    const match = res.msg.match(/\[(.+?)\]/)
    return { locked: false, holder: match ? match[1] : undefined }
  },

  /** 释放工作区锁 */
  async unlock(workspaceId: string, windowLabel: string): Promise<void> {
    await invoke('workspace_unlock', { workspaceId, windowLabel })
  },

  /** 查询工作区锁定状态（持有者 window_label；未锁定返回 null） */
  async lockStatus(workspaceId: string): Promise<string | null> {
    const res = await invoke<ResponseData<string | null>>('workspace_lock_status', { workspaceId })
    return res.code === 200 ? res.data : null
  },

  /** 查询工作区是否被**其他窗口**占用：返回占用者 label；未锁定或本窗口自己持有时返回 null */
  async lockedByOther(workspaceId: string): Promise<string | null> {
    const holder = await workspaceService.lockStatus(workspaceId).catch(() => null)
    return isOtherWindow(holder) ? holder! : null
  },
}
