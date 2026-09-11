/**
 * 日志行解析器
 *
 * 设计定位：启发式"尽力而为"解析，服务于前端高亮/悬浮提示等展示需求。
 * 后续批次接入后端索引后，级别与时间筛选将以服务端结果为准，本模块退化为兜底。
 */
import {
  FILE_LINE_RE,
  JAVA_CAUSE_RE,
  JAVA_FRAME_RE,
  JAVA_THROWN_RE,
  LEVEL_RE,
  LOGGER_RE,
  PYTHON_FRAME_RE,
  PYTHON_TRACEBACK_RE,
  STACK_MORE_RE,
  THREAD_RE,
  TS_RE,
} from './patterns'
import { LEVEL_ALIASES, type LogLevel, type ParsedLine, type StackInfo, type TextRange } from '../types'

/** 在原文中定位一段文本（从 from 之后首次出现），用于还原捕获组的真实位置 */
function locate(raw: string, text: string, from = 0): TextRange | null {
  if (!text) return null
  const i = raw.indexOf(text, from)
  return i === -1 ? null : { start: i, end: i + text.length }
}

/** 时间戳文本 → 毫秒值；无法解析（如缺年份）时返回 undefined */
function toMillis(text: string): number | undefined {
  const t = Date.parse(text.replace(',', '.').replace(' ', 'T'))
  return Number.isNaN(t) ? undefined : t
}

interface TsMatch {
  text: string
  ts?: number
  range: TextRange
}

function matchTimestamp(raw: string): TsMatch | null {
  const m = TS_RE.exec(raw)
  if (!m) return null
  const text = m[2]
  const lead = m[0].slice(0, m[0].indexOf(text))
  const start = (m.index ?? 0) + lead.length
  return { text, ts: toMillis(text), range: { start, end: start + text.length } }
}

/**
 * 识别异常堆栈行（Java / Python / 通用省略提示）
 * 返回值中的 level 仅由"异常首行"设置（thrown / Caused by 为 ERROR），
 * 堆栈帧行返回 undefined，由 parseLines 继承所属异常首行的级别。
 */
function matchStack(raw: string): StackInfo | null {
  let m = JAVA_FRAME_RE.exec(raw)
  if (m) {
    const parts: StackInfo['parts'] = []
    const cls = m[1].replace(/\.$/, '')
    let cursor = 0
    const clsRange = locate(raw, cls, cursor)
    if (clsRange) {
      parts.push({ ...clsRange, kind: 'cls' })
      cursor = clsRange.end
    }
    const methodRange = locate(raw, m[2], cursor)
    if (methodRange) {
      parts.push({ ...methodRange, kind: 'method' })
      cursor = methodRange.end
    }
    const fl = FILE_LINE_RE.exec(m[3])
    if (fl) {
      const fileRange = locate(raw, fl[1], cursor)
      if (fileRange) {
        parts.push({ ...fileRange, kind: 'file' })
        cursor = fileRange.end
      }
      const lineRange = locate(raw, fl[2], cursor)
      if (lineRange) parts.push({ ...lineRange, kind: 'line' })
    }
    return { lang: 'java', parts, signature: `${cls}.${m[2]}` }
  }

  m = PYTHON_FRAME_RE.exec(raw)
  if (m) {
    const parts: StackInfo['parts'] = []
    const fileRange = locate(raw, m[1])
    if (fileRange) parts.push({ ...fileRange, kind: 'file' })
    const lineRange = locate(raw, m[2], fileRange?.end ?? 0)
    if (lineRange) parts.push({ ...lineRange, kind: 'line' })
    if (m[3]) {
      const methodRange = locate(raw, m[3], lineRange?.end ?? 0)
      if (methodRange) parts.push({ ...methodRange, kind: 'method' })
    }
    return { lang: 'python', parts, signature: `${m[1]}:${m[2]}` }
  }

  m = JAVA_CAUSE_RE.exec(raw)
  if (m) {
    const parts: StackInfo['parts'] = []
    const clsRange = locate(raw, m[1])
    if (clsRange) parts.push({ ...clsRange, kind: 'cls' })
    if (m[2]) {
      const msgRange = locate(raw, m[2], clsRange?.end ?? 0)
      if (msgRange) parts.push({ ...msgRange, kind: 'message' })
    }
    return { lang: 'java', parts, signature: m[1], level: 'ERROR' }
  }

  m = JAVA_THROWN_RE.exec(raw)
  if (m) {
    const parts: StackInfo['parts'] = []
    const clsRange = locate(raw, m[1])
    if (clsRange) parts.push({ ...clsRange, kind: 'cls' })
    if (m[2]) {
      const msgRange = locate(raw, m[2], clsRange?.end ?? 0)
      if (msgRange) parts.push({ ...msgRange, kind: 'message' })
    }
    return { lang: 'java', parts, signature: m[1], level: 'ERROR' }
  }

  // Python 回溯起始行：本身不含级别信息，交回上层继承
  if (PYTHON_TRACEBACK_RE.test(raw)) {
    return { lang: 'python', parts: [] }
  }

  if (STACK_MORE_RE.test(raw)) return { lang: 'other', parts: [] }

  return null
}

/** 解析单行 */
export function parseLine(raw: string, index: number): ParsedLine {
  const stack = matchStack(raw)
  if (stack) {
    return {
      index,
      raw,
      level: stack.level ?? 'UNKNOWN',
      message: raw.trim(),
      isStack: true,
      stack,
      ranges: {},
    }
  }

  const ranges: ParsedLine['ranges'] = {}
  let cursor = 0
  let tsText: string | undefined
  let tsMs: number | undefined

  const ts = matchTimestamp(raw)
  if (ts) {
    tsText = ts.text
    tsMs = ts.ts
    ranges.ts = ts.range
    cursor = ts.range.end
  }

  // 级别：只在时间戳之后的窗口内扫描，避免正文里的 "error" 被误判为级别
  let level: LogLevel = 'UNKNOWN'
  const lm = LEVEL_RE.exec(raw.slice(cursor, cursor + 120))
  if (lm) level = LEVEL_ALIASES[lm[1].toUpperCase()] ?? 'UNKNOWN'

  let thread: string | undefined
  const tm = THREAD_RE.exec(raw.slice(cursor))
  if (tm) {
    const bracket = locate(raw, tm[0], cursor)
    // 时间戳本身带方括号时不要重复认作线程
    if (bracket && (!ranges.ts || bracket.start >= ranges.ts.end)) {
      thread = tm[1]
      ranges.thread = { start: bracket.start + 1, end: bracket.start + 1 + tm[1].length }
      cursor = bracket.end
    }
  }

  let logger: string | undefined
  const gm = LOGGER_RE.exec(raw.slice(cursor))
  if (gm) {
    const r = locate(raw, gm[1], cursor)
    if (r) {
      logger = gm[1]
      ranges.logger = r
      cursor = r.end
    }
  }

  return {
    index,
    raw,
    level,
    tsText,
    ts: tsMs,
    thread,
    logger,
    message: raw.slice(cursor).trim(),
    isStack: false,
    ranges,
  }
}

/**
 * 解析整段文本
 *
 * 级别继承规则：异常首行自带 ERROR（thrown / Caused by），其后紧跟的堆栈帧
 * 继承该异常首行的级别；异常块结束后，回到继承上一条普通日志行的级别。
 * 这样整个异常块会与其主行同色，而不是错用前一条无关日志的级别。
 */
export function parseLines(text: string, firstLineNumber = 1): ParsedLine[] {
  const rawLines = text.split(/\r?\n/)
  if (rawLines.length && rawLines[rawLines.length - 1] === '') rawLines.pop()

  const out: ParsedLine[] = []
  let lastLevel: LogLevel = 'UNKNOWN'
  let stackLevel: LogLevel | null = null

  for (let i = 0; i < rawLines.length; i++) {
    const line = parseLine(rawLines[i], firstLineNumber + i)
    if (line.isStack) {
      if (line.level === 'UNKNOWN') line.level = stackLevel ?? lastLevel
      stackLevel = line.level
    } else {
      if (line.level !== 'UNKNOWN') lastLevel = line.level
      stackLevel = null
    }
    out.push(line)
  }
  return out
}
