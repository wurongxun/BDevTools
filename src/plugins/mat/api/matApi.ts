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
  /** 是否命中了已有会话缓存（本进程内） */
  cached: boolean
  /** 是否由本地索引还原（上次运行落盘的 `.bdtidx`） */
  index_hit: boolean
  /**
   * 浅堆口径说明（后端按 dump 的引用宽度 + 配置推导，如
   * 「引用 8B / 压缩指针 / 对齐 8B ⇒ 实例头 16B、数组头 16B、类对象 112B」）。
   * 规模超限（未建图）时为空串。
   */
  shallow_caliber: string
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
  /** 本次返回的起始下标 */
  offset: number
  /** 单页上限 */
  limit: number
  /** shallow（浅堆降序）/ id（对象 id 升序） */
  sort: string
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
  /** 引用强度：strong / weak / soft / phantom / finalizer */
  ref_kind: string
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
  /** 引用强度：strong / weak / soft / phantom / finalizer */
  ref_kind: string
  referrer_shallow: number
}

/** Path to GC Roots 节点（后端 RefPathItem） */
export interface RefPathItem {
  class_name: string
  obj_id: string
  is_root: boolean
  /** 本节点如何被链上前一节点引用（root 首项为空） */
  via: string
  /** 这一跳的引用强度 */
  ref_kind: string
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
export async function fetchRefInstances(
  path: string,
  className: string,
  offset = 0,
  limit = 100,
  sort: 'shallow' | 'id' = 'shallow',
): Promise<RefInstances> {
  const res = await call<ApiResp<RefInstances>>('mat_refs_instances', {
    path,
    className,
    offset,
    limit,
    sort,
  })
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

// ==================== 支配树（mat_dom_*） ====================

/** Dominator Tree 的一行：对象级或分组聚合级 */
export interface DomRow {
  /** 对象 id（0x 十六进制）；分组行为空 */
  obj_id: string
  class_name: string
  /** instance / object-array / primitive-array / class / group */
  kind: string
  shallow_bytes: number
  retained_bytes: number
  retained_objects: number
  /** retained / 可达堆浅堆 的百分比 */
  pct: number
  /** 支配树直接孩子数（分组行恒 0） */
  children: number
  is_root: boolean
  /** 支配者对象 id（超根直接孩子为空串） */
  parent_obj_id: string
}

export interface DomTreeResult {
  /** 分页前总行数 */
  total: number
  rows: DomRow[]
  reach_nodes: number
  reach_shallow: number
  /** 因规模闸 / 无 GC Roots 未提供支配树 */
  truncated: boolean
  note: string
  cached: boolean
  elapsed_ms: number
}

export interface RetainedSetResult {
  obj_id: string
  total: number
  bytes: number
  rows: DomRow[]
  truncated: boolean
  note: string
  elapsed_ms: number
}

export interface DomTreeQuery {
  /** object（默认）/ class / package */
  groupBy?: string
  /** 给定对象 id 时只列其支配树直接孩子 */
  parentObjId?: string
  query?: string
  offset?: number
  limit?: number
}

/**
 * 支配树查询。注意 Tauri 命令形参按 camelCase 暴露（group_by → groupBy）。
 * 首次调用会触发一次支配树计算（与泄漏检测共用会话缓存），大 dump 可能耗时数十秒。
 */
export async function fetchDomTree(path: string, q: DomTreeQuery = {}): Promise<DomTreeResult> {
  const res = await call<ApiResp<DomTreeResult>>('mat_dom_tree', {
    path,
    groupBy: q.groupBy || 'object',
    parentObjId: q.parentObjId || '',
    query: q.query || '',
    offset: q.offset ?? 0,
    limit: q.limit ?? 200,
  })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '查询支配树失败')
}

/** Show Retained Set：某对象支配子树中的对象（按 retained 降序） */
export async function fetchRetainedSet(path: string, objId: string, limit = 200): Promise<RetainedSetResult> {
  const res = await call<ApiResp<RetainedSetResult>>('mat_dom_retained_set', { path, objId, limit })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '查询保留集失败')
}

// ==================== 对象值（mat_object_values） ====================

export interface ValueField {
  name: string
  /** object / boolean / char / float / double / byte / short / int / long */
  type_name: string
  value: string
  /** 引用字段的目标对象 id */
  target_obj_id: string
}

export interface ObjectValues {
  obj_id: string
  class_name: string
  /** instance / primitive-array / object-array / class */
  kind: string
  shallow_bytes: number
  fields: ValueField[]
  fields_total: number
  array_preview: string
  array_total: number
  /** String 实例解析出的文本 */
  string_value: string | null
  loader_obj_id: string
  note: string
  elapsed_ms: number
}

/**
 * 读取单个对象的字段值 / 数组内容 / String 值。
 * 后端按需扫描 HPROF（不常驻内存），首次点击对象详情可能耗时数百毫秒～数秒。
 */
export async function fetchObjectValues(path: string, objId: string, limit = 200): Promise<ObjectValues> {
  const res = await call<ApiResp<ObjectValues>>('mat_object_values', { path, objId, limit })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '读取对象值失败')
}

// ==================== 多条 GC Roots 路径（mat_gc_root_paths） ====================

export interface GcRootPath {
  root_kind: string
  /** 边数（items.length - 1） */
  length: number
  items: RefPathItem[]
}

export interface GcRootPathsResult {
  obj_id: string
  /** ok / is-root / unreachable */
  state: string
  note: string
  total: number
  offset: number
  limit: number
  exclude_weak: boolean
  weak_skipped: number
  truncated: boolean
  paths: GcRootPath[]
  elapsed_ms: number
}

/** 多条最短 Path to GC Roots（可排除弱/软/虚/终结器引用） */
export async function fetchGcRootPaths(
  path: string,
  objId: string,
  excludeWeak = true,
  offset = 0,
  limit = 5,
): Promise<GcRootPathsResult> {
  const res = await call<ApiResp<GcRootPathsResult>>('mat_gc_root_paths', {
    path,
    objId,
    excludeWeak,
    offset,
    limit,
  })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '枚举 GC Roots 路径失败')
}

// ==================== OQL（mat_oql_*） ====================

export interface OqlCell {
  name: string
  value: string
}

export interface OqlRow {
  obj_id: string
  class_name: string
  kind: string
  shallow_bytes: number
  cells: OqlCell[]
}

export interface OqlResult {
  query: string
  columns: string[]
  rows: OqlRow[]
  scanned: number
  matched: number
  limit: number
  truncated: boolean
  note: string
  elapsed_ms: number
}

export interface OqlCheck {
  columns: string[]
  class_pattern: string
  has_filter: boolean
  limit: number
}

/** 执行 OQL 安全子集查询（禁方法调用 / 反射） */
export async function fetchOqlQuery(path: string, query: string): Promise<OqlResult> {
  const res = await call<ApiResp<OqlResult>>('mat_oql_query', { path, query })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || 'OQL 查询失败')
}

/** OQL 语法静态校验（不读 dump，用于输入框即时提示） */
export async function fetchOqlCheck(query: string): Promise<OqlCheck> {
  const res = await call<ApiResp<OqlCheck>>('mat_oql_check', { query })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || 'OQL 语法错误')
}

// ==================== Top Consumers（mat_top_consumers） ====================

export interface ConsumerRow {
  name: string
  /** class / package */
  kind: string
  objects: number
  shallow_bytes: number
  retained_bytes: number
  pct: number
  top_class: string
}

export interface TopConsumersResult {
  group_by: string
  total: number
  rows: ConsumerRow[]
  reach_nodes: number
  reach_shallow: number
  cached: boolean
  note: string
  elapsed_ms: number
}

export async function fetchTopConsumers(
  path: string,
  groupBy: 'class' | 'package' = 'class',
  limit = 50,
): Promise<TopConsumersResult> {
  const res = await call<ApiResp<TopConsumersResult>>('mat_top_consumers', { path, groupBy, limit })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || 'Top Consumers 查询失败')
}

// ==================== 专用报告（mat_threads / classloaders / waste / unreachable） ====================

export interface ThreadRow {
  obj_id: string
  class_name: string
  name: string
  name_obj_id: string
  shallow_bytes: number
  thread_status: number
  is_root: boolean
}

export interface ThreadsResult {
  total: number
  thread_roots: number
  rows: ThreadRow[]
  note: string
  elapsed_ms: number
}

export async function fetchThreads(path: string, limit = 500): Promise<ThreadsResult> {
  const res = await call<ApiResp<ThreadsResult>>('mat_threads', { path, limit })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '线程报告失败')
}

export interface LoaderRow {
  loader_obj_id: string
  loader_class: string
  classes: number
  duplicate_classes: number
  sample: string[]
}

export interface DuplicateClassRow {
  simple_name: string
  occurrences: string[]
  loaders: number
}

export interface ClassLoadersResult {
  total_loaders: number
  total_classes: number
  rows: LoaderRow[]
  duplicates: DuplicateClassRow[]
  note: string
  elapsed_ms: number
}

export async function fetchClassLoaders(path: string, limit = 500): Promise<ClassLoadersResult> {
  const res = await call<ApiResp<ClassLoadersResult>>('mat_classloaders', { path, limit })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '类加载器报告失败')
}

export interface DupStringRow {
  value: string
  count: number
  shallow_bytes: number
  wasted_bytes: number
}

export interface CollectionFillRow {
  class_name: string
  instances: number
  empty_instances: number
  avg_fill: number
  max_size: number
  shallow_bytes: number
}

export interface WasteResult {
  duplicate_strings: DupStringRow[]
  duplicate_wasted_bytes: number
  duplicate_groups: number
  collections: CollectionFillRow[]
  empty_collections: number
  sampled: boolean
  strings_scanned: number
  note: string
  elapsed_ms: number
}

export async function fetchWasteReport(path: string, limit = 200): Promise<WasteResult> {
  const res = await call<ApiResp<WasteResult>>('mat_waste_report', { path, limit })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '内存浪费报告失败')
}

export interface UnreachableRow {
  class_name: string
  count: number
  shallow_bytes: number
}

export interface UnreachableResult {
  total_objects: number
  total_shallow: number
  reachable_nodes: number
  rows: UnreachableRow[]
  note: string
  elapsed_ms: number
}

export async function fetchUnreachable(path: string, limit = 200): Promise<UnreachableResult> {
  const res = await call<ApiResp<UnreachableResult>>('mat_unreachable_histogram', { path, limit })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '不可达对象查询失败')
}

// ==================== 多 dump 对比（mat_compare_histograms） ====================

export interface CompareRow {
  /** 分组名（类名 / 包名） */
  name: string
  /** class / package */
  kind: string
  count_a: number
  count_b: number
  /** B − A（可为负） */
  delta_count: number
  shallow_a: number
  shallow_b: number
  /** B − A（可为负，字节） */
  delta_shallow: number
  /** added / removed / changed / same */
  status: string
}

export interface CompareResult {
  group_by: string
  /** 过滤后、分页前的分组总数 */
  total: number
  rows: CompareRow[]
  only_changed: boolean
  sort: string
  objects_a: number
  objects_b: number
  shallow_a: number
  shallow_b: number
  delta_objects: number
  delta_shallow: number
  added: number
  removed: number
  changed: number
  truncated: boolean
  note: string
  elapsed_ms: number
}

export type CompareSort = 'delta_shallow' | 'delta_count' | 'shallow_b' | 'name'

export interface CompareOptions {
  groupBy?: 'class' | 'package'
  query?: string
  onlyChanged?: boolean
  sort?: CompareSort
  offset?: number
  limit?: number
}

/**
 * 多 dump 直方图对比（按类 / 包的 Δcount / Δshallow）。
 * Δretained 不在本命令范围（需要两份支配树），见返回的 note。
 */
export async function fetchCompareHistograms(
  pathA: string,
  pathB: string,
  opts: CompareOptions = {},
): Promise<CompareResult> {
  const res = await call<ApiResp<CompareResult>>('mat_compare_histograms', {
    pathA,
    pathB,
    groupBy: opts.groupBy || 'class',
    query: opts.query || '',
    onlyChanged: opts.onlyChanged ?? false,
    sort: opts.sort || 'delta_shallow',
    offset: opts.offset ?? 0,
    limit: opts.limit ?? 0,
  })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '堆转储对比失败')
}

// ==================== 压缩包（.hprof.gz）与解压缓存 ====================

export interface MatSourceInfo {
  /** 实际用于解析的 HPROF 路径（未压缩时 == 源路径） */
  hprof_path: string
  /** 是否是 gzip 压缩包 */
  compressed: boolean
  /** 本次是否命中已解压缓存 */
  cache_hit: boolean
  source_size: number
  hprof_size: number
  /** 后端给出的说明（已解压 / 缓存命中） */
  note: string
}

export interface MatCacheInfo {
  dir: string
  files: number
  total_bytes: number
  /** 残留的半成品（异常退出留下，可安全清理） */
  partial_files: number
}

export interface MatCacheClearResult {
  removed: number
  removed_bytes: number
  /** 删不掉的文件（仍被 mmap 占用） */
  skipped: string[]
}

/**
 * 解析来源：`.gz` 会在此完成解压（首次几十秒取决于体积，之后命中缓存秒回）。
 * 打开压缩包时前端先调这个，好在界面上说清「源 1.1MB → 解压 13.6MB」。
 */
export async function fetchSourceInfo(path: string): Promise<MatSourceInfo> {
  const res = await call<ApiResp<MatSourceInfo>>('mat_source_info', { path })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '解析压缩包失败')
}

/** 解压缓存占用（概览页展示） */
export async function fetchCacheInfo(): Promise<MatCacheInfo> {
  const res = await call<ApiResp<MatCacheInfo>>('mat_cache_info')
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '读取缓存信息失败')
}

/** 清理解压缓存；返回被跳过的文件（重启应用后可清掉） */
export async function clearCache(): Promise<MatCacheClearResult> {
  const res = await call<ApiResp<MatCacheClearResult>>('mat_cache_clear')
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '清理缓存失败')
}

/** 路径是否是 gzip 压缩的堆转储（仅按扩展名判断，真正的识别在后端做魔数校验） */
export function isCompressedDump(path: string): boolean {
  const p = path.toLowerCase()
  return p.endsWith('.gz') || p.endsWith('.gzip')
}

// ==================== 本地索引（.bdtidx / .bdtidx.dom） ====================

export interface MatIndexInfo {
  /** 索引是否启用（plugins/mat.json 的 indexEnabled） */
  enabled: boolean
  /** 图索引（对象图）是否可用：存在且源文件指纹匹配 */
  graph_valid: boolean
  graph_path: string
  graph_bytes: number
  /** 支配树索引是否可用（首次做泄漏检测 / 支配树分析后生成） */
  dom_valid: boolean
  dom_path: string
  dom_bytes: number
  /** 源 dump 超过该体积只读不写 */
  write_max_gb: number
  note: string
}

export interface MatIndexClearResult {
  removed: number
  removed_bytes: number
  /** 删不掉的文件（仍被 mmap 占用） */
  skipped: string[]
}

/** 索引状态：概览页展示「二次打开是否秒开」的依据 */
export async function fetchIndexInfo(path: string): Promise<MatIndexInfo> {
  const res = await call<ApiResp<MatIndexInfo>>('mat_index_info', { path })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '读取索引状态失败')
}

/** 删除索引；下次打开该 dump 会重新解析并重建 */
export async function clearIndex(path: string): Promise<MatIndexClearResult> {
  const res = await call<ApiResp<MatIndexClearResult>>('mat_index_clear', { path })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '清理索引失败')
}

// ==================== sidecar 后台分析进程 ====================

/**
 * 作业快照（后端 MatWorkerJob）。
 *
 * 分析在**独立子进程**里跑：主窗口全程可操作，进度经 `mat-worker` 事件推送。
 * 子进程产出的是 `.bdtidx` 索引 —— 之后任何查询都命中索引，毫秒级返回。
 */
export interface MatWorkerJob {
  job_id: string
  path: string
  /** 是否连支配树一起算 */
  dom: boolean
  /** starting / graph-meta / graph-edges / dom / dom-lt / dom-retained / done / error / canceled */
  phase: string
  done: number
  /** null = 未知总量（HPROF 不记录记录数，只能给不确定进度条） */
  total: number | null
  msg: string
  finished: boolean
  ok: boolean
  canceled: boolean
  error: string | null
  nodes: number | null
  edges: number | null
  /** ready / unavailable / skipped */
  dom_state: string | null
  pid: number | null
  elapsed_ms: number
}

/** `mat-worker` 事件载荷（子进程 NDJSON 一行 + job_id / path / dom） */
export interface MatWorkerEvent {
  /** hello / stage / done / error / exit / log */
  ev: string
  job_id: string
  path: string
  /** 本作业是否连支配树一起算（后端键名 `with_dom`，避免与 `dom_state` 撞名） */
  with_dom?: boolean
  /** 支配树结果：ready / unavailable / skipped */
  dom_state?: string
  stage?: string
  done?: number
  total?: number | null
  msg?: string
  message?: string
  code?: number | null
  canceled?: boolean
  ok?: boolean
  error?: string | null
  nodes?: number
  edges?: number
  ms?: number
}

/** 启动后台分析进程（同一路径已在跑时后端直接返回现有作业） */
export async function startWorker(path: string, dom = true): Promise<MatWorkerJob> {
  const res = await call<ApiResp<MatWorkerJob>>('mat_worker_start', { path, dom })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '启动后台分析失败')
}

/** 取消后台分析（杀进程；已结束返回 false） */
export async function cancelWorker(jobId: string): Promise<boolean> {
  const res = await call<ApiResp<boolean>>('mat_worker_cancel', { jobId })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '取消失败')
}

/** 查询作业状态（事件丢失 / 页面重挂载时用） */
export async function fetchWorkerStatus(jobId: string): Promise<MatWorkerJob> {
  const res = await call<ApiResp<MatWorkerJob>>('mat_worker_status', { jobId })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '查询作业状态失败')
}

/** 某 dump 最近一次作业（页面重新挂载恢复进度条） */
export async function fetchWorkerStatusForPath(path: string): Promise<MatWorkerJob | null> {
  const res = await call<ApiResp<MatWorkerJob | null>>('mat_worker_status_for_path', { path })
  if (res && res.code === 200) return res.data
  throw new Error(res?.msg || '查询作业状态失败')
}
