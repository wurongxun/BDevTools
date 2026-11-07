import { defineStore } from 'pinia'
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { invoke } from '@/api/rpc'
import { subscribe } from '@/core/runtime'
import type {
  RemoteConnection,
  ConnectionTab,
  ConnectionStatus,
  TabContentType
} from '../types'
import { DEFAULT_CONNECTION_OPTIONS } from '../types'
import NotificationUtil from '@/utils/NotificationUtil'
import { useWorkspaceConnections, WORKSPACE_CONNECTION_KEYS } from '@/composables/useWorkspaceConnections'
import { useFtpStore } from './ftp'

let tabIdCounter = 0
function generateTabId(): string {
  tabIdCounter++
  return `conn-tab-${Date.now()}-${tabIdCounter}`
}

export const useConnectionStore = defineStore('connection', () => {
  // 工作区连接隔离
  const wsConnections = useWorkspaceConnections(WORKSPACE_CONNECTION_KEYS.ssh)
  // 所有保存的连接配置（后端返回的全部）
  const allConnections = ref<RemoteConnection[]>([])
  // 当前工作区可见的连接（过滤后 + 置顶优先）
  const connections = computed(() => {
    const visible = wsConnections.filterVisible(allConnections.value)
    return [...visible].sort((a, b) => {
      if (!!a.affix !== !!b.affix) return a.affix ? -1 : 1
      return a.name.localeCompare(b.name, 'zh-CN')
    })
  })
  // 当前打开的 tab 列表
  const tabs = ref<ConnectionTab[]>([])
  // 当前激活的 tab id（主窗格）
  const activeTabId = ref<string | null>(null)
  // P1-D 分屏：副窗格激活的 tab；splitEnabled=false 时恒为 null
  const splitEnabled = ref(false)
  const splitOrientation = ref<'vertical' | 'horizontal'>('vertical')
  const secondaryTabId = ref<string | null>(null)
  // 撰写栏显隐
  const composeVisible = ref(false)
  // 是否显示连接管理面板
  const showConnectionPanel = ref(false)
  // 分组名称及顺序（持久化到 localStorage）
  const connectionGroups = ref<string[]>([])

  // 计算属性
  const activeTab = computed(() => {
    if (!activeTabId.value) return null
    return tabs.value.find(t => t.id === activeTabId.value) || null
  })

  const activeConnection = computed(() => {
    if (!activeTab.value) return null
    return connections.value.find(c => c.id === activeTab.value!.connectionId) || null
  })

  /**
   * 加载分组（从 localStorage）
   */
  function loadGroups() {
    try {
      const saved = localStorage.getItem('ssh-connection-groups')
      if (saved) {
        connectionGroups.value = JSON.parse(saved)
      } else {
        // 从连接中自动提取分组
        const names = new Set<string>()
        for (const conn of connections.value) {
          names.add(conn.favorites_name || '默认分组')
        }
        connectionGroups.value = Array.from(names)
        saveGroups()
      }
    } catch {
      connectionGroups.value = []
    }
  }

  function saveGroups() {
    try {
      localStorage.setItem('ssh-connection-groups', JSON.stringify(connectionGroups.value))
    } catch { /* ignore */ }
  }

  function addGroup(name: string) {
    if (!name || connectionGroups.value.includes(name)) return false
    connectionGroups.value.push(name)
    saveGroups()
    return true
  }

  function renameGroup(oldName: string, newName: string) {
    if (!newName || oldName === newName) return false
    const idx = connectionGroups.value.indexOf(oldName)
    if (idx < 0) return false
    if (connectionGroups.value.includes(newName)) return false
    connectionGroups.value[idx] = newName
    // 更新所有属于该分组的连接
    for (const conn of connections.value) {
      if (conn.favorites_name === oldName) {
        conn.favorites_name = newName
        void updateConnection(conn)
      }
    }
    saveGroups()
    return true
  }

  function deleteGroup(name: string) {
    const idx = connectionGroups.value.indexOf(name)
    if (idx < 0) return false
    connectionGroups.value.splice(idx, 1)
    // 将该分组下的连接移到"默认分组"
    for (const conn of connections.value) {
      if (conn.favorites_name === name) {
        conn.favorites_name = '默认分组'
        void updateConnection(conn)
      }
    }
    saveGroups()
    return true
  }

  function reorderGroups(newOrder: string[]) {
    connectionGroups.value = newOrder
    saveGroups()
  }

  /**
   * 移动连接到指定分组
   */
  function moveConnectionToGroup(connId: number, groupName: string) {
    const conn = connections.value.find(c => c.id === connId)
    if (!conn) return
    conn.favorites_name = groupName
    void updateConnection(conn)
  }

  /**
   * 从数据库加载连接列表。
   *
   * 遵守 CODEBUDDY.md 的持久化约定：store 创建时立即发起首次读取，后续调用复用同一个
   * Promise（MainPanel 与 SidebarPanel 都挂载时不会重复打 IPC），写操作前先 await 它，
   * 避免「先写后读」把磁盘数据用空列表覆盖。增删改后走 force=true 重新拉取。
   */
  let loadPromise: Promise<void> | null = null
  function loadConnections(force = false): Promise<void> {
    if (!force && loadPromise) return loadPromise
    const task = (async () => {
      try {
        const res = await invoke<any>('connection_list')
        if (res.code === 200) {
          allConnections.value = res.data.map((item: any) => ({
            ...DEFAULT_CONNECTION_OPTIONS,
            ...item,
            encoding: item.encoding || DEFAULT_CONNECTION_OPTIONS.encoding,
            term_type: item.term_type || DEFAULT_CONNECTION_OPTIONS.term_type,
            status: 'disconnected' as ConnectionStatus
          }))
          if (connectionGroups.value.length === 0) {
            loadGroups()
          }
          // 补齐导入/旧数据里出现但尚未建组的分组
          for (const conn of allConnections.value) {
            if (conn.favorites_name && !connectionGroups.value.includes(conn.favorites_name)) {
              connectionGroups.value.push(conn.favorites_name)
            }
          }
        }
      } catch (error) {
        console.error('Failed to load connections:', error)
      }
    })()
    loadPromise = task
    return task
  }

  // ==================== 标签布局持久化（P1-D） ====================
  // 只存 tab 的「身份」不存运行时状态：SSH/SFTP sessionId 进程内有效，
  // 重启后不可能复活；恢复出来的 tab 一律是未连接，由用户手动重连。
  const LAYOUT_KEY = 'ssh-layout-v1'

  function persistLayout() {
    try {
      localStorage.setItem(LAYOUT_KEY, JSON.stringify({
        tabs: tabs.value.map(t => ({
          connectionId: t.connectionId, type: t.type, title: t.title, icon: t.icon
        })),
        activeIndex: tabs.value.findIndex(t => t.id === activeTabId.value),
        secondaryIndex: tabs.value.findIndex(t => t.id === secondaryTabId.value),
        splitEnabled: splitEnabled.value,
        splitOrientation: splitOrientation.value,
        composeVisible: composeVisible.value
      }))
    } catch { /* localStorage 不可用时静默降级 */ }
  }

  function restoreLayout() {
    try {
      const raw = localStorage.getItem(LAYOUT_KEY)
      if (!raw) return
      const saved = JSON.parse(raw)
      const list: any[] = Array.isArray(saved?.tabs) ? saved.tabs : []
      const restored: ConnectionTab[] = []
      for (const m of list) {
        if (!allConnections.value.some(c => c.id === m.connectionId)) continue
        const type: TabContentType = ['terminal', 'ftp', 'tunnel'].includes(m.type) ? m.type : 'terminal'
        restored.push({
          id: generateTabId(),
          connectionId: m.connectionId,
          title: String(m.title || ''),
          icon: String(m.icon || ''),
          type,
          closable: true,
          status: 'disconnected' as ConnectionStatus
        })
      }
      if (restored.length === 0) return
      tabs.value = restored
      const idx = Number.isInteger(saved.activeIndex)
        ? Math.max(0, Math.min(saved.activeIndex, restored.length - 1)) : 0
      activeTabId.value = restored[idx].id
      showConnectionPanel.value = true
      splitOrientation.value = saved.splitOrientation === 'horizontal' ? 'horizontal' : 'vertical'
      composeVisible.value = !!saved.composeVisible
      if (saved.splitEnabled && Number.isInteger(saved.secondaryIndex)) {
        const s = restored[saved.secondaryIndex]
        secondaryTabId.value = s?.id ?? null
        splitEnabled.value = !!secondaryTabId.value
        ensureDistinctPanes()
      }
    } catch { /* 损坏的快照直接忽略 */ }
  }

  // 顺序与标题变化才落盘；status 高频变化不写，避免每次心跳都碰 localStorage
  watch(
    () => [
      tabs.value.map(t => `${t.id}:${t.title}`).join('|'),
      activeTabId.value,
      secondaryTabId.value,
      splitEnabled.value,
      splitOrientation.value,
      composeVisible.value
    ],
    () => persistLayout()
  )

  /** 撰写栏输入广播：TerminalView 用 window 事件接收（sessionId 只存在于组件内） */
  function sendCompose(text: string, broadcast: boolean, targetTabId: string | null) {
    if (!text) return
    window.dispatchEvent(new CustomEvent('ssh-compose-input', {
      detail: { text, broadcast, targetTabId }
    }))
  }

  // 立即发起首次读取（不依赖任何组件 onMounted），读完后才能校验快照里的 connectionId
  void loadConnections().then(restoreLayout)

  /**
   * 新建连接
   */
  async function addConnection(conn: Omit<RemoteConnection, 'id'>) {
    await loadConnections()
    try {
      // 记录当前已有的 ID，用于识别新建的连接
      const oldIds = new Set(allConnections.value.map(c => c.id))
      const res = await invoke<any>('connection_create', {
        map: {
          ...DEFAULT_CONNECTION_OPTIONS,
          ...conn,
          id: 0
        }
      })
      if (res.code === 200) {
        NotificationUtil.success('连接添加成功')
        await loadConnections(true)
        // 找到新建的连接并绑定到当前工作区
        const newConn = allConnections.value.find(c => !oldIds.has(c.id))
        if (newConn) {
          wsConnections.bindToCurrentWorkspace(String(newConn.id))
        }
      } else {
        NotificationUtil.error(res.msg || '添加失败')
      }
    } catch (error) {
      console.error('Failed to add connection:', error)
      NotificationUtil.error('添加连接失败')
    }
  }

  /** 复制会话：复用配置、另起名字，避免与后端 (分组, 名称) 唯一约束冲突 */
  async function duplicateConnection(conn: RemoteConnection) {
    const { id: _id, status: _status, ...rest } = conn
    const baseName = `${conn.name} (副本)`
    let name = baseName
    let n = 2
    while (allConnections.value.some(c => c.name === name && c.favorites_name === conn.favorites_name)) {
      name = `${baseName} ${n++}`
    }
    await addConnection({ ...DEFAULT_CONNECTION_OPTIONS, ...rest, name })
  }

  /**
   * 编辑连接
   */
  async function updateConnection(conn: RemoteConnection) {
    await loadConnections()
    try {
      const res = await invoke<any>('connection_edit', {
        map: { ...DEFAULT_CONNECTION_OPTIONS, ...conn }
      })
      if (res.code === 200) {
        NotificationUtil.success('连接修改成功')
        await loadConnections(true)
      } else {
        NotificationUtil.error(res.msg || '修改失败')
      }
    } catch (error) {
      console.error('Failed to update connection:', error)
      NotificationUtil.error('修改连接失败')
    }
  }

  /**
   * 删除连接
   */
  async function deleteConnection(id: number) {
    await loadConnections()
    try {
      const res = await invoke<any>('connection_delete', { id: String(id) })
      if (res.code === 200) {
        NotificationUtil.success('连接删除成功')
        wsConnections.removeConnection(String(id))
        // 关闭该连接相关的所有 tab
        const tabsToRemove = tabs.value.filter(t => t.connectionId === id)
        tabsToRemove.forEach(t => closeTab(t.id))
        await loadConnections(true)
      } else {
        NotificationUtil.error(res.msg || '删除失败')
      }
    } catch (error) {
      console.error('Failed to delete connection:', error)
      NotificationUtil.error('删除连接失败')
    }
  }

  /** 导出为 JSON 文本（含密码与私钥口令，由调用方提示用户谨慎保存） */
  function serializeConnections(): string {
    return JSON.stringify({
      version: 1,
      exportedAt: new Date().toISOString(),
      groups: connectionGroups.value,
      connections: allConnections.value.map(({ id: _id, status: _status, ...rest }) => rest)
    }, null, 2)
  }

  /**
   * 从 JSON 文本导入。逐条走 addConnection（保持工作区绑定与唯一约束），
   * 重名自动加后缀；返回成功导入条数。
   */
  async function importConnections(json: string): Promise<number> {
    let parsed: any
    try {
      parsed = JSON.parse(json)
    } catch {
      NotificationUtil.error('不是合法的 JSON 文本')
      return 0
    }
    const list: any[] = Array.isArray(parsed) ? parsed : parsed?.connections
    if (!Array.isArray(list) || list.length === 0) {
      NotificationUtil.error('未找到连接配置')
      return 0
    }
    if (Array.isArray(parsed?.groups)) {
      for (const g of parsed.groups) {
        if (typeof g === 'string') addGroup(g)
      }
    }
    let count = 0
    for (const raw of list) {
      if (!raw || typeof raw.address !== 'string' || typeof raw.username !== 'string') continue
      let name = String(raw.name || `${raw.username}@${raw.address}`)
      let n = 2
      while (allConnections.value.some(c => c.name === name)) {
        name = `${raw.name || raw.address} (${n++})`
      }
      const before = allConnections.value.length
      await addConnection({
        ...DEFAULT_CONNECTION_OPTIONS,
        ...raw,
        name,
        port: Number(raw.port) || 22,
        agreement: raw.agreement === 'sftp' ? 'sftp' : 'ssh'
      } as Omit<RemoteConnection, 'id'>)
      if (allConnections.value.length > before) count++
    }
    return count
  }

  /**
   * 打开连接 Tab（终端或 FTP）
   */
  function openTab(connection: RemoteConnection, type: TabContentType, force = false): string {
    // 默认沿用「同连接同类型只开一个」；force=true 用于「新建标签 / 复制标签」，
    // 满足一台机器需要多个 shell 的日常场景。
    if (!force) {
      const existingTab = tabs.value.find(
        t => t.connectionId === connection.id && t.type === type
      )
      if (existingTab) {
        switchTab(existingTab.id)
        return existingTab.id
      }
    }

    const typeLabel: Record<TabContentType, string> = {
      terminal: '终端',
      ftp: '文件',
      tunnel: '隧道'
    }
    const tab: ConnectionTab = {
      id: generateTabId(),
      connectionId: connection.id,
      title: `${connection.name} (${typeLabel[type]})`,
      icon: type === 'terminal' ? 'terminal' : type === 'tunnel' ? 'tunnel' : 'folder',
      type,
      closable: true,
      status: 'disconnected' as ConnectionStatus
    }

    tabs.value.push(tab)
    switchTab(tab.id)
    showConnectionPanel.value = true

    return tab.id
  }

  /**
   * 关闭 Tab
   */
  function closeTab(tabId: string) {
    const index = tabs.value.findIndex(t => t.id === tabId)
    if (index === -1) return

    tabs.value.splice(index, 1)
    if (secondaryTabId.value === tabId) {
      secondaryTabId.value = tabs.value.find(t => t.id !== activeTabId.value)?.id ?? null
      if (!secondaryTabId.value) splitEnabled.value = false
    }

    // 如果关闭的是激活 tab，切换到相邻 tab
    if (activeTabId.value === tabId) {
      if (tabs.value.length > 0) {
        const newIndex = Math.min(index, tabs.value.length - 1)
        activeTabId.value = tabs.value[newIndex].id
      } else {
        activeTabId.value = null
        showConnectionPanel.value = false
      }
    }
    ensureDistinctPanes()
  }

  /** 同一个 tab 不能同时占两个窗格：单实例组件只有一个 DOM 节点 */
  function ensureDistinctPanes() {
    if (!splitEnabled.value) { secondaryTabId.value = null; return }
    if (secondaryTabId.value === activeTabId.value || !tabs.value.some(t => t.id === secondaryTabId.value)) {
      secondaryTabId.value = tabs.value.find(t => t.id !== activeTabId.value)?.id ?? null
    }
    if (!secondaryTabId.value) splitEnabled.value = false
  }

  /**
   * 切换 Tab。pane='secondary' 时只改副窗格，主窗格保持不动（分屏下点击内容区用）。
   */
  function switchTab(tabId: string, pane: 'primary' | 'secondary' = 'primary') {
    if (pane === 'secondary' && splitEnabled.value) {
      secondaryTabId.value = tabId
    } else {
      activeTabId.value = tabId
    }
    ensureDistinctPanes()
    showConnectionPanel.value = true
  }

  function setSplit(enabled: boolean, orientation: 'vertical' | 'horizontal' = splitOrientation.value) {
    if (enabled && tabs.value.length < 2) {
      NotificationUtil.warn('至少需要打开两个标签才能分屏')
      return
    }
    splitEnabled.value = enabled
    splitOrientation.value = orientation
    if (enabled) ensureDistinctPanes()
    else secondaryTabId.value = null
  }

  function toggleSplit() {
    setSplit(!splitEnabled.value)
  }

  /** 复制标签：同一连接、同类型的新 tab（不复用既有 tab） */
  function duplicateTab(tabId: string): string | null {
    const src = tabs.value.find(t => t.id === tabId)
    if (!src) return null
    const conn = allConnections.value.find(c => c.id === src.connectionId)
    if (!conn) return null
    return openTab(conn, src.type, true)
  }

  /** 拖拽排序：把 dragId 插到 targetId 之前 */
  function moveTab(dragId: string, targetId: string) {
    const from = tabs.value.findIndex(t => t.id === dragId)
    const to = tabs.value.findIndex(t => t.id === targetId)
    if (from < 0 || to < 0 || from === to) return
    const [moved] = tabs.value.splice(from, 1)
    const insertAt = tabs.value.findIndex(t => t.id === targetId)
    tabs.value.splice(insertAt < 0 ? tabs.value.length : insertAt, 0, moved)
  }

  /**
   * 更新 Tab 连接状态
   */
  function updateTabStatus(tabId: string, status: ConnectionStatus) {
    const tab = tabs.value.find(t => t.id === tabId)
    if (tab) {
      tab.status = status
    }
  }

  /**
   * 重排序 Tab
   */
  function reorderTabs(newOrder: string[]) {
    const tabMap = new Map(tabs.value.map(t => [t.id, t]))
    const reordered: ConnectionTab[] = []
    for (const id of newOrder) {
      const tab = tabMap.get(id)
      if (tab) reordered.push(tab)
    }
    tabs.value = reordered
  }

  /**
   * 监听后端 SSH 输出事件
   */
  function setupSshOutputListener() {
    subscribe('ssh-output', (payload) => {
      // payload 格式: { tabId, data(base64), format: 'base64' }
      // 由具体 TerminalView 组件处理
      window.dispatchEvent(new CustomEvent('ssh-output', { detail: payload }))
    })
  }

  /**
   * 监听后端连接关闭事件。
   * reason=manual 是用户自己点断开（组件已改状态），其余是 EOF / 读错误，需要同步 tab 状态。
   */
  function setupSshClosedListener() {
    subscribe('ssh-closed', (payload: any) => {
      if (!payload?.tabId) return
      if (payload.reason !== 'manual') {
        updateTabStatus(payload.tabId, 'disconnected' as ConnectionStatus)
      }
      window.dispatchEvent(new CustomEvent('ssh-closed', { detail: payload }))
    })
  }

  /**
   * 监听后端传输进度事件
   * 后端 emit 的 transfer-progress 事件（payload: { tabId, taskId, progress, transferredSize, speed }）
   * 直接透传到 ftpStore.updateTransferProgress，驱动传输面板进度条实时更新。
   */
  function setupTransferProgressListener() {
    const ftpStore = useFtpStore()
    subscribe('transfer-progress', (payload: any) => {
      if (!payload || !payload.taskId) return
      ftpStore.updateTransferProgress(
        payload.tabId ?? '',
        payload.taskId,
        Number(payload.progress) || 0,
        Number(payload.transferredSize) || 0,
        payload.speed ?? '',
        Number(payload.totalSize) || undefined
      )
    })
  }

  // 监听工作区切换事件，重新加载连接列表
  function onWorkspaceChanged() {
    void loadConnections(true)
  }

  onMounted(() => {
    window.addEventListener('workspace-changed', onWorkspaceChanged)
  })

  onUnmounted(() => {
    window.removeEventListener('workspace-changed', onWorkspaceChanged)
  })

  return {
    // 状态
    connections,
    allConnections,
    tabs,
    activeTabId,
    splitEnabled,
    splitOrientation,
    secondaryTabId,
    composeVisible,
    showConnectionPanel,
    connectionGroups,
    // 计算属性
    activeTab,
    activeConnection,
    // 方法
    loadConnections,
    loadGroups,
    addGroup,
    renameGroup,
    deleteGroup,
    reorderGroups,
    moveConnectionToGroup,
    addConnection,
    duplicateConnection,
    updateConnection,
    deleteConnection,
    serializeConnections,
    importConnections,
    openTab,
    duplicateTab,
    closeTab,
    switchTab,
    setSplit,
    toggleSplit,
    moveTab,
    sendCompose,
    updateTabStatus,
    reorderTabs,
    setupSshOutputListener,
    setupSshClosedListener,
    setupTransferProgressListener
  }
})
