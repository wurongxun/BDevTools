/**
 * 日志搜索引擎（前端增量扫描）
 *
 * 目标：输入的关键词在 40ms 内于已加载内容中完成高亮刷新。
 * 做法：仅扫描前端已加载的行（默认数千行量级），子串模式为 O(n) indexOf，
 * 正则模式带零宽保护与命中上限，避免超长行或灾难性回溯卡死 UI。
 */
import type { LineHits, ParsedLine, SearchOptions, SearchResult, TextRange } from '../types'

export type Matcher = (line: string) => TextRange[]

export interface CompileResult {
  ok: boolean
  matcher?: Matcher
  error?: string
}

/** 单行最大命中数，超出即停止（防止巨型单行拖垮渲染） */
const MAX_HITS_PER_LINE = 200

/** 编译搜索条件；正则非法时返回 ok=false 与错误信息，由 UI 提示 */
export function compileMatcher(opts: SearchOptions): CompileResult {
  const keyword = opts.keyword
  if (!keyword) return { ok: true, matcher: () => [] }

  if (opts.mode === 'regex') {
    let re: RegExp
    try {
      re = new RegExp(keyword, opts.ignoreCase ? 'gi' : 'g')
    } catch (e) {
      return { ok: false, error: (e as Error).message }
    }
    return {
      ok: true,
      matcher: (line: string): TextRange[] => {
        const out: TextRange[] = []
        re.lastIndex = 0
        let m: RegExpExecArray | null
        let guard = 0
        while ((m = re.exec(line)) !== null) {
          if (m[0].length === 0) {
            re.lastIndex++
            continue
          }
          out.push({ start: m.index, end: m.index + m[0].length })
          if (out.length >= MAX_HITS_PER_LINE) break
          if (++guard > MAX_HITS_PER_LINE * 4) break
        }
        return out
      },
    }
  }

  const needle = opts.ignoreCase ? keyword.toLowerCase() : keyword
  return {
    ok: true,
    matcher: (line: string): TextRange[] => {
      const hay = opts.ignoreCase ? line.toLowerCase() : line
      const out: TextRange[] = []
      let from = 0
      while (out.length < MAX_HITS_PER_LINE) {
        const i = hay.indexOf(needle, from)
        if (i === -1) break
        out.push({ start: i, end: i + needle.length })
        from = i + needle.length
      }
      return out
    },
  }
}

/** 扫描指定下标区间 [from, to) 的行，返回命中行（供分块增量搜索） */
export function scanRange(lines: ParsedLine[], matcher: Matcher, from: number, to: number): LineHits[] {
  const out: LineHits[] = []
  const start = Math.max(0, from)
  const end = Math.min(to, lines.length)
  for (let i = start; i < end; i++) {
    const ranges = matcher(lines[i].raw)
    if (ranges.length) out.push({ lineIndex: lines[i].index, ranges })
  }
  return out
}

/** 扫描全部已加载行 */
export function scanAll(lines: ParsedLine[], matcher: Matcher): LineHits[] {
  return scanRange(lines, matcher, 0, lines.length)
}

/** 汇总为搜索结果（含命中总数与行号集合） */
export function toSearchResult(hits: LineHits[]): SearchResult {
  let total = 0
  const hitLines = new Set<number>()
  for (const h of hits) {
    total += h.ranges.length
    hitLines.add(h.lineIndex)
  }
  return { hits, total, hitLines }
}

/** 首个命中区间落在目标行内的下标（精确匹配行号） */
export function indexOfHitLine(hits: LineHits[], lineIndex: number): number {
  let lo = 0
  let hi = hits.length - 1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    const v = hits[mid].lineIndex
    if (v === lineIndex) {
      // 命中数组按行号升序，回退到首个同行的位置
      let first = mid
      while (first > 0 && hits[first - 1].lineIndex === lineIndex) first--
      return first
    }
    if (v < lineIndex) lo = mid + 1
    else hi = mid - 1
  }
  return -1
}

/** 行号严格大于 lineIndex 的第一个命中下标；无则 -1 */
export function findHitAfter(hits: LineHits[], lineIndex: number): number {
  let lo = 0
  let hi = hits.length - 1
  let ans = -1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    if (hits[mid].lineIndex > lineIndex) {
      ans = mid
      hi = mid - 1
    } else {
      lo = mid + 1
    }
  }
  return ans
}

/** 行号严格小于 lineIndex 的最后一个命中下标；无则 -1 */
export function findHitBefore(hits: LineHits[], lineIndex: number): number {
  let lo = 0
  let hi = hits.length - 1
  let ans = -1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    if (hits[mid].lineIndex < lineIndex) {
      ans = mid
      lo = mid + 1
    } else {
      hi = mid - 1
    }
  }
  return ans
}

/** 二分查找第 n 个命中（n 从 0 开始，即 hits 下标）所在的行号，越界时返回首尾 */
export function clampHitIndex(hits: LineHits[], n: number): number {
  if (!hits.length) return -1
  if (n < 0) return 0
  if (n >= hits.length) return hits.length - 1
  return n
}

/**
 * 行号数组中：行号 > lineIndex 的第一个下标；无则 -1。
 *
 * 全文件搜索的导航基于「行号数组」而非 LineHits[]（未加载的行没有命中区间），
 * 因此二分查找需要一份针对 number[] 的实现。
 */
export function firstIndexAfter(lines: number[], lineIndex: number): number {
  let lo = 0
  let hi = lines.length - 1
  let ans = -1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    if (lines[mid] > lineIndex) {
      ans = mid
      hi = mid - 1
    } else {
      lo = mid + 1
    }
  }
  return ans
}

/** 行号数组中：行号 < lineIndex 的最后一个下标；无则 -1 */
export function lastIndexBefore(lines: number[], lineIndex: number): number {
  let lo = 0
  let hi = lines.length - 1
  let ans = -1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    if (lines[mid] < lineIndex) {
      ans = mid
      lo = mid + 1
    } else {
      hi = mid - 1
    }
  }
  return ans
}

/**
 * 行号数组中：与 lineIndex 完全相等的下标；无则 -1。
 *
 * 用途：导航不能用"上次记住的下标"直接 +1 —— 补载新命中后 navLines 会增长，
 * 同一命中对应的下标可能已经位移，直接 +1 会跳到另一条命中上。
 * 因此每次导航前先用行号把下标校准回去。
 */
export function indexOfSorted(lines: number[], lineIndex: number): number {
  let lo = 0
  let hi = lines.length - 1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    if (lines[mid] === lineIndex) return mid
    if (lines[mid] < lineIndex) lo = mid + 1
    else hi = mid - 1
  }
  return -1
}

/**
 * 在**互不重叠且按 start 升序**的段列表中，二分定位行号所属段；未落在任何段内返回 -1。
 *
 * 跳转前必须先做这一步：否则为了找一个"根本不在筛选结果里"的行号，
 * 会把上千个段全部读一遍才肯罢休。
 */
export function segmentIndexOfLine(segs: { start: number; end: number }[], lineIndex: number): number {
  let lo = 0
  let hi = segs.length - 1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    const s = segs[mid]
    if (lineIndex < s.start) hi = mid - 1
    else if (lineIndex > s.end) lo = mid + 1
    else return mid
  }
  return -1
}

/** 重新编译时复用：判断搜索条件是否变化（避免无意义重扫） */
export function sameOptions(a: SearchOptions, b: SearchOptions): boolean {
  return a.keyword === b.keyword && a.mode === b.mode && a.ignoreCase === b.ignoreCase
}
