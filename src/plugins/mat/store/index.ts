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
  fetchAnalysisLimits,
  fetchHeader,
  fetchHistogram,
  fetchLeakSuspects,
  fetchRefsBuild,
  releaseRefsSession,
  toDump,
} from '../api/matApi'
import type {
  MatHistogramResult,
  MatLeakSuspectsResult,
  MatLimitInfo,
  RefBuildInfo,
} from '../api/matApi'
import { load as loadKv } from '@/core/runtime'
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
    const entry: MatDump = {
      id,
      path,
      name: path.split(/[\\/]/).pop() || path,
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
      const info = await fetchHeader(path)
      Object.assign(entry, toDump(path, info))
      entry.error = undefined
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
      const data = await fetchLeakSuspects(dump.path)
      leakCache.value[dumpId] = { loading: false, error: '', data }
      return data
    } catch (e: any) {
      leakCache.value[dumpId] = { loading: false, error: (e?.message || String(e)), data: null }
      return null
    }
  }

  /**
   * 惰性构建引用追踪会话（有缓存直接返回）。
   * 后端会话会缓存对象图与索引，大 dump 首次构建耗时较长（loading 提示）。
   */
  async function ensureRefsBuild(dumpId: string): Promise<RefBuildInfo | null> {
    const dump = dumps.value.find(d => d.id === dumpId)
    if (!dump) return null
    const cached = refsCache.value[dumpId]
    if (cached?.data) return cached.data
    if (cached?.loading) return null
    refsCache.value[dumpId] = { loading: true, error: '', data: null }
    try {
      const data = await fetchRefsBuild(dump.path)
      refsCache.value[dumpId] = { loading: false, error: '', data }
      return data
    } catch (e: any) {
      refsCache.value[dumpId] = { loading: false, error: (e?.message || String(e)), data: null }
      return null
    }
  }

  /** 重试构建（后端 LRU 可能已逐出会话，置空缓存再建） */
  async function rebuildRefs(dumpId: string): Promise<RefBuildInfo | null> {
    const dump = dumps.value.find(d => d.id === dumpId)
    if (!dump) return null
    refsCache.value[dumpId] = { loading: true, error: '', data: null }
    try {
      const data = await fetchRefsBuild(dump.path)
      refsCache.value[dumpId] = { loading: false, error: '', data }
      return data
    } catch (e: any) {
      refsCache.value[dumpId] = { loading: false, error: (e?.message || String(e)), data: null }
      return null
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
    settingsOpen,
    limitInfo,
    openSettings,
    closeSettings,
    loadLimitInfo,
    saveBudget,
  }
})
