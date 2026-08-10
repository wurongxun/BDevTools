import { defineStore } from 'pinia'
import { computed } from 'vue'
import { pluginManager, type ToolPlugin } from '@/core/pluginManager'

/**
 * 窗口布局配置 store（per-window 隔离）
 * 每个窗口拥有独立的 Pinia 实例，天然隔离
 */
const useLayoutStore = defineStore('layout', {
  state: () => ({
    /** 当前窗口启用的插件 ID 列表（空数组 = 显示全部） */
    enabledPlugins: [] as string[],
    /** 专注模式：隐藏侧边栏 */
    focusMode: false,
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

    /** 重置为默认布局（显示所有插件，非专注模式） */
    resetLayout() {
      this.enabledPlugins = []
      this.focusMode = false
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
      return allPlugins.filter(p => this.enabledPlugins.includes(p.id))
    }
  }
})

export default useLayoutStore
