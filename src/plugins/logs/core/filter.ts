/**
 * 筛选条件（基础筛选）
 *
 * 级别与时间范围的过滤由后端索引完成（logs_index_query），前端只负责：
 * 条件编辑、持久化、以及按返回的命中行段去取内容渲染。
 * 这样即使文件只加载了一部分，筛选结果依然是针对整个文件的。
 */
import type { LogLevel } from '../types'

/** 筛选条件（持久化到 plugins/logs.json） */
export interface FilterState {
  /** 选中的级别；空数组 = 不限级别 */
  levels: LogLevel[]
  /** 起始时间（UTC 毫秒，含） */
  tsFrom: number | null
  /** 结束时间（UTC 毫秒，含） */
  tsTo: number | null
  /** 反向过滤：排除命中条件的行 */
  invert: boolean
}

export const DEFAULT_FILTER: FilterState = {
  levels: [],
  tsFrom: null,
  tsTo: null,
  invert: false,
}

/** 条件是否生效（任一子条件非空即为筛选态） */
export function isFilterActive(f: FilterState): boolean {
  return f.levels.length > 0 || f.tsFrom !== null || f.tsTo !== null || f.invert
}

/** 归一化：补齐缺失字段，避免旧数据反序列化后字段缺失 */
export function normalizeFilter(raw: Partial<FilterState> | null | undefined): FilterState {
  if (!raw) return { ...DEFAULT_FILTER }
  const levels = Array.isArray(raw.levels)
    ? raw.levels.filter((l): l is LogLevel => typeof l === 'string')
    : []
  const tsFrom = typeof raw.tsFrom === 'number' && Number.isFinite(raw.tsFrom) ? raw.tsFrom : null
  const tsTo = typeof raw.tsTo === 'number' && Number.isFinite(raw.tsTo) ? raw.tsTo : null
  return { levels, tsFrom, tsTo, invert: !!raw.invert }
}

/** 两个筛选条件是否相同（避免无变化时重复请求后端） */
export function sameFilter(a: FilterState, b: FilterState): boolean {
  if (a.invert !== b.invert) return false
  if (a.tsFrom !== b.tsFrom || a.tsTo !== b.tsTo) return false
  if (a.levels.length !== b.levels.length) return false
  const sa = [...a.levels].sort().join(',')
  const sb = [...b.levels].sort().join(',')
  return sa === sb
}

function pad(n: number): string {
  return String(n).padStart(2, '0')
}

/** UTC 毫秒 → `<input type="datetime-local">` 格式（按本地时区展示） */
export function toLocalInput(ms: number | null | undefined): string {
  if (ms === null || ms === undefined || !Number.isFinite(ms)) return ''
  const d = new Date(ms)
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(
    d.getMinutes(),
  )}:${pad(d.getSeconds())}`
}

/** datetime-local 字符串 → UTC 毫秒（按本地时区解释） */
export function fromLocalInput(v: string): number | null {
  if (!v) return null
  const t = Date.parse(v)
  return Number.isNaN(t) ? null : t
}

/** 展示用时间格式：yyyy-MM-dd HH:mm:ss */
export function formatTs(ms: number | null | undefined): string {
  if (ms === null || ms === undefined || !Number.isFinite(ms)) return '--'
  const d = new Date(ms)
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
    d.getHours(),
  )}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

/** 毫秒时长 → 人类可读（用于时间范围跨度提示） */
export function formatSpan(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return '0 秒'
  const s = Math.floor(ms / 1000)
  if (s < 60) return `${s} 秒`
  const m = Math.floor(s / 60)
  if (m < 60) return `${m} 分钟`
  const h = Math.floor(m / 60)
  if (h < 24) return `${h} 小时 ${m % 60} 分`
  const d = Math.floor(h / 24)
  return `${d} 天 ${h % 24} 小时`
}

/** 行数量格式化（12345 → 12,345） */
export function formatCount(n: number): string {
  return n.toLocaleString('zh-CN')
}
