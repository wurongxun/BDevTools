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

  /** 获取插件外部脚本目录列表（用户级，跨工作区共享） */
  async function listExternalDirs(): Promise<string[]> {
    return workspaceDirService.listExternalDirs(pluginId)
  }

  /** 添加插件外部脚本目录 */
  async function addExternalDir(path: string): Promise<{ ok: boolean; msg?: string }> {
    if (!workspaceId.value) return { ok: false, msg: '工作区未就绪' }
    return workspaceDirService.addExternalDir(workspaceId.value, pluginId, path)
  }

  /** 移除插件外部脚本目录（仅解除挂载） */
  async function removeExternalDir(path: string): Promise<boolean> {
    return workspaceDirService.removeExternalDir(pluginId, path)
  }

  return {
    workspaceId,
    getDir,
    getSubDir,
    listExternalDirs,
    addExternalDir,
    removeExternalDir,
  }
}
