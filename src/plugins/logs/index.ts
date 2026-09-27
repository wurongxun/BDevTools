import { markRaw } from 'vue'
import type { ToolPlugin } from '@/core/pluginManager'
import SidebarPanel from './sidebar/SidebarPanel.vue'
import LogWorkspace from './components/LogWorkspace.vue'
import Logs from '@/components/Icon/Logs.vue'
/**
 * 日志工作台插件
 * 首批能力：大文件分块懒加载、关键词/正则搜索与命中导航、级别配色、
 * 自定义高亮规则、堆栈识别与悬停元信息。
 */
export default {
  id: 'logs',
  label: '日志',
  description: '大文件日志工作台：分块懒加载、正则检索、级别配色与高亮规则',
  icon: markRaw(Logs),
  sidebarPanel: markRaw(SidebarPanel),
  mainPanel: markRaw(LogWorkspace),
  workspaceDirs: ['logs'],
  order: 2,
} satisfies ToolPlugin
