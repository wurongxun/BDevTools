/**
 * 会话管理服务
 * 封装会话快照读写相关的 Tauri invoke 调用
 */
import { invoke } from '@tauri-apps/api/core'
import type { SessionSnapshot, SessionSummary, DraftCheckResult, DraftMeta } from '@/Types/workspace'
import type { ResponseData } from '@/entity/entitys'

export const sessionService = {
  /** 保存会话快照 */
  async save(snapshot: SessionSnapshot): Promise<boolean> {
    const res = await invoke<ResponseData<boolean>>('session_save', {
      snapshotData: snapshot
    })
    return res.code === 200
  },

  /** 加载指定窗口的会话快照（兼容旧键） */
  async load(windowLabel: string): Promise<SessionSnapshot | null> {
    const res = await invoke<ResponseData<SessionSnapshot | null>>('session_load', {
      windowLabel
    })
    return res.code === 200 ? res.data : null
  },

  /** 按工作区 ID 加载会话快照（多窗口体系主入口） */
  async loadByWorkspace(workspaceId: string): Promise<SessionSnapshot | null> {
    const res = await invoke<ResponseData<SessionSnapshot | null>>('session_load_workspace', {
      workspaceId
    })
    return res.code === 200 ? res.data : null
  },

  /** 按工作区 ID 删除会话快照 */
  async deleteByWorkspace(workspaceId: string): Promise<boolean> {
    const res = await invoke<ResponseData<boolean>>('session_delete_workspace', {
      workspaceId
    })
    return res.code === 200
  },

  /** 加载所有会话快照 */
  async loadAll(): Promise<SessionSnapshot[]> {
    const res = await invoke<ResponseData<SessionSnapshot[]>>('session_load_all')
    return res.code === 200 ? res.data : []
  },

  /** 列出所有会话摘要 */
  async list(): Promise<SessionSummary[]> {
    const res = await invoke<ResponseData<SessionSummary[]>>('session_list')
    return res.code === 200 ? res.data : []
  },

  /** 删除指定窗口的会话快照 */
  async delete(windowLabel: string): Promise<boolean> {
    const res = await invoke<ResponseData<boolean>>('session_delete', {
      windowLabel
    })
    return res.code === 200
  },

  /** 清除所有会话快照 */
  async clear(): Promise<boolean> {
    const res = await invoke<ResponseData<boolean>>('session_clear')
    return res.code === 200
  },

  // ===== 草稿管理（未保存内容保护，按工作区隔离） =====

  /** 保存草稿（返回包含目标文件指纹的元数据） */
  async draftSave(workspaceId: string, filePath: string, content: string): Promise<DraftMeta | null> {
    const res = await invoke<ResponseData<DraftMeta>>('draft_save', {
      workspaceId, filePath, content
    })
    return res.code === 200 ? res.data : null
  },

  /** 检查草稿状态（是否存在 + 目标文件是否被外部修改） */
  async draftCheck(workspaceId: string, filePath: string): Promise<DraftCheckResult | null> {
    const res = await invoke<ResponseData<DraftCheckResult>>('draft_check', {
      workspaceId, filePath
    })
    return res.code === 200 ? res.data : null
  },

  /** 读取草稿内容 */
  async draftLoad(workspaceId: string, filePath: string): Promise<string | null> {
    const res = await invoke<ResponseData<string | null>>('draft_load', {
      workspaceId, filePath
    })
    return res.code === 200 ? res.data : null
  },

  /** 删除指定文件的草稿 */
  async draftDelete(workspaceId: string, filePath: string): Promise<boolean> {
    const res = await invoke<ResponseData<boolean>>('draft_delete', {
      workspaceId, filePath
    })
    return res.code === 200
  }
}
