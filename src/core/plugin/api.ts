/**
 * 第三方插件后端命令封装（唯一入口：`@/api/rpc`）。
 *
 * 对应命令：plugin_list / plugin_install / plugin_uninstall /
 * plugin_set_enabled / plugin_get_order / plugin_set_order / plugin_call
 */
import { call } from '@/api/rpc'
import type { ApiResp, CallStats, InstalledPlugin, PluginRuntimeInfo } from './types'

/** 已安装的第三方插件列表（内置插件不在此列） */
export async function fetchInstalledPlugins(): Promise<InstalledPlugin[]> {
  const res = await call<ApiResp<InstalledPlugin[]>>('plugin_list', {})
  if (res?.code === 200) return res.data ?? []
  throw new Error(res?.msg || '读取插件列表失败')
}

/** 安装插件：`source` 为插件目录或 .zip 包路径；同名插件覆盖安装（升级） */
export async function installPlugin(source: string): Promise<InstalledPlugin> {
  const res = await call<ApiResp<InstalledPlugin>>('plugin_install', { source })
  if (res?.code === 200) return res.data
  throw new Error(res?.msg || '安装插件失败')
}

/** 卸载插件（`removeData=true` 时一并清理该插件在所有工作区的私有数据） */
export async function uninstallPlugin(id: string, removeData: boolean): Promise<void> {
  const res = await call<ApiResp<boolean>>('plugin_uninstall', { id, removeData })
  if (res?.code !== 200) throw new Error(res?.msg || '卸载插件失败')
}

/** 启用 / 禁用插件（内置与第三方同一套；宿主常驻入口会被后端拒绝） */
export async function setPluginEnabled(id: string, enabled: boolean): Promise<void> {
  const res = await call<ApiResp<boolean>>('plugin_set_enabled', { id, enabled })
  if (res?.code !== 200) throw new Error(res?.msg || '更新插件状态失败')
}

/** 被禁用的插件 ID（内置 + 第三方统一集合；空数组 = 全部启用） */
export async function fetchDisabledPlugins(): Promise<string[]> {
  const res = await call<ApiResp<string[]>>('plugin_get_disabled', {})
  if (res?.code === 200) return res.data ?? []
  throw new Error(res?.msg || '读取插件启用状态失败')
}

/** 读取统一排序（内置 + 第三方插件 ID 的全量顺序） */
export async function fetchPluginOrder(): Promise<string[]> {
  const res = await call<ApiResp<string[]>>('plugin_get_order', {})
  if (res?.code === 200) return res.data ?? []
  throw new Error(res?.msg || '读取插件排序失败')
}

/** 保存统一排序（内置 + 第三方插件 ID 的全量顺序） */
export async function savePluginOrder(order: string[]): Promise<void> {
  const res = await call<ApiResp<boolean>>('plugin_set_order', { order })
  if (res?.code !== 200) throw new Error(res?.msg || '保存插件排序失败')
}

/**
 * 调用插件后端逻辑。
 * 不抛异常：把失败统一折叠成 `ApiResp`（code != 200），
 * 沙箱面板据此把结构化错误回给插件页面。
 *
 * code 语义：200 成功；501 插件没有 WASM 后端或运行时未启用；500 执行期错误。
 */
export async function callPlugin(
  id: string,
  func: string,
  input: unknown,
): Promise<ApiResp<string>> {
  try {
    return await call<ApiResp<string>>('plugin_call', {
      id,
      func,
      input: JSON.stringify(input ?? null),
    })
  } catch (e) {
    return { code: 500, data: '', msg: e instanceof Error ? e.message : String(e) }
  }
}

/** WASM 运行时信息（是否启用 + 资源限制 + 支持的权限声明形态） */
export async function fetchPluginRuntimeInfo(): Promise<PluginRuntimeInfo> {
  const res = await call<ApiResp<PluginRuntimeInfo>>('plugin_runtime_info', {})
  if (res?.code === 200 && res.data) return res.data
  throw new Error(res?.msg || '读取插件运行时信息失败')
}

/**
 * 某插件上一次调用的资源占用；宿主本次会话内没调用过则返回 `null`。
 * 不抛异常：拿不到统计信息不该打断设置页渲染。
 */
export async function fetchPluginLastStats(id: string): Promise<CallStats | null> {
  try {
    const res = await call<ApiResp<CallStats | null>>('plugin_last_stats', { id })
    if (res?.code === 200) return res.data ?? null
    return null
  } catch (e) {
    console.warn('[PluginApi] 读取插件调用统计失败', e)
    return null
  }
}

/** 插件声明的 `fs` 作用域根目录（已展开 `$HOME`、已规范化；宿主侧解析） */
export interface PluginFsRoots {
  /** `fs:read[:作用域]` 的根目录（不带作用域 = 插件私有数据目录） */
  read: string[]
  /** `fs:write[:作用域]` 的根目录 */
  write: string[]
}

/**
 * 读取插件的 `fs` 作用域根目录。
 *
 * 不抛异常：拿不到就返回空数组 —— 空作用域意味着「对话框一律拒绝」，
 * 这正好是权限校验失败时想要的结果（失败必须收敛成拒绝，不能收敛成放行）。
 */
export async function fetchPluginFsRoots(id: string): Promise<PluginFsRoots> {
  try {
    const res = await call<ApiResp<PluginFsRoots>>('plugin_fs_roots', { id })
    if (res?.code === 200 && res.data) return res.data
  } catch (e) {
    console.warn('[PluginApi] 读取插件 fs 作用域失败', e)
  }
  return { read: [], write: [] }
}

/** 插件设置的当前值：已存的优先，没有的键回落到清单 default */
export async function fetchPluginSettings(id: string): Promise<Record<string, unknown>> {
  const res = await call<ApiResp<Record<string, unknown>>>('plugin_get_settings', { id })
  if (res?.code === 200) return res.data ?? {}
  throw new Error(res?.msg || '读取插件设置失败')
}

/** 写入插件设置值（宿主按清单定义校验类型与范围；只覆盖提交的键） */
export async function savePluginSettings(
  id: string,
  values: Record<string, unknown>,
): Promise<void> {
  const res = await call<ApiResp<boolean>>('plugin_set_settings', {
    id,
    values: JSON.stringify(values),
  })
  if (res?.code !== 200) throw new Error(res?.msg || '保存插件设置失败')
}

/** 设置「插件调用出错时导出线性内存快照」的调试开关（写 settings.json，下次调用自动重编生效） */
export async function setPluginMemdump(enabled: boolean): Promise<void> {
  const res = await call<ApiResp<boolean>>('plugin_set_memdump', { enabled })
  if (res?.code !== 200) throw new Error(res?.msg || '更新出错快照开关失败')
}
