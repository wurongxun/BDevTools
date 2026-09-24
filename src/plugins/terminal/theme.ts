/**
 * terminal 插件配色 / 字体
 *
 * 配色方案分两类：
 * - `follow`：跟随应用主题。背景/前景/光标/选区从主题 CSS 变量（`--bgColor` 等，
 *   由 `styles/theme.scss` 的 `themeVars()` 输出到 `html.light|dark`）实时读取，
 *   其余 ANSI 16 色用 VS Code 的浅/深两套标准色。
 * - 其余为内置固定配色（Dracula / One Dark / Solarized），选了就与宿主主题解耦。
 *
 * 这样做的原因：应用主题切换不需要重算 ANSI 色（那套色是终端语义色，
 * 不该随 UI 主题漂移），而背景/前景必须跟 UI 融为一体。
 */
import type { TermColorScheme, TermPalette } from './types'

/** 内置等宽字体栈：优先 Windows/macOS 常见等宽，再落到支持中文等宽的字体 */
export const DEFAULT_FONT_FAMILY =
  "'Cascadia Mono', 'JetBrains Mono', Consolas, 'Sarasa Mono SC', 'Microsoft YaHei Mono', Menlo, Monaco, 'Courier New', monospace"

/** VS Code 浅色 ANSI 调色板 */
const VSCODE_LIGHT: TermPalette = {
  background: '#ffffff',
  foreground: '#333333',
  cursor: '#000000',
  selectionBackground: '#add6ff80',
  black: '#000000',
  red: '#cd3131',
  green: '#00bc00',
  yellow: '#949800',
  blue: '#0451a5',
  magenta: '#bc05bc',
  cyan: '#0598bc',
  white: '#555555',
  brightBlack: '#666666',
  brightRed: '#cd3131',
  brightGreen: '#14ce14',
  brightYellow: '#b5ba00',
  brightBlue: '#0451a5',
  brightMagenta: '#bc05bc',
  brightCyan: '#0598bc',
  brightWhite: '#a5a5a5',
}

/** VS Code 深色 ANSI 调色板 */
const VSCODE_DARK: TermPalette = {
  background: '#1e1e1e',
  foreground: '#cccccc',
  cursor: '#ffffff',
  selectionBackground: '#264f78',
  black: '#000000',
  red: '#cd3131',
  green: '#0dbc79',
  yellow: '#e5e510',
  blue: '#2472c8',
  magenta: '#bc3fbc',
  cyan: '#11a8cd',
  white: '#e5e5e5',
  brightBlack: '#666666',
  brightRed: '#f14c4c',
  brightGreen: '#23d18b',
  brightYellow: '#f5f543',
  brightBlue: '#3b8eea',
  brightMagenta: '#d670d6',
  brightCyan: '#29b8db',
  brightWhite: '#e5e5e5',
}

/** Dracula */
const DRACULA: TermPalette = {
  background: '#282a36',
  foreground: '#f8f8f2',
  cursor: '#f8f8f2',
  selectionBackground: '#44475a',
  black: '#21222c',
  red: '#ff5555',
  green: '#50fa7b',
  yellow: '#f1fa8c',
  blue: '#bd93f9',
  magenta: '#ff79c6',
  cyan: '#8be9fd',
  white: '#f8f8f2',
  brightBlack: '#6272a4',
  brightRed: '#ff6e6e',
  brightGreen: '#69ff94',
  brightYellow: '#ffffa5',
  brightBlue: '#d6acff',
  brightMagenta: '#ff92df',
  brightCyan: '#a4ffff',
  brightWhite: '#ffffff',
}

/** One Dark */
const ONE_DARK: TermPalette = {
  background: '#282c34',
  foreground: '#abb2bf',
  cursor: '#528bff',
  selectionBackground: '#3e4451',
  black: '#1e2127',
  red: '#e06c75',
  green: '#98c379',
  yellow: '#d19a66',
  blue: '#61afef',
  magenta: '#c678dd',
  cyan: '#56b6c2',
  white: '#abb2bf',
  brightBlack: '#5c6370',
  brightRed: '#e06c75',
  brightGreen: '#98c379',
  brightYellow: '#e5c07b',
  brightBlue: '#61afef',
  brightMagenta: '#c678dd',
  brightCyan: '#56b6c2',
  brightWhite: '#ffffff',
}

/** Solarized Dark */
const SOLARIZED_DARK: TermPalette = {
  background: '#002b36',
  foreground: '#839496',
  cursor: '#93a1a1',
  selectionBackground: '#073642',
  black: '#073642',
  red: '#dc322f',
  green: '#859900',
  yellow: '#b58900',
  blue: '#268bd2',
  magenta: '#d33682',
  cyan: '#2aa198',
  white: '#eee8d5',
  brightBlack: '#002b36',
  brightRed: '#cb4b16',
  brightGreen: '#586e75',
  brightYellow: '#657b83',
  brightBlue: '#839496',
  brightMagenta: '#6c71c4',
  brightCyan: '#93a1a1',
  brightWhite: '#fdf6e3',
}

/** 可选配色方案（下拉数据源；`follow` 表示跟随应用主题） */
export const COLOR_SCHEMES: TermColorScheme[] = [
  { id: 'follow', label: '跟随应用主题', followTheme: true, dark: VSCODE_DARK, light: VSCODE_LIGHT },
  { id: 'vscode-dark', label: 'VS Code 暗色', followTheme: false, dark: VSCODE_DARK, light: VSCODE_DARK },
  { id: 'vscode-light', label: 'VS Code 浅色', followTheme: false, dark: VSCODE_LIGHT, light: VSCODE_LIGHT },
  { id: 'dracula', label: 'Dracula', followTheme: false, dark: DRACULA, light: DRACULA },
  { id: 'one-dark', label: 'One Dark', followTheme: false, dark: ONE_DARK, light: ONE_DARK },
  { id: 'solarized-dark', label: 'Solarized Dark', followTheme: false, dark: SOLARIZED_DARK, light: SOLARIZED_DARK },
]

/** 读取主题 CSS 变量（`styles/theme.scss` 的 themeVars() 输出到 html.light|dark） */
function cssVar(name: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return v || fallback
}

/**
 * 当前 **DOM 上实际生效** 的暗色状态（以 `<html>` 的 dark/light 类为准）。
 *
 * 为什么不直接用 `isDark`：`useDark` 是「先改 ref、后加类」——
 * 它把 `dark` 类加到 `<html>` 上是在 **post flush** 的 watcher 里做的
 * （`watch(state, onChanged, { flush: 'post', immediate: true })`）。
 * 任何在 **pre flush** 阶段读主题 CSS 变量的代码，都会读到切换前的旧值
 * （表现：切到暗色后终端画布仍是白底深字）。
 * 所以凡是「据主题变量算出来的东西」，都必须以 DOM 上的类为准。
 */
export function domIsDark(fallback: boolean): boolean {
  if (typeof document === 'undefined') return fallback
  const cl = document.documentElement.classList
  if (cl.contains('dark')) return true
  if (cl.contains('light')) return false
  return fallback
}

/**
 * 解析当前生效的调色板。
 * @param schemeId 方案 id（未知 id 回落 follow）
 * @param isDark 当前是否暗色主题（仅作 DOM 不可用时的兜底）
 */
export function resolvePalette(schemeId: string, isDark: boolean): TermPalette {
  const scheme = COLOR_SCHEMES.find((s) => s.id === schemeId) ?? COLOR_SCHEMES[0]
  // 以 DOM 上真实生效的主题为准，避免与 CSS 变量读到的新旧不一致
  isDark = domIsDark(isDark)
  const base = isDark ? scheme.dark : scheme.light
  if (!scheme.followTheme) return base
  // 跟随主题：UI 相关色从 CSS 变量取，ANSI 语义色保持 VS Code 标准
  return {
    ...base,
    background: cssVar('--bgColor', base.background),
    foreground: cssVar('--textColor', base.foreground),
    cursor: cssVar('--textColor', base.cursor),
    selectionBackground: isDark ? '#264f78' : '#add6ff80',
  }
}

/** 转成 xterm 的 ITheme（字段名一致，这里只做一次浅拷贝，避免外部改到常量对象） */
export function toXtermTheme(p: TermPalette): Record<string, string> {
  return { ...p }
}

/** 搜索命中高亮用色（跟随主题，保证在终端底色上可见） */
export function searchDecorations(isDark: boolean) {
  return {
    matchBackground: isDark ? '#5a4a00' : '#ffe9a8',
    matchBorder: isDark ? '#8a7300' : '#e0b400',
    matchOverviewRuler: '#e0b400',
    activeMatchBackground: isDark ? '#c46b00' : '#ffb300',
    activeMatchBorder: isDark ? '#ffd166' : '#ff8f00',
    activeMatchColorOverviewRuler: '#ff8f00',
  }
}
