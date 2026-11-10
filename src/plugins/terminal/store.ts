/**
 * terminal 插件 store
 *
 * 三块职责：
 * 1. **会话编排**：标签 / 分屏树 / 窗格元数据（标题、cwd、退出码、busy…）
 * 2. **外观配置**：字体、字号、光标、滚动回看、配色（持久化到 `plugins/terminal.json`）
 * 3. **停靠面板状态**：高度 / 显隐（镜像到 `layout` store，由 Layout 渲染与拖拽）
 *
 * `TerminalSession` 实例（含 xterm）**刻意不放进响应式状态**——xterm 内部有大量
 * 可变对象，被 Vue 代理后既慢又容易出诡异 bug。实例放模块级 `Map`，
 * 组件通过 `getSession(paneId)` 取。
 */
import { defineStore } from 'pinia'
import { computed, reactive, ref, watch, nextTick } from 'vue'
import { isDark } from '@/composables/useDark'
import { load as loadKv, openUrl } from '@/core/runtime'
import useStore from '@/store/index'
import { fetchTerminalDefaults, listShells, writeTerminal } from './api'
import { DEFAULT_FONT_FAMILY, domIsDark, resolvePalette } from './theme'
import { TerminalSession } from './session'
import {
  firstLeaf,
  hasLeaf,
  leafCount,
  leafIds,
  pathToLeaf,
  removeLeaf,
  replaceLeaf,
  setRatio,
  singlePaneTab,
} from './tree'
import type {
  CommandMark,
  PaneNode,
  PaneState,
  ShellInfo,
  TabState,
  TermDefaults,
  TermExecResult,
  TerminalAppearance,
} from './types'

/**
 * 新建会话的默认工作目录（`TermLaunchSpec.cwd` 缺省时的取值）。
 *
 * 优先级：
 * 1. **当前工作区目录**（`fileTreeStore.workingDirectory`，即工作区首个文件夹；
 *    工作区没绑定文件夹时该值为空，则再看 `currentWorkspace.folders[0]`）
 *    —— 终端默认落在用户正在开发的项目根，而不是系统主目录。
 * 2. 工作区没有目录 → 系统默认目录（后端探测的 HOME，如 `C:\Users\Star`）。
 *
 * 这里只负责「默认值」：目录是否真实存在由后端 `terminal_open` 兜底
 * （显式 cwd 不存在时回落 HOME），所以前端不必做文件系统校验。
 */
function resolveDefaultCwd(workspaceCwd: string, fallback: string | undefined): string {
  return workspaceCwd || fallback || ''
}

/** 插件用户级配置文件名（config_dir 下的 plugins/<id>.json） */
const CONFIG_FILE = 'plugins/terminal.json'
const CONFIG_KEY = 'appearance'

/** 一屏最多允许的窗格数（防止误触把面板切成网格） */
const MAX_PANES_PER_TAB = 8
/** 停靠面板只允许拖拽到的最小高度；低于它视为「拖到底隐藏」 */
export const DOCK_MIN_HEIGHT = 80
/** 主区（editor）至少保留的高度；拖到顶时主区隐藏 */
export const EDITOR_MIN_HEIGHT = 80

/** 新建窗格时的启动参数（非响应式：只在建会话那一瞬间需要） */
export interface TermLaunchSpec {
  shell?: string
  cwd?: string
  args?: string[]
  /** 标签标题（默认用 shell 名） */
  title?: string
  /**
   * M4 / F4：会话就绪后自动送入 PTY 的命令（不含回车）。
   * 只作为「启动意图」记录在这里，不传给 session.open —— 后者只认 PTY 启动参数。
   */
  command?: string
}

const DEFAULT_APPEARANCE: TerminalAppearance = {
  fontFamily: DEFAULT_FONT_FAMILY,
  fontSize: 13,
  lineHeight: 1.2,
  cursorStyle: 'block',
  cursorBlink: true,
  scrollback: 5000,
  copyOnSelect: false,
  colorScheme: 'follow',
  panelHeight: 280,
  panelVisible: false,
}

/** 窗格 id → 会话实例（非响应式） */
const sessions = new Map<string, TerminalSession>()
/** 窗格 id → 启动参数（组件挂载时消费一次） */
const launchSpecs = new Map<string, TermLaunchSpec>()
/** 窗格 id → 会话就绪后要自动执行的命令（M4 / F4） */
const launchCommands = new Map<string, string>()
/** `<html>` 主题类观察器（模块级单例，避免重复安装） */
let themeObserver: MutationObserver | null = null

let seq = 0
function uid(prefix: string): string {
  seq += 1
  return `${prefix}-${Date.now().toString(36)}-${seq.toString(36)}`
}

export const useTerminalStore = defineStore('terminal', () => {
  const { useLayoutStore, fileTreeStore, useWorkspaceStore } = useStore()

  // ==================== 状态 ====================
  const tabs = ref<TabState[]>([])
  const activeTabId = ref('')
  const panes = ref<Record<string, PaneState>>({})
  const appearance = reactive<TerminalAppearance>({ ...DEFAULT_APPEARANCE })

  const shells = ref<ShellInfo[]>([])
  const defaults = ref<TermDefaults | null>(null)
  /**
   * **DOM 上真正生效的**暗色状态（不是 `isDark` 的值）。
   *
   * `useDark` 是「先改 ref、后加类」：类是在 post flush 的 watcher 里加到 `<html>` 上的。
   * 若直接用 `isDark` 去算配色，在 pre flush 阶段读 `--bgColor` 会拿到切换前的旧值
   * （症状：切到暗色后终端画布仍是白底深字）。所以这里用一个自己的 ref，
   * 只在 DOM 类真的变了之后（`syncTheme`）才更新。
   */
  const darkApplied = ref(domIsDark(isDark.value))
  /** 配置 + 环境信息是否已就绪（面板首次渲染前 await） */
  const ready = ref(false)
  const initError = ref('')
  /** 最近一次错误提示（前端 toast 用） */
  const lastError = ref('')

  // 搜索栏
  const searchOpen = ref(false)
  const searchQuery = ref('')
  const searchCaseSensitive = ref(false)
  const searchRegex = ref(false)
  const searchWholeWord = ref(false)
  const searchInfo = ref('')

  // 设置 / 快速执行弹窗
  const settingsOpen = ref(false)
  const execOpen = ref(false)
  const execRunning = ref(false)
  const execHistory = ref<{ command: string; result: TermExecResult }[]>([])
  const execError = ref('')

  // ==================== 派生 ====================
  const activeTab = computed(() => tabs.value.find((t) => t.id === activeTabId.value) ?? null)
  const activePaneId = computed(() => activeTab.value?.activePaneId ?? '')
  const activePane = computed(() => (activePaneId.value ? (panes.value[activePaneId.value] ?? null) : null))
  const paneCount = computed(() => Object.keys(panes.value).length)
  const currentPalette = computed(() => resolvePalette(appearance.colorScheme, darkApplied.value))
  /** 当前 shell 集成程度：none 时快速执行只能靠超时兜底，UI 需要提示 */
  const integrationHint = computed(() => {
    const pane = activePane.value
    const level = pane?.integration ?? 'none'
    if (level === 'full') return ''
    if (level === 'partial') return '当前 shell 只支持部分集成（无退出码），快速执行拿不到退出码'
    if (!pane?.shellKind) return ''
    // WSL 是 Windows 侧的启动器，内部跑哪个发行版/shell 无法预知，注不进集成脚本：
    // 单独说明，否则用户看到「拿不到退出码」会以为是自己没配好
    if (pane.shellKind === 'wsl') {
      return 'WSL 会话未注入 shell 集成（内部 shell 未知）：拿不到退出码与工作目录，快速执行只能等到超时'
    }
    // 用 shellLabel（如「命令提示符 (cmd)」）而不是 shellKind（cmd）—— 提示是给人看的
    return `当前 shell（${pane.shellLabel}）未注入 shell 集成，快速执行只能等到超时`
  })
  /** 当前工作区目录（工作区绑定的首个文件夹）；无工作区目录时为空 */
  const workspaceCwd = computed(() => fileTreeStore.workingDirectory || useWorkspaceStore.currentFolders[0] || '')
  /** 新建会话时实际会使用的工作目录（UI 提示用，规则见 `resolveDefaultCwd`） */
  const newSessionCwd = computed(() => resolveDefaultCwd(workspaceCwd.value, defaults.value?.cwd))

  // ==================== 配置持久化 ====================
  let persistTimer: ReturnType<typeof setTimeout> | null = null
  function persist() {
    if (persistTimer) clearTimeout(persistTimer)
    persistTimer = setTimeout(() => {
      persistTimer = null
      void loadKv(CONFIG_FILE)
        .then((store) => store.set(CONFIG_KEY, { ...appearance }))
        .catch((e) => {
          lastError.value = `保存终端配置失败：${e instanceof Error ? e.message : String(e)}`
        })
    }, 250)
  }

  /**
   * 初始化：读配置 → 探测 shell → 把面板状态推给 layout store。
   *
   * 关键点：**返回同一个 Promise**，所以 `await ensureInit()` 一定等到 shell 探测结束。
   * 早先只用 `ready` 布尔量当「已开始初始化」的标记，第二个调用方会立刻返回，
   * 于是「面板可见」的 watcher 能在 shell 还没探测出来时就建出首个会话——
   * 窗格拿不到默认 shell 元数据，标题退化成字面量 `terminal`、
   * `windowsPty`（ConPTY 行重排）标志也拿不到。幂等，可重复调用。
   */
  let initPromise: Promise<void> | null = null
  function ensureInit(): Promise<void> {
    if (!initPromise) {
      // doInit 内部已逐段兜错；这里再兜一层，保证 Promise 永远不 reject（否则后续 await 全炸）
      initPromise = doInit().catch((e) => {
        initError.value = `初始化终端失败：${e instanceof Error ? e.message : String(e)}`
      })
    }
    return initPromise
  }

  async function doInit() {
    ready.value = true
    // 主题类可能在任意时刻由宿主切换，装上观察器让终端配色始终跟着走
    startThemeObserver()
    try {
      const store = await loadKv(CONFIG_FILE)
      const saved = await store.get<Partial<TerminalAppearance>>(CONFIG_KEY)
      if (saved && typeof saved === 'object') {
        Object.assign(appearance, { ...DEFAULT_APPEARANCE, ...saved })
      }
    } catch (e) {
      initError.value = `读取终端配置失败，已用默认值：${e instanceof Error ? e.message : String(e)}`
    }

    // 面板状态镜像到 layout store（Layout 据此渲染与拖拽）
    const layout = useLayoutStore
    layout.setDockPanelHeight('bottom', appearance.panelHeight)
    layout.setDockPanelVisible('bottom', appearance.panelVisible)

    try {
      const [list, def] = await Promise.all([listShells(), fetchTerminalDefaults()])
      shells.value = list
      defaults.value = def
    } catch (e) {
      initError.value = `探测本机 shell 失败：${e instanceof Error ? e.message : String(e)}`
    }
  }

  // ==================== 会话实例注册 ====================
  function getSession(paneId: string): TerminalSession | undefined {
    return sessions.get(paneId)
  }
  function registerSession(paneId: string, session: TerminalSession) {
    // 窗格可能已经没了（建会话是异步的：用户在建的过程中就关掉了标签/窗格）。
    // 这时不能登记，直接把实例释放掉——会话会把「正在建立的」后端 PTY 一并回收。
    if (!panes.value[paneId]) {
      session.dispose()
      return
    }
    sessions.set(paneId, session)
    session.isDarkCache = darkApplied.value
    session.applyAppearance({ ...appearance }, currentPalette.value)
  }
  function unregisterSession(paneId: string) {
    const s = sessions.get(paneId)
    sessions.delete(paneId)
    if (launchSpecs.has(paneId)) launchSpecs.delete(paneId)
    s?.dispose()
  }
  /** 消费（取出并删除）窗格的启动参数，组件挂载时调用一次 */
  function takeLaunchSpec(paneId: string): TermLaunchSpec {
    const spec = launchSpecs.get(paneId) ?? {}
    launchSpecs.delete(paneId)
    return spec
  }
  /** 把外部链接交给系统浏览器（终端里的链接点击） */
  function openExternal(uri: string) {
    void openUrl(uri).catch((e) => {
      lastError.value = `打开链接失败：${e instanceof Error ? e.message : String(e)}`
    })
  }

  // ==================== 窗格 ====================
  function createPaneState(paneId: string, spec: TermLaunchSpec): PaneState {
    const shellPath = spec.shell || defaults.value?.shell || ''
    const info = shells.value.find((s) => s.path === shellPath)
    return {
      id: paneId,
      sessionId: '',
      title: spec.title || info?.label || shellPath.split(/[\\/]/).pop() || 'terminal',
      cwd: spec.cwd || defaults.value?.cwd || '',
      shellLabel: info?.label || shellPath.split(/[\\/]/).pop() || 'shell',
      shellKind: info?.kind || '',
      integration: 'none',
      exited: false,
      exitCode: null,
      lastCommandOk: null,
      busy: false,
    }
  }

  function addPane(spec: TermLaunchSpec = {}): string {
    const paneId = uid('pane')
    // 启动参数与窗格元数据共用同一份 cwd：spec 显式给了就用它（分屏继承当前窗格目录），
    // 否则取工作区目录 → 系统默认目录。两处若各算一次，会出现
    //「界面显示工作区目录、PTY 实际开在 HOME」的错位。
    const { command, ...openSpec } = spec
    const launch: TermLaunchSpec = { ...openSpec, cwd: spec.cwd || resolveDefaultCwd(workspaceCwd.value, defaults.value?.cwd) }
    launchSpecs.set(paneId, launch)
    if (command) launchCommands.set(paneId, command)
    panes.value = { ...panes.value, [paneId]: createPaneState(paneId, launch) }
    return paneId
  }

  function removePane(paneId: string) {
    launchCommands.delete(paneId)
    unregisterSession(paneId)
    const { [paneId]: _removed, ...rest } = panes.value
    panes.value = rest
  }

  function patchPane(paneId: string, patch: Partial<PaneState>) {
    const cur = panes.value[paneId]
    if (!cur) return
    panes.value = { ...panes.value, [paneId]: { ...cur, ...patch } }
  }

  // ==================== 标签 / 分屏 ====================
  /** 新建标签（不传 spec 用默认 shell + 当前窗格目录） */
  function newTab(spec: TermLaunchSpec = {}) {
    const paneId = addPane(spec)
    const tab = singlePaneTab(uid('tab'), paneId, panes.value[paneId].title)
    tabs.value = [...tabs.value, tab]
    activeTabId.value = tab.id
    focusActivePane()
    return tab.id
  }

  /**
   * 在指定窗格旁分屏。
   * @param direction row = 左右并排；column = 上下堆叠
   */
  function splitPane(paneId: string, direction: 'row' | 'column', spec: TermLaunchSpec = {}) {
    const tab = tabs.value.find((t) => hasLeaf(t.root, paneId))
    if (!tab) return
    if (leafCount(tab.root) >= MAX_PANES_PER_TAB) {
      lastError.value = `单个标签最多 ${MAX_PANES_PER_TAB} 个窗格`
      return
    }
    const newPaneId = addPane({ ...spec, cwd: spec.cwd || panes.value[paneId]?.cwd })
    const nextRoot: PaneNode = {
      kind: 'split',
      direction,
      a: { kind: 'leaf', paneId },
      b: { kind: 'leaf', paneId: newPaneId },
      ratio: 0.5,
    }
    const root = replaceLeaf(tab.root, paneId, nextRoot)
    updateTab(tab.id, { root, activePaneId: newPaneId })
    focusActivePane()
  }

  /** 关闭一个窗格：树空则关标签 */
  function closePane(paneId: string) {
    const tab = tabs.value.find((t) => hasLeaf(t.root, paneId))
    if (!tab) {
      removePane(paneId)
      return
    }
    const root = removeLeaf(tab.root, paneId)
    removePane(paneId)
    if (!root) {
      closeTab(tab.id)
      return
    }
    // 激活窗格被关掉时，退到树里第一个叶子
    const nextActive = hasLeaf(root, tab.activePaneId) ? tab.activePaneId : firstLeaf(root)
    updateTab(tab.id, { root, activePaneId: nextActive })
  }

  function closeTab(tabId: string) {
    const tab = tabs.value.find((t) => t.id === tabId)
    if (!tab) return
    // 先释放该标签下所有窗格（kill PTY）
    for (const id of leafIds(tab.root)) removePane(id)
    const idx = tabs.value.findIndex((t) => t.id === tabId)
    tabs.value = tabs.value.filter((t) => t.id !== tabId)
    if (activeTabId.value === tabId) {
      const next = tabs.value[Math.min(idx, tabs.value.length - 1)]
      activeTabId.value = next?.id ?? ''
    }
  }

  function closeAllTabs() {
    for (const t of [...tabs.value]) closeTab(t.id)
  }

  function updateTab(tabId: string, patch: Partial<TabState>) {
    tabs.value = tabs.value.map((t) => (t.id === tabId ? { ...t, ...patch } : t))
  }

  function setActiveTab(tabId: string) {
    activeTabId.value = tabId
    focusActivePane()
  }

  function setActivePane(paneId: string) {
    const tab = tabs.value.find((t) => hasLeaf(t.root, paneId))
    if (!tab) return
    if (tab.activePaneId !== paneId) updateTab(tab.id, { activePaneId: paneId })
    focusActivePane()
  }

  /** 调整某个分屏节点的比例（分隔条拖拽） */
  function resizeSplit(tabId: string, path: number[], ratio: number) {
    const tab = tabs.value.find((t) => t.id === tabId)
    if (!tab) return
    updateTab(tabId, { root: setRatio(tab.root, path, ratio) })
  }

  function splitPathOf(paneId: string): number[] | null {
    const tab = tabs.value.find((t) => hasLeaf(t.root, paneId))
    return tab ? pathToLeaf(tab.root, paneId) : null
  }

  /** 聚焦当前激活窗格（并滚动其 xterm 到可见） */
  async function focusActivePane() {
    await nextTick()
    const id = activePaneId.value
    if (!id) return
    const s = sessions.get(id)
    s?.fitNow()
    s?.focus()
  }

  /** 让当前标签下所有窗格重新适配尺寸（面板高度变化 / 切标签后） */
  async function refitActiveTab() {
    await nextTick()
    const tab = activeTab.value
    if (!tab) return
    for (const id of leafIds(tab.root)) sessions.get(id)?.fitNow()
  }

  // ==================== 外观 ====================
  function applyAppearancePatch(patch: Partial<TerminalAppearance>) {
    Object.assign(appearance, patch)
  }

  function resetAppearance() {
    Object.assign(appearance, { ...DEFAULT_APPEARANCE, panelHeight: appearance.panelHeight, panelVisible: appearance.panelVisible })
  }

  /** 主题 / 外观变化 → 广播给所有会话实例 */
  function broadcastAppearance() {
    const palette = currentPalette.value
    const dark = darkApplied.value
    for (const s of sessions.values()) {
      s.isDarkCache = dark
      s.applyAppearance({ ...appearance }, palette)
    }
  }

  watch(appearance, () => {
    broadcastAppearance()
    persist()
  }, { deep: true })

  /**
   * 同步「DOM 上真正生效的主题」。
   *
   * 为什么不能只 `watch(isDark)`：`useDark` 把 `dark` 类加到 `<html>` 上是
   * **post flush** 阶段做的事（`watch(state, onChanged, { flush: 'post' })`），
   * 而默认的 `watch` 是 **pre flush** —— 会在类还没加上时就去读
   * `--bgColor` / `--textColor`，于是切到暗色后终端画布拿到的是浅色底
   * （面板外壳是静态 CSS 会正常变暗，只有会话区不一致）。
   *
   * 这里做两层保险：
   * 1. `{ flush: 'post' }`：与 useDark 同为 post，且本 watcher 创建更晚 → 排在它之后；
   * 2. 直接观察 `<html>` 的 class 变化：把「主题 CSS 变量」这个真正的依赖显式化，
   *    不依赖任何第三方库内部的 flush 顺序。
   * 命中变化时先更新 `darkApplied`（驱动 currentPalette / 窗格底色重算），再广播给 xterm 实例。
   */
  function syncTheme() {
    const now = domIsDark(isDark.value)
    if (now === darkApplied.value) return
    darkApplied.value = now
    broadcastAppearance()
  }

  watch(isDark, syncTheme, { flush: 'post' })

  /** 观察 html 上的主题类（幂等；由 ensureInit 调用一次） */
  function startThemeObserver() {
    if (typeof MutationObserver === 'undefined' || themeObserver) return
    themeObserver = new MutationObserver(syncTheme)
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })
  }

  // ==================== 停靠面板 ====================
  /** 面板是否可见（真实来源是 layout store，`appearance.panelVisible` 只是持久化镜像） */
  const panelVisible = computed(() => useLayoutStore.dockPanelVisible.bottom)
  const panelHeight = computed(() => useLayoutStore.dockPanelHeight.bottom)
  const panelExpanded = computed(() => useLayoutStore.dockPanelExpanded.bottom)

  function setPanelVisible(visible: boolean) {
    useLayoutStore.setDockPanelVisible('bottom', visible)
  }

  function setPanelHeight(height: number) {
    useLayoutStore.setDockPanelHeight('bottom', height)
  }

  function togglePanel() {
    useLayoutStore.toggleDockPanel('bottom')
  }

  /**
   * 面板第一次可见时创建首个会话（避免应用启动就拉起 shell）。
   * 先 `await ensureInit()`：首个窗格的标题 / ConPTY 标志都来自 shell 探测结果。
   */
  async function ensureFirstTab() {
    await ensureInit()
    if (tabs.value.length === 0) newTab()
    else await refitActiveTab()
  }

  // layout store 是面板状态的唯一真源，这里只做「镜像到持久化字段」
  watch(() => useLayoutStore.dockPanelHeight.bottom, (h) => {
    if (appearance.panelHeight !== Math.round(h)) appearance.panelHeight = Math.round(h)
  })
  watch(() => useLayoutStore.dockPanelVisible.bottom, (v) => {
    if (appearance.panelVisible !== v) appearance.panelVisible = v
    if (v) void ensureFirstTab()
  })

  // 设置弹窗里改「面板高度」→ 推到 layout（面板状态的真源在那儿）
  watch(() => Math.round(appearance.panelHeight), (h) => {
    if (useLayoutStore.dockPanelHeight.bottom !== h) useLayoutStore.setDockPanelHeight('bottom', h)
  })

  // ==================== 命令执行（异步 + 超时） ====================
  /**
   * 在指定窗格执行命令并等待结束。
   * 有 shell 集成时拿到真实退出码与输出；无集成时只能等到超时（UI 已提示）。
   */
  async function exec(paneId: string, command: string, timeoutMs = 30_000): Promise<TermExecResult> {
    const s = sessions.get(paneId)
    if (!s) throw new Error('窗格尚未就绪')
    patchPane(paneId, { busy: true })
    execError.value = ''
    try {
      const result = await s.exec(command, timeoutMs)
      execHistory.value = [{ command, result }, ...execHistory.value].slice(0, 20)
      patchPane(paneId, {
        busy: false,
        lastCommandOk: result.exitCode === null ? null : result.exitCode === 0,
      })
      return result
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      execError.value = msg
      patchPane(paneId, { busy: false })
      throw e
    }
  }

  async function runInActivePane(command: string, timeoutMs: number) {
    const id = activePaneId.value
    if (!id) throw new Error('没有可用的终端窗格')
    execRunning.value = true
    try {
      return await exec(id, command, timeoutMs)
    } finally {
      execRunning.value = false
    }
  }

  // ==================== 窗格元数据回调（由 TerminalPane 调用） ====================
  function onPaneTitle(paneId: string, title: string) {
    const clean = title.trim()
    if (!clean) return
    patchPane(paneId, { title: clean })
    // 单窗格标签跟随窗格标题，多窗格时保持标签名稳定（避免标签名乱跳）
    const tab = tabs.value.find((t) => t.id === activeTabId.value)
    if (tab && leafCount(tab.root) === 1) updateTab(tab.id, { title: clean })
  }

  function onPaneCwd(paneId: string, cwd: string) {
    patchPane(paneId, { cwd })
  }

  function onPaneMark(paneId: string, mark: CommandMark, exitCode: number | null) {
    if (mark === 'commandEnd' && exitCode !== null) {
      patchPane(paneId, { lastCommandOk: exitCode === 0, busy: false })
    }
  }

  function onPaneExit(paneId: string, code: number | null) {
    patchPane(paneId, { exited: true, exitCode: code, busy: false })
  }

  /**
   * 重启一个已退出（或启动失败）的窗格：杀掉旧会话、按原参数重开。
   * 用「换 paneId 之外的一切都不变」的方式实现，避免组件重建。
   */
  function restartPaneNode(paneId: string) {
    unregisterSession(paneId)
    const prev = panes.value[paneId]
    if (!prev) return
    launchSpecs.set(paneId, { cwd: prev.cwd || undefined })
    patchPane(paneId, {
      sessionId: '',
      exited: false,
      exitCode: null,
      lastCommandOk: null,
      busy: false,
      integration: 'none',
    })
  }

  /** 会话建立成功后写回元数据 */
  function onPaneOpened(paneId: string, info: { sessionId: string; cwd: string; integration: PaneState['integration']; shellKind: string }) {
    patchPane(paneId, {
      sessionId: info.sessionId,
      cwd: info.cwd,
      integration: info.integration,
      shellKind: info.shellKind,
      exited: false,
      exitCode: null,
    })

    // M4 / F4：外部插件请求「在集成终端运行当前脚本」。会话刚建立时 shell 可能还没
    // 接上 PTY，立即写入会被吞掉，稍等一拍再用唯一通道送入命令 + 回车。
    const command = launchCommands.get(paneId)
    if (command) {
      launchCommands.delete(paneId)
      window.setTimeout(() => {
        void writeTerminal(info.sessionId, `${command}\r`).catch((e) => {
          lastError.value = `发送命令到终端失败：${e instanceof Error ? e.message : String(e)}`
        })
      }, 250)
    }
  }

  return {
    // 状态
    tabs,
    activeTabId,
    panes,
    appearance,
    shells,
    defaults,
    ready,
    initError,
    lastError,
    searchOpen,
    searchQuery,
    searchCaseSensitive,
    searchRegex,
    searchWholeWord,
    searchInfo,
    settingsOpen,
    execOpen,
    execRunning,
    execHistory,
    execError,
    // 派生
    activeTab,
    activePaneId,
    activePane,
    paneCount,
    currentPalette,
    darkApplied,
    integrationHint,
    workspaceCwd,
    newSessionCwd,
    // 生命周期
    ensureInit,
    getSession,
    registerSession,
    unregisterSession,
    takeLaunchSpec,
    openExternal,
    // 标签 / 分屏
    newTab,
    splitPane,
    closePane,
    closeTab,
    closeAllTabs,
    setActiveTab,
    setActivePane,
    resizeSplit,
    splitPathOf,
    focusActivePane,
    refitActiveTab,
    // 外观
    applyAppearancePatch,
    resetAppearance,
    broadcastAppearance,
    // 停靠面板
    panelVisible,
    panelHeight,
    panelExpanded,
    setPanelVisible,
    setPanelHeight,
    togglePanel,
    ensureFirstTab,
    // 执行
    exec,
    runInActivePane,
    // 窗格回调
    onPaneTitle,
    onPaneCwd,
    onPaneMark,
    onPaneExit,
    onPaneOpened,
    restartPaneNode,
  }
})
