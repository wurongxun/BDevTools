import { markRaw } from 'vue'
import { Grid } from '@element-plus/icons-vue'
import type { ToolPlugin } from '@/core/pluginManager'
import MainPanel from './MainPanel.vue'

/**
 * 占位插件 —— 用于验证「插件不提供 sidebarPanel」时宿主的布局适配
 *
 * 本插件刻意只注册 mainPanel（无 sidebarPanel / tabBar），用于观察：
 * 侧栏是否收敛到活动栏宽度（48px）、拖拽手柄是否消失、面板切换按钮是否禁用、
 * 以及切回有侧栏插件后宽度能否恢复。自检清单见 MainPanel.vue。
 *
 * 验证完成后：删除本目录，并移除 src/plugins/index.ts 中的 import 与注册两行即可。
 */
export default {
  id: 'placeholder',
  label: '占位示例',
  description: '无侧栏插件的布局适配示例（只有主面板，无侧栏/标签条）',
  icon: markRaw(Grid),
  // 刻意不提供 sidebarPanel
  mainPanel: markRaw(MainPanel),
  order: 99,
} satisfies ToolPlugin
