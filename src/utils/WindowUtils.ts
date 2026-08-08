import { WebviewWindow,getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Window } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import type { WindowMeta } from '@/Types/workspace'
import type { ResponseData } from '@/entity/entitys'
const PAGE_CONFIGS: Record<string, { title: string; width: number; height: number }> = {
  '/App/setting': { title: '设置', width: 800, height: 650 },
  '/App/about': { title: '关于', width: 800, height: 650 },
  '/App/help': { title: '帮助', width: 800, height: 650 },
  '/App/terminal': { title: '终端', width: 900, height: 600 }
}

export async function openAppWindow(path: string) {
  const config = PAGE_CONFIGS[path]
  if (!config) return

  // 生成窗口标签：/App/setting → app-window-App-setting
  const windowLabel = `app-window-${path.replace(/^\//, '').replace(/\//g, '-')}`

  try {
    const existing = await WebviewWindow.getByLabel(windowLabel)
    if (existing) {
      const win = existing as unknown as Window
      await win.show()
      await win.unminimize();
      await win.setFocus()
      return
    }
  } catch (_) {}

  const appWindow = new WebviewWindow(windowLabel, {
    title: config.title,
    width: config.width,
    height: config.height,
    resizable: true,
    center: false,
    x: 100,
    y: 100,
    decorations: false,
    visible: false,
    url: `index.html#${path}`
  })

  appWindow.once('tauri://created', async function () {
    console.log(`窗口 ${windowLabel} 创建成功`)
    const win = appWindow as unknown as Window
    await win.center()
    await win.show()
    await win.setFocus()
  })

  appWindow.once('tauri://error', function (event) {
    console.error(`窗口 ${windowLabel} 创建失败:`, event)
  })
}

/**
 * 创建新的编辑器窗口（通过 Rust 后端创建，支持窗口注册表管理）
 * @param workspaceId 可选的工作区 ID，绑定到新窗口
 */
export async function createEditorWindow(workspaceId?: string): Promise<WindowMeta | null> {
  try {
    const res = await invoke<ResponseData<WindowMeta>>('create_editor_window', {
      workspaceId: workspaceId ?? null
    })
    if (res.code === 200) {
      return res.data
    }
    console.warn('创建编辑器窗口失败:', res.msg)
    return null
  } catch (e) {
    console.error('创建编辑器窗口异常:', e)
    return null
  }
}
