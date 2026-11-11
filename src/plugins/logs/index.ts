import { markRaw } from 'vue'
import type { BuiltinPluginImpl } from '@/core/pluginManager'
import { lazyView } from '@/core/plugin/lazy'
import Logs from '@/components/Icon/Logs.vue'
/**
 * 日志工作台插件
 * 首批能力：大文件分块懒加载、关键词/正则搜索与命中导航、级别配色、
 * 自定义高亮规则、堆栈识别与悬停元信息。
 */
export default {
  icon: markRaw(Logs),
  sidebarPanel: lazyView('logs', 'sidebar', () => import('./sidebar/SidebarPanel.vue')),
  mainPanel: lazyView('logs', 'main', () => import('./components/LogWorkspace.vue')),
  workspaceDirs: ['logs'],
  order: 2,
} satisfies BuiltinPluginImpl
