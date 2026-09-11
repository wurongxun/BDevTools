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
}

/** 命中行段（1-based 闭区间） */
export interface LineSegment {
  start: number
  end: number
}

export interface LogQueryResult {
  segments: LineSegment[]
  matched: number
  truncated: boolean
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

/** 按条件筛选，返回命中行段 */
export async function queryLines(
  path: string,
  encoding: string,
  filter: FilterState,
  maxSegments = 2000,
): Promise<LogQueryResult> {
  const res = await call<ApiResp<LogQueryResult>>('logs_index_query', {
    filePath: path,
    encoding,
    levels: filter.levels,
    tsFrom: filter.tsFrom,
    tsTo: filter.tsTo,
    invert: filter.invert,
    mergeGap: 0,
    maxSegments,
  })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '筛选失败')
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
