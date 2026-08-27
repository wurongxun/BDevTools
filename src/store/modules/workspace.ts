/**
 * 工作区 Pinia Store
 * 管理当前窗口绑定的工作区配置
 */
import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import type { WorkspaceConfig, WorkspaceSummary } from '@/Types/workspace'
import { workspaceService } from '@/services/workspaceService'
import { setPluginWorkspaceId } from '@/composables/usePluginDir'
import { fileTreeStore as getFileTreeStore } from '@/store/modules/fileTree'
import { get_file_Api } from '@/api/file'
import { load, getCurrentWebviewWindow } from '@/core/runtime'
import NotificationUtil from '@/utils/NotificationUtil'

export const useWorkspaceStore = defineStore('workspace', () => {
  /** 当前窗口绑定的工作区 */
  const currentWorkspace = ref<WorkspaceConfig | null>(null)
  /** 所有工作区列表 */
  const allWorkspaces = ref<WorkspaceSummary[]>([])
  /** 是否正在加载 */
  const loading = ref(false)
  /** 当前窗口的 label（用于工作区锁） */
  const windowLabel = ref('main')
  /** 是否正在等待用户选择工作区（新建窗口弹窗期间，抑制自动分配兜底逻辑） */
  const pendingWorkspaceSelection = ref(false)

  // 初始化窗口 label
  try {
    windowLabel.value = getCurrentWebviewWindow().label
  } catch { /* 默认 main */ }

  // 工作区变化时自动更新窗口标题，并同步插件目录上下文
  watch(currentWorkspace, (ws) => {
    if (ws) {
      setPluginWorkspaceId(ws.id)
    }
    if (ws?.name) {
      try {
        getCurrentWebviewWindow().setTitle(ws.name)
      } catch { /* ignore */ }
    }
  })

  // 监听工作目录变化（打开文件夹 / 最近文件夹等），自动同步到当前工作区的文件夹列表
  watch(
    () => getFileTreeStore().workingDirectory,
    async (dir) => {
      if (!dir || !currentWorkspace.value) return
      const folders = currentWorkspace.value.folders ?? []
      // 已是首位文件夹，无需重复写入
      if (folders[0] === dir) return
      // 新文件夹置顶，去重，最多保留 20 个
      const newFolders = [dir, ...folders.filter(f => f !== dir)].slice(0, 20)
      await updateCurrentWorkspace({ folders: newFolders })
      // 刷新工作区列表（管理对话框中展示文件夹路径）
      await loadAllWorkspaces()
    }
  )

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

  /** 将工作区的首个文件夹应用到文件树（并持久化到 settings.json） */
  async function applyWorkspaceFolder(config: WorkspaceConfig) {
    if (config.folders.length === 0) return
    const dir = config.folders[0]
    const ftStore = getFileTreeStore()
    if (ftStore.workingDirectory === dir) return
    ftStore.setWorkingDirectory(dir)
    await get_file_Api()
    try {
      const store = await load('settings.json', { autoSave: false, defaults: {} })
      await store.set('md_working_directory', { value: dir })
      await store.save()
    } catch { /* ignore */ }
  }

  /** 加载当前窗口的工作区配置（并自动加锁）
   * 如果目标工作区已被其他窗口占用，自动尝试找一个未锁定的工作区
   */
  async function loadCurrentWorkspace(workspaceId: string | null) {
    let config: WorkspaceConfig | null = null

    if (!workspaceId) {
      config = await workspaceService.getDefault()
    } else {
      config = await workspaceService.get(workspaceId)
    }

    // 尝试锁定目标工作区
    if (config) {
      const lockResult = await workspaceService.lock(config.id, windowLabel.value).catch(() => ({ locked: false }))
      if (lockResult.locked) {
        currentWorkspace.value = config
        await applyWorkspaceFolder(config)
        return
      }
      // 锁定失败（被其他窗口占用），尝试找未锁定的工作区
      console.warn(`[workspace] 工作区 [${config.name}] 已被占用，尝试寻找可用工作区`)
      const unlocked = await findUnlockedWorkspace()
      if (unlocked) {
        currentWorkspace.value = unlocked
        await applyWorkspaceFolder(unlocked)
        return
      }
    }

    // 所有工作区都被占用或没有工作区
    // 1. 优先复用同文件夹的已有未锁定工作区，避免同一目录反复创建重复工作区
    const dir = config?.folders?.[0] || ''
    if (dir) {
      const reusable = await findWorkspaceByFolder(dir)
      if (reusable) {
        currentWorkspace.value = reusable
        await applyWorkspaceFolder(reusable)
        return
      }
    }
    // 2. 没有可复用：创建临时工作区（isTemporary，关闭窗口时询问保存/丢弃），不产生永久垃圾
    const tempConfig = await createTempWorkspace()
    if (tempConfig) {
      await loadAllWorkspaces()
    }
  }

  /** 从工作区列表中找到一个未被锁定的工作区 */
  async function findUnlockedWorkspace(): Promise<WorkspaceConfig | null> {
    const list = allWorkspaces.value.length > 0
      ? allWorkspaces.value
      : await workspaceService.list()

    for (const summary of list) {
      const holder = await workspaceService.lockStatus(summary.id).catch(() => null)
      if (!holder) {
        // 未被锁定，尝试锁定
        const lockResult = await workspaceService.lock(summary.id, windowLabel.value).catch(() => ({ locked: false }))
        if (lockResult.locked) {
          return await workspaceService.get(summary.id)
        }
      }
    }
    return null
  }

  /** 在已有工作区中查找包含指定文件夹且未被锁定的工作区（复用，避免同目录重复创建） */
  async function findWorkspaceByFolder(dir: string): Promise<WorkspaceConfig | null> {
    const list = allWorkspaces.value.length > 0
      ? allWorkspaces.value
      : await workspaceService.list()

    for (const summary of list) {
      if (!summary.folders.includes(dir)) continue
      const holder = await workspaceService.lockStatus(summary.id).catch(() => null)
      if (!holder) {
        const lockResult = await workspaceService.lock(summary.id, windowLabel.value).catch(() => ({ locked: false }))
        if (lockResult.locked) {
          return await workspaceService.get(summary.id)
        }
      }
    }
    return null
  }

  /** 切换工作区（带锁保护）
   * @returns { success: boolean, error?: string }  */
  async function switchWorkspace(id: string): Promise<{ success: boolean; error?: string }> {
    // 如果已经是当前工作区，无需切换
    if (currentWorkspace.value?.id === id) {
      return { success: true }
    }

    // 尝试锁定目标工作区
    const lockResult = await workspaceService.lock(id, windowLabel.value)
    if (!lockResult.locked) {
      const holder = lockResult.holder || '未知窗口'
      const errorMsg = `该工作区已被窗口 [${holder}] 占用`
      NotificationUtil.warn(errorMsg)
      return { success: false, error: errorMsg }
    }

    // 锁定成功，释放旧工作区的锁
    const oldId = currentWorkspace.value?.id
    if (oldId) {
      await workspaceService.unlock(oldId, windowLabel.value).catch(() => {})
    }

    const config = await workspaceService.get(id)
    if (config) {
      currentWorkspace.value = config
      setPluginWorkspaceId(id)

      // 联动文件树：切换到工作区的第一个文件夹
      await applyWorkspaceFolder(config)

      // 广播工作区切换事件，供各插件监听
      window.dispatchEvent(new CustomEvent('workspace-changed', { detail: { workspaceId: id, config } }))
    }

    return { success: true }
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

  /** 判断工作区配置是否为临时工作区 */
  function isTemporary(config: WorkspaceConfig | null): boolean {
    return config?.settings?.isTemporary === true
  }

  /** 创建临时工作区并绑定到当前窗口
   * 临时标记写入 settings.isTemporary，关闭窗口时会询问是否保留
   */
  async function createTempWorkspace(): Promise<WorkspaceConfig | null> {
    const name = `临时工作区 ${new Date().toLocaleString('zh-CN', {
      month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit'
    })}`
    const config = await workspaceService.create(name, [])
    if (!config) return null
    // 打上临时标记（update 为整体替换 settings，需合并已有键）
    const marked = await workspaceService.update(config.id, {
      settings: { ...config.settings, isTemporary: true }
    })
    const finalConfig = marked ?? config
    await workspaceService.lock(finalConfig.id, windowLabel.value).catch(() => {})
    currentWorkspace.value = finalConfig
    setPluginWorkspaceId(finalConfig.id)
    await loadAllWorkspaces()
    return finalConfig
  }

  /** 保存临时工作区：移除临时标记，转为正式工作区 */
  async function saveTempWorkspace(id: string): Promise<boolean> {
    const config = currentWorkspace.value
    if (!config || config.id !== id) return false
    const settings = { ...config.settings }
    delete settings.isTemporary
    const updated = await workspaceService.update(id, { settings })
    if (updated) {
      currentWorkspace.value = updated
      await loadAllWorkspaces()
      return true
    }
    return false
  }

  // 窗口关闭时 Rust 端的 unregister_window 会自动释放工作区锁
  // 无需前端额外处理

  return {
    // 状态
    currentWorkspace,
    allWorkspaces,
    loading,
    pendingWorkspaceSelection,
    // 计算属性
    currentWorkspaceName,
    currentFolders,
    // 方法
    loadAllWorkspaces,
    loadCurrentWorkspace,
    switchWorkspace,
    createWorkspace,
    updateCurrentWorkspace,
    deleteWorkspace,
    isTemporary,
    createTempWorkspace,
    saveTempWorkspace
  }
})
