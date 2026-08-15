/**
 * 工作区连接隔离 composable
 * 管理数据库连接和 SSH 连接与当前工作区的绑定关系
 * 使用 localStorage 存储映射，不修改 Rust 后端
 *
 * 规则：
 * - 每个连接可以绑定到一个工作区
 * - 未绑定工作区的连接视为"全局连接"，在所有工作区可见
 * - 绑定到工作区的连接只在该工作区下可见
 */
import { ref, onMounted } from 'vue'
import { getCurrentWorkspaceId } from '@/composables/usePluginDir'

const STORAGE_KEY_DB = 'workspace-db-connections'
const STORAGE_KEY_SSH = 'workspace-ssh-connections'

/** 供外部引用的存储键 */
export const WORKSPACE_CONNECTION_KEYS = {
  db: STORAGE_KEY_DB,
  ssh: STORAGE_KEY_SSH,
}

/** 读取映射 */
function loadMapping(storageKey: string): Record<string, string | null> {
  try {
    const raw = localStorage.getItem(storageKey)
    return raw ? JSON.parse(raw) : {}
  } catch {
    return {}
  }
}

/** 写入映射 */
function saveMapping(storageKey: string, mapping: Record<string, string | null>) {
  try {
    localStorage.setItem(storageKey, JSON.stringify(mapping))
  } catch { /* ignore */ }
}

/**
 * 工作区连接过滤 hook
 * @param storageKey localStorage 键名（数据库用 STORAGE_KEY_DB，SSH 用 STORAGE_KEY_SSH）
 */
export function useWorkspaceConnections(storageKey: string) {
  const mapping = ref<Record<string, string | null>>({})

  onMounted(() => {
    mapping.value = loadMapping(storageKey)
  })

  /** 获取当前工作区可见的连接 ID 列表 */
  function getVisibleConnectionIds(): Set<string> {
    const wsId = getCurrentWorkspaceId()
    const result = new Set<string>()

    for (const [connId, connWsId] of Object.entries(mapping.value)) {
      // 全局连接（未绑定工作区）或属于当前工作区的连接
      if (connWsId === null || connWsId === undefined || connWsId === wsId) {
        result.add(connId)
      }
    }

    return result
  }

  /** 将连接绑定到当前工作区 */
  function bindToCurrentWorkspace(connId: string) {
    const wsId = getCurrentWorkspaceId()
    mapping.value[connId] = wsId || null
    saveMapping(storageKey, mapping.value)
  }

  /** 将连接设为全局（所有工作区可见） */
  function setAsGlobal(connId: string) {
    mapping.value[connId] = null
    saveMapping(storageKey, mapping.value)
  }

  /** 移除连接映射 */
  function removeConnection(connId: string) {
    delete mapping.value[connId]
    saveMapping(storageKey, mapping.value)
  }

  /**
   * 过滤连接列表：返回当前工作区可见的连接
   * @param allConnections 所有连接（需要有 id 字段）
   * @param includeUnmapped 是否包含未在映射中的连接（默认 true = 向后兼容）
   */
  function filterVisible<T extends { id?: string | number }>(
    allConnections: T[],
    includeUnmapped = true
  ): T[] {
    const visibleIds = getVisibleConnectionIds()
    return allConnections.filter(conn => {
      if (!conn.id) return includeUnmapped
      const connIdStr = String(conn.id)
      if (visibleIds.has(connIdStr)) return true
      // 未在映射中的连接：如果 includeUnmapped 则显示（向后兼容）
      if (includeUnmapped && !(connIdStr in mapping.value)) return true
      return false
    })
  }

  return {
    mapping,
    getVisibleConnectionIds,
    bindToCurrentWorkspace,
    setAsGlobal,
    removeConnection,
    filterVisible,
  }
}
