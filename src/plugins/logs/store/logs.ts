/**
 * 日志工作台状态管理
 *
 * 数据分两条路径：
 * - 全量视图：分块懒加载整个文件（lines），滚动到底继续取下一块
 * - 筛选视图：先向后端要"命中行段"（服务端全量扫描），再按段取内容（filteredLines）
 *
 * 之所以把筛选交给后端：前端只持有部分行，若在本地过滤，结果会随滚动不断变化。
 * 关键词搜索仍然在本地做——它作用于"当前已显示的行"，且需要实时高亮与逐条导航。
 */
import { defineStore } from 'pinia'
import { computed, ref, shallowRef, triggerRef, watch } from 'vue'
import { call } from '@/api/rpc'
import { subscribe, type Unsubscribe } from '@/core/runtime'
import { getCurrentWorkspaceId } from '@/composables/usePluginDir'
import { parseLines } from '../core/parser'
import {
  compileMatcher,
  firstIndexAfter,
  indexOfHitLine,
  indexOfSorted,
  lastIndexBefore,
  scanAll,
  scanRange,
  segmentIndexOfLine,
  toSearchResult,
} from '../core/search'
import {
  loadActiveSource,
  loadRules,
  loadSources,
  saveActiveSource,
  saveRules,
  saveSources,
} from '../core/storage'
import { DEFAULT_FILTER, isFilterActive, type FilterState } from '../core/filter'
import {
  buildIndex as apiBuildIndex,
  dropIndex as apiDropIndex,
  exportLines as apiExportLines,
  queryLines as apiQueryLines,
  type LineSegment,
  type LogExportResult,
  type LogIndexMeta,
  type LogQueryResult,
} from '../core/indexApi'
import type {
  HighlightRule,
  LineHits,
  LogSource,
  ParsedLine,
  SearchOptions,
  SearchResult,
} from '../types'

interface ApiResp<T> {
  code: number
  msg: string
  data: T
}

interface FileLinesResult {
  content: string
  first_line_number: number
  total_lines: number
  has_more: boolean
}

/** 单次读取行数（分块懒加载，避免一次性读入超大文件） */
const CHUNK_SIZE = 2000
/** 跳转补载时的单次读取行数：跳到文件尾部时不至于发几十次请求 */
const JUMP_CHUNK_SIZE = 20000
/**
 * 滚动补载的窗口大小。
 *
 * 刻意远小于 `WINDOW_MAX_LINES`：视口只有几十行，一次读 2 万行要 parse 2 万个 JS 对象，
 * 划一下滚动条就卡一下。5000 行 ≈ 80 屏，配上 1/4 的迟滞余量足够撑住一次长距离滚动。
 */
const SCROLL_WINDOW_LINES = 5000
/**
 * 单源保留的行窗口上限（§3.1）。
 *
 * `ParsedLine` 是 JS 对象（含原文字符串），每行数百字节——后端索引已压到 ~1B/行，
 * 但**前端行集才是内存大头**，不淘汰就会随滚动无限增长。2 万行约 6–12MB/源。
 */
const WINDOW_MAX_LINES = 20_000

/** 多源色标取色池 */
const SOURCE_COLORS = ['#409EFF', '#67C23A', '#E6A23C', '#F56C6C', '#909399', '#9B59B6', '#16A085']

export const useLogStore = defineStore('logs', () => {
  // ==================== 状态 ====================
  const sources = ref<LogSource[]>([])
  const activeSourceId = ref('')
  /** 已加载的解析后日志行（大数组用 shallowRef，避免深层响应式开销） */
  const lines = shallowRef<ParsedLine[]>([])
  /**
   * `lines[0]` 对应的**原始行号**（1-based）。
   *
   * 窗口淘汰后 `lines` 不再从第 1 行开始，行号 → 下标必须走 `lineNo - baseLineNo`；
   * 继续用 `lineNo - 1` 会整体错位（这正是"跳转跳错行"的典型成因）。
   */
  const baseLineNo = ref(1)
  const loading = ref(false)
  const loadingMore = ref(false)
  const errorMsg = ref('')
  const encoding = ref('UTF-8')
  const ready = ref(false)

  const rules = ref<HighlightRule[]>([])
  const search = ref<SearchOptions>({ keyword: '', mode: 'text', ignoreCase: false })
  const searchError = ref('')
  const result = ref<SearchResult>({ hits: [], total: 0, hitLines: new Set<number>() })
  const currentHitIndex = ref(-1)
  const currentRangeIndex = ref(0)
  const currentLine = ref(0)

  // ---------------- 合并查询视图（筛选 + 搜索一次请求） ----------------
  /**
   * 最近一次 `log_query_lines` 的结果（§2.5 / §3.2）。
   *
   * 它是"渲染段"与"导航行号"的**共同事实源**：二者出自同一次查询，
   * 因此核心不变量 `lines ⊆ ∪segments` 天然成立——旧实现里筛选与搜索是两次独立请求，
   * 搜索给出的行号可能根本不在筛选结果里，跳转必然断裂。
   */
  const view = ref<LogQueryResult | null>(null)
  /** 结果是否可用（不可用 = 无筛选且无关键词 / 实时流源 / 查询失败） */
  const viewActive = ref(false)
  /** 查询进行中 */
  const viewScanning = ref(false)
  /** 降级原因（查询失败 / 命中截断 / 索引截断） */
  const viewNote = ref('')
  /** 因超出索引上限而未参与本次查询的行数（>0 时 UI 必须提示） */
  const unindexedLines = ref(0)
  /** 竞态守卫：只有最后一次请求的结果允许落地 */
  let viewToken = 0
  /** 防抖定时器：避免每敲一个字符就查一遍 */
  let viewTimer: number | null = null

  /**
   * 跳转失败的说明（成功时为空）。
   *
   * 跳转打不通时**必须**给出可见原因：原先只是静默 `return`，
   * 用户看到的就是"点了没反应"，无法区分"没有命中"和"这个功能坏了"。
   */
  const jumpHint = ref('')

  // ---------------- 筛选（基础筛选） ----------------
  const filter = ref<FilterState>({ ...DEFAULT_FILTER })
  /** 文件级索引统计（级别计数 / 时间范围 / 时间桶） */
  const indexMeta = ref<LogIndexMeta | null>(null)
  const indexLoading = ref(false)
  const filterLoading = ref(false)
  const filterError = ref('')
  /** 命中总行数（后端返回） */
  const filterMatched = ref(0)
  /** 命中段是否被 maxSegments 截断 */
  const filterTruncated = ref(false)
  /** 筛选结果行（行号为原始行号，不连续） */
  const filteredLines = shallowRef<ParsedLine[]>([])
  const segments = ref<LineSegment[]>([])
  /** 下一个待加载的段下标 */
  const segIndex = ref(0)
  /** 当前段内下一个待读取的行号（0 = 从头开始） */
  const segPos = ref(0)
  const exporting = ref(false)

  /** 行号 → 在 displayLines 中的下标（筛选态下行号不连续，命中导航需要映射） */
  let linePosMap = new Map<number, number>()

  /**
   * 当前工作区 ID（"一键切换逻辑"的关键）。
   * 由 init() 在首次加载时取一次；随后在 onWorkspaceChanged 中随切换更新。
   * 注意：不能直接用 getCurrentWorkspaceId()，因为工作区切换事件发出时模块级 ID
   * 已经被 setPluginWorkspaceId 改成了"新"工作区，而本 store 此刻仍持有"旧"工作区数据。
   */
  const currentWsId = ref('')
  /** 取持久化用的有效工作区桶；为空时使用兜底桶，避免写入空键 */
  function ws(): string {
    return currentWsId.value || '__global__'
  }
  /** 工作区切换事件监听器是否已安装（store 单例，只装一次） */
  let watchInstalled = false

  // ==================== 实时流（WebSocket / TCP） ====================
  /** 每个流源独立的行缓冲（源真相），避免与其它源的全量 lines 互相干扰 */
  const streamBuffers = new Map<string, ParsedLine[]>()
  /** ws 协议的原生 WebSocket 句柄 */
  const streamSockets = new Map<string, WebSocket>()
  /** tcp 协议的事件订阅取消函数 */
  const streamUnsubs = new Map<string, Unsubscribe>()
  /** 已同步到 sources 列表的行数（节流侧栏计数刷新） */
  const streamLastSync = new Map<string, number>()
  /** 单流缓冲上限，超出后丢弃最旧一半并重排行号，防止无限增长 */
  const MAX_STREAM_LINES = 200_000
  /** 侧栏计数刷新节流步长 */
  const STREAM_SYNC_STEP = 500

  /** 已打开「文件类」源（本地 / SSH）的内容缓冲：按源 ID 缓存已加载的行与索引元数据，切换 tab 时直接复用、不重读磁盘 */
  interface FileBuffer {
    lines: ParsedLine[]
    /** 缓存内容首行的原始行号（窗口化后不再是 1） */
    baseLineNo: number
    loadedLines: number
    totalLines: number
    /** 后端索引元数据（时间轴 buckets / 级别计数 / 时间范围）。缓存下来供切回 tab 时同步恢复——否则 clearView 会把 indexMeta 抹空，Timeline 会变空态。 */
    indexMeta?: LogIndexMeta
  }
  const buffers = new Map<string, FileBuffer>()

  /** 每个源记忆的滚动位置（**逻辑下标**，非像素），切换回该 tab 时恢复，使 tab 像独立页面一样保留阅读进度 */
  const scrollMap = new Map<string, number>()

  /**
   * 各源是否正在读取磁盘 / 重新加载（用于 tab 上的"加载中"指示器）。
   * 仅真实读盘时置位；复用已缓存内容切换 tab 时不置位（瞬时完成，无圈，符合预期）。
   * 用 Set + 重新赋值触发响应式（直接 mutate Set 不会触发）。
   */
  const loadingSourceIds = ref<Set<string>>(new Set())
  function markLoading(id: string) {
    const next = new Set(loadingSourceIds.value)
    next.add(id)
    loadingSourceIds.value = next
  }
  function unmarkLoading(id: string) {
    const next = new Set(loadingSourceIds.value)
    next.delete(id)
    loadingSourceIds.value = next
  }
  /** 判断某源是否正在加载（tab 指示器用） */
  function isSourceLoading(id: string): boolean {
    return loadingSourceIds.value.has(id)
  }

  /**
   * 非活跃源的内容缓冲上限（§7.9）。
   *
   * 多 tab 打开时 `buffers` 是前端内存的主要增长源：每份都持有整份已加载 `ParsedLine[]`。
   * 超出上限淘汰**最久未用**者（Map 的插入顺序即访问顺序，set 前先 delete 即"提到最新"）；
   * 被淘汰的源切回时走重读磁盘路径——L2 已把按行取内容改成 O(1) 定位，代价可接受。
   */
  const MAX_BUFFERS = 2

  /** 把当前活跃源的文件视图内容快照进 buffers，供切回该 tab 时即时复用 */
  function snapshotBuffer(id: string) {
    if (!id) return
    const s = sources.value.find(x => x.id === id)
    buffers.delete(id)
    buffers.set(id, {
      lines: lines.value,
      baseLineNo: baseLineNo.value,
      loadedLines: s?.loadedLines ?? lines.value.length,
      totalLines: s?.totalLines ?? lines.value.length,
    })
    // 淘汰最久未用的**其它**源（当前源与活跃源不参与）
    while (buffers.size > MAX_BUFFERS) {
      let victim: string | null = null
      for (const k of buffers.keys()) {
        if (k !== id && k !== activeSourceId.value) {
          victim = k
          break
        }
      }
      if (!victim) break
      buffers.delete(victim)
    }
  }

/**
 * 记录某源滚动到的**逻辑下标**。
 *
 * 记下标而非像素：行高会随 `logicalTotal` 压缩（见 `LogView` 的 rowHeight），
 * 同一个像素值在两次打开时对应的行号可能完全不同，记像素会错位。
 */
function setScroll(id: string, idx: number) {
  if (!id) return
  scrollMap.set(id, Math.max(0, idx))
}

/** 读取某源记忆的逻辑下标（无则 0） */
function getScroll(id: string): number {
  return scrollMap.get(id) ?? 0
}

  /** 激活源变化即持久化（切换日志源 / 关闭源重选后恢复用） */
  watch(activeSourceId, () => {
    if (ready.value) void persistActive()
  })

  /**
   * 把指定源自身记忆的搜索 / 筛选条件载入"当前工作副本"（切换 tab / 打开源时调用）。
   * 未记忆过则回落到空搜索 + 默认筛选，使每个 tab 各自独立、互不干扰。
   */
  function loadSourceSearchFilter(s: LogSource) {
    search.value = s.search
      ? { ...s.search }
      : { keyword: '', mode: 'text', ignoreCase: false }
    filter.value = s.filter ? { ...s.filter } : { ...DEFAULT_FILTER }
  }

  /**
   * 当前源的搜索 / 筛选一旦变化，立即写回该源对象并持久化。
   * 这是"每 tab 独立搜索/筛选"的核心：全局 search/filter 仅作为激活源的临时工作副本，
   * 真正的归属是各 LogSource 自身的 search/filter 字段。
   */
  watch(
    [search, filter],
    () => {
      if (!ready.value) return
      const s = activeSource.value
      if (!s) return
      s.search = { ...search.value }
      s.filter = { ...filter.value }
      void persistSources()
    },
    { deep: true },
  )

  // ==================== 派生 ====================
  const activeSource = computed(() => sources.value.find(s => s.id === activeSourceId.value))
  const filterActive = computed(() => isFilterActive(filter.value))
  /** 当前实际渲染的行集：筛选态用命中行，否则用全量行 */
  const displayLines = computed<ParsedLine[]>(() =>
    filterActive.value ? filteredLines.value : lines.value,
  )
  const loadedCount = computed(() => displayLines.value.length)

  /**
   * 逻辑总行数——**滚动条高度的唯一依据**（§3.1 比例滚动）。
   *
   * 它必须回答"这个文件一共有多少行 / 筛选结果一共有多少条"，
   * 而**不是**"当前加载了多少行"。用它算高度，滚动条才代表数据位置而非加载进度：
   * 高度恒定 ⇒ 拖到哪就是哪一行 ⇒ 不再出现"滚到底还在加载、底部一直往前跑"。
   *
   * - 筛选态：`view.lines` 的长度（= **可导航**命中数，上限 `MAX_MATCH_LINES`）。
   *   刻意不用 `view.matched`：超出截断的部分没有行号，无法定位，
   *   把它算进高度等于又造一个"到底了还在加载"。截断事实由 `viewNote` 提示。
   * - 全量态：`source.totalLines`（后端 `count_file_lines` 给出，可靠）。
   * - 实时流：没有"总行数"概念，只能按已收到的行数（流在增长，滚动条随之变长是合理的）。
   */
  const logicalTotal = computed(() => {
    if (filterActive.value) {
      if (viewActive.value && view.value && view.value.lines.length) return view.value.lines.length
      return filteredLines.value.length
    }
    const s = activeSource.value
    if (!s) return lines.value.length
    return s.totalLines || lines.value.length
  })

  /**
   * 导航用的命中行号序列：优先用全文件扫描结果，否则回退到已加载行的本地命中。
   * 这是"搜索能覆盖整个文件"的关键——原先导航只走 result.hits（已加载部分）。
   */
  const navLines = computed<number[]>(() => {
    if (viewActive.value && view.value && view.value.lines.length) return view.value.lines
    return result.value.hits.map(h => h.lineIndex)
  })
  /** 导航总数：合并查询给出的命中真值；不可用时回退为本地命中行数 */
  const navTotal = computed(() =>
    viewActive.value && view.value ? view.value.matched : result.value.hits.length,
  )

  /** 当前聚焦命中所在行（按行号在本地命中里查；未加载的行没有命中区间，返回 null） */
  const currentHit = computed<LineHits | null>(() => {
    const lineNo = currentLine.value
    if (!lineNo) return null
    const i = indexOfHitLine(result.value.hits, lineNo)
    return i >= 0 ? result.value.hits[i] : null
  })

  // ==================== 后端读取 ====================
  async function readChunk(
    path: string,
    startLine: number,
    count = CHUNK_SIZE,
  ): Promise<FileLinesResult | null> {
    const res = await call<ApiResp<FileLinesResult>>('read_file_lines', {
      filePath: path,
      startLine,
      lineCount: count,
      encoding: encoding.value,
    })
    if (res && res.code === 200) return res.data
    return null
  }

  // ==================== 行集维护 ====================
  /** 追加解析后的行，并在筛选态下维护行号 → 下标的映射 */
  function appendLines(parsed: ParsedLine[]) {
    if (!parsed.length) return
    if (filterActive.value) {
      const base = filteredLines.value.length
      for (let i = 0; i < parsed.length; i++) linePosMap.set(parsed[i].index, base + i)
      filteredLines.value = filteredLines.value.concat(parsed)
    } else {
      // 窗口淘汰：向下滚动时丢最旧的（用户视线在尾部）
      let arr = lines.value.concat(parsed)
      if (arr.length > WINDOW_MAX_LINES) {
        const drop = arr.length - WINDOW_MAX_LINES
        arr = arr.slice(drop)
        baseLineNo.value += drop
      } else if (!lines.value.length) {
        baseLineNo.value = arr[0].index
      }
      lines.value = arr
    }
  }

  /**
   * 把整个窗口搬到以 `startLine` 开头的位置（跳转目标远在窗口之外时用）。
   *
   * 逐块续读会为了一个目标行把整段都加载进来，反而破坏窗口；直接搬窗口只发一次请求。
   */
  async function loadWindowAt(startLine: number, count: number) {
    const s = activeSource.value
    if (!s || startLine < 1) return
    const data = await readChunk(s.path, startLine, count)
    if (!data || !data.content) return
    const parsed = parseLines(data.content, data.first_line_number)
    if (!parsed.length) return
    lines.value = parsed
    baseLineNo.value = parsed[0].index
    s.loadedLines = baseLineNo.value + parsed.length - 1
    if (data.total_lines) s.totalLines = data.total_lines
    scanLocalHighlight()
  }

  /**
   * 行号 → 数组下标（导航用）；不在当前行集中返回 -1。
   *
   * 全量态也要走这里：窗口淘汰后下标是 `lineNo - baseLineNo`，不是 `lineNo - 1`。
   */
  function positionOfLine(lineNo: number): number {
    if (!filterActive.value) {
      const i = lineNo - baseLineNo.value
      return i >= 0 && i < lines.value.length ? i : -1
    }
    return linePosMap.get(lineNo) ?? -1
  }

  /**
   * 逻辑下标 → 原始行号（-1 = 越界）。
   *
   * 比例滚动的核心映射：滚动条上的第 `idx` 个槽对应哪一行。
   * - 全量态：行号连续，`idx + 1`。
   * - 筛选态（有合并查询结果）：行号不连续，查 `view.lines[idx]`。
   * - 筛选态但查询不可用：退化成按已加载顺序，`filteredLines[idx].index`。
   */
  function lineNoAt(idx: number): number {
    if (idx < 0) return -1
    if (filterActive.value) {
      if (viewActive.value && view.value) {
        const nav = view.value.lines
        return idx < nav.length ? nav[idx] : -1
      }
      return idx < filteredLines.value.length ? filteredLines.value[idx].index : -1
    }
    return idx + 1
  }

  /** 原始行号 → 逻辑下标（-1 = 不在当前视图里）。`lineNoAt` 的逆运算，跳转反算 scrollTop 用 */
  function indexOfLineNo(lineNo: number): number {
    if (!filterActive.value) return lineNo >= 1 ? lineNo - 1 : -1
    if (viewActive.value && view.value) return indexOfSorted(view.value.lines, lineNo)
    const arr = filteredLines.value
    for (let i = 0; i < arr.length; i++) if (arr[i].index === lineNo) return i
    return -1
  }

  /**
   * 逻辑下标区间 → 行内容；**尚未加载的位置返回 null**（渲染成等高空槽）。
   *
   * 这是比例滚动的必然结果：滚动条覆盖整个文件，但内存里只有 2 万行的窗口，
   * 视口落在窗口外时拿不到内容。返回 null 让渲染层画占位行而不是留白或报错。
   */
  function linesForRange(startIdx: number, endIdx: number): (ParsedLine | null)[] {
    const out: (ParsedLine | null)[] = []
    const filtered = filterActive.value
    for (let i = startIdx; i < endIdx; i++) {
      const lineNo = lineNoAt(i)
      if (lineNo < 0) {
        out.push(null)
        continue
      }
      const p = positionOfLine(lineNo)
      out.push(p >= 0 ? (filtered ? filteredLines.value[p] : lines.value[p]) : null)
    }
    return out
  }

  /**
   * 比例滚动的按需补载：确保以 `[startIdx, endIdx]` 中心为基准的一个窗口已加载。
   *
   * 与 `ensureLineLoaded`（跳转用）的差别在**居中**：跳转关心"目标行在不在"，
   * 滚动关心"视口这一屏有没有内容"，所以按视口中心搬窗口，且带迟滞
   * （中心离窗口两端还有 1/4 余量就不搬），否则滚到窗口边缘就会反复重搬、抖动。
   */
  async function ensureRangeLoaded(startIdx: number, endIdx: number): Promise<void> {
    const centerIdx = Math.floor((startIdx + endIdx) / 2)
    const lineNo = lineNoAt(centerIdx)
    if (lineNo <= 0) return

    if (filterActive.value) {
      if (positionOfLine(lineNo) >= 0) return
      // 筛选态只能按段向前推进（段一旦越过无法回头），沿用跳转那套补载
      await ensureFilteredLineLoaded(lineNo)
      return
    }

    const base = baseLineNo.value
    const end = base + lines.value.length - 1
    if (lines.value.length && lineNo >= base && lineNo <= end) {
      const margin = Math.max(1, Math.floor(lines.value.length / 4))
      if (lineNo >= base + margin && lineNo <= end - margin) return
    }
    if (loadingMore.value) return
    // 居中搬窗口：视口落在哪就在哪读一屏，而不是从文件头数过来
    const win = Math.min(WINDOW_MAX_LINES, SCROLL_WINDOW_LINES)
    const start = Math.max(1, lineNo - Math.floor(win / 2))
    loadingMore.value = true
    try {
      await loadWindowAt(start, win)
      snapshotBuffer(activeSourceId.value)
    } finally {
      loadingMore.value = false
    }
  }

  /** 增量补扫新增部分，保持命中数组按行号有序 */
  function scanIncremental(fromIdx: number) {
    if (!search.value.keyword) return
    const compiled = compileMatcher(search.value)
    if (!compiled.ok || !compiled.matcher) return
    const arr = displayLines.value
    const fresh = scanRange(arr, compiled.matcher, fromIdx, arr.length)
    if (fresh.length) {
      const merged = result.value.hits.concat(fresh).sort((a, b) => a.lineIndex - b.lineIndex)
      result.value = toSearchResult(merged)
    }
  }

  // ==================== 源管理 ====================
  async function persistSources() {
    try {
      await saveSources(
        sources.value.map(
          ({ id, kind, name, path, color, encoding: enc, loadedLines, totalLines, stream, search: srcSearch, filter: srcFilter }) => ({
            id,
            kind,
            name,
            path,
            color,
            encoding: enc,
            loadedLines,
            totalLines,
            // 每源独立记忆的搜索/筛选条件（切换 tab / 重新打开后恢复）
            search: srcSearch,
            filter: srcFilter,
            // 流源仅持久化描述，运行状态归位为 paused（不持久化 live/connecting/error 等瞬时态）
            ...(kind === 'stream'
              ? { stream, streamState: 'paused' as const, streamError: undefined }
              : {}),
          }),
        ),
        ws(),
      )
    } catch (e) {
      console.error('[logs] 保存日志源失败:', e)
    }
  }

  /** 清空筛选结果（切换源/关闭筛选时调用，条件本身保留） */
  function resetFiltered() {
    filteredLines.value = []
    segments.value = []
    segIndex.value = 0
    segPos.value = 0
    filterMatched.value = 0
    filterTruncated.value = false
    linePosMap = new Map()
  }

  /** 清空当前视图（切换源/工作区时调用） */
  function clearView() {
    lines.value = []
    baseLineNo.value = 1
    result.value = { hits: [], total: 0, hitLines: new Set<number>() }
    currentHitIndex.value = -1
    currentRangeIndex.value = 0
    currentLine.value = 0
    errorMsg.value = ''
    searchError.value = ''
    indexMeta.value = null
    filterError.value = ''
    resetView()
    resetFiltered()
  }

  async function openLocalFile(path: string, name?: string, kind: LogSource['kind'] = 'local') {
    // 每个源独立标记"正在读取"，便于在对应 tab 上显示加载指示器；
    // 复用缓存切换 tab 时不会进入此函数，故不会出现圈，符合预期。
    const sourceId = `${kind}:${path}`
    markLoading(sourceId)
    loading.value = true
    errorMsg.value = ''
    clearView()
    try {
      const data = await readChunk(path, 1)
      if (!data) {
        errorMsg.value = '读取文件失败'
        return null
      }
      lines.value = parseLines(data.content, data.first_line_number)
      baseLineNo.value = 1

      const fileName = name || path.split(/[\\/]/).pop() || path
      const existing = sources.value.find(s => s.id === sourceId)
      // 每源独立搜索/筛选：打开该源时载入其已记忆的条件（无则空搜索 + 默认筛选）
      search.value = existing?.search
        ? { ...existing.search }
        : { keyword: '', mode: 'text', ignoreCase: false }
      filter.value = existing?.filter ? { ...existing.filter } : { ...DEFAULT_FILTER }
      const source: LogSource = {
        id: sourceId,
        kind,
        name: fileName,
        path,
        // 重新打开已记录的源时保留其原有配色，避免每次切换/恢复都变颜色
        color: existing?.color ?? SOURCE_COLORS[sources.value.length % SOURCE_COLORS.length],
        encoding: encoding.value,
        loadedLines: lines.value.length,
        totalLines: data.total_lines || lines.value.length,
      }
      if (!existing) {
        sources.value = [...sources.value, source]
      } else {
        sources.value = sources.value.map(s => (s.id === source.id ? source : s))
      }
      activeSourceId.value = source.id
      // 编码选择器跟随当前源（重新打开已记录源时保留其原有编码）
      encoding.value = source.encoding
      await persistSources()

      // 索引统计用于筛选栏的级别计数与时间范围；失败不阻塞主流程
      void buildIndex()

      // 已有筛选条件时对新打开的源文件立即生效
      if (filterActive.value) await applyFilter()
      else await runSearch()
      // 缓存本次加载的文件视图内容，供切回该 tab / 重新聚焦时即时复用（不重读磁盘）
      snapshotBuffer(source.id)
      return source
    } catch (e) {
      errorMsg.value = (e as Error).message || '打开日志失败'
      return null
    } finally {
      unmarkLoading(sourceId)
      loading.value = false
    }
  }

  /**
   * 切换当前日志的查看编码并重新加载（重读文件 + 重建索引 + 重放筛选/搜索）。
   * 编码名需与后端 encoding_rs 支持范围一致（UTF-8 / GBK / GB18030 / UTF-16LE …）。
   */
  async function setEncoding(enc: string) {
    if (!enc) return
    const s = activeSource.value
    encoding.value = enc
    if (!s) return
    await openLocalFile(s.path, s.name, s.kind)
  }

  /**
   * 远程日志源：先经后端拉取远程文件到本地缓存，再按本地文件流程打开。
   * 这样远程日志自动复用分块加载 / 索引 / 筛选 / 时间轴 / 搜索等全部能力。
   */
  async function openRemoteFile(opts: {
    host: string
    port: number
    username: string
    password: string
    authType?: string
    privateKeyPath?: string
    remotePath: string
  }): Promise<LogSource | null> {
    loading.value = true
    errorMsg.value = ''
    try {
      const res = await call<ApiResp<{ localPath: string; size: number }>>('logs_pull_remote', {
        host: opts.host,
        port: opts.port,
        username: opts.username,
        password: opts.password,
        authType: opts.authType,
        privateKeyPath: opts.privateKeyPath,
        remotePath: opts.remotePath,
      })
      if (!res || res.code !== 200 || !res.data?.localPath) {
        errorMsg.value = res?.msg || '远程日志拉取失败'
        return null
      }
      const name = opts.remotePath.split(/[\\/]/).pop() || opts.remotePath
      return await openLocalFile(res.data.localPath, name, 'ssh')
    } catch (e) {
      errorMsg.value = (e as Error).message || '远程日志拉取失败'
      return null
    } finally {
      loading.value = false
    }
  }

  function setActiveSource(id: string) {
    const target = sources.value.find(s => s.id === id)
    if (!target) return
    // 实时流：active 且仍在线则仅切换视图；否则切到该源并自动恢复连接
    if (target.kind === 'stream') {
      if (activeSourceId.value === id && target.streamState === 'live') return
      showStreamSource(target)
      if (target.streamState !== 'live') void resumeStream(id)
      return
    }
    if (activeSourceId.value === id) return
    // 复用已缓存的文件内容，避免重复读取磁盘（tab 快速切换 / 重复打开同一文件）
    const buf = buffers.get(id)
    if (buf) {
      clearView()
      activeSourceId.value = id
      lines.value = buf.lines
      baseLineNo.value = buf.baseLineNo || 1
      target.loadedLines = buf.loadedLines
      target.totalLines = buf.totalLines
      // 同步编码选择器，使其反映该 tab 自身的编码
      encoding.value = target.encoding
      // 载入该 tab 自身记忆的搜索/筛选，使每个 tab 互不干扰
      loadSourceSearchFilter(target)
      // 恢复缓存的 indexMeta，否则 Timeline 会变空态；老 buffer 没缓存时（早期打开 / 预热失败）异步重建
      if (buf.indexMeta) {
        indexMeta.value = buf.indexMeta
      } else {
        void buildIndex()
      }
      if (filterActive.value) void applyFilter()
      else void runSearch()
      return
    }
    activeSourceId.value = id
    clearView()
    loadSourceSearchFilter(target)
    void openLocalFile(target.path, target.name)
  }

  /**
   * 打开本地 / SSH 日志：若同名源已打开则直接切换到其 tab（复用已加载内容，不重读磁盘）；
   * 否则新建并打开。用于 LogExplorer 双击 / 拖入等场景，确保"重复打开同一文件"只复用已有 tab。
   */
  function openLocalFileOrFocus(path: string, name?: string, kind: LogSource['kind'] = 'local') {
    const id = `${kind}:${path}`
    const existing = sources.value.find(s => s.id === id)
    if (existing) {
      setActiveSource(existing.id)
      return existing
    }
    return openLocalFile(path, name, kind)
  }

  /** 拖拽重排：把 fromId 移动到 toId 的位置，并重排后持久化 */
  function moveSource(fromId: string, toId: string) {
    if (fromId === toId) return
    const arr = [...sources.value]
    const from = arr.findIndex(s => s.id === fromId)
    const to = arr.findIndex(s => s.id === toId)
    if (from < 0 || to < 0) return
    const [item] = arr.splice(from, 1)
    arr.splice(to, 0, item)
    sources.value = arr
    void persistSources()
  }

  /** 移除单个源并做收尾（停流 / 清缓冲 / 清滚动记忆 / 删索引），不处理 active 切换 */
  function removeSource(id: string) {
    const s = sources.value.find(x => x.id === id)
    if (s?.kind === 'stream') {
      stopStream(id)
      streamBuffers.delete(id)
      streamLastSync.delete(id)
    }
    buffers.delete(id)
    scrollMap.delete(id)
    // 仅文件类源（本地/SSH）有后端索引可释放；流源 path 为 url/host:port，无需释放
    if (s && s.kind !== 'stream') void apiDropIndex(s.path)
    sources.value = sources.value.filter(x => x.id !== id)
  }

  /** 关闭后自动选中下一个源（优先复用已缓存内容） */
  async function selectAfterClose() {
    if (sources.value.some(s => s.id === activeSourceId.value)) return
    if (sources.value.length) await setActiveSource(sources.value[0].id)
    else {
      activeSourceId.value = ''
      clearView()
    }
  }

  async function closeSource(id: string) {
    const wasActive = activeSourceId.value === id
    removeSource(id)
    await persistSources()
    if (wasActive) await selectAfterClose()
  }

  /** 关闭除指定源以外的所有源 */
  async function closeOthers(id: string) {
    for (const s of [...sources.value]) {
      if (s.id !== id) removeSource(s.id)
    }
    await persistSources()
    await selectAfterClose()
  }

  /** 关闭指定源右侧的所有源（按当前顺序） */
  async function closeToRight(id: string) {
    const idx = sources.value.findIndex(s => s.id === id)
    if (idx < 0) return
    for (const s of sources.value.slice(idx + 1)) removeSource(s.id)
    await persistSources()
    await selectAfterClose()
  }

  /** 关闭所有源 */
  async function closeAll() {
    for (const s of [...sources.value]) removeSource(s.id)
    await persistSources()
    activeSourceId.value = ''
    clearView()
  }

  /** 重新加载指定源：文件类清空缓冲后重读磁盘（文件被外部改写/轮转后刷新）；流类重新连接 */
  async function reloadSource(id: string) {
    const s = sources.value.find(x => x.id === id)
    if (!s) return
    if (s.kind === 'stream') {
      if (s.streamState !== 'live') void resumeStream(id)
      else {
        // 已在线则先停再连，等价于重连
        pauseStream(id)
        void resumeStream(id)
      }
      return
    }
    buffers.delete(id)
    await openLocalFile(s.path, s.name, s.kind)
  }

  // ==================== 实时流（WebSocket / TCP） ====================

  /** 局部更新某个源（触发响应式） */
  function patchSource(id: string, patch: Partial<LogSource>) {
    sources.value = sources.value.map(s => (s.id === id ? { ...s, ...patch } : s))
  }

  /** 缓冲超上限：丢弃最旧一半并重排行号（同步修正仍在 displayLines 的命中） */
  function trimStreamBuffer(buf: ParsedLine[]) {
    const drop = Math.ceil(MAX_STREAM_LINES / 2)
    if (buf.length <= drop) return
    buf.splice(0, drop)
    for (let i = 0; i < buf.length; i++) buf[i].index = i + 1
    if (search.value.keyword) {
      result.value = toSearchResult(
        result.value.hits
          .filter(h => h.lineIndex > drop)
          .map(h => ({ ...h, lineIndex: h.lineIndex - drop })),
      )
    }
  }

  /** 收到一行实时日志：写入对应源缓冲，活跃源同步刷新视图 */
  function appendStreamLine(id: string, raw: string) {
    const buf = streamBuffers.get(id)
    if (!buf) return
    const parsed = parseLines(raw + '\n', buf.length + 1)
    buf.push(...parsed)
    if (buf.length > MAX_STREAM_LINES) trimStreamBuffer(buf)
    if (activeSourceId.value === id) {
      triggerRef(lines)
      const s = sources.value.find(x => x.id === id)
      if (s) {
        const last = streamLastSync.get(id) ?? 0
        // 节流：每累计 STREAM_SYNC_STEP 行才触发一次 sources 列表的响应式更新
        if (buf.length - last >= STREAM_SYNC_STEP) {
          patchSource(id, { loadedLines: buf.length, totalLines: buf.length })
          streamLastSync.set(id, buf.length)
        } else {
          s.loadedLines = buf.length
          s.totalLines = buf.length
        }
      }
      if (search.value.keyword) void runSearch()
    }
  }

  /** WebSocket：前端原生直连（避免新增后端依赖） */
  function startWs(source: LogSource) {
    const spec = source.stream
    const url = spec?.url || ''
    if (!url) {
      patchSource(source.id, { streamState: 'error', streamError: '缺少 WebSocket URL' })
      return
    }
    patchSource(source.id, { streamState: 'connecting', streamError: undefined })
    try {
      const ws = new WebSocket(url)
      streamSockets.set(source.id, ws)
      ws.onopen = () => patchSource(source.id, { streamState: 'live', streamError: undefined })
      ws.onmessage = (ev: MessageEvent) => {
        const text = typeof ev.data === 'string' ? ev.data : String(ev.data)
        appendStreamLine(source.id, text)
      }
      ws.onerror = () => patchSource(source.id, { streamState: 'error', streamError: 'WebSocket 连接错误' })
      ws.onclose = () => {
        // 主动暂停会先置 paused 再 close，避免误标 error
        const cur = sources.value.find(s => s.id === source.id)
        if (cur && cur.streamState !== 'paused') {
          patchSource(source.id, { streamState: 'error', streamError: 'WebSocket 已关闭' })
        }
        streamSockets.delete(source.id)
      }
    } catch (e) {
      patchSource(source.id, { streamState: 'error', streamError: (e as Error).message })
    }
  }

  /** TCP：后端监听，前端订阅事件 */
  async function startTcp(source: LogSource) {
    const spec = source.stream
    if (!spec?.host || !spec?.port) {
      patchSource(source.id, { streamState: 'error', streamError: '缺少 host / port' })
      return
    }
    patchSource(source.id, { streamState: 'connecting', streamError: undefined })
    // 先退订旧订阅，避免重复接收
    const old = streamUnsubs.get(source.id)
    if (old) {
      old()
      streamUnsubs.delete(source.id)
    }
    const unLine = await subscribe<{ streamId: string; line: string }>('logs-stream-line', p => {
      if (p.streamId === source.id) appendStreamLine(source.id, p.line)
    })
    const unStatus = await subscribe<{ streamId: string; status: string; message?: string }>(
      'logs-stream-status',
      p => {
        if (p.streamId !== source.id) return
        if (p.status === 'live') patchSource(source.id, { streamState: 'live', streamError: undefined })
        else if (p.status === 'closed') patchSource(source.id, { streamState: 'paused', streamError: p.message })
        else if (p.status === 'error') patchSource(source.id, { streamState: 'error', streamError: p.message })
      },
    )
    streamUnsubs.set(source.id, () => {
      unLine()
      unStatus()
    })
    const res = await call<ApiResp<string>>('logs_stream_start', {
      streamId: source.id,
      protocol: 'tcp',
      host: spec.host,
      port: spec.port,
      url: null,
    })
    if (!res || res.code !== 200) {
      patchSource(source.id, { streamState: 'error', streamError: res?.msg || '启动 TCP 流失败' })
    }
  }

  /** 停止单个流的连接（ws 关 socket / tcp 退订并通知后端） */
  function stopStream(id: string) {
    const sock = streamSockets.get(id)
    if (sock) {
      try {
        sock.close()
      } catch {
        /* 忽略关闭异常 */
      }
      streamSockets.delete(id)
    }
    const un = streamUnsubs.get(id)
    if (un) {
      un()
      streamUnsubs.delete(id)
    }
    void call('logs_stream_stop', { streamId: id }).catch(() => {})
  }

  /** 停止全部流（切换工作区 / 清理用） */
  function stopAllStreams() {
    for (const id of [...streamSockets.keys()]) stopStream(id)
    for (const id of [...streamUnsubs.keys()]) stopStream(id)
    streamBuffers.clear()
    streamLastSync.clear()
  }

  /** 暂停流（保留缓冲，可恢复） */
  function pauseStream(id: string) {
    const s = sources.value.find(x => x.id === id)
    stopStream(id)
    if (s) patchSource(id, { streamState: 'paused', streamError: undefined })
  }

  /** 恢复/启动流 */
  async function resumeStream(id: string) {
    const s = sources.value.find(x => x.id === id)
    if (!s || !s.stream) return
    if (s.stream.protocol === 'ws') startWs(s)
    else await startTcp(s)
  }

  /** 将某个流源作为当前活跃源展示（不自动连接） */
  function showStreamSource(target: LogSource) {
    if (activeSourceId.value === target.id) return
    activeSourceId.value = target.id
    clearView()
    const buf = streamBuffers.get(target.id) ?? []
    lines.value = buf
    target.loadedLines = buf.length
    target.totalLines = buf.length
    // 实时流不支持服务端筛选，确保回到全量视图
    if (filterActive.value) {
      filter.value = { ...DEFAULT_FILTER }
      resetFiltered()
    }
    void runSearch()
  }

  /**
   * 打开一个实时流源（WebSocket / TCP）。
   * - ws：前端原生直连
   * - tcp：后端监听并经事件推送
   */
  async function openStreamSource(opts: {
    name?: string
    protocol: 'ws' | 'tcp'
    url?: string
    host?: string
    port?: number
  }): Promise<LogSource | null> {
    const id = `stream:${opts.protocol}:${opts.url ?? `${opts.host}:${opts.port}`}`
    const existing = sources.value.find(s => s.id === id)
    const source: LogSource = {
      id,
      kind: 'stream',
      name: opts.name || (opts.protocol === 'ws' ? opts.url || id : `${opts.host}:${opts.port}`),
      path: opts.url ?? `${opts.host}:${opts.port}`,
      color: existing?.color ?? SOURCE_COLORS[sources.value.length % SOURCE_COLORS.length],
      encoding: 'UTF-8',
      loadedLines: 0,
      totalLines: 0,
      stream: { protocol: opts.protocol, url: opts.url, host: opts.host, port: opts.port },
      streamState: 'connecting',
    }
    if (!streamBuffers.has(id)) streamBuffers.set(id, [])
    if (!existing) sources.value = [...sources.value, source]
    else sources.value = sources.value.map(s => (s.id === source.id ? source : s))
    activeSourceId.value = source.id
    await persistSources()
    clearView()
    const buf = streamBuffers.get(id) ?? []
    lines.value = buf
    void runSearch()
    if (opts.protocol === 'ws') startWs(source)
    else await startTcp(source)
    return source
  }

  // ==================== 索引与筛选 ====================
  /** 建立/复用后端索引，拿到文件级统计（级别计数、时间范围、时间桶） */
  async function buildIndex() {
    const s = activeSource.value
    if (!s) return null
    indexLoading.value = true
    try {
      const meta = await apiBuildIndex(s.path, s.encoding)
      if (meta) {
        indexMeta.value = meta
        if (meta.total_lines) s.totalLines = meta.total_lines
        // 同步写入 buffer，切回 tab 时复用（避免重扫文件、Timeline 不会丢）
        const buf = buffers.get(s.id)
        if (buf) buf.indexMeta = meta
      }
      return meta
    } catch (e) {
      console.error('[logs] 建立日志索引失败:', e)
      return null
    } finally {
      indexLoading.value = false
    }
  }

  function advanceSegment() {
    segIndex.value += 1
    segPos.value = 0
  }

  /**
   * 按段加载筛选结果，直到新增 minLines 行**且 untilLine 已加载**（若指定）或读完所有段。
   *
   * `untilLine` 是跳转补载的停止条件：只看"新增行数"是不够的——目标行可能远远落在
   * 后面的段里，按行数加载会停在半路，跳转依旧打不通。
   *
   * 调用方**必须**先用 `segmentIndexOfLine` 确认目标行确实落在某个段内，
   * 否则这里会把上千个段全部读一遍才罢休。
   */
  async function loadMoreFiltered(minLines = CHUNK_SIZE, force = false, untilLine?: number) {
    const s = activeSource.value
    // applyFilter 自身已置 filterLoading=true 以阻断并发滚动加载，内部调用需 force 绕开
    if (!s || (filterLoading.value && !force)) return
    if (segIndex.value >= segments.value.length) return
    filterLoading.value = true
    try {
      let added = 0
      while (segIndex.value < segments.value.length) {
        if (added >= minLines && (untilLine == null || positionOfLine(untilLine) >= 0)) break
        const seg = segments.value[segIndex.value]
        const from = segPos.value || seg.start
        if (from > seg.end) {
          advanceSegment()
          continue
        }
        const want = Math.min(CHUNK_SIZE, seg.end - from + 1)
        const data = await readChunk(s.path, from, want)
        if (!data || !data.content) {
          advanceSegment()
          continue
        }
        // read_file_lines 可能多读，裁掉段外的行
        const parsed = parseLines(data.content, data.first_line_number).filter(l => l.index <= seg.end)
        if (!parsed.length) {
          advanceSegment()
          continue
        }
        const before = filteredLines.value.length
        appendLines(parsed)
        added += parsed.length
        scanIncremental(before)

        const nextPos = parsed[parsed.length - 1].index + 1
        if (nextPos > seg.end || parsed.length < want) advanceSegment()
        else segPos.value = nextPos
      }
    } catch (e) {
      const msg = (e as Error)?.message || '加载筛选结果失败'
      console.error('[logs] 加载筛选结果失败:', e)
      filterError.value = msg
    } finally {
      filterLoading.value = false
    }
  }

  /** 应用筛选条件（省略 patch 时按当前条件重新查询） */
  async function applyFilter(patch?: Partial<FilterState>) {
    const s = activeSource.value
    if (patch) filter.value = { ...filter.value, ...patch }

    if (!s) return
    if (s.kind === 'stream') {
      // 实时流不支持服务端索引筛选，直接回到全量视图（搜索仍可用）
      clearFilter()
      return
    }
    if (!isFilterActive(filter.value)) {
      clearFilter()
      return
    }
    filterLoading.value = true
    filterError.value = ''
    try {
      // 筛选条件变了 → 必须重新按段取内容
      await rebuildView({ reloadContent: true })
    } catch (e) {
      filterError.value = (e as Error).message || '筛选失败'
    } finally {
      filterLoading.value = false
    }
  }

  /** 清除筛选条件，回到全量视图（行数据已加载的部分直接可见） */
  function clearFilter() {
    filter.value = { ...DEFAULT_FILTER }
    resetFiltered()
    scanLocalHighlight()
    // 全量态若仍有关键词，仍需一次查询补全"命中全集"
    if (search.value.keyword) scheduleRebuild()
    else resetView()
  }

  // ==================== 导出 ====================
  /**
   * 导出日志到目标文件。
   * 全程在后端流式完成：大文件不会在前端拼装字符串，也不要求先加载全部内容。
   */
  async function exportLogs(opts: {
    destPath: string
    /** filter = 按当前筛选条件；all = 导出整个文件 */
    scope: 'filter' | 'all'
    withLineNumber: boolean
    withBom: boolean
  }): Promise<LogExportResult> {
    const s = activeSource.value
    if (!s) throw new Error('没有打开的日志源')
    exporting.value = true
    try {
      return await apiExportLines({
        path: s.path,
        encoding: s.encoding,
        destPath: opts.destPath,
        filter: opts.scope === 'filter' ? filter.value : { ...DEFAULT_FILTER },
        withLineNumber: opts.withLineNumber,
        withBom: opts.withBom,
      })
    } finally {
      exporting.value = false
    }
  }

  // ==================== 搜索与筛选的统一驱动（P4） ====================
  /** 段数上限（与后端 DEFAULT_MAX_SEGMENTS 对齐） */
  const MAX_SEGMENTS = 2000
  /** 单文件最多取回多少条命中行号（与后端 DEFAULT_MAX_MATCHES 对齐） */
  const MAX_MATCH_LINES = 50000
  /** 防抖：输入过程中每敲一个字符都查一遍代价太高 */
  const VIEW_DELAY = 250

  /**
   * 本地扫描：只负责**当前行的高亮区间**，不再参与导航（§3.3）。
   *
   * 导航一律走 `view.lines`（服务端全量口径）；本地命中仅用于把已渲染行里的
   * 关键词标出来，因此它可以是"部分"的——这正是它能保持实时的原因。
   */
  function scanLocalHighlight() {
    const opts = search.value
    if (!opts.keyword) {
      result.value = { hits: [], total: 0, hitLines: new Set<number>() }
      currentHitIndex.value = -1
      currentRangeIndex.value = 0
      searchError.value = ''
      return
    }
    const compiled = compileMatcher(opts)
    if (!compiled.ok || !compiled.matcher) {
      searchError.value = compiled.error || '搜索条件无效'
      result.value = { hits: [], total: 0, hitLines: new Set<number>() }
      currentHitIndex.value = -1
      return
    }
    searchError.value = ''
    result.value = toSearchResult(scanAll(displayLines.value, compiled.matcher))
    currentHitIndex.value = -1
    currentRangeIndex.value = 0
  }

  /** 清空合并查询结果（切换源 / 清空条件时调用） */
  function resetView() {
    if (viewTimer) {
      window.clearTimeout(viewTimer)
      viewTimer = null
    }
    viewToken++
    view.value = null
    viewActive.value = false
    viewNote.value = ''
    viewScanning.value = false
    unindexedLines.value = 0
  }

  /** 防抖触发一次合并查询 */
  function scheduleRebuild() {
    if (viewTimer) window.clearTimeout(viewTimer)
    viewTimer = window.setTimeout(() => {
      viewTimer = null
      void rebuildView({ reloadContent: false })
    }, VIEW_DELAY)
  }

  /**
   * 单一驱动入口：**filter 或 search 任一变化都走这里，一次请求同时落地
   * 渲染段与导航行号**（§3.3）。
   *
   * 这是本次改造的核心：旧实现里 `applyFilter` 与 `runFullSearch` 各发一次请求、
   * 各维护一份状态，两条链路互不知情——筛选态下搜索甚至直接被禁用，
   * 于是"下一个匹配"永远到不了未加载段里的命中。
   *
   * `reloadContent`：只有**筛选条件**变化时才需要重新按段取内容；
   * 仅关键词变化时段不变、已加载行也不变，重新加载只会造成无谓的跳动与 IO。
   */
  async function rebuildView(opts: { reloadContent?: boolean } = {}) {
    const token = ++viewToken
    const s = activeSource.value
    if (!s || s.kind === 'stream') {
      // 实时流无文件、无索引，全程走本地路径
      resetView()
      scanLocalHighlight()
      return
    }
    const kw = search.value.keyword
    const hasFilter = isFilterActive(filter.value)
    if (!hasFilter && !kw) {
      resetView()
      scanLocalHighlight()
      return
    }
    viewScanning.value = true
    viewNote.value = ''
    try {
      const res = await apiQueryLines({
        path: s.path,
        encoding: s.encoding || encoding.value,
        filter: hasFilter ? filter.value : null,
        search: kw
          ? { keyword: kw, mode: search.value.mode, ignoreCase: search.value.ignoreCase }
          : null,
        maxSegments: MAX_SEGMENTS,
        maxMatches: MAX_MATCH_LINES,
        taskId: `logs-${token}`,
      })
      if (token !== viewToken) return // 已有更新的请求，丢弃本次结果
      view.value = res
      viewActive.value = true
      unindexedLines.value = res.unindexed_lines
      if (res.matches_truncated) {
        viewNote.value = `命中过多，仅保留前 ${res.lines.length} 条导航`
      } else if (res.unindexed_lines > 0) {
        viewNote.value = `文件过大，末尾 ${res.unindexed_lines} 行未参与筛选`
      }
      if (hasFilter) {
        segments.value = res.segments
        filterMatched.value = res.filter_matched
        filterTruncated.value = res.truncated
        if (opts.reloadContent !== false) {
          segIndex.value = 0
          segPos.value = 0
          resetFiltered()
          await loadMoreFiltered(CHUNK_SIZE, true)
        }
      } else {
        resetFiltered()
      }
      // 内容变了要重扫高亮；仅关键词变化时也要扫（displayLines 未变但 result 需刷新）
      scanLocalHighlight()
    } catch (e) {
      if (token !== viewToken) return
      // 典型：正则语法连备用引擎都不支持 → 降级为"仅已加载"，并给出可见原因
      view.value = null
      viewActive.value = false
      const msg = (e as Error)?.message
      viewNote.value = msg ? `查询失败：${msg}` : '查询失败，仅搜索已加载部分'
      console.warn('[logs] 合并查询失败，降级为本地扫描:', e)
    } finally {
      if (token === viewToken) viewScanning.value = false
    }
  }

  /** 关键词变化：先本地扫描保证高亮即时生效，再防抖补全"命中全集" */
  function runSearch() {
    scanLocalHighlight()
    if (!search.value.keyword) {
      resetView()
      return
    }
    scheduleRebuild()
  }

  /** 等待正在进行的筛选加载结束（导航撞上滚动补载时，直接放弃就是"点了没反应"） */
  async function waitFilterIdle(timeoutMs = 3000) {
    const t0 = Date.now()
    while (filterLoading.value && Date.now() - t0 < timeoutMs) {
      await new Promise(r => setTimeout(r, 20))
    }
  }

  /** 跳转结果：`ok` = 已就位，其余都是**必须提示给用户**的失败原因 */
  type EnsureResult = 'ok' | 'not-in-filter' | 'truncated' | 'passed' | 'failed'

  const ENSURE_HINT: Record<EnsureResult, string> = {
    ok: '',
    'not-in-filter': '该行不在当前筛选结果中',
    truncated: '筛选结果过多已被截断，无法定位到该行',
    passed: '该行所在区段已跳过，请重新应用筛选',
    failed: '定位该行失败，请稍后重试',
  }

  /**
   * 筛选态：把筛选结果加载到**包含 lineNo** 为止。
   *
   * 与全量态的根本差别是行号不连续——不能按行号续读，只能按段推进；
   * 且段一旦越过就无法回头（`filteredLines` 只向前增长），所以必须先做段级定位。
   */
  async function ensureFilteredLineLoaded(lineNo: number): Promise<EnsureResult> {
    if (positionOfLine(lineNo) >= 0) return 'ok'
    const seg = segmentIndexOfLine(segments.value, lineNo)
    if (seg < 0) {
      // 段被 maxSegments 截断时，靠后的命中可能根本不在段列表里——这**不等于**没命中
      return filterTruncated.value ? 'truncated' : 'not-in-filter'
    }
    if (seg < segIndex.value) {
      // 该段已经读过却不含这一行，属状态不一致；不猜，直接暴露出来
      return 'passed'
    }
    await waitFilterIdle()
    await loadMoreFiltered(CHUNK_SIZE, false, lineNo)
    return positionOfLine(lineNo) >= 0 ? 'ok' : 'failed'
  }

  /**
   * 确保目标行已加载，返回"是否可用于跳转"。
   *
   * - 全量态：从当前末尾续读（lines 始终从第 1 行连续，行号 → 下标仍是 lineNo - 1），
   *   补载用更大的块，避免跳到文件尾部时发几十次请求。
   * - 筛选态：按段推进到目标行所在的段（见 `ensureFilteredLineLoaded`）。
   *
   * 失败时写入 `jumpHint`：原先只是静默 `return`，用户看到的就是"点了没反应"，
   * 无法区分"没有命中"与"功能坏了"。
   */
  async function ensureLineLoaded(lineNo: number): Promise<boolean> {
    jumpHint.value = ''
    const s = activeSource.value
    if (!s || lineNo <= 0) return false

    if (filterActive.value) {
      const r = await ensureFilteredLineLoaded(lineNo)
      if (r !== 'ok') jumpHint.value = ENSURE_HINT[r]
      return r === 'ok'
    }

    const base = baseLineNo.value
    const endLine = base + lines.value.length - 1
    if (lineNo >= base && lineNo <= endLine) return true
    if (loadingMore.value) {
      jumpHint.value = '正在加载，请稍后重试'
      return false
    }
    loadingMore.value = true
    try {
      // 目标在窗口之前，或远在窗口之后（距离超过窗口容量）→ 直接搬窗口。
      // 逐块续读会为了一个目标行把整段都加载进来，反而破坏窗口。
      if (lineNo < base || lineNo - endLine > WINDOW_MAX_LINES) {
        const start = Math.max(1, lineNo - Math.floor(CHUNK_SIZE / 2))
        await loadWindowAt(start, Math.min(WINDOW_MAX_LINES, JUMP_CHUNK_SIZE))
      } else {
        while (baseLineNo.value + lines.value.length - 1 < lineNo) {
          const startLine = baseLineNo.value + lines.value.length
          const want = Math.min(JUMP_CHUNK_SIZE, lineNo - startLine + 1)
          const data = await readChunk(s.path, startLine, want)
          if (!data || !data.content) break
          const parsed = parseLines(data.content, data.first_line_number)
          if (!parsed.length) break
          const before = lines.value.length
          appendLines(parsed)
          s.loadedLines = baseLineNo.value + lines.value.length - 1
          if (data.total_lines) s.totalLines = data.total_lines
          scanIncremental(before)
        }
      }
      snapshotBuffer(activeSourceId.value)
    } catch (e) {
      console.error('[logs] 补载目标行失败:', e)
      jumpHint.value = '加载目标行失败'
    } finally {
      loadingMore.value = false
    }
    return positionOfLine(lineNo) >= 0
  }

  /**
   * 反向定位：在**完整日志**中定位该行（退出筛选态后跳到同一原始行号）。
   *
   * 行号恒为原始文件行号，所以反向定位不需要任何"反查映射"——
   * 退出筛选后按同一个行号走全量态补载即可。这是 §1 不变量带来的直接好处。
   */
  async function revealInFullLog(lineNo: number): Promise<boolean> {
    if (lineNo <= 0) return false
    if (filterActive.value) {
      // 退出筛选：displayLines 随即切回全量 lines，行号恢复连续
      clearFilter()
    }
    const ok = await ensureLineLoaded(lineNo)
    if (ok) currentLine.value = lineNo
    return ok
  }

  function setSearch(patch: Partial<SearchOptions>) {
    search.value = { ...search.value, ...patch }
    void runSearch()
  }

  /**
   * 校准"当前命中下标"。
   *
   * 补载新行后 `navLines` 会增长（新命中可能插在当前位置之前），上次记住的下标就会
   * 指向另一条命中，直接 ±1 会跳错。因此每次导航前先按**行号**把下标校准回去
   * ——行号是不变量，下标不是。
   */
  function calibratedHitIndex(nav: number[]): number {
    const i = currentHitIndex.value
    if (i >= 0 && i < nav.length && nav[i] === currentLine.value) return i
    if (currentLine.value > 0) return indexOfSorted(nav, currentLine.value)
    return -1
  }

  /** 跳转到下一个/上一个命中，返回目标行号（无命中返回 null） */
  function gotoNext(): number | null {
    const nav = navLines.value
    if (!nav.length) return null
    let i = calibratedHitIndex(nav)
    if (i < 0) i = firstIndexAfter(nav, currentLine.value)
    else i = i + 1
    if (i < 0 || i >= nav.length) i = 0 // 循环回到首个
    currentHitIndex.value = i
    currentRangeIndex.value = 0
    currentLine.value = nav[i]
    return nav[i]
  }

  /** 跳转到上一个命中，返回目标行号（无命中返回 null） */
  function gotoPrev(): number | null {
    const nav = navLines.value
    if (!nav.length) return null
    let i = calibratedHitIndex(nav)
    if (i < 0) i = lastIndexBefore(nav, currentLine.value)
    else i = i - 1
    if (i < 0) i = nav.length - 1 // 循环到末尾
    currentHitIndex.value = i
    currentRangeIndex.value = 0
    currentLine.value = nav[i]
    return nav[i]
  }

  /** 跳转到第 n 个命中（n 为 1-based，来自输入框） */
  function gotoMatch(n: number): number | null {
    const nav = navLines.value
    if (!nav.length) return null
    const idx = Math.min(Math.max(Math.round(n) - 1, 0), nav.length - 1)
    currentHitIndex.value = idx
    currentRangeIndex.value = 0
    currentLine.value = nav[idx]
    return nav[idx]
  }

  /** 当前命中序号（1-based，用于导航显示） */
  const currentMatchNo = computed(() => currentHitIndex.value + 1)

  // ==================== 高亮规则 ====================
  async function persistRules() {
    try {
      await saveRules(rules.value, ws())
    } catch (e) {
      console.error('[logs] 保存高亮规则失败:', e)
    }
  }

  /** 保存"当前激活源"（一键切换后恢复用） */
  async function persistActive() {
    try {
      await saveActiveSource(activeSourceId.value, ws())
    } catch (e) {
      console.error('[logs] 保存激活日志源失败:', e)
    }
  }

  /** 批量保存某工作区的全部日志状态（切换离场时调用） */
  async function persistAll(targetWs: string) {
    try {
      await Promise.all([
        saveRules(rules.value, targetWs),
        saveSources(
          sources.value.map(
            ({ id, kind, name, path, color, encoding: enc, loadedLines, totalLines, search: srcSearch, filter: srcFilter }) => ({
              id,
              kind,
              name,
              path,
              color,
              encoding: enc,
              loadedLines,
              totalLines,
              // 每源独立记忆的搜索/筛选条件一并落盘，切换工作区不丢失
              search: srcSearch,
              filter: srcFilter,
            }),
          ),
          targetWs,
        ),
        saveActiveSource(activeSourceId.value, targetWs),
      ])
    } catch (e) {
      console.error('[logs] 保存日志工作区状态失败:', e)
    }
  }

  function addRule(rule: Omit<HighlightRule, 'id'>) {
    rules.value = [
      ...rules.value,
      { ...rule, id: `r_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}` },
    ]
    void persistRules()
  }

  function updateRule(id: string, patch: Partial<HighlightRule>) {
    rules.value = rules.value.map(r => (r.id === id ? { ...r, ...patch } : r))
    void persistRules()
  }

  function removeRule(id: string) {
    rules.value = rules.value.filter(r => r.id !== id)
    void persistRules()
  }

  function moveRule(id: string, delta: number) {
    const idx = rules.value.findIndex(r => r.id === id)
    if (idx < 0) return
    const next = idx + delta
    if (next < 0 || next >= rules.value.length) return
    const copy = [...rules.value]
    const [item] = copy.splice(idx, 1)
    copy.splice(next, 0, item)
    rules.value = copy
    void persistRules()
  }

  // ==================== 初始化 ====================
  /** 载入指定工作区的日志状态，并自动恢复其激活源 */
  async function loadWorkspaceState(targetWs: string, autoOpen = true) {
    const [r, s, a] = await Promise.all([
      loadRules(targetWs),
      loadSources(targetWs),
      loadActiveSource(targetWs),
    ])
    rules.value = r
    sources.value = s
    const validActive = a && s.some(x => x.id === a) ? a : s[0]?.id ?? ''
    activeSourceId.value = validActive
    if (autoOpen && validActive) {
      const active = s.find(x => x.id === validActive)
      if (!active) {
        /* 无激活源 */
      } else if (active.kind === 'local' || active.kind === 'ssh') {
        await openLocalFile(active.path, active.name, active.kind)
      } else if (active.kind === 'stream') {
        // 实时流恢复为 paused（不自动发起连接），由用户点击继续
        active.streamState = 'paused'
        showStreamSource(active)
      }
    }
  }

  /** 工作区切换事件：保存旧工作区 → 清空视图 → 加载新工作区 */
  async function onWorkspaceChanged(newWsId: string) {
    const oldWsId = currentWsId.value
    // 离场工作区的流连接先全部停止并清空缓冲，避免后台继续写入旧缓冲
    stopAllStreams()
    if (oldWsId && oldWsId !== newWsId) {
      await persistAll(oldWsId)
    }
    clearView()
    try {
      currentWsId.value = newWsId
      await loadWorkspaceState(newWsId, true)
    } catch (e) {
      console.error('[logs] 切换工作区日志状态失败:', e)
    }
  }

  /** 安装工作区切换监听（store 单例，仅一次） */
  function setupWorkspaceWatch() {
    if (watchInstalled) return
    watchInstalled = true
    window.addEventListener('workspace-changed', (e: Event) => {
      const detail = (e as CustomEvent<{ workspaceId: string }>).detail
      if (detail?.workspaceId) void onWorkspaceChanged(detail.workspaceId)
    })
  }

  async function init() {
    if (ready.value) return
    currentWsId.value = getCurrentWorkspaceId()
    try {
      await loadWorkspaceState(ws(), true)
    } catch (e) {
      console.error('[logs] 日志工作台初始化失败:', e)
    } finally {
      ready.value = true
      setupWorkspaceWatch()
    }
  }

  return {
    // state
    sources,
    activeSourceId,
    lines,
    baseLineNo,
    loading,
    loadingMore,
    errorMsg,
    encoding,
    rules,
    search,
    searchError,
    result,
    currentHitIndex,
    currentRangeIndex,
    currentLine,
    viewScanning,
    viewActive,
    viewNote,
    unindexedLines,
    jumpHint,
    filter,
    indexMeta,
    indexLoading,
    filterLoading,
    filterError,
    filterMatched,
    filterTruncated,
    filteredLines,
    segments,
    segIndex,
    segPos,
    exporting,
    loadingSourceIds,
    isSourceLoading,
    // computed
    activeSource,
    loadedCount,
    logicalTotal,
    currentHit,
    currentMatchNo,
    filterActive,
    displayLines,
    navLines,
    navTotal,
    // actions
    init,
    rebuildView,
    openLocalFile,
    openLocalFileOrFocus,
    openRemoteFile,
    setEncoding,
    setScroll,
    getScroll,
    openStreamSource,
    setActiveSource,
    closeSource,
    closeOthers,
    closeToRight,
    closeAll,
    reloadSource,
    moveSource,
    pauseStream,
    resumeStream,
    clearView,
    buildIndex,
    applyFilter,
    clearFilter,
    loadMoreFiltered,
    exportLogs,
    positionOfLine,
    lineNoAt,
    indexOfLineNo,
    linesForRange,
    ensureRangeLoaded,
    runSearch,
    setSearch,
    gotoNext,
    gotoPrev,
    gotoMatch,
    ensureLineLoaded,
    revealInFullLog,
    addRule,
    updateRule,
    removeRule,
    moveRule,
  }
})
