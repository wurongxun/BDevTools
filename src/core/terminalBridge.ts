/**
 * 「打开集成终端」的宿主事件契约（M4 / F4）。
 *
 * 为什么需要它：插件边界守卫禁止 files 直接 import terminal 插件，而 terminal 是一个
 * 停靠面板插件、没有注册命令面板命令。两边都只依赖 core 里的这份契约：
 * 请求方派发 CustomEvent，terminal 插件订阅；terminal 被裁剪 / 面板未加载时
 * `requestTerminalLaunch()` 返回 false，请求方可以优雅降级提示，不会抛异常。
 *
 * 刻意不用 runtime.publish：那是 Tauri emit（跨窗口广播 + IPC），这里只是同窗口
 * 的一次性本地意图，与 files 内部 `navigation/editorEvents.ts` 同范式。
 */

export interface TerminalLaunchRequest {
  /** 新会话的工作目录（文件所在目录 / 选中的文件夹） */
  cwd?: string
  /** 会话就绪后自动送入 PTY 的命令（不含回车） */
  command?: string
  /** 标签标题 */
  title?: string
  /** 便于日志定位的请求来源 */
  source?: string
}

const EVENT = 'bdevtools:terminal-launch'

let handlerCount = 0

/** 是否已经有订阅者（terminal 插件启用并加载）。 */
export function hasTerminalLaunchHandler(): boolean {
  return handlerCount > 0
}

/** 派发请求；返回是否有人接收。返回 false 时调用方应提示用户，不要静默失败。 */
export function requestTerminalLaunch(req: TerminalLaunchRequest): boolean {
  if (typeof window === 'undefined') return false
  window.dispatchEvent(new CustomEvent<TerminalLaunchRequest>(EVENT, { detail: req }))
  return handlerCount > 0
}

export function onTerminalLaunchRequest(
  handler: (req: TerminalLaunchRequest) => void
): () => void {
  const listener = (e: Event) => {
    const detail = (e as CustomEvent<TerminalLaunchRequest>).detail
    if (detail && typeof detail === 'object') handler(detail)
  }
  window.addEventListener(EVENT, listener)
  handlerCount += 1
  return () => {
    window.removeEventListener(EVENT, listener)
    handlerCount = Math.max(0, handlerCount - 1)
  }
}
