// ============================================================
// 设置页共享类型
//
// 为什么要单独一个文件：类型不能写在 `<script setup>` 里
// （SFC 编译器不允许 script setup 出现 ES 模块导出），
// 而左侧导航、顶部标题、右侧分区面板三处都要用同一个联合类型。
// ============================================================

/** 设置分区标识（左侧分类导航 ↔ 右侧面板一一对应） */
export type SettingTab = 'app' | 'window' | 'plugin' | 'help' | 'about'

/** 插件分区内部页签：管理（内置/第三方插件的安装与排序）/ 开发（第三方插件开发文档） */
export type PluginTab = 'manage' | 'dev'

/** 窗口预设配置 */
export interface WindowPresetConfig {
  id: string
  name: string
  enabledPlugins: string[]
  width: number
  height: number
  openOnStartup: boolean
  focusMode: boolean
  /** 绑定的工作区 ID（可选；多个预设可绑同一工作区，但同时只能打开一个） */
  workspaceId?: string
}
