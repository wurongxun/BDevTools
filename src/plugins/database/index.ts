import { markRaw } from 'vue'
import { Coin } from '@element-plus/icons-vue'
import type { ToolPlugin } from '@/core/pluginManager'
import SidebarPanel from './SidebarPanel.vue'
import MainPanel from './MainPanel.vue'

export default {
  id: 'database',
  label: '数据库',
  icon: markRaw(Coin),
  sidebarPanel: markRaw(SidebarPanel),
  mainPanel: markRaw(MainPanel),
  order: 3
} satisfies ToolPlugin
