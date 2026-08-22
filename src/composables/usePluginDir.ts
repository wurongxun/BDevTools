/**
 * 插件工作目录 composable
 * 提供便捷的插件目录访问能力
 */
import { ref, computed } from 'vue'
import { workspaceDirService } from '@/services/workspaceDirService'

/** 当前窗口的工作区 ID（模块级单例，每窗口独立；由窗口加载工作区时通过 setPluginWorkspaceId 设置） */
const currentWorkspaceId = ref<string>('')

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

  /** 获取自定义目录（按当前工作区） */
  async function getCustomDir(): Promise<string | null> {
    if (!workspaceId.value) return null
    return workspaceDirService.getCustomDir(workspaceId.value, pluginId)
  }

  /** 设置自定义目录（按当前工作区） */
  async function setCustomDir(path: string): Promise<boolean> {
    if (!workspaceId.value) return false
    return workspaceDirService.setCustomDir(workspaceId.value, pluginId, path)
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
