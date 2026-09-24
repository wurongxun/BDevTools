/**
 * 后端日志索引 API 封装（logs 插件专用）
 *
 * 级别 / 时间筛选以服务端全量扫描结果为准：
 * 前端分块懒加载时只持有部分行，若在本地过滤会导致"结果随滚动变化"。
 * 本模块是前端与 src-tauri/src/command/log_index.rs 之间的唯一接口。
 */
import { call } from '@/api/rpc'
import type { LogLevel } from '../types'
import type { FilterState } from './filter'

interface ApiResp<T> {
  code: number
  msg: string
  data: T
}

// ==================== 响应数据结构（与后端结构体字段一一对应） ====================

/** 时间桶（时间轴视图直接消费） */
export interface TimeBucket {
  start: number
  end: number
  total: number
  warn: number
  /** ERROR 及以上 */
  error: number
  /** 该桶内第一条带时间戳日志的行号（1-based），空桶为 null */
  first_line: number | null
}

/** 索引元信息（文件级统计） */
export interface LogIndexMeta {
  total_lines: number
  /** 实际参与索引的行数（超长文件会被截断） */
  indexed_lines: number
  truncated: boolean
  /** 各级别行数，下标与级别码一致：[UNKNOWN, TRACE, DEBUG, INFO, WARN, ERROR, FATAL] */
  levels: number[]
  ts_min: number | null
  ts_max: number | null
  /** 自带时间戳的行数（其余行的时间戳是继承来的） */
  ts_count: number
  buckets: TimeBucket[]
  elapsed_ms: number
  cached: boolean
  /**
   * L1 块级聚合是否已就绪。
   *
   * `false` 表示阶段二尚未完成：此时只能定位与滚动（L2 已就绪），**不能**做级别/时间筛选。
   * 当前 `logs_index_build` 会等完整索引，故恒为 `true`；该字段为后续"边建边用"预留。
   */
  blocks_ready: boolean
}

/** 命中行段（1-based 闭区间） */
export interface LineSegment {
  start: number
  end: number
}

/**
 * 合并查询结果（§2.5）：**筛选 + 关键词一次查询产出的两个视图**。
 *
 * - `segments`：渲染用（前端按段取内容）；
 * - `lines`：导航用（命中行号全集）。
 *
 * 二者同出一源，因此天然满足核心不变量 `lines ⊆ ∪segments`（§1.1）——
 * 旧实现里筛选与搜索是两次独立请求，行号可能落在筛选结果之外，跳转必然断裂。
 */
export interface LogQueryResult {
  segments: LineSegment[]
  /** 命中行号（1-based，升序）。无关键词时为空（此时导航走段） */
  lines: number[]
  /** 最终命中行数真值（不受任何截断影响） */
  matched: number
  /** 仅筛选条件的命中行数（不含关键词） */
  filter_matched: number
  /** 段数是否因超过 maxSegments 被截断 */
  truncated: boolean
  /** lines 是否因超过 maxMatches 被截断（matched 仍是真值） */
  matches_truncated: boolean
  /** 实际回读原文的行数（剪枝收益越小说明筛选命中率越高） */
  scanned_lines: number
  /** 被块级剪枝整块跳过的块数 */
  pruned_blocks: number
  /** 因超出索引上限而未参与本次查询的行数（>0 时 UI 必须提示） */
  unindexed_lines: number
  elapsed_ms: number
}

export interface LogExportResult {
  path: string
  lines: number
  bytes: number
  elapsed_ms: number
}

/** 级别码顺序（与后端 LV_* 常量一致） */
export const LEVEL_CODES: LogLevel[] = [
  'UNKNOWN',
  'TRACE',
  'DEBUG',
  'INFO',
  'WARN',
  'ERROR',
  'FATAL',
]

// ==================== 调用封装 ====================

/** 建立（或复用）索引，返回文件级统计 */
export async function buildIndex(path: string, encoding: string): Promise<LogIndexMeta | null> {
  const res = await call<ApiResp<LogIndexMeta>>('logs_index_build', {
    filePath: path,
    encoding,
  })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '建立日志索引失败')
}

/**
 * 合并查询：**筛选 + 关键词一次请求**（§2.5）。
 *
 * 传 `taskId` 后可用 `cancelQuery` 中止——大文件查询是长任务，
 * 输入关键词的过程中旧结果已经没意义了。
 */
export async function queryLines(opts: {
  path: string
  encoding: string
  filter?: FilterState | null
  search?: { keyword: string; mode: 'text' | 'regex'; ignoreCase: boolean } | null
  maxSegments?: number
  maxMatches?: number
  taskId?: string
}): Promise<LogQueryResult> {
  const f = opts.filter
  const s = opts.search
  const res = await call<ApiResp<LogQueryResult>>('logs_query_lines', {
    filePath: opts.path,
    encoding: opts.encoding,
    levels: f?.levels ?? [],
    tsFrom: f?.tsFrom ?? null,
    tsTo: f?.tsTo ?? null,
    invert: f?.invert ?? false,
    mergeGap: 0,
    maxSegments: opts.maxSegments ?? 2000,
    keyword: s?.keyword ?? '',
    mode: s?.mode ?? 'text',
    ignoreCase: s?.ignoreCase ?? false,
    maxMatches: opts.maxMatches ?? 50000,
    taskId: opts.taskId ?? null,
  })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '查询失败')
}

/** 取消正在进行的查询（前端静默丢弃旧结果即可） */
export async function cancelQuery(taskId: string): Promise<void> {
  try {
    await call<ApiResp<boolean>>('logs_cancel', { taskId })
  } catch (e) {
    console.warn('[logs] 取消查询失败:', e)
  }
}

/** 筛选条件导出到文件（流式写入，后端一次扫描完成） */
export async function exportLines(opts: {
  path: string
  encoding: string
  destPath: string
  filter: FilterState
  withLineNumber: boolean
  withBom: boolean
}): Promise<LogExportResult> {
  const res = await call<ApiResp<LogExportResult>>('logs_index_export', {
    filePath: opts.path,
    encoding: opts.encoding,
    destPath: opts.destPath,
    levels: opts.filter.levels,
    tsFrom: opts.filter.tsFrom,
    tsTo: opts.filter.tsTo,
    invert: opts.filter.invert,
    withLineNumber: opts.withLineNumber,
    withBom: opts.withBom,
  })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '导出失败')
}

/** 释放指定文件的索引缓存（关闭日志源时回收内存） */
export async function dropIndex(path: string): Promise<void> {
  try {
    await call<ApiResp<boolean>>('logs_index_drop', { filePath: path })
  } catch (e) {
    console.error('[logs] 释放索引缓存失败:', e)
  }
}

/** 取某个级别的行数（越界返回 0） */
export function levelCount(meta: LogIndexMeta | null, level: LogLevel): number {
  if (!meta || !Array.isArray(meta.levels)) return 0
  const i = LEVEL_CODES.indexOf(level)
  return i < 0 ? 0 : meta.levels[i] ?? 0
}
