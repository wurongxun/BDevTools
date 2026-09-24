/**
 * 统一 RPC 层 —— 前端唯一允许调用后端命令的入口。
 *
 * 业务代码只允许从本文件获取 `invoke` / `call`，禁止直接 import '@tauri-apps/api/core'。
 * 将来更换框架或改为 Web 版时，只需重写本文件的实现（invoke → fetch/axios 等）。
 */
import { invoke } from '@tauri-apps/api/core'

/** 调用后端命令（与 Tauri invoke 同名同签名，便于调用点零改动地迁移） */
export { invoke }

/**
 * IPC 流式通道（Tauri v2 Channel）。
 * 用于「一次调用、多次回推」的场景（终端输出、流式日志），
 * 与全局事件（runtime.publish/subscribe）的区别：Channel 的生命周期绑定在单次 invoke 上，
 * 天然按会话隔离且带消息序号（IPC 层保证按序投递）。
 */
export { Channel } from '@tauri-apps/api/core'

/** 推荐使用的统一调用入口 */
export function call<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args)
}
