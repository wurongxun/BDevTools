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
  /** 侧边栏面板组件 */
  sidebarPanel: Component
  /** 主内容区面板组件 */
  mainPanel: Component
  /** Tab 栏组件（可选，不提供则不显示 Tab 栏） */
  tabBar?: Component
  /** 排序权重，越小越靠前 */
  order?: number
  /** 插件激活回调 */
  onActivate?: () => void | Promise<void>
  /** 插件停用回调 */
  onDeactivate?: () => void
}

/**
 * 插件管理器
 * 管理所有工具插件的注册、卸载和查询
 * 使用 Vue 响应式，UI 自动跟随插件列表变化
 */
class PluginManager {
  /** 已注册插件列表（响应式） */
  private _plugins = ref<ToolPlugin[]>([])

  /** 当前激活的工具 ID（响应式） */
  activeToolId = ref<string>('')

  /** 获取排序后的插件列表 */
  get plugins(): ToolPlugin[] {
    return [...this._plugins.value].sort((a, b) => (a.order ?? 99) - (b.order ?? 99))
  }

  /** 获取指定插件 */
  getPlugin(id: string): ToolPlugin | undefined {
    return this._plugins.value.find(p => p.id === id)
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
    // 如果卸载的是当前激活的插件，切换到第一个
    if (this.activeToolId.value === id) {
      this.activeToolId.value = this._plugins.value[0]?.id || ''
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
}

/** 全局单例 */
export const pluginManager = new PluginManager()
