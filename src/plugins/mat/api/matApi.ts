/**
 * MAT 插件后端 API 封装
 *
 * 前端与 src-tauri/src/command/mat_hprof.rs 之间的唯一接口。
 * 字段与后端结构体一一对应（snake_case）。
 */
import { call } from '@/api/rpc'
import type { MatDump } from '../types'

interface ApiResp<T> {
  code: number
  msg: string
  data: T
}

// ==================== 响应数据结构 ====================

/** 堆转储头部元信息（后端 MatHeaderInfo） */
export interface MatHeaderInfo {
  format: string
  id_size: number
  timestamp_ms: number
  timestamp_text: string
  file_size: number
  supported: boolean
  detail: string
}

/** 直方图行（后端 MatHistogramRow） */
export interface MatHistogramRow {
  class_name: string
  count: number
  shallow_bytes: number
}

/** 直方图结果（后端 MatHistogramResult） */
export interface MatHistogramResult {
  total_classes: number
  total_objects: number
  total_shallow_bytes: number
  truncated: boolean
  rows: MatHistogramRow[]
  elapsed_ms: number
}

/** 泄漏检测堆统计汇总（后端 MatLeakSummary） */
export interface MatLeakSummary {
  /** 图中全部节点（实例 + 各类数组 + 类对象） */
  total_nodes: number
  /** 全部节点浅堆字节 */
  total_shallow: number
  /** GC Roots 去重后的节点数 */
  root_nodes: number
  /** 引用边总数（含自环去重后的有效边） */
  edge_count: number
  /** 指向图外对象的引用数（畸形数据，已忽略） */
  skipped_edge_count: number
  /** 从 GC Roots 可达的对象数 */
  reachable_nodes: number
  /** 可达对象浅堆字节 */
  reachable_shallow: number
  /** 不可达对象数（垃圾，不构成泄漏） */
  unreachable_nodes: number
  /** 不可达对象浅堆字节 */
  unreachable_shallow: number
  /** 是否因规模超限跳过了支配树分析（suspects 为空） */
  truncated: boolean
  elapsed_ms: number
}

/** 引用路径上的一个节点（后端 MatLeakPathNode） */
export interface MatLeakPathNode {
  class_name: string
  /** 该节点是否是 GC Root 对象 */
  is_root: boolean
}

/** 单个泄漏嫌疑（后端 MatLeakSuspect） */
export interface MatLeakSuspect {
  rank: number
  /** 对象 id（0x 十六进制展示） */
  obj_id: string
  class_name: string
  /** 是否为类对象（泄漏源在静态字段上） */
  is_class: boolean
  /** 所在 GC Root 分类标签（thread / jni-global / system-class / …） */
  root_kind: string
  shallow_bytes: number
  /** 深堆：仅由此嫌疑支配的对象浅堆之和（MAT Retained Heap 口径） */
  retained_bytes: number
  /** 支配子树对象数（含自身） */
  retained_objects: number
  /** retained / 可达堆浅堆 的百分比 */
  pct: number
  /** GC Root → … → 嫌疑对象的支配链（含两端） */
  path: MatLeakPathNode[]
}

/** 泄漏检测结果（后端 MatLeakSuspectsResult） */
export interface MatLeakSuspectsResult {
  summary: MatLeakSummary
  suspects: MatLeakSuspect[]
  notes: string[]
}

// ==================== 调用封装 ====================

/** 读取堆转储头部（格式识别 + dump 元信息）；文件不可用 / 无法读取时抛错 */
export async function fetchHeader(path: string): Promise<MatHeaderInfo> {
  const res = await call<ApiResp<MatHeaderInfo>>('mat_hprof_header', { path })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '读取堆转储头部失败')
}

/**
 * 生成浅堆直方图（按浅堆字节降序）。
 * 不传 limit = 后端返回全部类行（配合虚拟滚动表格）；传 limit 仅取前 N 行（预留）。
 */
export async function fetchHistogram(path: string, limit?: number): Promise<MatHistogramResult> {
  const payload: Record<string, unknown> = { path }
  if (limit != null) payload.limit = limit
  const res = await call<ApiResp<MatHistogramResult>>('mat_hprof_histogram', payload)
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '生成直方图失败')
}

/**
 * 泄漏检测：对象引用图 → 支配树 → Retained Heap → Top 嫌疑对象。
 * 大堆文件（可达对象 / 引用边超限）时后端返回 summary + 空 suspects + truncated 标记。
 */
export async function fetchLeakSuspects(path: string): Promise<MatLeakSuspectsResult> {
  const res = await call<ApiResp<MatLeakSuspectsResult>>('mat_leak_suspects', { path })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '泄漏检测失败')
}

/** 由后端响应构造一个 MatDump（header 失败时 error 字段携带原因） */
export function toDump(path: string, info: MatHeaderInfo): MatDump {
  return {
    id: 'mat_' + path,
    path,
    name: path.split(/[\\/]/).pop() || path,
    size: info.file_size,
    format: info.format,
    idSize: info.id_size,
    timestampText: info.timestamp_text,
    supported: info.supported,
    detail: info.detail,
    loading: false,
  }
}

/** 字节数 → 人类可读（B/KB/MB/GB） */
export function formatBytes(n: number): string {
  if (!n || n <= 0) return '0 B'
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`
}

/** 千分位格式化整数 */
export function formatInt(n: number): string {
  return n.toLocaleString('en-US')
}

// ==================== 对象引用追踪（mat_refs_*） ====================

/** 引用追踪会话构建汇总（后端 RefBuildInfo） */
export interface RefBuildInfo {
  total_nodes: number
  total_shallow: number
  root_nodes: number
  edge_count: number
  skipped_edge_count: number
  field_mismatch: number
  classes: number
  reachable_nodes: number
  reachable_shallow: number
  unreachable_nodes: number
  unreachable_shallow: number
  /** 是否因规模超限无法提供引用查询 */
  truncated: boolean
  /** 是否命中了已有会话缓存 */
  cached: boolean
  elapsed_ms: number
  notes: string[]
}

/** 类名搜索命中行（后端 RefClassHit） */
export interface RefClassHit {
  class_name: string
  count: number
  shallow_bytes: number
}

export interface RefSearchResult {
  total: number
  hits: RefClassHit[]
  elapsed_ms: number
}

/** 实例样本行 */
export interface RefInstanceRow {
  obj_id: string
  shallow_bytes: number
}

export interface RefInstances {
  class_name: string
  total: number
  rows: RefInstanceRow[]
  elapsed_ms: number
}

/** outgoing 明细项 / group（后端 RefOutItem） */
export interface RefOutItem {
  kind: 'field' | 'elem' | 'group'
  /** 字段名 / [下标] / “× N 个 类名” */
  label: string
  target_class: string
  target_obj_id: string
  target_shallow: number
  target_kind: 'instance' | 'object-array' | 'primitive-array' | 'class'
  count: number
}

/** incoming 聚合组（后端 RefIncomingGroup） */
export interface RefIncomingGroup {
  class_name: string
  count: number
  shallow_bytes: number
}

/** incoming 样例（后端 RefIncomingSample） */
export interface RefIncomingSample {
  referrer_obj_id: string
  referrer_class: string
  via: string
  referrer_shallow: number
}

/** Path to GC Roots 节点（后端 RefPathItem） */
export interface RefPathItem {
  class_name: string
  obj_id: string
  is_root: boolean
  /** 本节点如何被链上前一节点引用（root 首项为空） */
  via: string
}

/** 单对象详情（后端 RefObjectDetail） */
export interface RefObjectDetail {
  obj_id: string
  class_name: string
  kind: 'instance' | 'object-array' | 'primitive-array' | 'class'
  shallow_bytes: number
  reachable: boolean
  is_root: boolean
  root_kind: string
  outgoing_total: number
  outgoing_truncated: boolean
  outgoing: RefOutItem[]
  incoming_total: number
  incoming_truncated: boolean
  incoming_by_class: RefIncomingGroup[]
  incoming_samples: RefIncomingSample[]
  /** ok / unreachable / truncated / is-root */
  path_state: string
  path_note: string
  path: RefPathItem[]
  elapsed_ms: number
}

// ==================== 引用追踪调用封装 ====================

/** 构建（或复用）对象图会话；大 dump 首次可能耗时数十秒（后端不缓存时） */
export async function fetchRefsBuild(path: string): Promise<RefBuildInfo> {
  const res = await call<ApiResp<RefBuildInfo>>('mat_refs_build', { path })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '构建引用索引失败')
}

/** 释放某 dump 的引用追踪会话缓存（关闭 dump 时调用，非关键路径） */
export async function releaseRefsSession(path: string): Promise<void> {
  try {
    await call<ApiResp<boolean>>('mat_refs_release', { path })
  } catch {
    /* 释放失败不影响关闭流程 */
  }
}

/** 类名搜索（子串，大小写不敏感，按浅堆合计降序） */
export async function fetchRefSearchClasses(path: string, query: string): Promise<RefSearchResult> {
  const res = await call<ApiResp<RefSearchResult>>('mat_refs_search_classes', { path, query, limit: 40 })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '类名搜索失败')
}

/**
 * 某类的实例样本（浅堆降序 Top 300）。
 * 注意：Tauri 命令形参默认按 camelCase 键暴露（class_name → className），
 * 传 snake_case 键会报 missing required key className。
 */
export async function fetchRefInstances(path: string, className: string): Promise<RefInstances> {
  const res = await call<ApiResp<RefInstances>>('mat_refs_instances', { path, className, limit: 300 })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '获取实例列表失败')
}

/** 单对象详情（outgoing / incoming / Path to GC Roots）。形参 obj_id → 键 objId */
export async function fetchRefInspect(path: string, objId: string): Promise<RefObjectDetail> {
  const res = await call<ApiResp<RefObjectDetail>>('mat_refs_inspect', { path, objId })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '对象详情查询失败')
}

// ==================== 分析预算与规模上限（mat_analysis_limits + plugins/mat.json） ====================

/** 后端 mat_analysis_limits 返回：本机内存 / 预算 / 各环节上限换算明细 */
export interface MatLimitInfo {
  /** 本机物理内存（GB） */
  phys_gb: number
  /** 自动预算（GB）：(物理 − 2GB) × 55%，夹 [2, 48] */
  auto_budget_gb: number
  /** 用户覆盖值（GB，plugins/mat.json 的 analysisBudgetGb；未设置 = null） */
  override_gb: number | null
  /** 生效预算（GB）= 覆盖值或自动值 */
  effective_budget_gb: number
  /** 引用追踪节点上限（= 泄漏检测建图闸） */
  refs_nodes: number
  /** 引用追踪引用边上限 */
  refs_edges: number
  /** 泄漏支配树可达对象上限 */
  dom_nodes: number
  /** 泄漏支配树可达引用边上限 */
  dom_edges: number
}

/** 查询当前分析预算与换算出的上限（不依赖任何 dump，供「分析预算」设置弹窗） */
export async function fetchAnalysisLimits(): Promise<MatLimitInfo> {
  const res = await call<ApiResp<MatLimitInfo>>('mat_analysis_limits', {})
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '查询分析预算失败')
}
