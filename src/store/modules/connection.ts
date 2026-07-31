import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  RemoteConnection,
  ConnectionTab,
  ConnectionStatus,
  TabContentType
} from '@/entity/entitys'
import { ConnectionType } from '@/entity/entitys'
import NotificationUtil from '@/utils/NotificationUtil'

let tabIdCounter = 0
function generateTabId(): string {
  tabIdCounter++
  return `conn-tab-${Date.now()}-${tabIdCounter}`
}

export const useConnectionStore = defineStore('connection', () => {
  // 所有保存的连接配置
  const connections = ref<RemoteConnection[]>([])
  // 当前打开的 tab 列表
  const tabs = ref<ConnectionTab[]>([])
  // 当前激活的 tab id
  const activeTabId = ref<string | null>(null)
  // 是否显示连接管理面板
  const showConnectionPanel = ref(false)

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
   * 从数据库加载连接列表
   */
  async function loadConnections() {
    try {
      const res = await invoke<any>('connection_list')
      if (res.code === 200) {
        connections.value = res.data.map((item: any) => ({
          ...item,
          status: 'disconnected' as ConnectionStatus
        }))
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
      const res = await invoke<any>('connection_create', {
        map: {
          ...conn,
          id: 0
        }
      })
      if (res.code === 200) {
        NotificationUtil.success('连接添加成功')
        await loadConnections()
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
    listen<string>('ssh-output', (event) => {
      // event.payload 格式: { tabId: string, data: string }
      // 由具体 TerminalView 组件处理
      window.dispatchEvent(new CustomEvent('ssh-output', { detail: event.payload }))
    })
  }

  /**
   * 监听后端传输进度事件
   */
  function setupTransferProgressListener() {
    listen<any>('transfer-progress', (event) => {
      window.dispatchEvent(new CustomEvent('transfer-progress', { detail: event.payload }))
    })
  }

  return {
    // 状态
    connections,
    tabs,
    activeTabId,
    showConnectionPanel,
    // 计算属性
    activeTab,
    activeConnection,
    // 方法
    loadConnections,
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
