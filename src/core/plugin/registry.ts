/**
 * 第三方插件注册表（宿主侧唯一事实源）。
 *
 * 职责：
 * 1. 从后端拉取已安装插件（含被禁用的）并同步进 `pluginManager`
 * 2. 维护「内置 + 第三方」统一排序（order 统一序列，持久化在后端 settings.json）
 * 3. 提供安装 / 卸载 / 启用禁用 / 上移下移的封装，供设置页调用
 */
import { computed, ref } from 'vue'
import { pluginManager } from '@/core/pluginManager'
import type { ToolPlugin } from '@/core/pluginManager'
import { publish, subscribe } from '@/core/runtime'
import { activationMsOf } from '@/core/plugin/activation'
import * as pluginApi from './api'
import { toToolPlugin } from './adapter'
import { initPluginAssetOrigin } from './url'
import { closePluginWindow } from './window'
import type { CallStats, InstalledPlugin, PluginListItem, PluginRuntimeInfo } from './types'

/** 已安装第三方插件（含 disabled；disabled 的不注册进 pluginManager） */
const installed = ref<InstalledPlugin[]>([])

/**
 * 内置插件的**静态清单**（由 `initPluginHost` 注入，来源见 `@/plugins`）。
 *
 * 为什么不直接用 `pluginManager.plugins`：内置插件现在也支持启用/禁用，禁用会把它从
 * `pluginManager` 里**注销**（活动栏与布局都不再为它留位）。如果「有哪些内置插件」也取自
 * `pluginManager.plugins`，被禁用的内置插件连设置页的卡片都会一起消失 —— 那个把它重新
 * 打开的开关就再也找不到了。**「有哪些插件」是静态事实，「当前启用了哪些」是运行时状态**。
 */
const builtinPlugins = ref<ToolPlugin[]>([])

/**
 * 被禁用的插件 ID（内置 + 第三方**同一集合**，持久化在后端 settings.json 的 `plugin_disabled`）。
 *
 * 唯一事实源：不再各看各的（内置看源码常量、第三方看 `InstalledPlugin.enabled`）——
 * 那本来就是同一份后端数据的两种投影，分开维护迟早不一致。
 * 读失败时按空集合降级：宁可全部启用，也不要因一次读取失败让所有插件集体消失。
 */
const disabledIds = ref<Set<string>>(new Set())
/** 后端持久化的统一排序（ID 序列） */
const savedOrder = ref<string[]>([])
/** WASM 运行时信息（首次读取失败时按「未启用」降级展示） */
const runtime = ref<PluginRuntimeInfo | null>(null)
/**
 * 各插件「上一次调用」的资源占用（插件 ID → 统计）。
 * 只做**展示缓存**：宿主侧只在本次会话真正调用过才有记录，没有就是没记录（不伪造 0）。
 */
const stats = ref<Record<string, CallStats>>({})

/** 已安装第三方插件（只读视图） */
export const installedPlugins = installed

/** WASM 运行时信息（只读视图；null = 尚未读到） */
export const pluginRuntimeInfo = runtime

/** 各插件上一次调用的资源占用（只读视图） */
export const pluginLastStats = stats

/** 重新读取 WASM 运行时信息（失败不抛，按未启用降级） */
export async function refreshPluginRuntimeInfo(): Promise<void> {
  try {
    runtime.value = await pluginApi.fetchPluginRuntimeInfo()
  } catch (e) {
    console.warn('[PluginRegistry] 读取插件运行时信息失败', e)
    runtime.value = {
      available: false,
      runtime: '',
      fuel: 0,
      timeout_ms: 0,
      max_memory_mb: 0,
      max_http_response_mb: 0,
      max_fs_read_mb: 0,
      max_fs_chunk_mb: 0,
      max_fs_handles: 0,
      permissions: [],
      memdump_on_error: false,
    }
  }
}

/**
 * 拉取某插件上一次调用的资源占用并写入缓存。
 * 返回最新值（宿主没有记录时返回 `null`，调用方据此显示「暂无记录」）。
 */
export async function refreshPluginLastStats(id: string): Promise<CallStats | null> {
  const next = await pluginApi.fetchPluginLastStats(id)
  const cur = { ...stats.value }
  if (next) cur[id] = next
  else delete cur[id]
  stats.value = cur
  return next
}

/**
 * 设置「插件调用出错时导出线性内存快照」开关。
 * 开关是**编译期**选项（已折进编译缓存指纹），切换后下一次调用自动重编，无需手工清缓存；
 * 成功后重新拉运行时信息，让界面上的开关状态与后端一致。
 */
export async function setPluginMemdumpEnabled(enabled: boolean): Promise<void> {
  await pluginApi.setPluginMemdump(enabled)
  await refreshPluginRuntimeInfo()
}

// ==================== 插件依赖 ====================
// 依赖是「声明的关系」，不是「打包的关系」：宿主据此阻止停用别人还依赖着的插件，
// 并在依赖缺失时给出明确提示。刻意**不**自动启用/安装依赖 —— 一次点击产生一串隐式
// 副作用（连带启用、连带拉起窗口）比多一步手动操作难排查得多。

/** 全部已知插件 ID（内置静态清单 + 已安装第三方） */
function knownIds(): string[] {
  return [...builtinPlugins.value.map(p => p.id), ...installed.value.map(p => p.id)]
}

/** 某插件声明的依赖（内置取自静态清单，第三方取自安装记录） */
function declaredDeps(id: string): string[] {
  return (
    builtinPlugins.value.find(p => p.id === id)?.dependencies ??
    installed.value.find(p => p.id === id)?.dependencies ??
    []
  )
}

/** 依赖了 `id` 且**当前处于启用状态**的插件（停用保护用） */
export function dependentsOf(id: string): string[] {
  return knownIds().filter(
    other => other !== id && !disabledIds.value.has(other) && declaredDeps(other).includes(id),
  )
}

/**
 * `id` 声明了但当前不可用的依赖：没打包（内置） / 没安装（第三方） / 被停用。
 *
 * 内置插件「没打包」也能查出来：静态清单 `builtinPlugins` 只包含本次编译进来的插件，
 * 裁剪掉的插件不在里面 —— 这正是「只打包 database 却裁了 files」时该报警的地方。
 */
export function missingDepsOf(id: string): string[] {
  const known = new Set(knownIds())
  return declaredDeps(id).filter(d => !known.has(d) || disabledIds.value.has(d))
}

/** 当前生效的插件 ID 顺序：内置在前（按已生效顺序），其后为已启用的第三方 */
function effectiveIds(): string[] {
  // 固定底部的宿主入口（如「设置」）不进入可排序序列
  const builtinIds = pluginManager.plugins
    .filter(p => p.source !== 'third-party' && !p.pinnedBottom)
    .map(p => p.id)
  const thirdIds = installed.value.filter(p => !disabledIds.value.has(p.id)).map(p => p.id)
  return [...builtinIds, ...thirdIds]
}

/**
 * 合并「后端保存的顺序」与「当前存在的插件」得到最终顺序：
 * 保存的顺序优先（过滤掉已不存在的 ID），未记录的新插件按默认顺序追加到末尾。
 */
export function computeOrder(): string[] {
  const all = effectiveIds()
  const known = new Set(all)
  const head = savedOrder.value.filter(id => known.has(id))
  const headSet = new Set(head)
  return [...head, ...all.filter(id => !headSet.has(id))]
}

/**
 * 展示用顺序：生效顺序 + 末尾追加「当前未启用」的插件（内置与第三方）。
 *
 * 末尾这一批是设置页能把它们重新打开的前提：它们不在 `pluginManager` 里，
 * 只能靠内置静态清单与已安装列表补齐（详见 `builtinPlugins` 的注释）。
 */
export const displayOrder = computed<string[]>(() => {
  const base = computeOrder()
  const seen = new Set(base)
  const rest = [
    ...builtinPlugins.value.filter(p => !seen.has(p.id)).map(p => p.id),
    ...installed.value.filter(p => !seen.has(p.id)).map(p => p.id),
  ]
  return [...base, ...rest]
})

/** 统一列表（内置 + 第三方），顺序即当前生效顺序 */
export const pluginListItems = computed<PluginListItem[]>(() => {
  const items = new Map<string, PluginListItem>()
  // 内置插件取自静态清单而不是 pluginManager：被禁用的内置插件已经注销，
  // 但设置页必须仍然能看到它、并把开关拨回去
  for (const p of builtinPlugins.value) {
    items.set(p.id, {
      id: p.id,
      label: p.label,
      source: 'builtin',
      description: p.description ?? '',
      version: '内置',
      permissions: [],
      settings: [],
      pageCapabilities: [],
      deniedPageCapabilities: [],
      standaloneWindow: false,
      pinned: p.pinnedBottom === true,
      enabled: !disabledIds.value.has(p.id),
      hasWasm: false,
      sizeBytes: 0,
      installedAt: '',
      lastStats: null,
      dependencies: p.dependencies ?? [],
      missingDependencies: missingDepsOf(p.id),
      activationMs: activationMsOf(p.id),
    })
  }
  for (const p of installed.value) {
    items.set(p.id, {
      id: p.id,
      label: p.name,
      source: 'third-party',
      description: p.description,
      version: p.version,
      permissions: p.permissions,
      settings: p.settings ?? [],
      pageCapabilities: p.page_capabilities ?? [],
      // 只可能 ⊆ pageCapabilities：后端按白名单校验过，脏值在读的时候就被过滤掉了
      deniedPageCapabilities: p.denied_page_capabilities ?? [],
      standaloneWindow: p.standalone_window === true,
      pinned: false,
      // 与内置同一口径：以禁用集合为准，不看后端那条已经重复表达同一事实的 enabled
      enabled: !disabledIds.value.has(p.id),
      hasWasm: p.has_wasm,
      sizeBytes: p.size_bytes,
      installedAt: p.installed_at,
      lastStats: stats.value[p.id] ?? null,
      dependencies: p.dependencies ?? [],
      missingDependencies: missingDepsOf(p.id),
      activationMs: activationMsOf(p.id),
    })
  }
  return displayOrder.value
    .map(id => items.get(id))
    .filter((x): x is PluginListItem => Boolean(x))
})

/**
 * 按启用状态同步注册：启用中的（内置 + 第三方）注册进 `pluginManager`，
 * 禁用/已卸载的注销，最后按统一顺序重排。
 *
 * 内置与第三方共用这一份逻辑 —— 它们本来就只是同一张启用状态表的两个来源。
 * 注销会触发插件的 `onDeactivate`（如终端释放 PTY），注册会触发 `onActivate`，
 * 所以禁用内置插件同样是一次真正的「停用」，不只是隐藏入口。
 */
function syncRegistrations() {
  const wanted = new Map<string, ToolPlugin>()
  for (const p of builtinPlugins.value) {
    if (!disabledIds.value.has(p.id)) wanted.set(p.id, p)
  }
  installed.value
    .filter(p => !disabledIds.value.has(p.id))
    .forEach((p, i) => {
      wanted.set(p.id, toToolPlugin(p, i))
    })

  for (const p of [...pluginManager.plugins]) {
    if (!wanted.has(p.id)) pluginManager.unregister(p.id)
  }
  for (const [id, p] of wanted) {
    if (!pluginManager.hasPlugin(id)) pluginManager.register(p)
  }
  pluginManager.applyOrder(computeOrder())
}

/** 该 ID 是否是内置插件（启用状态已统一，这里只用于决定「要不要重扫插件目录」） */
export function isBuiltinPluginId(id: string): boolean {
  return builtinPlugins.value.some(p => p.id === id)
}

/** 重新读取「被禁用的插件 ID」并同步注册状态 */
export async function refreshDisabledPlugins(): Promise<void> {
  disabledIds.value = new Set(await pluginApi.fetchDisabledPlugins())
  syncRegistrations()
}

/** 重新拉取第三方插件列表并同步注册状态 */
export async function refreshThirdPartyPlugins(): Promise<void> {
  installed.value = await pluginApi.fetchInstalledPlugins()
  syncRegistrations()
}

/** 重新读取统一排序并应用 */
export async function refreshPluginOrder(): Promise<void> {
  savedOrder.value = await pluginApi.fetchPluginOrder()
  pluginManager.applyOrder(computeOrder())
}

/**
 * 首次初始化：注入内置清单 → 探测资源地址前缀 → 读排序 / 启用状态 / 第三方插件 → 注册。
 *
 * `builtins` 由调用方（`@/plugins`）传入而不是在这里 import：内置插件的模块又（间接）
 * 依赖本模块，静态互相 import 会绕成一个环，把清单当参数传可以把依赖保持单向。
 *
 * 任一步失败都只降级（不阻塞应用启动）：插件体系坏掉不应该让主界面打不开。
 */
export async function initPluginHost(builtins: ToolPlugin[]): Promise<void> {
  builtinPlugins.value = builtins
  await initPluginAssetOrigin()
  await refreshPluginRuntimeInfo()
  try {
    savedOrder.value = await pluginApi.fetchPluginOrder()
  } catch (e) {
    console.warn('[PluginRegistry] 读取插件排序失败，使用默认顺序', e)
  }
  // 启用状态必须在注册之前拿到：它决定内置插件注册哪些
  try {
    disabledIds.value = new Set(await pluginApi.fetchDisabledPlugins())
  } catch (e) {
    console.warn('[PluginRegistry] 读取插件启用状态失败，按全部启用处理', e)
  }
  try {
    installed.value = await pluginApi.fetchInstalledPlugins()
  } catch (e) {
    console.warn('[PluginRegistry] 读取第三方插件失败', e)
    installed.value = []
  }
  syncRegistrations()

  // 订阅别的窗口发来的插件变更：本窗口同步停用/关掉对应插件。失败只降级。
  try {
    await subscribe<PluginStateChangePayload>(PLUGIN_STATE_CHANGED_EVENT, (payload) => {
      void applyPluginStateChange(payload)
    })
  } catch (e) {
    console.warn('[PluginRegistry] 订阅插件状态变更事件失败，跨窗口同步不可用', e)
  }
}

// ==================== 跨窗口同步 ====================
// 插件启用状态与排序都持久化在后端 settings.json，但 `pluginManager` 是**每个窗口各自
// 一份**实例：在 A 窗口禁用一个插件，B 窗口不会知道 —— 它的活动栏、面板、甚至该插件的
// 独立窗口都会继续开着（被关掉的插件还在另一个窗口里运行）。所以任何状态变更都广播一次，
// 其余窗口按磁盘真实状态重新同步注册（注销即触发 `onDeactivate`），并关掉该插件的独立窗口。

/** 插件状态变更事件名（启用/禁用、安装/卸载、排序） */
export const PLUGIN_STATE_CHANGED_EVENT = 'plugin-state-changed'

/** 插件设置被修改后的广播（设置页 → 已打开的插件面板；payload: `{ id, values }`） */
export const PLUGIN_SETTINGS_CHANGED_EVENT = 'plugin-settings-changed'

export type PluginStateChangeKind = 'toggle' | 'install' | 'uninstall' | 'order'

export interface PluginStateChangePayload {
  /** 发生变更的插件 ID */
  id: string
  kind: PluginStateChangeKind
}

/** 广播插件状态变更（失败只告警：跨窗口同步坏了不该让本窗口的操作也跟着失败） */
function publishPluginState(payload: PluginStateChangePayload): void {
  publish(PLUGIN_STATE_CHANGED_EVENT, payload).catch((e: unknown) => {
    console.warn('[PluginRegistry] 广播插件状态变更失败', e)
  })
}

/**
 * 响应（别的窗口发来的，或本窗口自己发的）插件状态变更：本窗口也不该再留着这个插件。
 *
 * 幂等 —— 发起方同样会收到这条广播，重复刷新只是多几次读取，换来的是「同步逻辑只有一处」。
 */
async function applyPluginStateChange(payload: PluginStateChangePayload): Promise<void> {
  if (!payload?.id) return
  // 独立窗口（web3d-<id>）是系统级窗口，哪个窗口都关得掉；先关它，再同步注册
  try {
    await closePluginWindow(payload.id)
  } catch (e) {
    console.warn(`[PluginRegistry] 关闭插件 ${payload.id} 的独立窗口失败`, e)
  }
  try {
    if (payload.kind === 'order') {
      await refreshPluginOrder()
      return
    }
    // 卸载后残留的统计缓存要清掉，否则同名插件重装后旧数据会显示出来
    if (payload.kind === 'uninstall' && stats.value[payload.id]) {
      const cur = { ...stats.value }
      delete cur[payload.id]
      stats.value = cur
    }
    // 启用状态每次都从磁盘重读；装卸/安装还要重扫插件目录（体积与 WASM 产物会变），
    // 并重读统一排序（安装/卸载都会改变那条持久化序列）
    await refreshDisabledPlugins()
    if (payload.kind !== 'toggle') {
      await refreshThirdPartyPlugins()
      await refreshPluginOrder()
    }
  } catch (e) {
    console.warn('[PluginRegistry] 同步其他窗口的插件变更失败', e)
  }
}

/** 把当前顺序整体落盘（安装/卸载后调用，保证新插件位置被记住） */
async function persistOrder(): Promise<void> {
  const ids = computeOrder()
  await pluginApi.savePluginOrder(ids)
  savedOrder.value = ids
  pluginManager.applyOrder(ids)
}

/** 安装插件（source 为插件目录或 .zip 路径），返回安装记录 */
export async function installThirdPartyPlugin(source: string): Promise<InstalledPlugin> {
  const item = await pluginApi.installPlugin(source)
  await refreshThirdPartyPlugins()
  await persistOrder()
  publishPluginState({ id: item.id, kind: 'install' })
  return item
}

/** 卸载插件；`removeData=true` 时一并清理其私有数据目录 */
export async function uninstallThirdPartyPlugin(id: string, removeData: boolean): Promise<void> {
  await pluginApi.uninstallPlugin(id, removeData)
  pluginManager.unregister(id)
  // 卸载后残留的统计缓存要清掉，否则同名插件的旧数据会在重装后显示出来
  if (stats.value[id]) {
    const cur = { ...stats.value }
    delete cur[id]
    stats.value = cur
  }
  await refreshThirdPartyPlugins()
  publishPluginState({ id, kind: 'uninstall' })
}

/**
 * 启用 / 禁用插件（内置与第三方统一；宿主常驻入口会被后端拒绝）。
 *
 * 分支只为省一次目录扫描：第三方要重拉列表（体积、WASM 产物可能刚被回收），
 * 内置只需重读启用状态后本地重排，切开关不会卡一下。
 */
export async function setPluginEnabled(id: string, enabled: boolean): Promise<void> {
  // 停用保护：别人还依赖着就不能停。放在最前面，避免「后端已经停了、前端才报错」的半截状态。
  // 反过来（启用）不检查依赖：依赖缺失时插件仍可启用，只是设置页会标出「缺少依赖」，
  // 由用户自己决定要不要先补上 —— 强行连带启用会把一次点击变成一串隐式副作用。
  if (!enabled) {
    const blockers = dependentsOf(id)
    if (blockers.length) {
      throw new Error(`无法停用「${id}」：${blockers.join('、')} 仍依赖它，请先停用这些插件`)
    }
  }
  await pluginApi.setPluginEnabled(id, enabled)
  if (isBuiltinPluginId(id)) {
    await refreshDisabledPlugins()
  } else {
    await refreshDisabledPlugins()
    await refreshThirdPartyPlugins()
  }
  // 让其它窗口也把这个插件停掉（含关掉它的独立窗口）
  publishPluginState({ id, kind: 'toggle' })
}

/**
 * 收紧某插件的页面能力（`denied` 传空数组 = 恢复插件自己的声明）。
 *
 * **只能减不能加**：用户不能替插件声明它没要过的 `pageCapabilities`。
 *
 * 刻意**不**广播跨窗口状态变更：那条广播会顺手关掉该插件的独立窗口
 * （见 `applyPluginStateChange`），而改一项能力显然不该把用户开着的窗口关掉。
 * CSP 是按响应算的，已打开的插件页重新打开才会拿到新策略 —— 设置页里会明确提示这一点。
 */
export async function setPluginPageCapsDenied(id: string, denied: string[]): Promise<void> {
  await pluginApi.setPluginPageCaps(id, denied)
  await refreshThirdPartyPlugins()
}

/**
 * 在统一顺序里上移/下移一个插件（delta = -1 / +1）。
 * 返回是否真的移动了（到边界时返回 false，调用方据此禁用按钮）。
 */
export async function movePluginInOrder(id: string, delta: number): Promise<boolean> {
  const ids = computeOrder()
  const from = ids.indexOf(id)
  const to = from + delta
  if (from < 0 || to < 0 || to >= ids.length) return false
  const next = [...ids]
  next[from] = ids[to]
  next[to] = id
  await pluginApi.savePluginOrder(next)
  savedOrder.value = next
  pluginManager.applyOrder(next)
  publishPluginState({ id, kind: 'order' })
  return true
}
