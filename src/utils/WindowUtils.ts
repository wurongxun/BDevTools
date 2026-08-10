import { WebviewWindow,getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Window } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import { load } from '@tauri-apps/plugin-store'
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
 * @param width 窗口宽度（可选，默认 960）
 * @param height 窗口高度（可选，默认 640）
 * @param title 窗口标题（可选）
 */
export async function createEditorWindow(
  workspaceId?: string,
  width?: number,
  height?: number,
  title?: string
): Promise<WindowMeta | null> {
  console.log('[createEditorWindow] 开始调用, workspaceId=', workspaceId, 'size=', width, 'x', height)
  try {
    const res = await invoke<ResponseData<WindowMeta>>('create_editor_window', {
      workspaceId: workspaceId ?? null,
      width: width ?? null,
      height: height ?? null,
      title: title ?? null
    })
    console.log('[createEditorWindow] 响应:', res)
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

/** 窗口预设配置（与 AppSetting.vue 中的 WindowPresetConfig 保持一致） */
export interface WindowPresetConfig {
  id: string
  name: string
  enabledPlugins: string[]
  width: number
  height: number
  openOnStartup: boolean
  focusMode: boolean
}

/**
 * 根据预设配置创建编辑器窗口
 * 创建后将布局配置保存到 settings.json，供新窗口读取
 */
export async function createPresetWindow(preset: WindowPresetConfig): Promise<WindowMeta | null> {
  const meta = await createEditorWindow(
    undefined,
    preset.width,
    preset.height,
    preset.name
  )
  if (!meta) return null

  // 将布局配置保存到 settings.json，供新窗口初始化时读取
  try {
    const store = await load('settings.json', { autoSave: false, defaults: {} })
    await store.set(`window_layout_${meta.label}`, {
      enabledPlugins: preset.enabledPlugins,
      focusMode: preset.focusMode,
      presetId: preset.id
    })
    await store.save()
  } catch (e) {
    console.warn('保存窗口布局配置失败:', e)
  }

  return meta
}
