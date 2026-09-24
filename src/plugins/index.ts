import type { ToolPlugin } from '@/core/pluginManager'
import { initPluginHost } from '@/core/plugin/registry'
import filesPlugin from './files'
import sshPlugin from './ssh'
import databasePlugin from './database'
import logsPlugin from './logs'
import matPlugin from './mat'
import terminalPlugin from './terminal'
import placeholderPlugin from './placeholder'
import web3dPlugin from './web3d'
import settingsPlugin from './settings'

/**
 * 全部内置插件（**静态清单**，随应用编译）。
 *
 * 与「当前注册了哪些插件」是两件事，不能合并：内置插件也支持启用/禁用，禁用会把它从
 * `pluginManager` **注销**（活动栏与布局都不再为它留位）。如果设置页的可见性只看
 * `pluginManager.plugins`，被禁用的内置插件连卡片都会消失 —— 那个把它开回来的开关
 * 就再也找不到了。清单还要传给宿主侧注册表（`initPluginHost`）作为禁用开关的作用域。
 */
export const BUILTIN_PLUGINS: ToolPlugin[] = [
  filesPlugin,
  sshPlugin,
  logsPlugin,
  databasePlugin,
  matPlugin,
  // 终端：停靠面板插件（底部面板，活动栏图标 = 显隐开关，不占主区）
  terminalPlugin,
  // 占位插件：刻意不提供 sidebarPanel，用于验证「无侧栏插件」的布局适配（验证完可删）
  placeholderPlugin,
  // 3D 与游戏：独立窗口型插件（pageCapabilities / standaloneWindow）的集中入口与兜底
  web3dPlugin,
  // 设置：固定在活动栏底部的宿主入口（pinnedBottom，不可禁用；主面板复用 AppSetting.vue）
  settingsPlugin,
]

/**
 * 初始化插件体系：把内置清单交给注册表 → 按「启用状态」注册内置与第三方 → 应用统一排序。
 *
 * 必须在应用挂载前 `await`：启用状态与排序要先就位，否则首屏会先按默认形态渲染、
 * 随后跳变成保存的状态。任一步失败都只降级告警，不影响主界面启动。
 */
export async function initPlugins() {
  await initPluginHost(BUILTIN_PLUGINS)
}
