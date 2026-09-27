/**
 * 窗口预设的**窗口内共享状态 + 唯一落盘口**。
 *
 * 为什么必须是模块级单例（而不是各面板各持一份 ref）：设置页有两个分区都会写同一份
 * `window_presets`，且写法不同 ——
 *  - 「窗口设置」改的是预设本身，保存时**整份数组写回**；
 *  - 「插件管理」的「加入 / 移出」只改**某一个预设**的 `enabledPlugins`。
 * 各持一份内存副本时，后写的一方会把对方刚做的改动整片覆盖（无提示丢改动）。
 * 所以状态与所有写盘动作都收敛到这里，两个面板共用同一份数据。
 *
 * 跨窗口仍走 `core/windowPresets.ts` 的事件：别的窗口改完广播、本窗口重读。同窗口自己发起的
 * 变更已经直接改在这份共享状态里，不必重读（只有编辑弹窗开着时要挂起 —— 弹窗里那份是打开
 * 时的副本，见 `WindowPresetPane.onExternalPresetsChanged`）。
 */
import { computed, ref } from 'vue'
import type { Store, Unsubscribe } from '@/core/runtime'
import { pluginManager } from '@/core/pluginManager'
import { currentPresetId, currentWindowLabel, publishWindowPresetsChanged } from '@/core/windowPresets'
import { windowService } from '@/services/windowService'
import type { WindowMeta } from '@/Types/workspace'
import useStore from '@/store'
import { useSettingsStore } from './useSettingsStore'
import type { WindowPresetConfig } from './types'

/** `settings.json` 句柄（与其他分区共用同一份，见 useSettingsStore） */
let store: Store | undefined

/** 除主窗口外的窗口预设（对应 `settings.json` 的 `window_presets`） */
export const windowPresets = ref<WindowPresetConfig[]>([])
/** 主窗口预设（存在 `main_window_preset`；不可删除、始终启动） */
export const mainWindowPreset = ref<WindowPresetConfig>({
  id: 'main',
  name: '主窗口',
  enabledPlugins: [],
  width: 960,
  height: 640,
  openOnStartup: true,
  focusMode: false,
})

/** 主窗口 + 其它预设（卡片列表、重名检查、启动计数都用这一份） */
export const allPresets = computed(() => [mainWindowPreset.value, ...windowPresets.value])

/** 按 ID 取预设（主窗口也在内；找不到 / 传 null 返回 undefined） */
export function findPreset(id: string | null | undefined): WindowPresetConfig | undefined {
  if (!id) return undefined
  return allPresets.value.find(p => p.id === id)
}

/** 从磁盘重读预设（首次加载、其他窗口广播后刷新都走它） */
export async function loadWindowPresets(): Promise<void> {
  if (!store) return
  const val = await store.get<WindowPresetConfig[]>('window_presets')
  if (val && Array.isArray(val)) {
    windowPresets.value = val
  }
  const mainVal = await store.get<WindowPresetConfig>('main_window_preset')
  if (mainVal) {
    mainWindowPreset.value = { ...mainVal, id: 'main', name: mainVal.name || '主窗口' }
  }
}

let loading: Promise<void> | null = null

/**
 * 幂等初始化 + 首次加载（设置页的两个分区都可能在挂载时调用）。
 *
 * 失败不缓存：否则一次后端抖动会把 rejected 的 Promise 永久钉在这里，
 * 之后所有调用都拿到同一个错误、再也读不到配置。
 */
export function ensureWindowPresetsLoaded(): Promise<void> {
  if (!loading) {
    loading = (async () => {
      store = await useSettingsStore()
      await loadWindowPresets()
      // 「谁在运行哪个预设」也是编辑权限判定的前提，跟预设本体一起就绪
      await initOpenedPresetWindowsSync()
    })().catch((e: unknown) => {
      loading = null
      throw e
    })
  }
  return loading
}

/**
 * 写回 `window_presets`。
 *
 * 整份数组写回是这段代码最脆的地方（见文件头）：调用前必须保证内存里的
 * `windowPresets` 与磁盘一致 —— 其它窗口的改动靠事件重读，本次窗口内其它面板的改动
 * 靠共享同一份 ref（这正是本模块存在的理由）。
 */
export async function saveWindowPresets(): Promise<void> {
  if (!store) return
  await store.set('window_presets', windowPresets.value)
  await store.save()
}

/**
 * 写入主窗口预设。
 *
 * 要**同时**落 `main_window_preset`（启动时是否显示、绑定的工作区）与 `window_layout_main`
 * （插件清单 / 专注模式），漏一处就会出现「开关显示已存、下次启动不认」。
 * 主窗口现在**可以绑定工作区**：绑定的 `workspaceId` 随预设一起保存，启动时直接打开该工作区；
 * 未绑定则主窗口启动时弹出「打开时选择」（与编辑器窗口行为一致）。
 */
export async function persistMainWindowPreset(config: WindowPresetConfig): Promise<void> {
  const saved: WindowPresetConfig = { ...config, id: 'main', name: config.name || '主窗口' }
  mainWindowPreset.value = saved
  if (!store) return
  await store.set('main_window_preset', saved)
  // 同时写入 window_layout_main 供主窗口启动时读取
  await store.set('window_layout_main', {
    enabledPlugins: saved.enabledPlugins,
    focusMode: saved.focusMode,
    presetId: 'main',
  })
  await store.save()
}

/**
 * 把某个预设的插件清单写回磁盘。
 *
 * 主窗口要额外维护 `window_layout_main`，所以单独走 `persistMainWindowPreset`。
 */
export async function savePresetPlugins(presetId: string, plugins: string[]): Promise<void> {
  const preset = findPreset(presetId)
  if (!preset) return
  if (presetId === 'main') {
    await persistMainWindowPreset({ ...preset, enabledPlugins: plugins })
    return
  }
  const idx = windowPresets.value.findIndex(p => p.id === presetId)
  if (idx === -1) return
  windowPresets.value[idx] = { ...windowPresets.value[idx], enabledPlugins: plugins }
  await saveWindowPresets()
}

/**
 * 把「本窗口刚选定的工作区」记到它所属的窗口预设上（`window_presets[].workspaceId`）。
 *
 * 为什么需要：预设的「绑定工作区」是**可选**的（留空 = 打开时让用户选，见 `WindowPresetPane`
 * 的「不绑定（打开时选择）」）。用户选了之后若不写回预设，这个选择只活在当前窗口的内存里 ——
 * 关掉再打开同一个预设又弹一次选择框，用户看到的就是「已经选过的工作区没被记住」。
 * 写回后：`createPresetWindow` 与启动时的 `create_preset_window_from_preset` 都会带着它创建
 * 窗口，下次打开直接进该工作区；设置页卡片也会显示这个绑定（用户随时可以清掉）。
 *
 * 不写的情形：
 *  - **预设已经绑定了工作区**：那是用户在设置页明确写下的配置；窗口内切换工作区属运行期动作
 *    （可能是临时用一下），静默改写会让卡片上的绑定莫名其妙地变掉。要让绑定跟着走，去设置页改。
 *  - 本窗口不属于任何预设（「新建窗口」建的、辅助窗口）：没有可承载绑定的持久身份。
 * 临时工作区由调用方（`workspace` store）挡掉：它关窗即删，写进去只会留下悬空绑定。
 */
export async function rememberCurrentWindowWorkspace(workspaceId: string): Promise<void> {
  if (!workspaceId) return
  await ensureWindowPresetsLoaded()
  const preset = findPreset(await currentPresetId())
  if (!preset) return
  // 已绑定（含正好相同）→ 不动：运行期选择不覆盖设置页里的显式配置
  if (preset.workspaceId) return
  const idx = windowPresets.value.findIndex(p => p.id === preset.id)
  if (idx === -1) return
  windowPresets.value[idx] = { ...windowPresets.value[idx], workspaceId }
  await saveWindowPresets()
  // 其它窗口（窗口菜单 / 设置页）据此重读；本窗口靠共享状态，无需重读
  await publishWindowPresetsChanged()
}

// ---------------------------------------------------------------
// 本窗口显示清单（「插件管理」里的「加入 / 移出」）
//
// 它就是本窗口所属**窗口预设的 enabledPlugins**（空清单 = 显示全部）。
// 「加入 / 移出」本质是编辑该预设的插件清单的快捷方式，所以必须落盘到
// `window_presets` / `main_window_preset` —— 落盘后：本窗口立即生效，
// 以后用该预设打开的新窗口也按新清单显示。
// ---------------------------------------------------------------

/** 本窗口所属的窗口预设（辅助窗口 / 未绑定预设的窗口返回 null） */
export async function resolveCurrentWindowPreset(): Promise<WindowPresetConfig | null> {
  return findPreset(await currentPresetId()) ?? null
}

// ---------------------------------------------------------------
// 空清单 = 显示全部：展开 / 归一化
//
// 这个语义散落在四个地方实现（`layout.filteredPlugins`、`isPluginVisibleInWindow`、
// Rust 的 `update_window_presets`），而「编辑窗口预设」弹窗曾经漏了它 —— 弹窗拿
// `enabledPlugins.includes(id)` 直接画勾选框，空清单于是画成「一个都没勾」，与活动栏
// （全都在）正相反，一保存还会被「至少选择一个插件」挡住。所以展开 / 归一化在这里
// 收成一对导出函数，任何要展示或写回这份清单的地方都走它们。
// ---------------------------------------------------------------

/** 参与窗口清单的插件 ID（= 已注册即已启用的插件；排除固定底部入口如「设置」） */
export function allWindowPluginIds(): string[] {
  return pluginManager.plugins.filter(p => !p.pinnedBottom).map(p => p.id)
}

/**
 * 把「空清单 = 显示全部」展开成显式清单（原样返回非空清单的副本）。
 *
 * **展示清单时必须先展开**：`[].includes(id)` 恒为 false，直接把空清单喂给勾选框就会
 * 出现「明明全都加入了窗口，checkbox 一个都没勾」。
 */
export function expandWindowPluginIds(ids: string[]): string[] {
  return ids.length > 0 ? [...ids] : allWindowPluginIds()
}

/** 清单是否已覆盖全部插件（空清单天然算覆盖 —— 它的语义就是「全部」） */
export function coversAllWindowPlugins(ids: string[]): boolean {
  if (ids.length === 0) return true
  const all = allWindowPluginIds()
  return all.length > 0 && ids.every(x => all.includes(x)) && all.every(x => ids.includes(x))
}

/**
 * 「清单里一个不缺」⇒ 归一化回空数组（= 显示全部）。
 *
 * 空清单与「显式列出全部插件」显示上等价，但语义不同：空清单还能让**以后新装的**插件
 * 自动出现。用户把移出的插件逐个加回来后应当回到前者，否则那份显式清单会悄悄冻结住
 * 后续新装的插件（装了却在活动栏里找不到）。
 */
export function normalizeWindowPluginIds(ids: string[]): string[] {
  return coversAllWindowPlugins(ids) ? [] : [...ids]
}

/** 「加入 / 移出」的结果：成功给出操作后的可见状态，失败给出原因（调用方直接提示） */
export type WindowPluginToggleResult =
  | { ok: true; visible: boolean }
  | { ok: false; reason: string }

/**
 * 加入 / 移出本窗口活动栏的插件（= 编辑本窗口所属窗口预设的插件清单）。
 *
 * 预设侧的既有约束必须在这里同样成立，否则会写出一个「保存时会被拒」的非法预设：
 *  - 至少要有一个插件（空清单不是「一个都不要」，而是「显示全部」，见 `normalizeWindowPluginIds`）；
 *  - 专注模式恰好一个插件 ⇒ 两个方向都不可用。
 */
export async function toggleWindowPluginInCurrentWindow(
  id: string,
): Promise<WindowPluginToggleResult> {
  const layout = useStore().useLayoutStore
  const preset = await resolveCurrentWindowPreset()
  if (!preset) return { ok: false, reason: '本窗口不属于任何窗口预设，无法保存该清单' }
  if (preset.focusMode) {
    return { ok: false, reason: '专注模式下只能保留一个插件：先在「窗口设置」里关掉专注模式' }
  }
  const visible = layout.isPluginVisibleInWindow(id)
  const base = layout.resolveWindowPluginList()
  const nextRaw = visible ? base.filter(x => x !== id) : [...base, id]
  if (nextRaw.length === 0) return { ok: false, reason: '至少保留一个插件' }
  const next = normalizeWindowPluginIds(nextRaw)
  // 先改本窗口（立即生效），再落盘；两步用的是同一个清单，不会出现「界面变了、配置没变」
  layout.setWindowPluginList(next)
  await savePresetPlugins(preset.id, next)
  // 其它窗口的「窗口设置」与窗口菜单据此重读（本窗口靠共享状态，无需重读）
  await publishWindowPresetsChanged()
  return { ok: true, visible: !visible }
}
// ---------------------------------------------------------------
// 预设的「占用者」＝ 正在运行这个预设的窗口
//
// 预设的编辑权归**正在运行它的那个窗口**：一个预设同时只有一个窗口在用它（硬约束见
// `create_editor_window` 返回 409 的那段），所以「谁在用它」是唯一的。在别的窗口里编辑它，
// 写回时会把本窗口（编辑者）的运行期副本当成基准 —— 「加入 / 移出」就是这条路径 ——
// 于是把占用者刚做的改动静默顶掉。所以被别的窗口占用的预设一律不给编辑，只提供「切过去改」。
//
// 占用状态直接来自窗口注册表（`window-created` / `window-destroyed` 广播驱动刷新），
// 前端不再自己维护一份「谁开了什么」的影子状态，否则两边迟早不一致。
// ---------------------------------------------------------------

/** 正在运行某个预设的窗口（label 用于聚焦，title 用于显示） */
export interface PresetOccupier {
  label: string
  title: string
}

/** 预设 ID → 占用它的窗口 */
export const openedPresetWindows = ref<Record<string, PresetOccupier>>({})

/**
 * 从窗口列表构建「预设 → 占用窗口」表（纯函数，便于直接跑断言）。
 *
 * - `preset_id` 为空的窗口（工具菜单「新建窗口」建的）不进表：它不属于任何预设；
 * - 主窗口（`preset_id = 'main'`）**算真实占用** —— 与 Rust 的
 *   `registry::pick_window_by_preset` 刚好相反：那里判的是「该不该新建窗口」，主窗口永远
 *   只有一个且由 tauri.conf 声明，所以要排除；这里判的是「预设归谁编辑」，主窗口预设
 *   就该归主窗口，故保留；
 * - 同一预设出现多个窗口时取第一个：正常不该发生（有硬约束），真发生了也只是提示信息
 *   取哪一个的问题，不影响「被占用」这个判断本身。
 */
export function buildOpenedPresetWindows(list: WindowMeta[]): Record<string, PresetOccupier> {
  const map: Record<string, PresetOccupier> = {}
  for (const w of list) {
    const pid = w.preset_id
    if (!pid || map[pid]) continue
    map[pid] = { label: w.label, title: w.title || w.label }
  }
  return map
}

/**
 * 该预设是否正被**别的窗口**占用（是则返回占用者）。
 *
 * 本窗口就是它的运行者时不算占用 —— 否则本窗口连自己的预设都改不了
 * （设置页在每个窗口里都有入口，这条是自锁陷阱）。
 */
export function pickPresetOccupier(
  opened: Record<string, PresetOccupier>,
  presetId: string,
  selfLabel: string,
): PresetOccupier | null {
  const occupier = opened[presetId]
  if (!occupier) return null
  if (selfLabel && occupier.label === selfLabel) return null
  return occupier
}

/** 该预设是否正被别的窗口占用（读当前占用状态） */
export function presetOccupierElsewhere(presetId: string): PresetOccupier | null {
  return pickPresetOccupier(openedPresetWindows.value, presetId, currentWindowLabel())
}

/**
 * 从窗口注册表刷新占用状态。
 *
 * 读失败时**沿用上一次的值**，不清空：清空等于「没人占用」，会把编辑权放给一个不该编辑的
 * 窗口（回到「两边互相顶掉」的老问题）。宁可少放开，也不要静默放开。
 */
export async function refreshOpenedPresetWindows(): Promise<void> {
  try {
    const list = await windowService.getWindowList()
    openedPresetWindows.value = buildOpenedPresetWindows(list)
  } catch (e) {
    console.warn('[WindowPresets] 读取窗口列表失败：占用状态沿用上一次的值', e)
  }
}

let occupierInited = false
const occupierUnlisten: Unsubscribe[] = []
let occupierTimer: ReturnType<typeof setTimeout> | null = null

/** 合并密集的刷新请求（新建窗口时 `window-created` 与随后的布局写入几乎同时到达） */
function scheduleOccupierRefresh(): void {
  if (occupierTimer) return
  occupierTimer = setTimeout(() => {
    occupierTimer = null
    void refreshOpenedPresetWindows()
  }, 120)
}

/**
 * 就绪占用状态并订阅窗口增删（幂等）。
 *
 * 窗口创建 / 销毁都由 Rust `app.emit` **广播到所有窗口**（见 `lib.rs` 的
 * `on_window_event` 与 `create_editor_window`），所以任何窗口都能据此刷新。不订阅的话占用
 * 状态会停在设置页打开的那一刻 —— 那个窗口关掉后预设仍显示「使用中」，再也编辑不了。
 */
export async function initOpenedPresetWindowsSync(): Promise<void> {
  if (occupierInited) return
  occupierInited = true
  try {
    await refreshOpenedPresetWindows()
    occupierUnlisten.push(await windowService.onWindowCreated(() => scheduleOccupierRefresh()))
    occupierUnlisten.push(await windowService.onWindowDestroyed(() => scheduleOccupierRefresh()))
  } catch (e) {
    // 订阅失败要允许下次重试（否则一次抖动就永久没有自动刷新）
    occupierInited = false
    console.warn('[WindowPresets] 订阅窗口增删事件失败：占用状态不会自动刷新', e)
  }
}
