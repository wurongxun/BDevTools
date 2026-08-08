import { get_file_Api } from '@/api/file'
import useStore from '@/store'
import ToolUtils from '@/utils/ToolUtils'
import { load, Store } from '@tauri-apps/plugin-store'

/**
 * @description 更换工作区：打开目录选择对话框，更新当前窗口的工作目录并刷新文件树
 * 工作目录存储在 per-window Pinia 状态中，每个窗口独立管理
 */
export async function changeWorkspace() {
  const { open } = await import('@tauri-apps/plugin-dialog')
  const { routerNoteStore } = useStore()

  const file_directory = await open({
    multiple: false,
    directory: true
  })
  if (file_directory == null) {
    return
  }

  // 更新当前窗口的 Pinia 状态（per-window 隔离）
  routerNoteStore.setWorkingDirectory(file_directory)

  // 持久化到 settings.json（仅作为主窗口下次启动的初始值）
  let store: Store | undefined
  if (!store) {
    store = await load('settings.json', {
      autoSave: false,
      defaults: {}
    })
  }
  if (store) {
    await store.set('md_working_directory', { value: file_directory })
    await store.save()
  }

  // 刷新当前窗口的文件树
  await routerNoteStore.setRoutes()
}

export default class IndexUtils {
  /**
   * @description 监听窗口的变化 ，处理侧边菜单的展开收缩
   */
  static windowResize() {
    const { system } = useStore()
    window.addEventListener(
      'resize',
      ToolUtils.debounce(() => {
        if (window.innerWidth <= 900) {
          system.packCollapse()
        }
      }, 300)
    )
  }

  /**
   * @description 判断当前环境是否为 development （本地开发环境）
   * @returns boolean
   */
  static isDevelopment() {
    return import.meta.env.MODE === 'development'
  }
}


