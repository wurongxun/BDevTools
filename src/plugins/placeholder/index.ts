import { markRaw } from 'vue'
import { Grid } from '@element-plus/icons-vue'
import type { BuiltinPluginImpl } from '@/core/pluginManager'
import { lazyView } from '@/core/plugin/lazy'

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
  icon: markRaw(Grid),
  // 刻意不提供 sidebarPanel
  mainPanel: lazyView('placeholder', 'main', () => import('./MainPanel.vue')),
  order: 99,
} satisfies BuiltinPluginImpl
