import { markRaw } from 'vue'
import { Connection } from '@element-plus/icons-vue'
import type { BuiltinPluginImpl } from '@/core/pluginManager'
import { lazyView } from '@/core/plugin/lazy'

export default {
  icon: markRaw(Connection),
  sidebarPanel: lazyView('ssh', 'sidebar', () => import('./SidebarPanel.vue')),
  mainPanel: lazyView('ssh', 'main', () => import('./MainPanel.vue')),
  tabBar: lazyView('ssh', 'tabBar', () => import('./components/RemoteConnection/ConnectionTabBar.vue')),
  workspaceDirs: ['keys', 'sessions'],
  order: 3
} satisfies BuiltinPluginImpl
