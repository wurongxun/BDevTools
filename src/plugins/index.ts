import { initPluginHost } from '@/core/plugin/registry'
import { BUILTIN_PLUGINS } from './builtin.generated'

/**
 * 初始化插件体系：把内置清单交给注册表 → 按「启用状态」注册内置与第三方 → 应用统一排序。
 *
 * 必须在应用挂载前 `await`：启用状态与排序要先就位，否则首屏会先按默认形态渲染、
 * 随后跳变成保存的状态。任一步失败都只降级告警，不影响主界面启动。
 *
 * ## 打包哪些内置插件
 *
 * 清单不再写死在这里，而是由构建前置脚本 `scripts/gen-builtin-plugins.mjs` 生成
 * `./builtin.generated.ts`——**只 import 被选中的插件**，其余插件整棵模块树都不会进 bundle。
 * 想只打包 database：`node scripts/gen-builtin-plugins.mjs database && npx vite build`。
 *
 * 后端侧是同一套开关（Cargo feature `plugin-database` 等），两边必须同开同关，否则会出现
 * 「前端有入口、后端无命令」—— 由 `.workbuddy/tools/verify-plugin-selection.mjs` 守着。
 *
 * 注意清单与「当前注册了哪些插件」是两件事，不能合并：内置插件也支持运行期启用/禁用，
 * 禁用会把它从 `pluginManager` **注销**（活动栏与布局都不再为它留位）。如果设置页的可见性
 * 只看 `pluginManager.plugins`，被禁用的内置插件连卡片都会消失 —— 那个把它开回来的开关
 * 就再也找不到了。
 */
export async function initPlugins() {
  await initPluginHost(BUILTIN_PLUGINS)
}

export { BUILTIN_PLUGINS }
