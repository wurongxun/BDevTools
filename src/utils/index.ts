import { get_file_Api } from '@/api/file'
import useStore from '@/store'
import ToolUtils from '@/utils/ToolUtils'
import { load, Store } from '@tauri-apps/plugin-store'

/**
 * @description 更换工作区：打开目录选择对话框，保存选择并刷新文件树
 */
export async function changeWorkspace() {
  const { open } = await import('@tauri-apps/plugin-dialog')
  let store: Store | undefined
  const file_directory = await open({
    multiple: false,
    directory: true
  })
  if (file_directory == null) {
    return
  }
  if (!store) {
    store = await load('settings.json', {
      autoSave: false,
      defaults: {}
    })
  }
  if (store) {
    await store.set('md_working_directory', { value: file_directory })
    await store.save()
    await get_file_Api()
  }
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


