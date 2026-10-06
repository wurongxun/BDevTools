import { defineStore } from 'pinia'
import { pluginManager, type ToolPlugin } from '@/core/pluginManager'
import { contributionRegistry } from '@/core/registry'
import { getCurrentWindow, publish } from '@/core/runtime'
import { IMMERSIVE_STATE_EVENT } from '@/composables/useImmersiveShortcut'

/**
 * 窗口布局配置 store（per-window 隔离）
 * 每个窗口拥有独立的 Pinia 实例，天然隔离
 */
const useLayoutStore = defineStore('layout', {
  state: () => ({
    /**
     * 当前窗口**显示**的插件 ID 列表（空数组 = 显示全部）。
     *
     * 来源是窗口预设的 `enabledPlugins`（见 `applyPreset`），之后用户可以在本窗口的
     * 插件管理页用「加入 / 移出」**只改本窗口**。所以它是「窗口自己的显示清单」，
     * 与预设里那份是两件事：预设改动影响的是以后新建的窗口，这里只是本窗口的当前视图。
     */
    enabledPlugins: [] as string[],
    /** 专注模式：隐藏侧边栏 */
    focusMode: false,
    /**
     * 沉浸式：只留插件界面（插件侧栏面板 + 主面板），隐藏标题栏 / 活动栏 / 状态栏。
     *
     * 与 `focusMode` 的区别（两者都会隐藏活动栏，别混）：
     * - `focusMode` 由**窗口预设**声明，面板固定显示**第一个**启用插件；
     * - `immersiveMode` 是**用户随时切的临时视图态**（不持久化、不写预设），保留**当前激活**插件，
     *   进入时把折叠的侧边栏与插件面板展开、退出时还原（见 Layout 的 watch）。
     */
    immersiveMode: false,
    /** 插件面板收起：仅隐藏 sidebarPanel（panel-area），保留活动栏图标列 */
    sidebarPanelCollapsed: false,
    /**
     * 停靠面板状态（key = 停靠位置，目前只有 'bottom'）。
     * 停靠面板由 `ToolPlugin.dockPanel` 声明，**不参与 activeToolId 切换**——
     * 活动栏图标是「显示/隐藏」开关，面板与主区内容并存（典型：终端）。
     */
    dockPanelVisible: { bottom: false } as Record<string, boolean>,
    /** 停靠面板高度（px） */
    dockPanelHeight: { bottom: 280 } as Record<string, number>,
    /** 停靠面板是否已把主区（editor 区）挤到最小：拖到顶时为 true，此时面板占满主区 */
    dockPanelExpanded: { bottom: false } as Record<string, boolean>,
    /** 关联的窗口预设 ID（可选） */
    presetId: ''
  }),
  actions: {
    /** 从预设配置应用布局 */
    applyPreset(config: { enabledPlugins: string[]; focusMode: boolean; presetId?: string }) {
      this.enabledPlugins = config.enabledPlugins
      this.focusMode = config.focusMode
      if (config.presetId) {
        this.presetId = config.presetId
      }
    },

    /**
     * 本窗口是否显示某插件（活动栏图标 / 侧栏面板）。
     *
     * 与 `filteredPlugins` 同一口径：空清单 = 显示全部。固定底部入口（`pinnedBottom`，
     * 如「设置」）不受清单影响，所以调用方不必再单独判断它们。
     */
    isPluginVisibleInWindow(id: string): boolean {
      return this.enabledPlugins.length === 0 || this.enabledPlugins.includes(id)
    },

    /**
     * 本窗口显示清单的**显式形式**：把「空清单 = 显示全部」展开成具体插件 ID。
     *
     * 加入 / 移出都拿它当基准：空清单直接过滤掉一个 id 得到的**仍是空数组**（语义还是
     * 「显示全部」），用户点完毫无变化 —— 这类静默失效只能靠统一展开来避免。
     * 展开基准取 `pluginManager.plugins`（已注册 = 已启用的插件），并排除固定底部入口。
     */
    resolveWindowPluginList(): string[] {
      if (this.enabledPlugins.length > 0) return [...this.enabledPlugins]
      return pluginManager.plugins.filter(p => !p.pinnedBottom).map(p => p.id)
    },

    /**
     * 直接设定本窗口的显示清单（`[]` = 显示全部）。
     *
     * 「加入 / 移出」与「窗口设置里编辑本窗口所属预设」都走它，所以「图标没了、面板还在」
     * 的后果必须在这里一次收干净：
     *  - 被隐藏的恰是当前激活插件 ⇒ 激活项让给还可见的第一个插件（与 unregister 同一思路）；
     *    一个都不剩则清空，面板区随之收起（活动栏底部的「设置」入口仍在，不会走投无路）；
     *  - 被隐藏的是停靠面板插件（如终端）⇒ 顺手收起其面板（活动栏图标就是它的显隐开关，
     *    图标没了面板就再也关不掉）。
     */
    setWindowPluginList(ids: string[]) {
      this.enabledPlugins = [...ids]
      // 空清单 = 显示全部：没有任何插件被隐藏，不存在孤儿面板
      if (ids.length === 0) return
      const active = pluginManager.activeToolId.value
      if (active && !ids.includes(active)) {
        const next = pluginManager.plugins.find(p => !p.pinnedBottom && ids.includes(p.id))
        // 当活动插件被隐藏时，设置活动插件为第一个可见插件 但是当前的活动插件是"设置"插件则不切换
        if (next && active !== 'settings'){
          if (next) pluginManager.switchTool(next.id)
          else pluginManager.activeToolId.value = ''
        }
      }
      // 查注册表而不是读插件的 `dockPanel` 字段：「有没有停靠面板、停在哪」是贡献事实，
      // 与 Layout 渲染时用的是同一张表 —— 两处判定不一致就会出现「面板还在、开关没了」
      for (const e of contributionRegistry.views('dock')) {
        if (ids.includes(e.pluginId)) continue
        const pos = e.payload.position
        if (pos && this.dockPanelVisible[pos]) this.setDockPanelVisible(pos, false)
      }
    },

    /** 切换插件面板显隐 */
    toggleSidebarPanel() {
      this.sidebarPanelCollapsed = !this.sidebarPanelCollapsed
    },

    /** 设置插件面板显隐 */
    setSidebarPanelCollapsed(collapsed: boolean) {
      this.sidebarPanelCollapsed = collapsed
    },

    /**
     * 切换沉浸式（标题栏上的按钮、以及后端系统层热键 F11 / macOS Control+Command+F）。
     *
     * 变更后必须上报（见 `reportImmersiveState`）：后端靠它决定是否开始拦截 Esc。
     */
    toggleImmersive() {
      this.immersiveMode = !this.immersiveMode
      this.reportImmersiveState()
    },

    /**
     * 设置沉浸式（顶部弹出条的「退出沉浸式」、Esc 都走这里）。
     *
     * 变更后必须上报，理由同 `toggleImmersive`。
     */
    setImmersiveMode(enabled: boolean) {
      this.immersiveMode = enabled
      this.reportImmersiveState()
    },

    /**
     * 把本窗口的沉浸式状态上报给后端。
     *
     * 后端（`immersive_shortcut.rs`）据此在「有窗口处于沉浸式」且「本应用有焦点」时
     * 才注册系统级 Esc 热键 —— Esc 被 global-shortcut 拦下后就彻底离开了 WebView，
     * 因此**绝不能**常态注册，否则用户按 Esc 会毫无反应（关不了弹窗、退不出全屏）。
     *
     * 上报带上 `label` 是为了让后端用集合按窗口去重：同一窗口重复上报同一状态不会
     * 把计数叠歪，窗口正常退出沉浸式时总能精确注销。
     */
    reportImmersiveState() {
      void publish(IMMERSIVE_STATE_EVENT, {
        active: this.immersiveMode,
        label: getCurrentWindow().label,
      })
    },

    /** 停靠面板：显示/隐藏（活动栏图标点击） */
    toggleDockPanel(position: string) {
      this.dockPanelVisible = { ...this.dockPanelVisible, [position]: !this.dockPanelVisible[position] }
    },

    /** 停靠面板：显式设置显隐 */
    setDockPanelVisible(position: string, visible: boolean) {
      if (this.dockPanelVisible[position] === visible) return
      this.dockPanelVisible = { ...this.dockPanelVisible, [position]: visible }
    },

    /** 停靠面板：设置高度（px） */
    setDockPanelHeight(position: string, height: number) {
      this.dockPanelHeight = { ...this.dockPanelHeight, [position]: Math.max(60, Math.round(height)) }
    },

    /** 停靠面板：设置「占满主区」（editor 区隐藏） */
    setDockPanelExpanded(position: string, expanded: boolean) {
      if (this.dockPanelExpanded[position] === expanded) return
      this.dockPanelExpanded = { ...this.dockPanelExpanded, [position]: expanded }
    },

    /** 重置为默认布局（显示所有插件，非专注模式） */
    resetLayout() {
      this.enabledPlugins = []
      this.focusMode = false
      this.immersiveMode = false
      this.sidebarPanelCollapsed = false
      this.dockPanelVisible = { bottom: false }
      this.dockPanelHeight = { bottom: 280 }
      this.dockPanelExpanded = { bottom: false }
      this.presetId = ''
    }
  },
  getters: {
    /** 过滤后的插件列表（根据 enabledPlugins 过滤，保持排序） */
    filteredPlugins(): ToolPlugin[] {
      const allPlugins = pluginManager.plugins
      if (this.enabledPlugins.length === 0) {
        return allPlugins
      }
      // 固定在活动栏底部的宿主入口（如「设置」）不受窗口预设清单影响，始终可见
      return allPlugins.filter(p => p.pinnedBottom || this.enabledPlugins.includes(p.id))
    }
  }
})

export default useLayoutStore
