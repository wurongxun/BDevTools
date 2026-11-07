/**
 * P3 自动化：期望-发送（自动登录）规则 + 声明式宏引擎。
 *
 * 安全口径：**不使用 eval / new Function**。导入的配置只能发字符串、等待或匹配输出，
 * 不能变成任意代码执行。密码只在变量展开时短暂出现，且由 sensitive 规则标记，
 * 调用方据此跳过日志录制与界面回显。
 */
import type { ExpectRule, MacroRunState, MacroScript, MacroStep } from '../types'
import { stripAnsi } from './highlight'

export interface AutomationVars {
  password: string
  passphrase: string
  username: string
  host: string
}

export interface AutomationContext {
  write(data: string): void
  vars(): AutomationVars
  onMacroState?(state: MacroRunState, stepIndex: number, message: string): void
  onNotice?(message: string): void
}

export function expandVars(text: string, vars: AutomationVars): string {
  return text.replace(/\$\{(password|passphrase|username|host)\}/gi, (_, key: string) => {
    return String((vars as any)[key.toLowerCase()] ?? '')
  })
}

function compilePattern(pattern: string): RegExp | null {
  // 编辑器启用了多行文本域，粘贴正则时可能带换行；去掉换行但保留首尾空格
  // （提示符正则如 `\$ ` 的末尾空格是有意义的，不能 trim）
  const text = (pattern || '').replace(/[\r\n]+/g, '')
  if (!text) return null
  try { return new RegExp(text, 'i') } catch { return null }
}

/** 自动登录规则引擎：每次连接 reset，命中一次即消费缓冲，避免重复发送密码 */
export class ExpectRuleEngine {
  private fired = new Set<string>()
  private buffer = ''

  constructor(
    private rules: () => ExpectRule[],
    private ctx: AutomationContext
  ) {}

  reset() {
    this.fired.clear()
    this.buffer = ''
  }

  /** 命中时返回 true，调用方据此跳过这行输入的日志记录 */
  feed(text: string): boolean {
    if (!text) return false
    this.buffer = (this.buffer + stripAnsi(text)).slice(-8192)
    for (const rule of this.rules()) {
      if (!rule.enabled) continue
      if (rule.once && this.fired.has(rule.id)) continue
      const re = compilePattern(rule.pattern)
      if (!re || !re.test(this.buffer)) continue
      this.fired.add(rule.id)
      const payload = expandVars(rule.send, this.ctx.vars())
      if (payload) {
        const endsWithNewline = /[\r\n]$/.test(payload)
        this.ctx.write(endsWithNewline ? payload : `${payload}\r`)
      }
      this.buffer = ''
      this.ctx.onNotice?.(`自动登录规则已触发：${rule.name || rule.pattern}`)
      return true
    }
    return false
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, Math.max(0, ms)))
}

/**
 * 宏运行器。步骤只有 send / wait / expect 三种：
 * - send：展开变量后写入（可选自动回车）
 * - wait：等待固定毫秒
 * - expect：等待远端输出匹配正则，超时即失败（不会静默卡死）
 */
export class MacroRunner {
  state: MacroRunState = 'idle'
  private stopped = false
  private buffer = ''
  private waiters: Array<{ re: RegExp; resolve: () => void; reject: (e: Error) => void; timer: any }> = []

  constructor(private ctx: AutomationContext) {}

  get running(): boolean {
    return this.state === 'running'
  }

  feedOutput(text: string) {
    const plain = stripAnsi(text)
    this.buffer = (this.buffer + plain).slice(-16384)
    for (const waiter of [...this.waiters]) {
      if (waiter.re.test(this.buffer)) {
        clearTimeout(waiter.timer)
        this.waiters = this.waiters.filter(w => w !== waiter)
        this.buffer = ''
        waiter.resolve()
      }
    }
  }

  stop() {
    if (this.state !== 'running') return
    this.stopped = true
    for (const waiter of this.waiters) {
      clearTimeout(waiter.timer)
      waiter.reject(new Error('宏已停止'))
    }
    this.waiters = []
  }

  private waitFor(pattern: string, timeoutMs: number): Promise<void> {
    const re = compilePattern(pattern)
    if (!re) return Promise.reject(new Error(`无效的等待表达式：${pattern}`))
    if (re.test(this.buffer)) {
      this.buffer = ''
      return Promise.resolve()
    }
    return new Promise<void>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.waiters = this.waiters.filter(w => w !== entry)
        reject(new Error(`等待超时：${pattern}`))
      }, Math.max(200, Math.min(timeoutMs || 15000, 120000)))
      const entry = { re, resolve, reject, timer }
      this.waiters.push(entry)
    })
  }

  async run(macro: MacroScript) {
    if (this.state === 'running') {
      this.ctx.onNotice?.('已有宏正在运行')
      return
    }
    this.stopped = false
    this.buffer = ''
    this.state = 'running'
    this.ctx.onMacroState?.('running', 0, `开始运行：${macro.name}`)
    try {
      for (let i = 0; i < macro.steps.length; i++) {
        if (this.stopped) throw new Error('__stopped__')
        const step: MacroStep = macro.steps[i]
        if (step.kind === 'send') {
          const payload = expandVars(step.value, this.ctx.vars())
          // 多行编辑框里是 \n；PTY 行结束需要 \r，否则部分 shell 不会把每行当独立命令执行。
          // newline=true 时先去掉文本自带的尾部换行，避免额外多敲一次空回车。
          const normalized = payload.replace(/\r\n?|\n/g, '\r')
          const body = step.newline ? normalized.replace(/\r+$/, '') : normalized
          if (body) this.ctx.write(step.newline ? `${body}\r` : body)
        } else if (step.kind === 'wait') {
          // 切片等待，stop() 能及时打断长延时
          let left = Math.max(0, Math.min(step.arg || 0, 60000))
          while (left > 0 && !this.stopped) {
            const slice = Math.min(100, left)
            await sleep(slice)
            left -= slice
          }
        } else if (step.kind === 'expect') {
          await this.waitFor(step.value, step.arg)
        }
        this.ctx.onMacroState?.('running', i + 1, describeStep(step))
      }
      if (this.stopped) throw new Error('__stopped__')
      this.state = 'done'
      this.ctx.onMacroState?.('done', macro.steps.length, `宏执行完成：${macro.name}`)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      if (message === '__stopped__' || this.stopped) {
        this.state = 'stopped'
        this.ctx.onMacroState?.('stopped', 0, `宏已停止：${macro.name}`)
      } else {
        this.state = 'failed'
        this.ctx.onMacroState?.('failed', 0, message)
      }
    }
  }
}

export function describeStep(step: MacroStep): string {
  if (step.kind === 'send') return `发送：${step.value}`
  if (step.kind === 'wait') return `等待 ${step.arg}ms`
  return `等待输出匹配：${step.value}`
}

/** 开箱即用的两条常见规则（用户可在设置中心改 / 删） */
export function defaultExpectRules(): ExpectRule[] {
  return [
    {
      id: 'expect-password',
      name: '密码提示',
      pattern: '(password|密码|passphrase)[:\\s：]*$',
      send: '${password}',
      once: true,
      enabled: false,
      sensitive: true
    },
    {
      id: 'expect-passphrase',
      name: '私钥口令提示',
      pattern: 'enter passphrase for key',
      send: '${passphrase}',
      once: true,
      enabled: false,
      sensitive: true
    }
  ]
}
