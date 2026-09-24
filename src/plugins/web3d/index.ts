import { markRaw } from 'vue'
import { Monitor } from '@element-plus/icons-vue'
import type { ToolPlugin } from '@/core/pluginManager'
import MainPanel from './MainPanel.vue'

/**
 * 3D 与游戏（宿主级容器）
 *
 * 第三方插件一旦声明了 `pageCapabilities`（要编译 WASM / 读包内资源 / 音视频 / Worker /
 * 跨源隔离），就不适合待在面板内的沙箱 iframe 里跑：重资源会跟主界面抢渲染线程，
 * 而 iframe 的 sandbox 天然没有存档与指针锁定。本插件负责把这些插件用**独立窗口**打开 ——
 * 窗口 label 刻意不在 `capabilities` 的 windows 列表里 → ACL 全拒 → 插件页拿不到任何宿主命令
 * （推理见 `core/plugin/window.ts` 文件头）。
 *
 * 注意它**不是**唯一入口：声明了 `standaloneWindow` 的插件，点自己的活动栏图标就会直接开窗口
 * （入口语义见 `AppSidebar.onActivityClick`）。本容器是集中入口 + 兜底，列表也顺带展示
 * 「入口形态」与「页面能力」，便于排查「为什么这个插件没按预期打开」。
 *
 * 只提供 `mainPanel`（无侧栏）→ 天然走「无侧栏插件」布局，列表占满主区。
 */
export default {
  id: 'web3d',
  label: '3D 与游戏',
  description: '在新窗口里运行声明了页面能力 / 独立窗口的第三方 3D / 游戏插件',
  icon: markRaw(Monitor),
  mainPanel: markRaw(MainPanel),
  order: 98,
} satisfies ToolPlugin
