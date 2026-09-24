import { ref, type Component } from 'vue'

/**
 * 工具插件接口
 * 每个插件必须实现此接口，定义自己的元数据和组件
 */
export interface ToolPlugin {
  /** 唯一标识 */
  id: string
  /** 显示名称 */
  label: string
  /** 活动栏图标组件 */
  icon: Component
  /** 插件简介（插件管理页展示；第三方由适配层填充） */
  description?: string
  /**
   * 插件来源：缺省视为 `builtin`（内置，随应用编译）。
   * 第三方插件由适配层显式标注 `third-party`，宿主据此判断「不可卸载 / 不可禁用」等策略。
   */
  source?: 'builtin' | 'third-party'
  /**
   * 侧边栏面板组件（**可选**）
   * 不提供时宿主收起侧栏（收缩到活动栏宽度）并让主面板占满宽度，
   * 活动栏图标仍是插件入口。第三方插件的沙箱壳组件同样适用该契约。
   */
  sidebarPanel?: Component
  /**
   * 主内容区面板组件（**可选**）
   * 只提供 `dockPanel` 的插件可以不声明它——此时该插件不会占用主区，
   * 只作为「停靠面板开关」存在（活动栏图标 = 显隐开关）。
   */
  mainPanel?: Component
  /**
   * 停靠面板（可选）：把面板固定在主区某一侧的边缘，与主区内容**并存**。
   *
   * 与 `mainPanel` 的关键区别：停靠面板**不参与 `activeToolId` 切换**。
   * 点击活动栏图标 = 显示/隐藏该面板，不会把当前插件的主面板顶掉。
   * 典型场景：底部终端。`Layout` 负责渲染面板与它和主区之间的拖拽分隔条。
   */
  dockPanel?: {
    /** 停靠位置（目前只支持底部） */
    position: 'bottom'
    /** 面板组件 */
    component: Component
    /** 默认高度（px） */
    defaultHeight?: number
    /** 最小高度（px，低于此值视为「拖到底隐藏」） */
    minHeight?: number
  }
  /** Tab 栏组件（可选，不提供则不显示 Tab 栏） */
  tabBar?: Component
  /** 排序权重，越小越靠前（未声明 = 99；会被 `applyOrder` 的统一序列覆盖） */
  order?: number
  /**
   * 固定在活动栏底部（语义对齐 VS Code 活动栏底部的齿轮按钮）
   * 开启后：不参与活动栏图标列表迭代、不参与统一排序序列，也不受窗口预设插件清单过滤。
   * 用于「设置」这类宿主级常驻入口。
   */
  pinnedBottom?: boolean
  /** 插件需要的工作目录子路径（相对于插件工作目录根） */
  workspaceDirs?: string[]
  /** 插件激活回调 */
  onActivate?: () => void | Promise<void>
  /** 插件停用回调 */
  onDeactivate?: () => void
  /**
   * 主界面在**独立窗口**里（第三方清单 `standaloneWindow`；内置插件不用这个字段）。
   *
   * 语义变化：活动栏图标不再是「切主面板」，而是「打开（或前置）它的独立窗口」
   * —— 与 `dockPanel` 一样属于「图标不是切换开关」的既有先例。
   * `mainPanel` 此时是宿主生成的启动卡（兜底重开入口），不是插件页面本身。
   */
  standaloneWindow?: boolean
  // ===== 侧边栏布局声明（可选，未声明时全局 Layout 用默认值） =====
  /** 侧边栏最小宽度（px） */
  sidebarMinWidth?: number
  /** 侧边栏最大宽度（px） */
  sidebarMaxWidth?: number
  /** 侧边栏默认宽度（px，首次切换该插件且未手动拖拽过时使用） */
  sidebarDefaultWidth?: number
}

/** 未纳入统一排序的插件权重基准（保证它们排在已排序插件之后） */
const DEFAULT_ORDER_BASE = 1000

/**
 * 插件管理器
 * 管理所有工具插件的注册、卸载和查询
 * 使用 Vue 响应式，UI 自动跟随插件列表变化
 */
class PluginManager {
  /** 已注册插件列表（响应式） */
  private _plugins = ref<ToolPlugin[]>([])

  /**
   * 统一排序覆盖表：`id → 序号`，内置与第三方共用同一序列。
   * 由 `applyOrder` 整体写入；未记录的插件回落自带的 `order` 字段。
   */
  private _orderIndex = ref<Record<string, number>>({})

  /** 当前激活的工具 ID（响应式） */
  activeToolId = ref<string>('')

  /**
   * 排序权重：统一排序优先，其次插件自带 order（未声明 = 99）。
   * 未记录者加基准偏移，保证新装插件不会插到已排序插件前面。
   */
  private rank(plugin: ToolPlugin): number {
    const idx = this._orderIndex.value[plugin.id]
    if (idx !== undefined) return idx
    return DEFAULT_ORDER_BASE + (plugin.order ?? 99)
  }

  /** 获取排序后的插件列表 */
  get plugins(): ToolPlugin[] {
    return [...this._plugins.value].sort((a, b) => this.rank(a) - this.rank(b))
  }

  /** 获取指定插件 */
  getPlugin(id: string): ToolPlugin | undefined {
    return this._plugins.value.find(p => p.id === id)
  }

  /**
   * 停靠在指定位置的插件（Layout 渲染 dockPanel 用）
   * 同一位置只取排序最靠前的一个。
   */
  dockPluginAt(position: 'bottom'): ToolPlugin | undefined {
    return this.plugins.find(p => p.dockPanel?.position === position)
  }

  /**
   * 当前激活插件是否提供侧栏面板
   * 供布局层判断侧栏宽度收敛、拖拽手柄与面板切换按钮是否可用
   */
  get activeHasSidebar(): boolean {
    return Boolean(this.getPlugin(this.activeToolId.value)?.sidebarPanel)
  }

  /** 注册插件 */
  register(plugin: ToolPlugin) {
    // 避免重复注册
    if (this._plugins.value.some(p => p.id === plugin.id)) {
      console.warn(`[PluginManager] 插件 "${plugin.id}" 已存在，跳过注册`)
      return
    }
    this._plugins.value.push(plugin)
    // 如果是第一个插件，自动激活
    if (this._plugins.value.length === 1) {
      this.activeToolId.value = plugin.id
    }
    plugin.onActivate?.()
    console.log(`[PluginManager] 插件已注册: ${plugin.id}`)
  }

  /** 注销插件 */
  unregister(id: string) {
    const idx = this._plugins.value.findIndex(p => p.id === id)
    if (idx === -1) return
    const plugin = this._plugins.value[idx]
    plugin.onDeactivate?.()
    this._plugins.value.splice(idx, 1)
    // 如果卸载的是当前激活的插件，切换到排序后的第一个
    if (this.activeToolId.value === id) {
      this.activeToolId.value = this.plugins[0]?.id || ''
    }
    console.log(`[PluginManager] 插件已注销: ${id}`)
  }

  /** 切换激活的工具 */
  switchTool(id: string) {
    if (this._plugins.value.some(p => p.id === id)) {
      this.activeToolId.value = id
    }
  }

  /** 是否已注册 */
  hasPlugin(id: string): boolean {
    return this._plugins.value.some(p => p.id === id)
  }

  /**
   * 应用统一排序（内置 + 第三方插件 ID 的全量序列）。
   * 序列外的插件回落自带 order 并排在末尾，因此传入局部序列也不会打乱其余顺序。
   */
  applyOrder(ids: string[]) {
    const next: Record<string, number> = {}
    ids.forEach((id, i) => {
      next[id] = i
    })
    this._orderIndex.value = next
  }
}

/** 全局单例 */
export const pluginManager = new PluginManager()
