/**
 * 「独立窗口型插件」的统一入口。
 *
 * 谁需要它：任何「点击插件入口」的地方 —— 活动栏图标（`AppSidebar`）、
 * 宿主生成的启动卡（`_sandbox/StandaloneLauncher.vue`）、「3D 与游戏」容器。
 *
 * 为什么不让调用方自己拼：入口路径与显示名都在已安装记录里（`InstalledPlugin`），
 * 而 `window.ts` 刻意不依赖注册表（避免 `registry → adapter → window` 的循环）。
 * 于是把「查记录 → 开窗口 → 失败提示」收在这里，调用方只认插件 ID。
 */
import NotificationUtil from '@/utils/NotificationUtil'
import { installedPlugins } from './registry'
import { openPluginWindow } from './window'
import type { InstalledPlugin } from './types'

/** 按 ID 找已安装的第三方插件（找不到返回 undefined） */
export function findInstalledPlugin(pluginId: string): InstalledPlugin | undefined {
  return installedPlugins.value.find(p => p.id === pluginId)
}

/**
 * 打开（或前置）插件的独立窗口。
 *
 * **不抛异常**：失败时写日志 + 提示用户后返回 false。调用方多为点击处理器，
 * 抛出去只会变成「点了没反应」的哑 bug。
 *
 * @returns 是否成功发起（false = 插件记录缺失或创建失败）
 */
export async function launchStandalonePlugin(pluginId: string): Promise<boolean> {
  const plugin = findInstalledPlugin(pluginId)
  if (!plugin) {
    console.error(`[Plugin] 找不到插件记录，无法打开独立窗口: ${pluginId}`)
    NotificationUtil.error(`插件 ${pluginId} 已不存在，请刷新插件列表`)
    return false
  }
  try {
    await openPluginWindow(plugin.id, plugin.entry, `${plugin.name} · 独立窗口`)
    return true
  } catch (e) {
    console.error(`[Plugin] 打开「${plugin.name}」独立窗口失败`, e)
    NotificationUtil.error(`打开「${plugin.name}」独立窗口失败`)
    return false
  }
}
