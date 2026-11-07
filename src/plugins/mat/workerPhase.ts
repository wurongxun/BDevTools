/**
 * 后台分析作业的「阶段 → 文案 / 进度」映射（唯一真源）。
 *
 * 为什么不是精确进度：HPROF 头部不记录总记录数，解析阶段只能报「已处理 N 条」，
 * 给不出比例。所以阶段内部用**权重**画进度条 —— 目的是让用户知道「在动、没死」，
 * 而不是精确完成度。有 `total` 时（支配树的按可达对象数）才用真比例。
 *
 * 改阶段名要同步 `crates/plugin-mat/src/mat_progress.rs` 的打点与
 * `crates/plugin-mat/src/mat_worker.rs` 的事件，否则这里会落到 `?? 5` 的兜底值。
 */
import type { MatWorkerJob } from './api/matApi'

export const WORKER_PHASE_TEXT: Record<string, string> = {
  starting: '启动中',
  'graph-meta': '解析类元数据',
  'graph-edges': '构建对象图',
  dom: '构建反向边',
  'dom-lt': '计算支配树',
  'dom-retained': '累加 Retained Heap',
  done: '已完成',
  error: '失败',
  canceled: '已取消',
}

const WORKER_PHASE_PCT: Record<string, number> = {
  starting: 3,
  'graph-meta': 25,
  'graph-edges': 55,
  dom: 75,
  'dom-lt': 88,
  'dom-retained': 96,
  done: 100,
}

/** 阶段中文名（未知阶段原样返回） */
export function workerPhaseText(phase?: string | null): string {
  if (!phase) return ''
  return WORKER_PHASE_TEXT[phase] || phase
}

/** 进度百分比（0~100） */
export function workerPct(job: MatWorkerJob | null): number {
  if (!job) return 0
  if (job.finished) return 100
  if (job.total && job.total > 0) return Math.min(99, Math.round((job.done / job.total) * 100))
  return WORKER_PHASE_PCT[job.phase] ?? 5
}
