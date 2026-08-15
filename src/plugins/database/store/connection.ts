/**
 * 数据库连接状态管理
 */
import { defineStore } from 'pinia'
import { ref, computed, onMounted, onUnmounted } from 'vue'
import type { DbConnectionConfig } from '../types'
import { dbConnectionList, dbConnectionCreate, dbConnectionUpdate, dbConnectionDelete, dbConnect, dbDisconnect } from '../api/connection'
import { useWorkspaceConnections, WORKSPACE_CONNECTION_KEYS } from '@/composables/useWorkspaceConnections'

export const useConnectionStore = defineStore('dbConnection', () => {
  // 工作区连接隔离
  const wsConnections = useWorkspaceConnections(WORKSPACE_CONNECTION_KEYS.db)
  /** 所有连接配置（后端返回的全部） */
  const allConnections = ref<DbConnectionConfig[]>([])
  /** 当前工作区可见的连接（过滤后） */
  const connections = computed(() => wsConnections.filterVisible(allConnections.value))
  /** 已连接（活跃）的连接ID集合（使用 Record 保证响应性） */
  const activeConnections = ref<Record<string, boolean>>({})
  /** 当前选中的连接ID */
  const selectedConnectionId = ref<string>('')
  /** 加载状态 */
  const loading = ref(false)

  /** 当前选中的连接配置 */
  const selectedConnection = computed(() =>
    connections.value.find(c => c.id === selectedConnectionId.value)
  )

  /** 加载连接列表 */
  async function loadConnections() {
    loading.value = true
    try {
      const res = await dbConnectionList()
      if (res.code === 200) {
        allConnections.value = res.data
      }
    } finally {
      loading.value = false
    }
  }

  /** 创建连接（自动绑定到当前工作区） */
  async function createConnection(config: DbConnectionConfig) {
    const res = await dbConnectionCreate(config)
    if (res.code === 200 && res.data) {
      wsConnections.bindToCurrentWorkspace(res.data)
      await loadConnections()
    }
    return res
  }

  /** 更新连接 */
  async function updateConnection(config: DbConnectionConfig) {
    const res = await dbConnectionUpdate(config)
    if (res.code === 200) {
      await loadConnections()
    }
    return res
  }

  /** 删除连接 */
  async function deleteConnection(id: string) {
    const res = await dbConnectionDelete(id)
    if (res.code === 200) {
      delete activeConnections.value[id]
      wsConnections.removeConnection(id)
      await loadConnections()
    }
    return res
  }

  /** 建立连接 */
  async function connect(configId: string) {
    const res = await dbConnect(configId)
    if (res.code === 200) {
      activeConnections.value[configId] = true
      selectedConnectionId.value = configId
    }
    return res
  }

  /** 断开连接 */
  async function disconnect(configId: string) {
    const res = await dbDisconnect(configId)
    if (res.code === 200) {
      delete activeConnections.value[configId]
    }
    return res
  }

  /** 是否已连接 */
  function isConnected(configId: string): boolean {
    return !!activeConnections.value[configId]
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
    connections,
    activeConnections,
    selectedConnectionId,
    selectedConnection,
    loading,
    loadConnections,
    createConnection,
    updateConnection,
    deleteConnection,
    connect,
    disconnect,
    isConnected,
  }
})
