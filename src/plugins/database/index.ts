import { markRaw } from 'vue'
import { Coin } from '@element-plus/icons-vue'
import type { BuiltinPluginImpl } from '@/core/pluginManager'
import { lazyView } from '@/core/plugin/lazy'

export default {
  icon: markRaw(Coin),
  sidebarPanel: lazyView('database', 'sidebar', () => import('./SidebarPanel.vue')),
  mainPanel: lazyView('database', 'main', () => import('./MainPanel.vue')),
  workspaceDirs: ['scripts', 'exports'],
  order: 4,
  // 数据库侧边栏内容多（连接树 + 脚本树 + 插件设置），声明更宽的默认/最小宽度
  sidebarDefaultWidth: 260,
  sidebarMinWidth: 180
} satisfies BuiltinPluginImpl
