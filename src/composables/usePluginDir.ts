/**
 * 插件工作目录 composable
 * 提供便捷的插件目录访问能力
 */
import { ref, computed } from 'vue'
import { workspaceDirService } from '@/services/workspaceDirService'
import { workspaceService } from '@/services/workspaceService'

/** 当前工作区 ID（全局共享） */
const currentWorkspaceId = ref<string>('')

/** 初始化工作区 ID（应用启动时调用一次） */
export async function initPluginWorkspace() {
  const ws = await workspaceService.getDefault()
  if (ws) {
    currentWorkspaceId.value = ws.id
  }
}

/** 获取当前工作区 ID */
export function getCurrentWorkspaceId(): string {
  return currentWorkspaceId.value
}

/** 切换工作区时更新插件目录上下文 */
export function setPluginWorkspaceId(id: string) {
  currentWorkspaceId.value = id
}

/**
 * 插件工作目录 hook
 * @param pluginId 插件标识
 */
export function usePluginDir(pluginId: string) {
  const workspaceId = computed(() => currentWorkspaceId.value)

  /** 获取插件工作目录根 */
  async function getDir(): Promise<string> {
    if (!workspaceId.value) return ''
    return workspaceDirService.getPluginDir(workspaceId.value, pluginId)
  }

  /** 获取插件子目录 */
  async function getSubDir(subdir: string): Promise<string> {
    if (!workspaceId.value) return ''
    return workspaceDirService.getPluginSubDir(workspaceId.value, pluginId, subdir)
  }

  /** 获取全局插件目录 */
  async function getGlobalDir(): Promise<string> {
    return workspaceDirService.getGlobalPluginDir(pluginId)
  }

  /** 获取自定义目录 */
  async function getCustomDir(): Promise<string | null> {
    return workspaceDirService.getCustomDir(pluginId)
  }

  /** 设置自定义目录 */
  async function setCustomDir(path: string): Promise<boolean> {
    return workspaceDirService.setCustomDir(pluginId, path)
  }

  return {
    workspaceId,
    getDir,
    getSubDir,
    getGlobalDir,
    getCustomDir,
    setCustomDir,
  }
}
