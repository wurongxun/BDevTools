import { markRaw } from 'vue'
import { DataAnalysis } from '@element-plus/icons-vue'
import type { ToolPlugin } from '@/core/pluginManager'
import SidebarPanel from './SidebarPanel.vue'
import MainPanel from './MainPanel.vue'

/**
 * MAT（Memory Analyzer Tool）插件 —— Java 堆转储内存分析
 *
 * 布局与 logs 一致：左 sidebar（打开 .hprof / .phd）+ 主区多堆转储 tab。
 * 每个堆转储 tab 内部再按分析能力拆分子 tab（概览/直方图/泄漏检测/支配树/引用追踪/OQL）。
 *
 * 当前能力（第一批）：格式识别（HPROF 家族 / PHD）+ 浅堆直方图（真解析）。
 * 支配树（Retained Heap）、Leak Suspects、GC Roots 引用追踪、OQL 为占位，后续迭代接入。
 */
export default {
  id: 'mat',
  label: '内存分析',
  description: 'Java 堆转储分析：直方图、泄漏检测、支配树与引用追踪',
  icon: markRaw(DataAnalysis),
  sidebarPanel: markRaw(SidebarPanel),
  mainPanel: markRaw(MainPanel),
  order: 5,
  // 侧边栏内容较简，用默认宽度即可
} satisfies ToolPlugin
