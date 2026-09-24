/**
 * terminal 插件后端 API 封装
 *
 * 前端与 `src-tauri/src/command/terminal.rs` 之间的唯一接口。
 * 输出流走 Tauri v2 `Channel`（一次 invoke、多次回推，IPC 层带序号保证按序投递），
 * 与全局事件的区别见 `@/api/rpc` 的注释。
 */
import { call, Channel } from '@/api/rpc'
import type { ShellInfo, TermDefaults, TermEvent, TermOpenOptions, TermOpenResult } from './types'

interface ApiResp<T> {
  code: number
  msg: string
  data: T
}

function unwrap<T>(res: ApiResp<T>, what: string): T {
  if (res.code !== 200) {
    throw new Error(res.msg || `${what}失败`)
  }
  return res.data
}

/**
 * 打开一个 PTY 会话。
 *
 * `onEvent` 会持续收到三类事件：
 * - `data`：base64 编码的原始输出字节（前端解码为 Uint8Array 交给 xterm）
 * - `exit`：子进程退出（唯一一次）
 * - `error`：读线程异常（不影响已收到的输出）
 */
export async function openTerminal(
  options: TermOpenOptions,
  onEvent: (ev: TermEvent) => void,
): Promise<TermOpenResult> {
  const channel = new Channel<TermEvent>()
  channel.onmessage = onEvent
  const res = await call<ApiResp<TermOpenResult>>('terminal_open', { options, onEvent: channel })
  return unwrap(res, '打开终端')
}

/** 写入按键 / 粘贴内容 */
export async function writeTerminal(sessionId: string, data: string): Promise<void> {
  const res = await call<ApiResp<boolean>>('terminal_write', { sessionId, data })
  unwrap(res, '写入终端')
}

/** 调整终端尺寸（通知 PTY → shell 收到 SIGWINCH / ConPTY 同步） */
export async function resizeTerminal(sessionId: string, cols: number, rows: number): Promise<void> {
  const res = await call<ApiResp<boolean>>('terminal_resize', { sessionId, cols, rows })
  unwrap(res, '调整终端尺寸')
}

/** 关闭会话（kill shell + 释放资源） */
export async function closeTerminal(sessionId: string): Promise<void> {
  const res = await call<ApiResp<boolean>>('terminal_close', { sessionId })
  unwrap(res, '关闭终端')
}

/** 会话是否存活 */
export async function terminalAlive(sessionId: string): Promise<boolean> {
  const res = await call<ApiResp<boolean>>('terminal_alive', { sessionId })
  return res.code === 200 ? res.data : false
}

/** 本机可用 shell 列表 */
export async function listShells(): Promise<ShellInfo[]> {
  const res = await call<ApiResp<ShellInfo[]>>('terminal_list_shells')
  return res.code === 200 ? res.data : []
}

/** 终端默认启动信息（默认 shell / 默认 cwd / 平台） */
export async function fetchTerminalDefaults(): Promise<TermDefaults> {
  const res = await call<ApiResp<TermDefaults>>('terminal_defaults')
  return unwrap(res, '读取终端默认配置')
}
