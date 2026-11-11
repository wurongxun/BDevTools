/**
 * files → 集成终端（M4 / F4）。
 *
 * 只依赖 `core/terminalBridge` 的事件契约，不 import terminal 插件（插件边界守卫）。
 * terminal 被裁剪时 `hasTerminalLaunchHandler()` 为 false，直接给出明确提示，
 * 绝不会静默失败或抛异常。命令拼串只发生在「用户显式右键运行」这条路径上。
 */
import { hasTerminalLaunchHandler, requestTerminalLaunch } from '@/core/terminalBridge'
import NotificationUtil from '@/utils/NotificationUtil'
import { useDocumentManagerStore } from '@/store/modules/documents'

/** 文件所在目录；路径分隔符兼容 Windows / POSIX，无目录时返回空串。 */
export function dirnameOf(path: string): string {
  if (!path) return ''
  const idx = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'))
  return idx > 0 ? path.slice(0, idx) : ''
}

export function basenameOf(path: string): string {
  return path.split(/[/\\]/).pop() || path
}

function quote(p: string): string {
  return `"${p.replace(/"/g, '\\"')}"`
}

/** 按扩展名生成运行命令；返回 null 表示没有预设运行器（只打开终端，让用户自己输）。 */
export function runnerFor(path: string): string | null {
  const ext = (basenameOf(path).split('.').pop() || '').toLowerCase()
  switch (ext) {
    case 'py': return `python ${quote(path)}`
    case 'js': case 'mjs': case 'cjs': return `node ${quote(path)}`
    case 'ts': case 'mts': return `npx tsx ${quote(path)}`
    case 'sh': return `bash ${quote(path)}`
    case 'ps1': return `powershell -ExecutionPolicy Bypass -File ${quote(path)}`
    case 'bat': case 'cmd': return quote(path)
    case 'rb': return `ruby ${quote(path)}`
    case 'go': return `go run ${quote(path)}`
    case 'rs': return `cargo run --manifest-path ${quote(path)}`
    default: return null
  }
}

function unavailable(): boolean {
  if (hasTerminalLaunchHandler()) return false
  NotificationUtil.warn('集成终端插件未启用或未加载，无法在终端中打开。请先在插件管理中启用 terminal。')
  return true
}

/** 在集成终端打开目标所在目录（未给路径时用当前活动文档）。 */
export function openInTerminal(path?: string): boolean {
  if (unavailable()) return false
  const target = path || useDocumentManagerStore().activeDocumentPath || ''
  const cwd = dirnameOf(target)
  if (!cwd) {
    NotificationUtil.warn('无法从当前文件推导工作目录')
    return false
  }
  return requestTerminalLaunch({
    cwd,
    title: basenameOf(cwd) || 'files',
    source: 'files:open'
  })
}

/** 文件树文件夹节点：直接以该目录为 cwd 打开新会话（不再往上取父目录）。 */
export function openDirectoryInTerminal(dir: string): boolean {
  if (unavailable()) return false
  if (!dir) {
    NotificationUtil.warn('目录路径为空')
    return false
  }
  return requestTerminalLaunch({
    cwd: dir,
    title: basenameOf(dir) || 'files',
    source: 'files:tree'
  })
}

/** 运行当前脚本：新开一个以文件目录为 cwd 的终端会话，会话就绪后送入命令。 */
export function runCurrentFile(path?: string): boolean {
  const target = path || useDocumentManagerStore().activeDocumentPath || ''
  if (!target) {
    NotificationUtil.warn('请先打开一个可运行的文件')
    return false
  }
  const command = runnerFor(target)
  if (!command) {
    NotificationUtil.warn(`没有 ${basenameOf(target)} 的预设运行器，已改为只打开终端`)
    return openInTerminal(target)
  }
  if (unavailable()) return false
  return requestTerminalLaunch({
    cwd: dirnameOf(target),
    command,
    title: basenameOf(target),
    source: 'files:run'
  })
}
