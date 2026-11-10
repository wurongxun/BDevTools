/**
 * files 分屏视图组（模块级单例）。
 *
 * 设计取舍（M3 定案）：
 * - **最多两个 pane**，且**禁止同一路径在两个 pane 同时打开**。docStore 的 viewState 是
 *   文档级单份（documents.ts:65），同路径双开会让两个 Monaco 实例互抢同一份
 *   scrollTop/cursor —— 这正是 `editor-and-tabs.md:42` 警告的「多实例串号」。
 *   禁止同路径后，每个 pane 的 update-state 仍可直接按上报者 path 收口，无需改 5 个
 *   EditorType 的 props，改动面最小、风险最低。
 * - pane 各自一个 `<KeepAlive>`，且 component key 带 paneId：否则两个 pane 会命中
 *   同一个缓存实例（Vue 的 KeepAlive 按 key 缓存，与所在 KeepAlive 实例无关）。
 * - 同步滚动走**纯前端本地订阅**（不用 runtime.publish 的 Tauri emit）：滚动事件频率
 *   高，走 IPC 会拖慢界面。订阅者带 source 自身 id 防止回声。
 * - 布局（enabled / syncScroll / 左右路径）持久化到 settings.json 的 files_split_view，
 *   但**不主动 openDocument**：会话恢复由 App.vue 的 restoreOpenFiles 负责，路径若未
 *   恢复则右 pane 显示占位提示，避免把不存在的文件塞进 store。
 */
import { computed, ref, watch } from 'vue'
import { load, type Store } from '@/core/runtime'
import { useDocumentManagerStore } from '@/store/modules/documents'

const STORE_KEY = 'files_split_view'

export type PaneId = 'left' | 'right'
export interface PaneDef {
  id: PaneId
  path: string
  mode: 'editor' | 'diff'
}
export interface ScrollSyncPayload {
  /** 发出者实例 id，接收端据此忽略自己的回声 */
  source: string
  path: string
  line: number
  scrollTop: number
}

const enabled = ref(false)
const activePaneId = ref<PaneId>('left')
const leftPath = ref('')
const rightPath = ref('')
const syncScroll = ref(false)
const diffMode = ref<{ originalPath: string; modifiedPath: string } | null>(null)

const scrollHandlers = new Set<(p: ScrollSyncPayload) => void>()

let storeHandle: Promise<Store> | null = null
function store(): Promise<Store> {
  if (!storeHandle) {
    storeHandle = load('settings.json', { autoSave: false, defaults: {} }).catch((e: unknown) => {
      storeHandle = null
      throw e
    })
  }
  return storeHandle
}

let initialized = false
let saveTimer: ReturnType<typeof setTimeout> | null = null

export async function initSplitView(): Promise<void> {
  if (initialized) return
  initialized = true
  try {
    const s = await store()
    const saved = await s.get<{
      value: {
        enabled?: boolean
        syncScroll?: boolean
        left?: string
        right?: string
        activePane?: PaneId
      }
    }>(STORE_KEY)
    const v = saved?.value
    if (v && typeof v === 'object') {
      syncScroll.value = !!v.syncScroll
      leftPath.value = typeof v.left === 'string' ? v.left : ''
      rightPath.value = typeof v.right === 'string' ? v.right : ''
      // 只恢复布局开关与路径，不主动 openDocument：会话恢复由 App.vue::restoreOpenFiles 负责。
      // 路径若没被恢复，右 pane 显示占位提示，不会把不存在的文件塞进 store。
      enabled.value = !!v.enabled && !!leftPath.value
      if (leftPath.value === rightPath.value) rightPath.value = ''
      // 不变量：activePane 必须指向有文档的 pane，否则「焦点在空 pane、保存却作用于左文档」。
      activePaneId.value = v.activePane === 'right' && !!rightPath.value ? 'right' : 'left'
      syncActiveDocument()
    }
  } catch (e) {
    console.warn('[files/split] 读取分屏布局失败，使用默认值', e)
  }
  watch([enabled, syncScroll, leftPath, rightPath, activePaneId], scheduleSave)
}

function scheduleSave() {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(async () => {
    saveTimer = null
    try {
      const s = await store()
      await s.set(STORE_KEY, {
        value: {
          enabled: enabled.value,
          syncScroll: syncScroll.value,
          left: leftPath.value,
          right: rightPath.value,
          activePane: activePaneId.value
        }
      })
    } catch (e) {
      console.warn('[files/split] 保存分屏布局失败', e)
    }
  }, 300)
}

function panePath(id: PaneId): string {
  return id === 'left' ? leftPath.value : rightPath.value
}

const panes = computed<PaneDef[]>(() => {
  const docStore = useDocumentManagerStore()
  if (!enabled.value) {
    return [{ id: 'left', path: docStore.activeDocumentPath || '', mode: 'editor' }]
  }
  return [
    { id: 'left', path: leftPath.value, mode: 'editor' },
    { id: 'right', path: rightPath.value, mode: diffMode.value ? 'diff' : 'editor' }
  ]
})

/** 让全局「当前文档」与当前 pane 对齐（保存、命令面板都基于 activeDocumentPath）。 */
function syncActiveDocument() {
  if (!enabled.value) return
  const path = panePath(activePaneId.value)
  const docStore = useDocumentManagerStore()
  if (path && docStore.documents.has(path)) docStore.setActiveDocument(path)
}

/**
 * 切换当前 pane，并让全局「当前文档」指向该 pane（保存/命令面板才作用于正确文件）。
 * 目标 pane 为空（占位提示）时不抢焦点：否则视觉高亮在空 pane，
 * activeDocumentPath 却仍指向另一个文档，Ctrl+S 会保存用户没在看的文件。
 */
function setActivePane(id: PaneId) {
  if (!enabled.value || activePaneId.value === id) return
  const path = panePath(id)
  if (!path) return
  activePaneId.value = id
  useDocumentManagerStore().setActiveDocument(path)
}

/**
 * 标签点击统一入口：非分屏只切 activeDocument；分屏则把文档放进当前 pane。
 * 同路径双开会被拒绝并提示（见文件头设计取舍）。
 */
function activateDocument(path: string) {
  const docStore = useDocumentManagerStore()
  if (!enabled.value) {
    docStore.setActiveDocument(path)
    return
  }
  const other: PaneId = activePaneId.value === 'left' ? 'right' : 'left'
  if (panePath(other) === path) {
    // 已在另一个 pane 打开 → 直接把焦点交给它，不创建第二个实例
    setActivePane(other)
    return
  }
  if (activePaneId.value === 'left') leftPath.value = path
  else rightPath.value = path
  diffMode.value = null
  docStore.setActiveDocument(path)
}

/** 开启分屏。path 优先作为右 pane；不给则从已打开文档里挑一个不同于左侧的。 */
function enableSplit(path?: string) {
  const docStore = useDocumentManagerStore()
  const current = docStore.activeDocumentPath || ''
  leftPath.value = current
  let target = path && path !== current ? path : ''
  if (!target) {
    target =
      docStore.documentList.find((d) => d.id !== current)?.id || ''
  }
  if (target === current) target = ''
  rightPath.value = target
  diffMode.value = null
  enabled.value = true
  // 右 pane 没文档时把焦点留在左侧，避免「高亮空 pane / activeDocument 指向别处」
  activePaneId.value = target ? 'right' : 'left'
  if (target) docStore.setActiveDocument(target)
}

function disableSplit() {
  if (!enabled.value) return
  const keep = panePath(activePaneId.value) || leftPath.value
  enabled.value = false
  diffMode.value = null
  rightPath.value = ''
  activePaneId.value = 'left'
  if (keep) useDocumentManagerStore().setActiveDocument(keep)
}

function toggleSplit(path?: string) {
  if (enabled.value) disableSplit()
  else enableSplit(path)
}

/** 标签右键「在右侧分屏打开」：已分屏时只换右 pane，不重置左侧。 */
function openInRightPane(path: string) {
  const docStore = useDocumentManagerStore()
  if (!enabled.value) {
    enableSplit(path)
    return
  }
  if (leftPath.value === path) {
    setActivePane('left')
    return
  }
  rightPath.value = path
  diffMode.value = null
  activePaneId.value = 'right'
  docStore.setActiveDocument(path)
}

/** 文本比较：右 pane 切成 diff 视图（内容由 DiffEditor 自行按路径读取）。 */
function openDiff(originalPath: string, modifiedPath: string) {
  if (!originalPath || !modifiedPath || originalPath === modifiedPath) return false
  if (!enabled.value) enableSplit(modifiedPath)
  diffMode.value = { originalPath, modifiedPath }
  leftPath.value = originalPath
  activePaneId.value = 'right'
  useDocumentManagerStore().setActiveDocument(modifiedPath)
  return true
}

function closeDiff() {
  diffMode.value = null
}

function publishScroll(payload: ScrollSyncPayload) {
  if (!enabled.value || !syncScroll.value || diffMode.value) return
  for (const handler of scrollHandlers) handler(payload)
}

function subscribeScroll(handler: (p: ScrollSyncPayload) => void): () => void {
  scrollHandlers.add(handler)
  return () => {
    scrollHandlers.delete(handler)
  }
}

export function useSplitView() {
  return {
    enabled,
    activePaneId,
    leftPath,
    rightPath,
    syncScroll,
    diffMode,
    panes,
    panePath,
    syncActiveDocument,
    setActivePane,
    activateDocument,
    enableSplit,
    disableSplit,
    toggleSplit,
    openInRightPane,
    openDiff,
    closeDiff,
    publishScroll,
    subscribeScroll
  }
}
