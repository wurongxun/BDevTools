import { markRaw } from 'vue'
import { DataAnalysis } from '@element-plus/icons-vue'
import type { BuiltinPluginImpl } from '@/core/pluginManager'
import { lazyView } from '@/core/plugin/lazy'

/**
 * MAT（Memory Analyzer Tool）插件 —— Java 堆转储内存分析
 *
 * 布局与 logs 一致：左 sidebar（打开 .hprof / .phd）+ 主区多堆转储 tab。
 * 每个堆转储 tab 内部再按分析能力拆分子 tab（概览 / 类 / 泄漏检测 / 支配树 / 引用追踪 /
 * 大对象排行 / 报告 / 对比 / OQL）。
 *
 * 当前能力：格式识别（HPROF 家族 / PHD）、`.hprof.gz` 透明解压（缓存复用）、
 * 本地索引持久化（`.bdtidx` 图 / `.bdtidx.dom` 支配树，二次打开秒开）、
 * 浅堆 / 深堆直方图、Leak Suspects（支配树 Retained Heap + 问题叙述 + HTML 报告）、
 * Dominator Tree、对象引用追踪（GC Roots 多路径 / 弱引用开关）、大对象排行、
 * 专用报告（线程 / 类加载器 / 内存浪费 / 不可达对象）、多 dump 直方图对比、
 * OQL 安全子集、对象字段值与结果导出。
 * 差距清单与后续计划见 docs/design/mat-eclipse-parity-roadmap.md
 * （P0-P2 与 P3-1/P3-2/P3-3(gz) 已完成，剩 P3-4/P3-5/P3-7 与样例覆盖）。
 */
export default {
  icon: markRaw(DataAnalysis),
  sidebarPanel: lazyView('mat', 'sidebar', () => import('./SidebarPanel.vue')),
  mainPanel: lazyView('mat', 'main', () => import('./MainPanel.vue')),
  order: 5,
  // 侧边栏内容较简，用默认宽度即可
} satisfies BuiltinPluginImpl
