/**
 * 统一 RPC 层 —— 前端唯一允许调用后端命令的入口。
 *
 * 业务代码只允许从本文件获取 `invoke` / `call`，禁止直接 import '@tauri-apps/api/core'。
 * 将来更换框架或改为 Web 版时，只需重写本文件的实现（invoke → fetch/axios 等）。
 */
import { invoke } from '@tauri-apps/api/core'

/** 调用后端命令（与 Tauri invoke 同名同签名，便于调用点零改动地迁移） */
export { invoke }

/** 推荐使用的统一调用入口 */
export function call<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args)
}
