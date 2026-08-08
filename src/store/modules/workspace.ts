/**
 * 工作区 Pinia Store
 * 管理当前窗口绑定的工作区配置
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { WorkspaceConfig, WorkspaceSummary } from '@/Types/workspace'
import { workspaceService } from '@/services/workspaceService'

export const useWorkspaceStore = defineStore('workspace', () => {
  /** 当前窗口绑定的工作区 */
  const currentWorkspace = ref<WorkspaceConfig | null>(null)
  /** 所有工作区列表 */
  const allWorkspaces = ref<WorkspaceSummary[]>([])
  /** 是否正在加载 */
  const loading = ref(false)

  /** 当前工作区名称 */
  const currentWorkspaceName = computed(() => {
    return currentWorkspace.value?.name ?? '未绑定工作区'
  })

  /** 当前工作区的文件夹列表 */
  const currentFolders = computed(() => {
    return currentWorkspace.value?.folders ?? []
  })

  /** 加载所有工作区列表 */
  async function loadAllWorkspaces() {
    loading.value = true
    try {
      allWorkspaces.value = await workspaceService.list()
    } catch (e) {
      console.error('加载工作区列表失败:', e)
    } finally {
      loading.value = false
    }
  }

  /** 加载当前窗口的工作区配置 */
  async function loadCurrentWorkspace(workspaceId: string | null) {
    if (!workspaceId) {
      // 没有指定 ID，获取默认工作区
      currentWorkspace.value = await workspaceService.getDefault()
    } else {
      currentWorkspace.value = await workspaceService.get(workspaceId)
    }
  }

  /** 切换工作区 */
  async function switchWorkspace(id: string) {
    const config = await workspaceService.get(id)
    if (config) {
      currentWorkspace.value = config
    }
  }

  /** 创建新工作区 */
  async function createWorkspace(name: string, folders: string[]): Promise<WorkspaceConfig | null> {
    const config = await workspaceService.create(name, folders)
    if (config) {
      await loadAllWorkspaces()
    }
    return config
  }

  /** 更新当前工作区配置 */
  async function updateCurrentWorkspace(options: {
    name?: string
    folders?: string[]
    settings?: Record<string, any>
  }) {
    if (!currentWorkspace.value) return
    const updated = await workspaceService.update(currentWorkspace.value.id, options)
    if (updated) {
      currentWorkspace.value = updated
    }
  }

  /** 删除工作区 */
  async function deleteWorkspace(id: string) {
    const ok = await workspaceService.delete(id)
    if (ok) {
      if (currentWorkspace.value?.id === id) {
        currentWorkspace.value = null
      }
      await loadAllWorkspaces()
    }
  }

  return {
    // 状态
    currentWorkspace,
    allWorkspaces,
    loading,
    // 计算属性
    currentWorkspaceName,
    currentFolders,
    // 方法
    loadAllWorkspaces,
    loadCurrentWorkspace,
    switchWorkspace,
    createWorkspace,
    updateCurrentWorkspace,
    deleteWorkspace
  }
})
