import { markRaw } from 'vue'
import { Connection } from '@element-plus/icons-vue'
import type { ToolPlugin } from '@/core/pluginManager'
import SidebarPanel from './SidebarPanel.vue'
import MainPanel from './MainPanel.vue'
import ConnectionTabBar from './components/RemoteConnection/ConnectionTabBar.vue'

export default {
  id: 'ssh',
  label: 'SSH 连接',
  icon: markRaw(Connection),
  sidebarPanel: markRaw(SidebarPanel),
  mainPanel: markRaw(MainPanel),
  tabBar: markRaw(ConnectionTabBar),
  order: 2
} satisfies ToolPlugin
