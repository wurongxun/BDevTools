import { markRaw } from 'vue'
import { Setting } from '@element-plus/icons-vue'
import type { BuiltinPluginImpl } from '@/core/pluginManager'
import { lazyView } from '@/core/plugin/lazy'

/**
 * 设置（宿主级常驻入口）
 *
 * 与普通插件的区别（见 CODEBUDDY.md「插件契约」）：
 * - `pinnedBottom: true` → 入口按钮固定在活动栏**底部**（对齐 VS Code 的齿轮位），
 *   不参与活动栏图标列表迭代、不进入统一排序序列，也不受窗口预设的插件清单过滤；
 * - 只提供 `mainPanel`（无 `sidebarPanel`）→ 天然走「无侧栏插件」布局：
 *   侧栏收敛到活动栏宽度，设置内容占满主面板。
 *
 * 内容直接复用 `src/views/app/AppSetting.vue`（原独立窗口页面），避免两份维护。
 */
export default {
  icon: markRaw(Setting),
  mainPanel: lazyView('settings', 'main', () => import('./MainPanel.vue')),
  pinnedBottom: true,
  order: 999,
} satisfies BuiltinPluginImpl
