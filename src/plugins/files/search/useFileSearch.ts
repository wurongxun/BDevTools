/**
 * 跨文件搜索面板的共享状态（模块级单例，与 useCommandPalette 同范式）。
 *
 * 长任务取消双保险：① 后端 `files_search_cancel` 置标志；② 前端 token 丢弃过期结果，
 * 避免慢查询回灌覆盖新结果（logs 插件的 viewToken 范式）。
 */
import { computed, ref, watch } from 'vue'
import NotificationUtil from '@/utils/NotificationUtil'
import { load, open, type Store } from '@/core/runtime'
import {
  cancelSearch,
  replaceInFiles,
  searchInFiles,
  toReplaceTargets,
  type FileHits,
  type ReplaceOutcome,
  type ReplaceTarget,
  type SearchMode
} from './fileSearchApi'
import {
  canUndo,
  initUndoHistory,
  pushUndoBatch,
  refreshOpenDocuments,
  restoreUndoBatch,
  undoBatches
} from './useUndoHistory'

/** 侧栏视图开关。命令面板 / Ctrl+Shift+F 都通过 openSearchPanel() 拉起。 */
export const searchPanelVisible = ref(false)
export function openSearchPanel() {
  searchPanelVisible.value = true
}

const rootPath = ref('')
/** 用户显式选的搜索根（覆盖工作区目录，持久化到 settings.json） */
const rootOverride = ref('')
const rootSource = computed<'workspace' | 'custom'>(() => (rootOverride.value ? 'custom' : 'workspace'))
const keyword = ref('')
const replacement = ref('')
const mode = ref<SearchMode>('text')
const ignoreCase = ref(true)
const include = ref('')
const exclude = ref('')

const results = ref<FileHits[]>([])
const totalMatches = ref(0)
const filesScanned = ref(0)
const filesSkipped = ref(0)
const truncated = ref(false)
const errorMsg = ref('')
const searching = ref(false)
const replacing = ref(false)
const preview = ref<ReplaceOutcome | null>(null)
const selectedPaths = ref<Set<string>>(new Set())

// ===== 搜索根持久化（M4：M3 遗留「根绑 workingDirectory、空则不可用」） =====
const ROOT_KEY = 'files_search_root'
let rootReady = false
let rootInitPromise: Promise<void> | null = null

function initSearchRoot(): Promise<void> {
  if (rootInitPromise) return rootInitPromise
  rootInitPromise = (async () => {
    if (rootReady) return
    rootReady = true
    try {
      const s = await load('settings.json', { autoSave: false, defaults: {} })
      const saved = await s.get<{ value: string }>(ROOT_KEY)
      if (typeof saved?.value === 'string' && saved.value) {
        rootOverride.value = saved.value
        rootPath.value = saved.value
      }
    } catch (e) {
      console.warn('[files/search] 读取自定义搜索根失败', e)
    }
  })()
  return rootInitPromise
}

/** 选择任意目录作为搜索根；取消选择时保持现状。 */
async function chooseRoot(): Promise<boolean> {
  try {
    const picked = await open({ directory: true, multiple: false, title: '选择搜索根目录' })
    if (!picked || typeof picked !== 'string') return false
    await initSearchRoot()
    rootOverride.value = picked
    rootPath.value = picked
    const s = await load('settings.json', { autoSave: false, defaults: {} })
    await s.set(ROOT_KEY, { value: picked })
    clearResults()
    NotificationUtil.info(`搜索根已设为 ${picked}`)
    return true
  } catch (e: any) {
    NotificationUtil.error(e?.message || '选择目录失败')
    return false
  }
}

/** 回到「跟随工作区目录」模式，并清掉落盘的覆盖值。 */
async function resetRoot(workspaceRoot = ''): Promise<void> {
  // 必须先等在飞的读盘回填完成，否则它随后写入的旧覆盖值会把本次重置覆盖掉
  await initSearchRoot()
  rootOverride.value = ''
  rootPath.value = workspaceRoot
  try {
    const s = await load('settings.json', { autoSave: false, defaults: {} })
    await s.set(ROOT_KEY, { value: '' })
  } catch (e) {
    console.warn('[files/search] 清除自定义搜索根失败', e)
  }
  clearResults()
}

// ===== 查找历史（C8）：settings.json / files_search_history，最多 20 条 =====
const HISTORY_KEY = 'files_search_history'
const HISTORY_LIMIT = 20
const history = ref<string[]>([])
let historyHandle: Promise<Store> | null = null
let historyReady = false
let historyTimer: ReturnType<typeof setTimeout> | null = null

function historyStore(): Promise<Store> {
  if (!historyHandle) {
    historyHandle = load('settings.json', { autoSave: false, defaults: {} }).catch((e: unknown) => {
      historyHandle = null
      throw e
    })
  }
  return historyHandle
}

/** 懒初始化：面板首次被用到才读盘，避免插件加载时就发 RPC。 */
async function initSearchHistory() {
  if (historyReady) return
  historyReady = true
  try {
    const s = await historyStore()
    const saved = await s.get<{ value: string[] }>(HISTORY_KEY)
    if (Array.isArray(saved?.value)) {
      history.value = saved.value.filter((v) => typeof v === 'string' && v).slice(0, HISTORY_LIMIT)
    }
  } catch (e) {
    console.warn('[files/search] 读取查找历史失败', e)
  }
  watch(history, () => {
    if (historyTimer) clearTimeout(historyTimer)
    historyTimer = setTimeout(async () => {
      historyTimer = null
      try {
        const s = await historyStore()
        await s.set(HISTORY_KEY, { value: history.value.slice(0, HISTORY_LIMIT) })
      } catch (e) {
        console.warn('[files/search] 保存查找历史失败', e)
      }
    }, 300)
  })
}

function pushHistory(value: string) {
  const v = value.trim()
  if (!v) return
  history.value = [v, ...history.value.filter((item) => item !== v)].slice(0, HISTORY_LIMIT)
}

/** 每一个长任务都用递增 token 标记，回包时只认最新的那个。 */
let taskSeq = 0
let activeTaskId = ''
const busy = computed(() => searching.value || replacing.value)
const hasResults = computed(() => results.value.length > 0)
/** 面板上是一个「正则」勾选框，内部仍用 text/regex 两种模式，避免两套真源。 */
const useRegex = computed({
  get: () => mode.value === 'regex',
  set: (v: boolean) => {
    mode.value = v ? 'regex' : 'text'
  }
})

function newTaskId(prefix: string): string {
  const id = `${prefix}-${Date.now()}-${++taskSeq}`
  activeTaskId = id
  return id
}

/** 取消在飞任务；不抛错，返回是否命中。 */
async function requestCancel(): Promise<boolean> {
  if (!activeTaskId) return false
  const id = activeTaskId
  activeTaskId = ''
  taskSeq++ // 让所有在飞回包立刻过期
  try {
    const res = await cancelSearch(id)
    return res?.code === 200 && res.data === true
  } catch (e) {
    console.warn('[files/search] 取消任务失败（旧任务可能已结束）', e)
    return false
  }
}

async function cancel() {
  if (!busy.value) return
  await requestCancel()
  searching.value = false
  replacing.value = false
  errorMsg.value = '已取消'
}

/**
 * 工作区目录同步入口。用户显式选过自定义根时不再被工作区切换覆盖；
 * 首次调用会异步读回落盘的自定义根（避免插件加载阶段就发 RPC）。
 */
async function setRoot(path: string): Promise<void> {
  // 等读盘回填完成再判断：否则「已保存的自定义根」会被工作区目录短暂覆盖，
  // 用户在这几百毫秒里点查找就会搜错目录。
  await initSearchRoot()
  if (rootOverride.value) return
  if (path && path !== rootPath.value) {
    rootPath.value = path
    clearResults()
  }
}

function commonParams() {
  return {
    keyword: keyword.value,
    replacement: replacement.value,
    mode: mode.value,
    ignoreCase: ignoreCase.value
  }
}

/** 勾选为空时默认对全部结果生效。 */
function effectiveTargets(): FileHits[] {
  if (selectedPaths.value.size === 0) return results.value
  return results.value.filter((f) => selectedPaths.value.has(f.path))
}

async function search() {
  if (!keyword.value) {
    NotificationUtil.warn('请输入搜索关键词')
    return
  }
  if (!rootPath.value) {
    NotificationUtil.warn('没有搜索根目录，请打开工作区或点「选目录」指定搜索范围')
    return
  }
  if (activeTaskId) await requestCancel()
  const token = ++taskSeq
  const taskId = newTaskId('files-search')
  searching.value = true
  errorMsg.value = ''
  preview.value = null
  try {
    const res = await searchInFiles({
      ...commonParams(),
      rootPath: rootPath.value,
      include: include.value,
      exclude: exclude.value,
      taskId
    })
    if (token !== taskSeq) return // 已有更新的任务
    if (res?.code !== 200) {
      errorMsg.value = res?.msg || '搜索失败'
      NotificationUtil.error(errorMsg.value)
      results.value = []
      return
    }
    const data = res.data
    pushHistory(keyword.value)
    results.value = data.files
    totalMatches.value = data.total_matches
    filesScanned.value = data.files_scanned
    filesSkipped.value = data.files_skipped
    truncated.value = data.truncated
    selectedPaths.value = new Set()
    if (data.cancelled) errorMsg.value = '搜索已取消，只返回部分结果'
    else if (data.truncated) errorMsg.value = '命中过多已截断，请缩小范围或加上 glob 过滤'
  } catch (e: any) {
    if (token !== taskSeq) return
    errorMsg.value = e?.message || '搜索异常'
    NotificationUtil.error(errorMsg.value)
  } finally {
    if (token === taskSeq) {
      searching.value = false
      activeTaskId = ''
    }
  }
}

/** 预览：后端 dry_run=true，绝不落盘。返回是否成功拿到预览。 */
async function previewReplace(): Promise<boolean> {
  const files = effectiveTargets()
  if (!replacement.value && keyword.value === '') {
    NotificationUtil.warn('请填写替换内容')
    return false
  }
  if (files.length === 0) {
    NotificationUtil.warn('没有可替换的文件，请先搜索')
    return false
  }
  const taskId = newTaskId('files-replace-preview')
  replacing.value = true
  try {
    const res = await replaceInFiles({
      ...commonParams(),
      targets: toReplaceTargets(files),
      dryRun: true,
      taskId
    })
    if (res?.code !== 200) {
      NotificationUtil.error(res?.msg || '替换预览失败')
      return false
    }
    preview.value = res.data
    return true
  } catch (e: any) {
    NotificationUtil.error(e?.message || '替换预览异常')
    return false
  } finally {
    replacing.value = false
    activeTaskId = ''
  }
}

/** 应用：逐文件备份 + 原子写 + 409 检测。成功后记下 undo 条目。 */
async function applyReplace(): Promise<boolean> {
  const files = effectiveTargets()
  if (files.length === 0) {
    NotificationUtil.warn('没有可替换的文件')
    return false
  }
  const taskId = newTaskId('files-replace-apply')
  replacing.value = true
  try {
    const res = await replaceInFiles({
      ...commonParams(),
      targets: toReplaceTargets(files),
      dryRun: false,
      taskId
    })
    if (res?.code !== 200) {
      NotificationUtil.error(res?.msg || '批量替换失败')
      return false
    }
    const data = res.data
    // 落盘撤销清单（M4）：重启后仍能恢复，批次上限与失效探测由 useUndoHistory 负责
    await pushUndoBatch('批量替换', data.backups)
    preview.value = data
    // 磁盘已变：把本编辑器里打开的干净 tab 同步过来，脏 tab 只标记冲突（见函数注释）
    void refreshOpenDocuments(data.items.filter((i) => i.code === 200).map((i) => i.path))
    const failed = data.items.filter((i) => i.code !== 200 && i.msg !== '无匹配')
    if (failed.length > 0) {
      NotificationUtil.warn(
        `已替换 ${data.total_replacements} 处 / ${data.files_changed} 个文件；${failed.length} 个文件被跳过（多为外部已修改 409），详见结果列表`
      )
    } else {
      NotificationUtil.success(`已替换 ${data.total_replacements} 处 / ${data.files_changed} 个文件`)
    }
    // 磁盘已变，旧搜索结果的 mtime 基线失效，必须重新搜索才能再替换
    results.value = []
    totalMatches.value = 0
    return true
  } catch (e: any) {
    NotificationUtil.error(e?.message || '批量替换异常')
    return false
  } finally {
    replacing.value = false
    activeTaskId = ''
  }
}

/** 撤销最近一批写盘（默认行为）；面板另有「撤销历史」可指定批次。 */
async function undo(): Promise<boolean> {
  const ok = await restoreUndoBatch()
  if (ok) {
    preview.value = null
    results.value = []
    totalMatches.value = 0
  }
  return ok
}

function toggleSelected(path: string) {
  const next = new Set(selectedPaths.value)
  if (next.has(path)) next.delete(path)
  else next.add(path)
  selectedPaths.value = next
}

function clearResults() {
  results.value = []
  totalMatches.value = 0
  filesScanned.value = 0
  filesSkipped.value = 0
  truncated.value = false
  errorMsg.value = ''
  preview.value = null
  selectedPaths.value = new Set()
}

export function useFileSearch() {
  void initSearchHistory()
  void initUndoHistory()
  void initSearchRoot()
  return {
    searchPanelVisible,
    history,
    openSearchPanel,
    rootPath,
    rootOverride,
    rootSource,
    chooseRoot,
    resetRoot,
    keyword,
    replacement,
    mode,
    useRegex,
    ignoreCase,
    include,
    exclude,
    results,
    totalMatches,
    filesScanned,
    filesSkipped,
    truncated,
    errorMsg,
    searching,
    replacing,
    busy,
    hasResults,
    preview,
    undoBatches,
    canUndo,
    selectedPaths,
    setRoot,
    search,
    cancel,
    previewReplace,
    applyReplace,
    undo,
    toggleSelected,
    clearResults,
    effectiveTargets
  }
}
