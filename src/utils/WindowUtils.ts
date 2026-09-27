import { WebviewWindow, load } from '@/core/runtime'
import type { Window } from '@/core/runtime'
import { invoke } from '@/api/rpc'
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
      // 窗口已存在，通过 Rust 后端强制显示在最顶层（Win32 SetForegroundWindow）
      await invoke('focus_window', { label: windowLabel })
      return
    }
  } catch (e) {
    console.warn(`查找窗口 ${windowLabel} 失败:`, e)
  }

  const appWindow = new WebviewWindow(windowLabel, {
    title: config.title,
    width: config.width,
    height: config.height,
    resizable: true,
    center: false,
    x: 100,
    y: 100,
    decorations: false,
    // 无边框窗口必须启用阴影，否则 Windows 下边缘露出 1px 黑线
    shadow: true,
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
 * @param presetId 窗口预设 ID（可选，用于窗口菜单识别已打开的预设窗口）
 */
export async function createEditorWindow(
  workspaceId?: string,
  width?: number,
  height?: number,
  title?: string,
  presetId?: string
): Promise<WindowMeta | null> {
  console.log('[createEditorWindow] 开始调用, workspaceId=', workspaceId, 'size=', width, 'x', height)
  try {
    const res = await invoke<ResponseData<WindowMeta>>('create_editor_window', {
      workspaceId: workspaceId ?? null,
      width: width ?? null,
      height: height ?? null,
      title: title ?? null,
      presetId: presetId ?? null
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
  /** 绑定的工作区 ID（可选；多个预设可绑同一工作区，但同时只能打开一个） */
  workspaceId?: string
}

/**
 * 由窗口预设创建窗口的结果。
 *
 * 分三种而不是布尔，是因为提示完全不同：「duplicate」是正常情况（后端说该预设的窗口已开着，
 * 已经聚焦过去了），而「failed」才是真出错 —— 混成一个 null 会让前者也弹「创建窗口失败」。
 */
export type CreatePresetWindowResult =
  | { ok: true; meta: WindowMeta }
  | { ok: false; reason: 'duplicate' | 'workspace-locked' | 'failed' }

/**
 * 根据预设配置创建编辑器窗口
 *
 * 创建后将布局配置保存到 settings.json，供新窗口读取。
 *
 * **不在前端判重**：后端保证「一个预设同时只存在一个窗口」（命中已有窗口时返回 409 + 那个窗口
 * 的 meta，见 `create_editor_window`），这里只把 409 翻译成「聚焦已有窗口」。前端判重要多一次
 * IPC 往返，而且那次查询失败时只能选择「照常创建」（于是开出两个共用同一预设的窗口）或
 * 「什么都不做」——两条路都不如把判据放在进程内的注册表里。
 */
export async function createPresetWindow(
  preset: WindowPresetConfig
): Promise<CreatePresetWindowResult> {
  let res: ResponseData<WindowMeta>
  try {
    res = await invoke<ResponseData<WindowMeta>>('create_editor_window', {
      workspaceId: preset.workspaceId || null,
      width: preset.width,
      height: preset.height,
      title: preset.name,
      presetId: preset.id
    })
  } catch (e) {
    console.error('创建预设窗口异常:', e)
    return { ok: false, reason: 'failed' }
  }

  // 409：该预设的窗口已经开着（后端拒绝重复创建）⇒ 聚焦它，不要再开一个
  if (res.code === 409) {
    const label = res.data?.label
    if (label) await invoke('focus_window', { label })
    return { ok: false, reason: 'duplicate' }
  }
  // 300：预设绑定的工作区已被其他窗口占用（新窗口未绑定则弹窗选择工作区）
  if (res.code === 300) {
    return { ok: false, reason: 'workspace-locked' }
  }
  if (res.code !== 200 || !res.data) {
    console.warn('创建编辑器窗口失败:', res.msg)
    return { ok: false, reason: 'failed' }
  }

  const meta = res.data

  // 将布局配置保存到 settings.json，供新窗口初始化时读取（含工作区绑定）
  try {
    const store = await load('settings.json', { autoSave: false, defaults: {} })
    await store.set(`window_layout_${meta.label}`, {
      enabledPlugins: preset.enabledPlugins,
      focusMode: preset.focusMode,
      presetId: preset.id,
      workspaceId: preset.workspaceId || ''
    })
    await store.save()
  } catch (e) {
    console.warn('保存窗口布局配置失败:', e)
  }

  return { ok: true, meta }
}
