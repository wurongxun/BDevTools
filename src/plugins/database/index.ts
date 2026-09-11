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
  workspaceDirs: ['scripts', 'exports'],
  order: 4,
  // 数据库侧边栏内容多（连接树 + 脚本树 + 插件设置），声明更宽的默认/最小宽度
  sidebarDefaultWidth: 260,
  sidebarMinWidth: 180
} satisfies ToolPlugin
