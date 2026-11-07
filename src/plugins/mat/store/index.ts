/**
 * MAT（Memory Analyzer Tool）会话 store
 *
 * 职责：
 * - 已打开的堆转储文件列表（顶层 tab 数据源）
 * - 激活 dump 与每个 dump 的当前分析子视图
 * - 直方图结果缓存（按 dump 缓存，切子 tab 不重复解析）
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import {
  cancelWorker,
  fetchAnalysisLimits,
  fetchHeader,
  fetchHistogram,
  fetchIndexInfo,
  fetchLeakSuspects,
  fetchRefsBuild,
  fetchSourceInfo,
  fetchWorkerStatus,
  fetchWorkerStatusForPath,
  isCompressedDump,
  releaseRefsSession,
  startWorker as startWorkerApi,
  toDump,
} from '../api/matApi'
import type {
  MatHistogramResult,
  MatLeakSuspectsResult,
  MatLimitInfo,
  MatWorkerEvent,
  MatWorkerJob,
  RefBuildInfo,
} from '../api/matApi'
import { load as loadKv, subscribe } from '@/core/runtime'
import type { Unsubscribe } from '@/core/runtime'
import type { MatDump, MatViewKey } from '../types'

/** 引用追踪跳转预设（直方图 / 泄漏检测联动到 refs 子视图） */
export type RefIntentPreset =
  | { kind: 'class'; query: string }
  | { kind: 'object'; objId: string; className?: string }

export const useMatStore = defineStore('mat', () => {
  /** 已打开的堆转储文件（顶层 tab） */
  const dumps = ref<MatDump[]>([])
  /** 当前激活的 dump id */
  const activeDumpId = ref('')
  /** 每个 dump 当前激活的分析子视图 */
  const activeView = ref<Record<string, MatViewKey>>({})
  /** 直方图结果缓存：dumpId → { loading, error, data } */
  const histCache = ref<Record<string, { loading: boolean; error: string; data: MatHistogramResult | null }>>({})
  /** 泄漏检测结果缓存：dumpId → { loading, error, data } */
  const leakCache = ref<Record<string, { loading: boolean; error: string; data: MatLeakSuspectsResult | null }>>({})
  /** 引用追踪会话缓存：dumpId → { loading, error, data } */
  const refsCache = ref<Record<string, { loading: boolean; error: string; data: RefBuildInfo | null }>>({})

  /** 当前待消费的跳转意图（seq 自增触发 RefView watch） */
  const refsIntent = ref<{ dumpId: string; seq: number; preset: RefIntentPreset | null }>({
    dumpId: '',
    seq: 0,
    preset: null,
  })
  /** 「分析预算」设置弹窗可见性（各视图经 openSettings 打开） */
  const settingsOpen = ref(false)
  /** 预算/上限信息缓存（mat_analysis_limits） */
  const limitInfo = ref<{ loading: boolean; error: string; data: MatLimitInfo | null }>({
    loading: false,
    error: '',
    data: null,
  })

  const activeDump = computed(() => dumps.value.find(d => d.id === activeDumpId.value) || null)

  /**
   * 打开堆转储文件：先识别格式与元信息再入列表。
   * 重复打开同一路径 → 直接激活已有 tab（复用 database editor 去重语义）。
   */
  async function openDump(path: string): Promise<boolean> {
    const exist = dumps.value.find(d => d.path === path)
    if (exist) {
      activeDumpId.value = exist.id
      return true
    }
    const id = 'mat_' + path
    const compressed = isCompressedDump(path)
    const entry: MatDump = {
      id,
      path,
      name: path.split(/[\\/]/).pop() || path,
      compressed,
      size: 0,
      format: '',
      idSize: 0,
      timestampText: '',
      supported: false,
      detail: '',
      loading: true,
    }
    dumps.value.push(entry)
    activeDumpId.value = id
    try {
      // 压缩包先解压：header 之后的所有命令都要读「解压后的那份」，
      // 在这里等一次比让每个视图各自触发解压要好（用户只看到一次 loading）。
      if (compressed) {
        entry.sourceInfo = await fetchSourceInfo(path)
      }
      const info = await fetchHeader(path)
      Object.assign(entry, toDump(path, info))
      entry.error = undefined
      // 大 dump 打开即在后台预热索引：等用户点进「引用追踪 / 泄漏检测」时早已就绪。
      // 小 dump 不预热 —— 主进程几百毫秒就能算完，起个进程反而更慢。
      if (entry.supported) autoPrewarm(id)
      return entry.supported
    } catch (e: any) {
      entry.loading = false
      entry.error = (e?.message || String(e))
      ElMessage.error('打开失败：' + entry.error)
      return false
    }
  }

  /** 关闭 dump：剔除并释放其直方图 / 泄漏检测 / 引用追踪缓存 */
  function closeDump(dumpId: string) {
    const idx = dumps.value.findIndex(d => d.id === dumpId)
    if (idx === -1) return
    const closedPath = dumps.value[idx].path
    dumps.value.splice(idx, 1)
    delete activeView.value[dumpId]
    delete histCache.value[dumpId]
    delete leakCache.value[dumpId]
    delete refsCache.value[dumpId]
    // 释放后端对象图会话缓存（非关键路径，不阻塞关闭）
    void releaseRefsSession(closedPath)
    if (activeDumpId.value === dumpId) {
      activeDumpId.value = dumps.value[Math.min(idx, dumps.value.length - 1)]?.id || ''
    }
  }

  /** 切换激活 dump */
  function setActiveDump(dumpId: string) {
    if (dumps.value.some(d => d.id === dumpId)) activeDumpId.value = dumpId
  }

  /** 切换某 dump 的分析子视图 */
  function setActiveView(dumpId: string, view: MatViewKey) {
    activeView.value[dumpId] = view
  }

  /** 获取某 dump 当前子视图（默认概览） */
  function viewOf(dumpId: string): MatViewKey {
    return activeView.value[dumpId] || 'overview'
  }

  /** 惰性加载直方图（有缓存直接返回），失败信息写入缓存 */
  async function ensureHistogram(dumpId: string): Promise<MatHistogramResult | null> {
    const dump = dumps.value.find(d => d.id === dumpId)
    if (!dump) return null
    const cached = histCache.value[dumpId]
    if (cached?.data) return cached.data
    if (cached?.loading) return null
    histCache.value[dumpId] = { loading: true, error: '', data: null }
    try {
      // 不传 limit：后端返回全部类行（虚拟滚动表格承载，行多不卡）
      const data = await fetchHistogram(dump.path)
      histCache.value[dumpId] = { loading: false, error: '', data }
      return data
    } catch (e: any) {
      histCache.value[dumpId] = { loading: false, error: (e?.message || String(e)), data: null }
      return null
    }
  }

  /**
   * 惰性加载泄漏检测结果（有缓存直接返回），失败信息写入缓存。
   * 支配树分析较重，同一 dump 首次进入「泄漏检测」子 tab 触发一次。
   */
  async function ensureLeakSuspects(dumpId: string): Promise<MatLeakSuspectsResult | null> {
    const dump = dumps.value.find(d => d.id === dumpId)
    if (!dump) return null
    const cached = leakCache.value[dumpId]
    if (cached?.data) return cached.data
    if (cached?.loading) return null
    leakCache.value[dumpId] = { loading: true, error: '', data: null }
    try {
      // 泄漏检测要支配树：大 dump 先让后台进程算完，别在主进程里现跑 Lengauer-Tarjan
      await ensureAnalysisReady(dumpId, true)
      const data = await fetchLeakSuspects(dump.path)
      leakCache.value[dumpId] = { loading: false, error: '', data }
      return data
    } catch (e: any) {
      leakCache.value[dumpId] = { loading: false, error: (e?.message || String(e)), data: null }
      return null
    }
  }

  /**
   * 构建引用追踪会话（`force` = 忽略已有缓存重来）。
   *
   * **关键：构建前先看有没有后台分析在跑。** 解析整份 dump + 建图是唯一的慢环节
   * （GB 级是分钟级，且在**主进程**里跑就是界面冻结）。如果 sidecar 正在为同一份
   * dump 干活，这里就等它落完 `.bdtidx` 再读 —— 读索引是毫秒级，等于把「慢活」
   * 整体挪出了主进程，而且不会白算两遍。
   */
  async function doBuildRefs(dumpId: string, force: boolean): Promise<RefBuildInfo | null> {
    const dump = dumps.value.find(d => d.id === dumpId)
    if (!dump) return null
    const cached = refsCache.value[dumpId]
    if (!force) {
      if (cached?.data) return cached.data
      if (cached?.loading) return null
    }
    refsCache.value[dumpId] = { loading: true, error: '', data: null }
    try {
      // 只要对象图（`needDom=false`）—— 支配树留给真正要它的视图，
      // 别为了看一眼引用就让用户等一遍 Lengauer-Tarjan
      if ((await ensureAnalysisReady(dumpId, false)) === 'canceled') {
        refsCache.value[dumpId] = {
          loading: false,
          error: '后台分析已取消。可点「重新分析」再跑，或直接重新加载（会在主进程里硬算，大 dump 会卡住界面）',
          data: null,
        }
        return null
      }
      const data = await fetchRefsBuild(dump.path)
      refsCache.value[dumpId] = { loading: false, error: '', data }
      return data
    } catch (e: any) {
      refsCache.value[dumpId] = { loading: false, error: (e?.message || String(e)), data: null }
      return null
    }
  }

  /** 按需构建（已有缓存直接返回） */
  const ensureRefsBuild = (dumpId: string) => doBuildRefs(dumpId, false)

  /** 重试构建（后端 LRU 可能已逐出会话，置空缓存再建） */
  const rebuildRefs = (dumpId: string) => doBuildRefs(dumpId, true)

  // ==================== sidecar 后台分析 ====================

  /**
   * 后台分析作业：`path → 快照`。
   *
   * 用 path 而不是 dumpId 作键：事件载荷里只有 path（子进程不认识前端的 dumpId），
   * 且同一份 dump 在同一进程里 path 唯一。
   */
  const workerJobs = ref<Record<string, MatWorkerJob>>({})
  /** 事件订阅（进程级单例：store 活得比任何视图都久） */
  let workerUnsub: Unsubscribe | null = null
  /** 自动预热的在途 Promise（`doBuildRefs` 要等它登记完才好看作业状态） */
  const prewarmInflight = new Map<string, Promise<boolean>>()

  /**
   * 打开即自动预热索引的体积门槛（按**解压后**大小算，`.gz` 不能看压缩包大小）。
   *
   * 256MB 的依据：13.6MB 的样例建图 666ms，线性外推 256MB 约 12s —— 主进程连续
   * 冻结 12 秒已经不能算「只是慢一下」了。再小就没必要起进程（进程启动本身也要几十毫秒）。
   */
  const AUTO_WORKER_MIN_BYTES = 256 * 1024 * 1024

  /** dump 的实际解析体积（`.gz` 用解压后大小） */
  function dumpBytes(d: MatDump): number {
    return d.sourceInfo?.hprof_size || d.size || 0
  }

  /** 事件的「最终态」字段映射：把 NDJSON 行合并进快照 */
  function applyWorkerEvent(p: MatWorkerEvent) {
    const prev = workerJobs.value[p.path]
    const j: MatWorkerJob =
      prev ||
      {
        job_id: p.job_id,
        path: p.path,
        dom: !!p.with_dom,
        phase: 'starting',
        done: 0,
        total: null,
        msg: '',
        finished: false,
        ok: false,
        canceled: false,
        error: null,
        nodes: null,
        edges: null,
        dom_state: null,
        pid: null,
        elapsed_ms: 0,
      }
    const next: MatWorkerJob = { ...j, job_id: p.job_id }
    switch (p.ev) {
      case 'hello':
        next.phase = 'starting'
        next.msg = '分析进程已启动'
        break
      case 'stage':
        if (p.stage) next.phase = p.stage
        if (typeof p.done === 'number') next.done = p.done
        next.total = p.total ?? null
        if (p.msg) next.msg = p.msg
        break
      case 'done':
        next.ok = true
        next.finished = true
        next.phase = 'done'
        next.nodes = p.nodes ?? null
        next.edges = p.edges ?? null
        next.dom_state = p.dom_state ?? null
        next.elapsed_ms = p.ms ?? 0
        next.msg = '分析完成'
        break
      case 'error':
        next.finished = true
        next.phase = 'error'
        next.error = p.message || '分析进程报错'
        next.elapsed_ms = p.ms ?? next.elapsed_ms
        break
      case 'exit':
        next.finished = true
        next.ok = !!p.ok
        next.canceled = !!p.canceled
        if (p.error) next.error = p.error
        if (!next.ok && !p.error) next.error = next.error || '分析进程异常退出'
        if (p.canceled) next.phase = 'canceled'
        break
      default:
        // log / 未知事件：只写进 msg，不改变阶段（仍要落盘，否则调试输出会丢）
        if (p.msg) next.msg = p.msg
        break
    }
    workerJobs.value[p.path] = next
  }

  /** 懒建立事件订阅（第一次启动作业时装，之后复用） */
  function ensureWorkerListener() {
    if (workerUnsub) return
    void subscribe<MatWorkerEvent>('mat-worker', applyWorkerEvent).then((un) => {
      workerUnsub = un
    })
  }

  /** 某 dump 的后台作业（无则 null） */
  function workerJobOf(dumpId: string): MatWorkerJob | null {
    const d = dumps.value.find((x) => x.id === dumpId)
    return d ? workerJobs.value[d.path] || null : null
  }

  /**
   * 启动后台分析（独立进程）。
   * `dom` = 是否连支配树一起算（决定之后「泄漏检测 / 支配树」是否也秒开）。
   */
  async function startWorker(dumpId: string, dom = true, silent = false): Promise<boolean> {
    const d = dumps.value.find((x) => x.id === dumpId)
    if (!d) return false
    ensureWorkerListener()
    try {
      workerJobs.value[d.path] = await startWorkerApi(d.path, dom)
      return true
    } catch (e: any) {
      // 自动预热失败不弹窗：用户没主动点，弹出来只会莫名其妙
      if (!silent) ElMessage.error(e?.message || String(e))
      return false
    }
  }

  /**
   * 大 dump 打开后自动在后台预热索引（不弹窗、不阻塞）。
   *
   * **同时只跑一个**：两份 GB 级 dump 并行分析会把内存吃光，而预热的收益只是
   * 「省一次等待」，没必要为此冒险。正在跑别的就跳过，用户仍可手动点「开始分析」。
   */
  function autoPrewarm(dumpId: string) {
    const d = dumps.value.find((x) => x.id === dumpId)
    if (!d || !d.supported) return
    const p = (async () => {
      // 同时只跑一个：两份 GB 级 dump 并行分析会把内存吃光，而预热只是「省一次等待」
      if (Object.values(workerJobs.value).some((j) => !j.finished)) return false
      if (!(await indexWorthBuilding(d, true))) return false
      return startWorker(dumpId, true, true)
    })().finally(() => {
      prewarmInflight.delete(d.path)
    })
    prewarmInflight.set(d.path, p)
  }

  /**
   * 索引还值不值得建。
   *
   * 两个前置：① 索引**可写** —— 超过 `indexWriteMaxGb` 的 dump 只读不写，
   * 子进程算完也没法共享，只会白跑一遍；② 目标产物**还没生成** —— 已有时直接读。
   */
  async function indexWorthBuilding(d: MatDump, needDom: boolean): Promise<boolean> {
    const idx = await fetchIndexInfo(d.path).catch(() => null)
    if (!idx) return true // 查不到就按可用处理：至少能把冻结挪出主进程
    if (dumpBytes(d) > idx.write_max_gb * 1024 ** 3) return false
    return needDom ? !idx.dom_valid : !idx.graph_valid
  }

  /**
   * **分析就绪闸门**：需要的东西已算好就立刻返回，否则把重活交给后台进程并等它。
   *
   * 所有「重」视图（引用追踪 / 泄漏检测 / 支配树 / 大对象排行 / 深堆列 / 不可达查询）
   * 都必须先过这道闸 —— 否则它们在主进程里现解析、现跑 Lengauer-Tarjan，
   * 大 dump 上就是几十秒到几分钟的界面冻结（P3-5 要解决的正是这个）。
   *
   * - `needDom=false`：只要对象图（引用追踪 / 不可达查询）
   * - `needDom=true`：还要支配树（泄漏 / 支配树 / 大对象排行 / 深堆列）
   *
   * 小 dump 直接放行（主进程几百毫秒，起进程反而更慢）；索引只读不写也放行。
   * 返回 `canceled` 时调用方应展示「已取消」而不是继续硬算。
   */
  async function ensureAnalysisReady(
    dumpId: string,
    needDom = false,
  ): Promise<'ok' | 'canceled'> {
    const d = dumps.value.find((x) => x.id === dumpId)
    if (!d) return 'ok'
    // 自动预热可能还在「起进程」的路上（openDump 与这里是并发的），先等它登记完
    const pending = prewarmInflight.get(d.path)
    if (pending) await pending
    // ① 已有作业在跑就先等它 —— 这时再算一遍才是真的浪费（还把主进程冻住）
    const running = workerJobs.value[d.path]
    if (running && !running.finished) {
      if ((await waitForWorker(d.path)) === 'canceled') return 'canceled'
    }
    if (dumpBytes(d) < AUTO_WORKER_MIN_BYTES) return 'ok'
    // ② 起作业；最多两轮 —— 后端按 path 去重，中途插进来的作业可能是 dom=false 的，
    //    等完发现支配树仍缺就补一次
    for (let i = 0; i < 2; i++) {
      if (!(await indexWorthBuilding(d, needDom))) return 'ok'
      if (!(await startWorker(dumpId, needDom, true))) return 'ok'
      if ((await waitForWorker(d.path)) === 'canceled') return 'canceled'
    }
    return 'ok'
  }

  /**
   * 等某份 dump 的后台作业跑完。
   *
   * 主要靠事件推进（`applyWorkerEvent` 已在写 `workerJobs`），但事件只推给**当时**
   * 订阅着的窗口 —— 重挂载 / 事件丢失时会一直停在旧快照，所以每约 2s 主动拉一次快照兜底。
   *
   * 返回：`done` 成功 / `failed` 失败或超时 / `canceled` 被取消 / `none` 没有作业。
   */
  async function waitForWorker(
    path: string,
    timeoutMs = 20 * 60 * 1000,
  ): Promise<'done' | 'failed' | 'canceled' | 'none'> {
    const started = Date.now()
    let ticks = 0
    for (;;) {
      const job = workerJobs.value[path]
      if (!job) return 'none'
      if (job.finished) {
        if (job.ok) return 'done'
        return job.canceled ? 'canceled' : 'failed'
      }
      if (Date.now() - started > timeoutMs) return 'failed'
      await new Promise((r) => setTimeout(r, 400))
      if (++ticks % 5 === 0 && job.job_id) {
        try {
          const snap = await fetchWorkerStatus(job.job_id)
          if (snap) workerJobs.value[path] = snap
        } catch {
          // 作业已被后端回收（只保留最近几条）：继续按 store 里的终态判断
        }
      }
    }
  }

  /** 取消后台分析（杀进程） */
  async function cancelWorkerOf(dumpId: string): Promise<void> {
    const job = workerJobOf(dumpId)
    if (!job) return
    try {
      await cancelWorker(job.job_id)
    } catch (e: any) {
      ElMessage.error(e?.message || String(e))
    }
  }

  /**
   * 恢复某 dump 的后台作业状态（视图重新挂载时调）。
   * 事件只推给当时的监听者，切走再回来要靠这次查询把进度条接上。
   */
  async function restoreWorker(dumpId: string): Promise<void> {
    const d = dumps.value.find((x) => x.id === dumpId)
    if (!d) return
    ensureWorkerListener()
    try {
      const job = await fetchWorkerStatusForPath(d.path)
      if (job) workerJobs.value[d.path] = job
    } catch {
      // 恢复状态失败不影响主流程（大不了进度条不显示）
    }
  }

  /** 跳到「引用追踪」子视图并携带跳转预设（直方图/泄漏检测入口） */
  function openRefs(dumpId: string, preset: RefIntentPreset | null = null) {
    setActiveView(dumpId, 'refs')
    refsIntent.value = { dumpId, seq: refsIntent.value.seq + 1, preset }
  }

  /** 打开「分析预算」设置（懒加载预算信息） */
  function openSettings() {
    settingsOpen.value = true
    void loadLimitInfo()
  }

  function closeSettings() {
    settingsOpen.value = false
  }

  /** 拉取预算/上限明细（带缓存，重复打开不重查） */
  async function loadLimitInfo(): Promise<void> {
    if (limitInfo.value.loading) return
    limitInfo.value = { loading: true, error: '', data: limitInfo.value.data }
    try {
      limitInfo.value = { loading: false, error: '', data: await fetchAnalysisLimits() }
    } catch (e: any) {
      limitInfo.value = { loading: false, error: (e?.message || String(e)), data: limitInfo.value.data }
    }
  }

  /**
   * 保存分析预算并重试所有被截断的分析。
   * gb = null → 恢复自动（删除覆盖项）；后端按 plugins/mat.json 现算，下次调用即生效。
   */
  async function saveBudget(gb: number | null): Promise<boolean> {
    try {
      const st = await loadKv('plugins/mat.json')
      if (gb == null || !(gb > 0)) {
        await st.delete('analysisBudgetGb')
      } else {
        await st.set('analysisBudgetGb', Math.round(gb * 10) / 10)
      }
      ElMessage.success(gb ? `分析预算已设为 ${gb} GB` : '已恢复自动预算')
      await loadLimitInfo()
      await retryTruncatedAnalyses()
      return true
    } catch (e: any) {
      ElMessage.error('保存失败：' + (e?.message || String(e)))
      return false
    }
  }

  /** 预算变更后：仅对“超限被截断”的分析结果自动重试（成功的保留，避免无谓重算） */
  async function retryTruncatedAnalyses(): Promise<void> {
    const jobs = dumps.value.map(async (d) => {
      const r = refsCache.value[d.id]
      if (r?.data?.truncated) {
        refsCache.value[d.id] = { loading: true, error: '', data: null }
        try {
          refsCache.value[d.id] = { loading: false, error: '', data: await fetchRefsBuild(d.path) }
        } catch (e: any) {
          refsCache.value[d.id] = { loading: false, error: (e?.message || String(e)), data: null }
        }
      }
      const l = leakCache.value[d.id]
      if (l?.data?.summary?.truncated) {
        leakCache.value[d.id] = { loading: true, error: '', data: null }
        try {
          leakCache.value[d.id] = { loading: false, error: '', data: await fetchLeakSuspects(d.path) }
        } catch (e: any) {
          leakCache.value[d.id] = { loading: false, error: (e?.message || String(e)), data: null }
        }
      }
    })
    await Promise.all(jobs)
  }

  return {
    dumps,
    activeDumpId,
    activeDump,
    activeView,
    histCache,
    leakCache,
    refsCache,
    refsIntent,
    openDump,
    closeDump,
    setActiveDump,
    setActiveView,
    viewOf,
    ensureHistogram,
    ensureLeakSuspects,
    ensureRefsBuild,
    rebuildRefs,
    openRefs,
    workerJobs,
    workerJobOf,
    startWorker,
    cancelWorkerOf,
    restoreWorker,
    autoPrewarm,
    ensureAnalysisReady,
    dumpBytes,
    AUTO_WORKER_MIN_BYTES,
    settingsOpen,
    limitInfo,
    openSettings,
    closeSettings,
    loadLimitInfo,
    saveBudget,
  }
})
