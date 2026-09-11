/**
 * 行高亮渲染引擎
 *
 * 输入一行解析结果 + 命中/规则，输出可直接 v-html 的安全 HTML。
 * 采用"区间收集 + 优先级裁剪"：区间重叠时优先级高者占位，低者被丢弃，
 * 避免嵌套标签导致的渲染错乱。
 *
 * 优先级（数字越小越优先）：
 *   1 当前聚焦命中 > 2 自定义规则 > 3 搜索命中 > 4 堆栈片段 > 5 元信息（时间戳/线程/logger）
 */
import type { HighlightRule, ParsedLine, TextRange } from '../types'

interface Segment {
  start: number
  end: number
  cls: string
  style?: string
  pri: number
}

export interface RenderOptions {
  /** 当前行的搜索命中区间 */
  hitRanges?: TextRange[]
  /** 当前聚焦的命中区间 */
  currentHit?: TextRange | null
  /** 用户自定义规则 */
  rules?: HighlightRule[]
  /** 单条规则在单行内的最大命中数（防超长行卡顿） */
  maxRuleHits?: number
}

const ESC_MAP: Record<string, string> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
  "'": '&#39;',
}

/** HTML 转义（日志内容来自外部文件，渲染前必须转义） */
export function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, c => ESC_MAP[c])
}

/** 按规则匹配文本，返回命中区间 */
function matchRanges(text: string, rule: HighlightRule, limit: number): TextRange[] {
  const out: TextRange[] = []
  if (!rule.pattern) return out

  if (rule.useRegex) {
    let re: RegExp
    try {
      re = new RegExp(rule.pattern, rule.ignoreCase ? 'gi' : 'g')
    } catch {
      return out // 非法正则静默跳过，不阻断渲染
    }
    let m: RegExpExecArray | null
    let guard = 0
    while ((m = re.exec(text)) !== null) {
      if (m[0].length === 0) {
        re.lastIndex++ // 零宽匹配保护
        continue
      }
      out.push({ start: m.index, end: m.index + m[0].length })
      if (out.length >= limit) break
      if (++guard > limit * 4) break
    }
    return out
  }

  const hay = rule.ignoreCase ? text.toLowerCase() : text
  const needle = rule.ignoreCase ? rule.pattern.toLowerCase() : rule.pattern
  if (!needle) return out
  let from = 0
  while (out.length < limit) {
    const i = hay.indexOf(needle, from)
    if (i === -1) break
    out.push({ start: i, end: i + needle.length })
    from = i + needle.length
  }
  return out
}

/** 渲染一行为高亮 HTML */
export function renderLine(line: ParsedLine, opts: RenderOptions = {}): string {
  const raw = line.raw
  if (!raw) return ''

  const segs: Segment[] = []

  const { ts, thread, logger } = line.ranges
  if (ts) segs.push({ ...ts, cls: 'lg-meta lg-ts', pri: 5 })
  if (thread) segs.push({ ...thread, cls: 'lg-meta lg-thread', pri: 5 })
  if (logger) segs.push({ ...logger, cls: 'lg-meta lg-logger', pri: 5 })

  if (line.stack) {
    for (const p of line.stack.parts) {
      segs.push({ start: p.start, end: p.end, cls: `lg-stack lg-stack-${p.kind}`, pri: 4 })
    }
  }

  const limit = opts.maxRuleHits ?? 50
  for (const rule of opts.rules ?? []) {
    if (!rule.enabled || !rule.pattern) continue
    const style = `background:${rule.color};${rule.textColor ? `color:${rule.textColor};` : ''}`
    for (const r of matchRanges(raw, rule, limit)) {
      segs.push({ ...r, cls: 'lg-rule', style, pri: 2 })
    }
  }

  for (const r of opts.hitRanges ?? []) {
    segs.push({ ...r, cls: 'lg-hit', pri: 3 })
  }
  if (opts.currentHit) {
    segs.push({ ...opts.currentHit, cls: 'lg-hit lg-hit-current', pri: 1 })
  }

  if (!segs.length) return escapeHtml(raw)

  segs.sort((a, b) => a.start - b.start || a.pri - b.pri || a.end - b.end)

  let out = ''
  let pos = 0
  for (const s of segs) {
    if (s.start < pos || s.end <= pos) continue // 与已输出区间重叠则丢弃
    if (s.start > pos) out += escapeHtml(raw.slice(pos, s.start))
    out += `<span class="${s.cls}"${s.style ? ` style="${s.style}"` : ''}>${escapeHtml(
      raw.slice(s.start, s.end),
    )}</span>`
    pos = s.end
  }
  if (pos < raw.length) out += escapeHtml(raw.slice(pos))
  return out
}

/** 根据背景色估算可读前景色（自定义规则未指定文字色时使用） */
export function contrastText(bg: string): string {
  const hex = bg.replace('#', '')
  if (hex.length !== 3 && hex.length !== 6) return 'inherit'
  const full =
    hex.length === 3
      ? hex
          .split('')
          .map(c => c + c)
          .join('')
      : hex
  const r = parseInt(full.slice(0, 2), 16)
  const g = parseInt(full.slice(2, 4), 16)
  const b = parseInt(full.slice(4, 6), 16)
  if ([r, g, b].some(Number.isNaN)) return 'inherit'
  // 感知亮度（ITU-R BT.601）
  const lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255
  return lum > 0.6 ? '#1f1f1f' : '#ffffff'
}
