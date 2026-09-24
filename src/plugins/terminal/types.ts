/**
 * terminal 插件类型定义
 *
 * 与后端 `src-tauri/src/command/terminal.rs` 的结构体一一对应。
 */

/** 后端可选的 shell（terminal_list_shells） */
export interface ShellInfo {
  id: string
  label: string
  path: string
  /** powershell / pwsh / cmd / bash / zsh / fish / nu / sh / wsl / unknown */
  kind: string
  isDefault: boolean
  /**
   * 该 shell 能拿到的 shell 集成程度（与打开会话后 `TermOpenResult.integration` 同源，
   * 由后端 `shell_integration_level()` 统一判定）：full 命令边界+退出码+cwd /
   * partial 只有边界+cwd（cmd）/ none 未注入（WSL、fish…）
   */
  integration: IntegrationLevel
}

/** 终端默认启动信息（terminal_defaults） */
export interface TermDefaults {
  shell: string
  shellKind: string
  cwd: string
  platform: string
}

/** shell 集成程度 */
export type IntegrationLevel = 'full' | 'partial' | 'none'

/** 打开会话的返回（terminal_open） */
export interface TermOpenResult {
  sessionId: string
  shell: string
  shellKind: string
  cwd: string
  pid: number | null
  integration: IntegrationLevel
}

/** 后端 → 前端事件（Channel） */
export type TermEvent =
  | { type: 'data'; data: string }
  | { type: 'exit'; code: number | null; signal: string | null }
  | { type: 'error'; message: string }

/** 打开会话的入参 */
export interface TermOpenOptions {
  cols: number
  rows: number
  cwd?: string
  shell?: string
  args?: string[]
  env?: Record<string, string>
  noIntegration?: boolean
}

/** 一次 exec 的结果（异步执行 + 超时） */
export interface TermExecResult {
  /** 退出码；shell 集成不可用或超时后命令仍在跑时为 null */
  exitCode: number | null
  /** 是否超时（超时不等于命令被终止，只是不再等待） */
  timedOut: boolean
  /** 命令输出（含回显）；拼接自 OSC 133;B..D 之间的原始字符流 */
  output: string
  /** 耗时（毫秒） */
  elapsedMs: number
}

/** xterm 外观配置（持久化到 plugins/terminal.json） */
export interface TerminalAppearance {
  /** 字体族；留空则用内置等宽字体栈 */
  fontFamily: string
  fontSize: number
  lineHeight: number
  cursorStyle: 'block' | 'underline' | 'bar'
  cursorBlink: boolean
  /** 回滚缓冲行数 */
  scrollback: number
  /** 选中即复制（类 X11 习惯；默认关） */
  copyOnSelect: boolean
  /** 配色方案：follow = 跟随应用主题 */
  colorScheme: string
  /** 终端面板高度（px，拖拽分隔条后写入） */
  panelHeight: number
  /** 终端面板是否可见（上次退出时的状态） */
  panelVisible: boolean
}

/** 配色方案（term 的 ITheme 子集 + 元信息） */
export interface TermColorScheme {
  id: string
  label: string
  /** false 表示按应用主题（isDark）在下面两套里选 */
  followTheme: boolean
  dark: TermPalette
  light: TermPalette
}

/** 调色板（缺省字段回落到「跟随主题」的派生值） */
export interface TermPalette {
  background: string
  foreground: string
  cursor: string
  selectionBackground: string
  black: string
  red: string
  green: string
  yellow: string
  blue: string
  magenta: string
  cyan: string
  white: string
  brightBlack: string
  brightRed: string
  brightGreen: string
  brightYellow: string
  brightBlue: string
  brightMagenta: string
  brightCyan: string
  brightWhite: string
}

// ==================== 分屏 / 标签 ====================

/** 单个终端窗格（一个窗格 = 一个后端 PTY 会话） */
export interface PaneState {
  /** 前端窗格 id（分屏树里的叶子标识） */
  id: string
  /** 后端会话 id；建会话失败时为空串 */
  sessionId: string
  /** 标签/窗格标题（shell 集成可用时由 OSC 0/2 更新） */
  title: string
  /** 当前工作目录（由 OSC 7 跟踪） */
  cwd: string
  /** shell 展示名 */
  shellLabel: string
  shellKind: string
  integration: IntegrationLevel
  /** 子进程是否已退出 */
  exited: boolean
  /** 退出码（shell 集成可用时来自 OSC 133;D） */
  exitCode: number | null
  /** 上一条命令是否成功（用于标签上的状态点） */
  lastCommandOk: boolean | null
  /** 是否正在等待某条命令结束（exec 进行中） */
  busy: boolean
}

/** 分屏树节点：叶子 = 一个窗格，split = 两个子节点的二分布局 */
export type PaneNode =
  | { kind: 'leaf'; paneId: string }
  | {
      kind: 'split'
      /** row = 左右并排（分隔条竖直）；column = 上下堆叠（分隔条水平） */
      direction: 'row' | 'column'
      a: PaneNode
      b: PaneNode
      /** a 占的比例（0.15~0.85） */
      ratio: number
    }

/** 一个标签 = 一棵分屏树 */
export interface TabState {
  id: string
  title: string
  root: PaneNode
  activePaneId: string
}

/** 终端命令边界标记（OSC 133） */
export type CommandMark = 'promptStart' | 'commandStart' | 'outputStart' | 'commandEnd'
