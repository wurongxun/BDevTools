/**
 * P3 会话日志录制：把终端输出 / 输入按行加时间戳，攒批写到后端日志文件。
 *
 * 两个刻意的设计：
 * - 写日志前 `stripAnsi`，日志是给人看的纯文本，不该混入光标控制序列；
 * - 会话密码 / 私钥口令在写盘前一律替换成 `***`，避免「录制日志」变成新的凭据泄漏点。
 */
import { invoke } from '@/api/rpc'
import type { LogSettings } from '../types'
import { stripAnsi } from './highlight'

function pad(n: number): string {
  return String(n).padStart(2, '0')
}

function stamp(): string {
  const d = new Date()
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ` +
    `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

/** 后端只接受 ASCII 字母数字 . _ -，会话名里的中文与空格全部折叠 */
export function logFileName(connectionName: string, tabId: string): string {
  const name = (connectionName || 'session')
    .replace(/[^A-Za-z0-9_-]/g, '_')
    .replace(/_+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 40) || 'session'
  const suffix = tabId.replace(/[^A-Za-z0-9]/g, '').slice(-8) || 'tab'
  return `ssh-${name}-${stamp().replace(/[-: ]/g, '')}-${suffix}.log`
}

export class SessionRecorder {
  active = false
  fileName = ''
  private buffer = ''
  private lineTail = ''
  private timer: ReturnType<typeof setInterval> | null = null
  private flushing = false

  constructor(
    private settings: () => LogSettings,
    private secrets: () => string[]
  ) {}

  start(connectionName: string, tabId: string): string {
    if (this.active) return this.fileName
    this.fileName = logFileName(connectionName, tabId)
    this.buffer = ''
    this.lineTail = ''
    this.active = true
    this.writeLine(`===== 开始录制 ${connectionName} @ ${stamp()} =====`, 'out')
    this.timer = setInterval(() => { void this.flush() }, 800)
    return this.fileName
  }

  /** direction: out = 远端输出，in = 本地输入 */
  append(text: string, direction: 'in' | 'out') {
    if (!this.active || !text) return
    const cfg = this.settings()
    if (direction === 'in' && !cfg.logInput) return
    // 终端输出常用 \r\n，用户输入回车则是单独的 \r；两者都当行结束，
    // 否则「输入了一整行命令」会一直挂在 lineTail 里不落盘。
    const plain = stripAnsi(text).replace(/\r\n/g, '\n').replace(/\r/g, '\n')
    const lines = (this.lineTail + plain).split('\n')
    this.lineTail = lines.pop() || ''
    for (const line of lines) {
      if (line.length || direction === 'out') {
        this.writeLine(line, direction)
      }
    }
    if (this.buffer.length >= 32 * 1024) void this.flush()
  }

  private writeLine(line: string, direction: 'in' | 'out') {
    const cfg = this.settings()
    let safe = line
    if (cfg.maskSensitive) {
      for (const secret of this.secrets()) {
        if (secret && secret.length >= 2) safe = safe.split(secret).join('***')
      }
    }
    const prefix = cfg.timestamp ? `[${stamp()}] ` : ''
    const tag = direction === 'in' ? '> ' : ''
    this.buffer += `${prefix}${tag}${safe}\n`
  }

  async flush(): Promise<void> {
    if (!this.active || !this.buffer || this.flushing) return
    this.flushing = true
    const text = this.buffer
    this.buffer = ''
    try {
      const res = await invoke<any>('ssh_log_append', { name: this.fileName, text })
      if (res?.code !== 200) {
        // 写失败时把内容放回队首，下一次 flush 重试（不会因为一次 IPC 抖动丢日志）
        this.buffer = text + this.buffer
      }
    } catch {
      this.buffer = text + this.buffer
    } finally {
      this.flushing = false
    }
  }

  async stop(): Promise<void> {
    if (!this.active) return
    this.writeLine(`===== 结束录制 @ ${stamp()} =====`, 'out')
    if (this.lineTail) {
      this.writeLine(this.lineTail, 'out')
      this.lineTail = ''
    }
    if (this.timer) {
      clearInterval(this.timer)
      this.timer = null
    }
    await this.flush()
    this.active = false
  }
}
