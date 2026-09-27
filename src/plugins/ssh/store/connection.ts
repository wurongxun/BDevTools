import { defineStore } from 'pinia'
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@/api/rpc'
import { subscribe } from '@/core/runtime'
import type {
  RemoteConnection,
  ConnectionTab,
  ConnectionStatus,
  TabContentType
} from '../types'
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
  // 当前工作区可见的连接（过滤后）
  const connections = computed(() => wsConnections.filterVisible(allConnections.value))
  // 当前打开的 tab 列表
  const tabs = ref<ConnectionTab[]>([])
  // 当前激活的 tab id
  const activeTabId = ref<string | null>(null)
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
        updateConnection(conn)
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
        updateConnection(conn)
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
    updateConnection(conn)
  }

  /**
   * 从数据库加载连接列表
   */
  async function loadConnections() {
    try {
      const res = await invoke<any>('connection_list')
      if (res.code === 200) {
        allConnections.value = res.data.map((item: any) => ({
          ...item,
          status: 'disconnected' as ConnectionStatus
        }))
        // 加载连接后，如果没有分组数据则自动提取
        if (connectionGroups.value.length === 0) {
          loadGroups()
        }
      }
    } catch (error) {
      console.error('Failed to load connections:', error)
    }
  }

  /**
   * 新建连接
   */
  async function addConnection(conn: Omit<RemoteConnection, 'id'>) {
    try {
      // 记录当前已有的 ID，用于识别新建的连接
      const oldIds = new Set(allConnections.value.map(c => c.id))
      const res = await invoke<any>('connection_create', {
        map: {
          ...conn,
          id: 0
        }
      })
      if (res.code === 200) {
        NotificationUtil.success('连接添加成功')
        await loadConnections()
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

  /**
   * 编辑连接
   */
  async function updateConnection(conn: RemoteConnection) {
    try {
      const res = await invoke<any>('connection_edit', { map: conn })
      if (res.code === 200) {
        NotificationUtil.success('连接修改成功')
        await loadConnections()
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
    try {
      const res = await invoke<any>('connection_delete', { id: String(id) })
      if (res.code === 200) {
        NotificationUtil.success('连接删除成功')
        wsConnections.removeConnection(String(id))
        // 关闭该连接相关的所有 tab
        const tabsToRemove = tabs.value.filter(t => t.connectionId === id)
        tabsToRemove.forEach(t => closeTab(t.id))
        await loadConnections()
      } else {
        NotificationUtil.error(res.msg || '删除失败')
      }
    } catch (error) {
      console.error('Failed to delete connection:', error)
      NotificationUtil.error('删除连接失败')
    }
  }

  /**
   * 打开连接 Tab（终端或 FTP）
   */
  function openTab(connection: RemoteConnection, type: TabContentType): string {
    // 检查是否已有该连接的相同类型 tab
    const existingTab = tabs.value.find(
      t => t.connectionId === connection.id && t.type === type
    )
    if (existingTab) {
      switchTab(existingTab.id)
      return existingTab.id
    }

    const tab: ConnectionTab = {
      id: generateTabId(),
      connectionId: connection.id,
      title: `${connection.name} (${type === 'terminal' ? '终端' : '文件'})`,
      icon: type === 'terminal' ? 'terminal' : 'folder',
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
  }

  /**
   * 切换 Tab
   */
  function switchTab(tabId: string) {
    activeTabId.value = tabId
    showConnectionPanel.value = true
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
      // payload 格式: { tabId: string, data: string }
      // 由具体 TerminalView 组件处理
      window.dispatchEvent(new CustomEvent('ssh-output', { detail: payload }))
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
        payload.speed ?? ''
      )
    })
  }

  // 监听工作区切换事件，重新加载连接列表
  function onWorkspaceChanged() {
    loadConnections()
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
    tabs,
    activeTabId,
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
    updateConnection,
    deleteConnection,
    openTab,
    closeTab,
    switchTab,
    updateTabStatus,
    reorderTabs,
    setupSshOutputListener,
    setupTransferProgressListener
  }
})
