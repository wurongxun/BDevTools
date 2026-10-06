/**
 * 插件模型 → 贡献条目的投影。
 *
 * 唯一职责：把 `ToolPlugin` 上那一堆「宿主约定的字段」（icon / sidebarPanel / mainPanel /
 * dockPanel / tabBar）翻译成注册表认识的贡献条目。注册表不认识 `ToolPlugin`，
 * 插件也不认识注册表 —— 这个翻译层就是两者之间的适配面。
 *
 * 将来插件改用清单声明式贡献（`contributes.views`）时，只要再写一个投影函数从清单生成条目，
 * 下游 UI 一行都不用改。
 */
import type { ToolPlugin } from '@/core/pluginManager'
import { contributionRegistry } from '@/core/registry'
import type { Contribution } from '@/core/registry'

/** 条目 ID 约定（`<pluginId>:<局部名>`），撤销与查询都靠它定位 */
export const entryId = (pluginId: string, local: string): string => `${pluginId}:${local}`

/**
 * 把一个插件翻译成贡献条目。
 *
 * 返回的条目不带 `pluginId`（由 `contribute` 统一补），保证同一个插件不可能把条目记到别人名下。
 */
export function projectPlugin(plugin: ToolPlugin): Omit<Contribution, 'pluginId'>[] {
  const items: Omit<Contribution, 'pluginId'>[] = []

  // 活动栏图标是插件的**入口**：没有它就没有任何办法切换到该插件，因此无条件注册。
  // 其余面板都是可选的 —— 没有就说明该插件不占那个槽位。
  items.push({
    kind: 'activityBarItem',
    id: entryId(plugin.id, 'activity'),
    payload: {
      icon: plugin.icon,
      label: plugin.label,
      pinnedBottom: plugin.pinnedBottom === true,
      dockPosition: plugin.dockPanel?.position,
      standaloneWindow: plugin.standaloneWindow === true,
    },
  })

  if (plugin.sidebarPanel) {
    items.push({
      kind: 'view',
      id: entryId(plugin.id, 'sidebar'),
      payload: {
        slot: 'sidebar',
        component: plugin.sidebarPanel,
        minWidth: plugin.sidebarMinWidth,
        maxWidth: plugin.sidebarMaxWidth,
        defaultWidth: plugin.sidebarDefaultWidth,
      },
    })
  }

  if (plugin.mainPanel) {
    items.push({
      kind: 'view',
      id: entryId(plugin.id, 'main'),
      payload: { slot: 'main', component: plugin.mainPanel },
    })
  }

  if (plugin.dockPanel) {
    items.push({
      kind: 'view',
      id: entryId(plugin.id, 'dock'),
      payload: {
        slot: 'dock',
        component: plugin.dockPanel.component,
        position: plugin.dockPanel.position,
        defaultHeight: plugin.dockPanel.defaultHeight,
        minHeight: plugin.dockPanel.minHeight,
      },
    })
  }

  if (plugin.tabBar) {
    items.push({
      kind: 'view',
      id: entryId(plugin.id, 'tabBar'),
      payload: { slot: 'tabBar', component: plugin.tabBar },
    })
  }

  return items
}

/**
 * 把插件的贡献写进注册表（幂等）。
 *
 * 由 `pluginManager` 在注册 / 注销时调用：**注册入口只有那一处**，第三方插件与内置插件
 * 都走同一条路径，不会出现"注册了但没贡献"的中间态。
 */
export function contributePlugin(plugin: ToolPlugin): void {
  contributionRegistry.contribute(plugin.id, projectPlugin(plugin))
}

/** 撤销插件的全部贡献（插件停用 / 卸载） */
export function withdrawPlugin(pluginId: string): void {
  contributionRegistry.withdraw(pluginId)
}
