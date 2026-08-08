/**
 * 会话管理服务
 * 封装会话快照读写相关的 Tauri invoke 调用
 */
import { invoke } from '@tauri-apps/api/core'
import type { SessionSnapshot, SessionSummary } from '@/Types/workspace'
import type { ResponseData } from '@/entity/entitys'

export const sessionService = {
  /** 保存会话快照 */
  async save(snapshot: SessionSnapshot): Promise<boolean> {
    const res = await invoke<ResponseData<boolean>>('session_save', {
      snapshotData: snapshot
    })
    return res.code === 200
  },

  /** 加载指定窗口的会话快照 */
  async load(windowLabel: string): Promise<SessionSnapshot | null> {
    const res = await invoke<ResponseData<SessionSnapshot | null>>('session_load', {
      windowLabel
    })
    return res.code === 200 ? res.data : null
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
  }
}
