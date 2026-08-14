/**
 * 数据库连接状态管理
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { DbConnectionConfig } from '../types'
import { dbConnectionList, dbConnectionCreate, dbConnectionUpdate, dbConnectionDelete, dbConnect, dbDisconnect } from '../api/connection'

export const useConnectionStore = defineStore('dbConnection', () => {
  /** 所有连接配置 */
  const connections = ref<DbConnectionConfig[]>([])
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
        connections.value = res.data
      }
    } finally {
      loading.value = false
    }
  }

  /** 创建连接 */
  async function createConnection(config: DbConnectionConfig) {
    const res = await dbConnectionCreate(config)
    if (res.code === 200) {
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
