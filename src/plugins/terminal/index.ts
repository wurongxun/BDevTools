import { markRaw } from 'vue'
import type { ToolPlugin } from '@/core/pluginManager'
import TerminalPanel from './components/TerminalPanel.vue'
import Terminal from "@/components/Icon/Terminal.vue"

/**
 * 终端插件 —— 本地交互式终端（PTY）
 *
 * **特殊之处：它是一个「停靠面板」插件，不占用主区。**
 * 面板由 `Layout` 渲染主区（editor-wrapper）下方，与主区内容并存；
 * 活动栏上的图标是**显示/隐藏开关**（不是切换主面板）——
 * 这正是 `dockPanel` 与 `mainPanel` 的分工（见 `core/pluginManager.ts` 的注释）。
 *
 * 没有网络层：前端 xterm.js ←→ 宿主桥 ←→ 系统 PTY（Windows ConPTY / Unix openpty）。
 * 输出流走 Tauri v2 Channel，`SIGWINCH` 通过 `terminal_resize` 通知 PTY。
 *
 * 顺带说明为什么不做成 `mainPanel`：终端几乎总是要和主区内容（编辑器/日志/数据表）
 * 同时看，做成主面板就只能二选一。
 */
export default {
  id: 'terminal',
  label: '终端',
  description: '本地交互式终端：多标签 / 分屏 / 可拖拽停靠，支持 shell 集成（cwd、退出码）',
  icon: markRaw(Terminal),
  // 只有停靠面板，不注册 mainPanel（活动栏点击 = 显示/隐藏底部面板）
  dockPanel: {
    position: 'bottom',
    component: markRaw(TerminalPanel),
    defaultHeight: 280,
    minHeight: 80,
  },
  order: 7,
} satisfies ToolPlugin
