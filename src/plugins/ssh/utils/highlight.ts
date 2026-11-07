/**
 * P3 关键字高亮集：在写入 xterm **之前**给输出流注入 SGR 颜色序列。
 *
 * 为什么不用 xterm 的 registerDecoration：装饰层只能改背景矩形，改不了前景色，
 * 且要自己换算行列坐标；而输出流注入是纯字符串变换，对虚拟滚动 / 回滚缓冲 /
 * 分屏双实例都天然生效。
 *
 * 三条正确性约束：
 * 1. 只对「文本段」做匹配，ANSI 控制序列原样透传，否则会破坏远端程序的彩色输出；
 * 2. 注入着色后必须 `\x1b[0m` + 重放之前累积的 SGR，否则会把后续整行染成高亮色；
 * 3. 关键字可能跨 base64 数据块 ⇒ 尾部保留 `maxKeywordLen-1` 字符到下一块，
 *    且不完整的 ESC 序列也要整体保留（否则 `\x1b[3` + `1m` 会被切开）。
 */
import type { HighlightRule } from '../types'

export interface Highlighter {
  feed(chunk: string): string
  flush(): string
  updateRules(rules: HighlightRule[]): void
  dispose(): void
}

interface CompiledRule {
  re: RegExp
  rule: HighlightRule
}

/** CSI / OSC / 单字符 ESC 序列，global 匹配用于切段 */
const ANSI_SEQUENCE = /\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07\x1b]*(?:\x07|\x1b\\)|[@-Z\\-_])/g

export function stripAnsi(text: string): string {
  return text.replace(ANSI_SEQUENCE, '')
}

function escapeRegex(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function toRgb(hex: string): [number, number, number] | null {
  const v = hex.trim()
  const m = /^#?([0-9a-f]{6})$/i.exec(v) || /^#?([0-9a-f]{3})$/i.exec(v)
  if (!m) return null
  const raw = m[1].length === 3 ? m[1].split('').map(c => c + c).join('') : m[1]
  return [
    parseInt(raw.slice(0, 2), 16),
    parseInt(raw.slice(2, 4), 16),
    parseInt(raw.slice(4, 6), 16)
  ]
}

function sgrFor(rule: HighlightRule): string {
  const parts: string[] = []
  const fg = toRgb(rule.foreground)
  const bg = toRgb(rule.background)
  if (fg) parts.push(`\x1b[38;2;${fg[0]};${fg[1]};${fg[2]}m`)
  if (bg) parts.push(`\x1b[48;2;${bg[0]};${bg[1]};${bg[2]}m`)
  if (rule.bold) parts.push('\x1b[1m')
  if (!parts.length) return ''
  // 先清掉远端遗留的属性，再设高亮色；匹配文本之后由调用方 0m + 重放 restore
  return '\x1b[0m' + parts.join('')
}

function compile(rule: HighlightRule): CompiledRule | null {
  if (!rule.enabled || !rule.pattern) return null
  let source = rule.regex ? rule.pattern : escapeRegex(rule.pattern)
  if (rule.wholeWord) source = `\\b(?:${source})\\b`
  try {
    return { re: new RegExp(source, `g${rule.caseSensitive ? '' : 'i'}`), rule }
  } catch {
    return null
  }
}

/** 从 ESC 起找 ANSI 序列的结束下标（不含）；找不到返回 -1 */
function ansiEnd(input: string, esc: number): number {
  const head = input[esc + 1]
  if (head === '[') {
    for (let i = esc + 2; i < input.length; i++) {
      const code = input.charCodeAt(i)
      if (code >= 0x40 && code <= 0x7e) return i + 1
    }
    return -1
  }
  if (head === ']') {
    for (let i = esc + 2; i < input.length; i++) {
      if (input[i] === '\x07') return i + 1
      if (input[i] === '\x1b' && input[i + 1] === '\\') return i + 2
    }
    return -1
  }
  return head ? esc + 2 : -1
}

/** 末尾未闭合的 ESC 序列起点；没有就返回 input.length */
function incompleteAnsiStart(input: string): number {
  const esc = input.lastIndexOf('\x1b')
  if (esc < 0) return input.length
  return ansiEnd(input, esc) < 0 ? esc : input.length
}

/**
 * 「可能还没写完的关键字前缀」长度：固定保留 N 个字符解决不了跨块匹配
 * （关键字起点恰好落在切点前一位时仍会被切开）。这里按规则反查 process 的末尾：
 * - 字面量规则：末尾是否等于 pattern 的真前缀；
 * - 正则规则：无法静态判断，保守保留一小段，交给 60ms 的 tail 补写兜底。
 */
function trailingPartial(text: string, compiled: CompiledRule[]): number {
  if (!text) return 0
  let keep = 0
  for (const { rule } of compiled) {
    if (rule.regex) {
      keep = Math.max(keep, Math.min(24, Math.max(1, rule.pattern.length)))
      continue
    }
    const pattern = rule.caseSensitive ? rule.pattern : rule.pattern.toLowerCase()
    const lower = rule.caseSensitive ? text : text.toLowerCase()
    for (let k = Math.min(pattern.length - 1, text.length); k > 0; k--) {
      if (lower.endsWith(pattern.slice(0, k))) {
        keep = Math.max(keep, k)
        break
      }
    }
  }
  return Math.min(keep, 64, text.length)
}

interface Match {
  start: number
  end: number
  rule: HighlightRule
}

function applyRules(segment: string, compiled: CompiledRule[], restore: string): string {
  if (!segment || !compiled.length) return segment
  const matches: Match[] = []
  for (const { re, rule } of compiled) {
    re.lastIndex = 0
    let hit: RegExpExecArray | null
    let guard = 0
    while ((hit = re.exec(segment)) !== null && guard++ < 400) {
      if (!hit[0].length) {
        re.lastIndex++
        continue
      }
      matches.push({ start: hit.index, end: hit.index + hit[0].length, rule })
    }
  }
  if (!matches.length) return segment
  // 先出现优先；同起点取更长的匹配，避免 `error` 把 `error_code` 截半
  matches.sort((a, b) => a.start - b.start || (b.end - b.start) - (a.end - a.start))
  let out = ''
  let cursor = 0
  for (const m of matches) {
    if (m.start < cursor) continue
    const prefix = sgrFor(m.rule)
    if (!prefix) continue
    out += segment.slice(cursor, m.start)
    out += prefix + segment.slice(m.start, m.end) + '\x1b[0m' + restore
    cursor = m.end
  }
  return out + segment.slice(cursor)
}

function transform(text: string, compiled: CompiledRule[]): string {
  if (!text) return ''
  let out = ''
  let cursor = 0
  // 只保留最近 6 条 SGR：远端程序反复下发 `\x1b[31m` 时，
  // 无限累积会让每次高亮都夹带一段越来越长的恢复串（内存与输出双膨胀）。
  let sgrParts: string[] = []
  const restore = () => sgrParts.join('')
  ANSI_SEQUENCE.lastIndex = 0
  let hit: RegExpExecArray | null
  while ((hit = ANSI_SEQUENCE.exec(text)) !== null) {
    const seq = hit[0]
    out += applyRules(text.slice(cursor, hit.index), compiled, restore())
    out += seq
    if (/^\x1b\[[0-9;]*m$/.test(seq)) {
      if (seq === '\x1b[0m') sgrParts = []
      else {
        sgrParts.push(seq)
        if (sgrParts.length > 6) sgrParts = sgrParts.slice(-6)
      }
    }
    cursor = hit.index + seq.length
  }
  out += applyRules(text.slice(cursor), compiled, restore())
  return out
}

/**
 * `onTail` 是必须的补全：feed 会为「跨块关键字」保留尾部若干字符，
 * 若远端恰好在此停止输出，这几个字会永远不显示。60ms 无新数据就由调用方
 * 把尾部补写回终端 —— 延迟不可感知，且同批输出通常几毫秒内就到齐。
 */
export function createHighlighter(
  getRules: () => HighlightRule[],
  onTail?: (text: string) => void
): Highlighter {
  let compiled: CompiledRule[] = []
  let pending = ''
  let tailTimer: ReturnType<typeof setTimeout> | null = null

  function flush(): string {
    if (tailTimer) {
      clearTimeout(tailTimer)
      tailTimer = null
    }
    const tail = pending
    pending = ''
    return transform(tail, compiled)
  }

  function updateRules(rules: HighlightRule[]) {
    // 重编译前先把旧规则的尾部补出去，别在用户改规则的瞬间吞掉几个字符
    if (pending) {
      const tail = flush()
      if (tail) onTail?.(tail)
    }
    compiled = rules.map(compile).filter((r): r is CompiledRule => !!r)
  }

  updateRules(getRules())

  return {
    updateRules,
    flush,
    dispose() {
      if (tailTimer) {
        clearTimeout(tailTimer)
        tailTimer = null
      }
      pending = ''
    },
    feed(chunk: string): string {
      if (!compiled.length) return chunk
      const input = pending + chunk
      const ansiCut = incompleteAnsiStart(input)
      let process = input.slice(0, ansiCut)
      const partial = trailingPartial(process, compiled)
      if (partial > 0) {
        pending = process.slice(-partial) + input.slice(ansiCut)
        process = process.slice(0, -partial)
      } else {
        pending = input.slice(ansiCut)
      }
      // 定时器**不重置**：远端逐字符持续输出时也能保证每 60ms 必刷一次，不会饿死终端。
      if (onTail && pending && !tailTimer) {
        tailTimer = setTimeout(() => {
          tailTimer = null
          const tail = flush()
          if (tail) onTail(tail)
        }, 60)
      }
      return transform(process, compiled)
    }
  }
}
