/**
 * 窗口预设的**跨窗口约定**（事件名 / payload / 「我是谁」判定 / 关闭对应窗口）。
 *
 * 多窗口应用里，窗口设置页可以开在**任意一个窗口**里（「设置」是宿主常驻内置插件，
 * 每个窗口的活动栏底部都有它的入口）。于是有两类事情只有在这里判定才靠得住：
 *
 * 1. **当前窗口属于哪个预设**。`get_window_list` 里的 `preset_id` 是唯一权威来源，
 *    主窗口也算一个（label `main` / `preset_id` `main`），不必在各调用点各写一遍
 *    label 前缀判断（`editor-` / `app-window-` / `web3d-` …）。
 *    辅助窗口（`app-window-*`）由 `openAppWindow` 直接创建、不进注册表 ⇒ 返回 null，
 *    语义正好是「这个窗口不属于任何窗口预设」。
 * 2. **删掉预设时要关哪些窗口**。发起方查一次注册表即可得到全部 label，故关闭动作
 *    集中在发起方做（`closePresetWindows`）：判断只存在一处，也不会出现多个窗口
 *    同时抢着关同一个窗口。
 *
 * 关闭一律走 Rust 的 `close_window` ⇒ `WebviewWindow::close()`，因此会照常触发各窗口的
 * `onCloseRequested`（保存会话快照 / 未保存文档、临时工作区询问），**不是强杀**。
 */
import { getCurrentWebviewWindow, publish, subscribe } from '@/core/runtime'
import type { Unsubscribe } from '@/core/runtime'
import { windowService } from '@/services/windowService'

/** 窗口预设变更事件名（窗口菜单、设置页、预设窗口自己都会订阅） */
export const WINDOW_PRESETS_CHANGED_EVENT = 'window-presets-changed'

/** 窗口预设变更事件 payload */
export interface WindowPresetsChangedPayload {
  /** 发起变更的窗口 label：发起方据此跳过自刷新（它的本地状态已经是最新的） */
  from: string
  /** 被删除的预设 ID：对应窗口需要关闭 */
  removed?: string[]
  /** 被改名的预设：对应窗口需要同步自己的标题 */
  renamed?: { id: string; name: string }
}

/** 当前窗口 label（`label` 是同步属性；取不到时返回空串而不是抛异常） */
export function currentWindowLabel(): string {
  try {
    const label = getCurrentWebviewWindow().label
    return typeof label === 'string' ? label : ''
  } catch {
    return ''
  }
}

/**
 * 当前窗口所属的窗口预设 ID。
 *
 * - 主窗口 → `'main'`
 * - 由预设创建的编辑器窗口 → 该预设 ID（读注册表 meta，比 `window_layout_*` 更权威：
 *   布局配置在窗口初始化时就被删掉了）
 * - 辅助窗口（`app-window-*`）、未绑定预设的编辑器窗口 → `null`
 */
export async function currentPresetId(): Promise<string | null> {
  const label = currentWindowLabel()
  if (!label) return null
  if (label === 'main') return 'main'
  try {
    const list = await windowService.getWindowList()
    return list.find(w => w.label === label)?.preset_id ?? null
  } catch (e) {
    console.warn('[WindowPresets] 读取窗口注册表失败，按「不属于任何预设」处理', e)
    return null
  }
}

/**
 * 关闭该预设打开的其它窗口，返回实际关闭数量。
 *
 * @param presetId    预设 ID
 * @param excludeLabel 不关的窗口（通常是发起方自己）
 */
export async function closePresetWindows(presetId: string, excludeLabel: string): Promise<number> {
  // 排除项缺失时宁可不关：空 label 通常意味着读不到自己窗口的 label，
  // 而「谁是发起方」正是这里唯一要保护的对象，判断不了就别动手。
  if (!excludeLabel) {
    console.warn('[WindowPresets] 拿不到发起窗口的 label，跳过关闭其它窗口')
    return 0
  }
  let labels: string[] = []
  try {
    const list = await windowService.getWindowList()
    labels = list.filter(w => w.preset_id === presetId && w.label !== excludeLabel).map(w => w.label)
  } catch (e) {
    console.warn('[WindowPresets] 读取窗口注册表失败，跳过关闭', e)
    return 0
  }
  let closed = 0
  for (const label of labels) {
    if (await windowService.closeWindow(label)) closed++
  }
  return closed
}

/** 广播「窗口预设已变更」（自动带上发起方 label） */
export function publishWindowPresetsChanged(
  payload: Omit<WindowPresetsChangedPayload, 'from'> = {},
): Promise<void> {
  return publish(WINDOW_PRESETS_CHANGED_EVENT, { ...payload, from: currentWindowLabel() })
}

/** 本窗口是否就是「被改名的那个预设」开的窗口 ⇒ 同步自己的标题（任务栏/标题栏都看它） */
async function syncSelfTitle(payload: WindowPresetsChangedPayload): Promise<void> {
  const renamed = payload.renamed
  // 主窗口标题是应用标识（tauri.conf 固定为 BDevTools），不跟随预设名
  if (!renamed || renamed.id === 'main') return
  try {
    if ((await currentPresetId()) !== renamed.id) return
    const win = getCurrentWebviewWindow()
    await (win as unknown as { setTitle(t: string): Promise<void> }).setTitle(renamed.name)
  } catch (e) {
    console.warn('[WindowPresets] 同步窗口标题失败', e)
  }
}

let inited = false
let unlisten: Unsubscribe | null = null

/**
 * 注册「窗口预设变更」的全局监听（每个窗口都要调，幂等）。
 *
 * 这里**只做各窗口自身的响应**（改名后改自己的标题）；「关窗口」集中在发起方
 * （见 `closePresetWindows`），否则每个窗口都会试图去关同一个目标。
 */
export async function initWindowPresetsSync(): Promise<void> {
  if (inited) return
  inited = true
  try {
    unlisten = await subscribe<WindowPresetsChangedPayload>(
      WINDOW_PRESETS_CHANGED_EVENT,
      (payload) => {
        if (!payload) return
        void syncSelfTitle(payload)
      },
    )
  } catch (e) {
    // 订阅失败只降级：窗口标题不会跟着改名走，其它功能不受影响
    inited = false
    console.warn('[WindowPresets] 订阅窗口预设变更事件失败', e)
  }
}

/** 注销全局监听（仅测试/热重载用） */
export function disposeWindowPresetsSync(): void {
  unlisten?.()
  unlisten = null
  inited = false
}
