/**
 * 第三方插件 → 宿主 `ToolPlugin` 适配层。
 *
 * 「兼容 ToolPlugin」不等于「第三方要填满 ToolPlugin」：第三方只填能安全映射的子集，
 * 缺失项由宿主补成安全默认值。这里刻意**不给 sidebarPanel、不给 tabBar**：
 * - 无 `sidebarPanel` → 宿主自动把侧栏收敛到活动栏宽度、隐藏拖拽手柄、禁用面板开关
 * - 无 `tabBar` → 回落宿主默认标签条（多视图请在 mainPanel 内部自行实现）
 */
import { defineComponent, h, markRaw, type Component } from 'vue'
import type { ToolPlugin } from '@/core/pluginManager'
import type { InstalledPlugin } from './types'
import PluginIcon from '@/plugins/_sandbox/PluginIcon.vue'
import SandboxPanel from '@/plugins/_sandbox/SandboxPanel.vue'
import StandaloneLauncher from '@/plugins/_sandbox/StandaloneLauncher.vue'

/** 第三方插件默认 order 基准：永远排在内置插件（order ≤ 99）之后 */
export const THIRD_PARTY_ORDER_BASE = 1000

/** 活动栏图标：清单给描述（图标名/资源路径），宿主解析成组件 */
function createIconComponent(icon: string, pluginId: string): Component {
  return markRaw(
    defineComponent({
      name: `PluginIcon_${pluginId}`,
      setup: () => () => h(PluginIcon, { icon, pluginId }),
    }),
  )
}

/**
 * 主面板：宿主生成的壳。
 *
 * 两种形态二选一，由清单 `standaloneWindow` 决定：
 * - 默认：沙箱 iframe（插件页只能经 postMessage 与宿主通信）
 * - `standaloneWindow: true`：**启动卡**——插件界面在独立窗口里，面板不再渲染页面
 *   （否则用户会看到一份能力被削掉一半的副本，例如 `isolate` 永远 false）
 */
function createPanelComponent(plugin: InstalledPlugin): Component {
  if (plugin.standalone_window) {
    return markRaw(
      defineComponent({
        name: `StandaloneLauncher_${plugin.id}`,
        setup: () => () => h(StandaloneLauncher, { pluginId: plugin.id }),
      }),
    )
  }
  return markRaw(
    defineComponent({
      name: `SandboxPanel_${plugin.id}`,
      setup: () => () => h(SandboxPanel, { pluginId: plugin.id, entry: plugin.entry }),
    }),
  )
}

/** 把一条已安装插件记录适配成宿主插件对象 */
export function toToolPlugin(plugin: InstalledPlugin, index: number): ToolPlugin {
  return {
    id: plugin.id,
    label: plugin.name,
    description: plugin.description || `第三方插件 v${plugin.version}`,
    icon: createIconComponent(plugin.icon, plugin.id),
    mainPanel: createPanelComponent(plugin),
    order: THIRD_PARTY_ORDER_BASE + index,
    source: 'third-party',
    standaloneWindow: plugin.standalone_window === true,
  }
}
