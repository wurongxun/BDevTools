import { markRaw } from 'vue'
import { Connection } from '@element-plus/icons-vue'
import type { ToolPlugin } from '@/core/pluginManager'
import SidebarPanel from './SidebarPanel.vue'
import MainPanel from './MainPanel.vue'
import ConnectionTabBar from './components/RemoteConnection/ConnectionTabBar.vue'

export default {
  id: 'ssh',
  label: 'SSH 连接',
  description: 'SSH 终端与 SFTP 文件传输，支持远程日志拉取',
  icon: markRaw(Connection),
  sidebarPanel: markRaw(SidebarPanel),
  mainPanel: markRaw(MainPanel),
  tabBar: markRaw(ConnectionTabBar),
  workspaceDirs: ['keys', 'sessions'],
  order: 3
} satisfies ToolPlugin
