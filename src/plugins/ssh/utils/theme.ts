/**
 * P3 配色方案：内置两套 + .xcs / JSON 导入。
 *
 * Xshell 的 .xcs 是 INI：`[Colors]` 下 `Color0..Color15` / `ForegroundColor` /
 * `BackgroundColor` 等，颜色值常见 `R,G,B`、`#RRGGBB`、`0xRRGGBB` 三种写法。
 * 这里做宽松解析，读不到的键一律回落内置深色，保证任何残缺方案都能安全应用。
 */
import type { TerminalColorScheme, TerminalTheme } from '../types'

export const BUILTIN_DARK: TerminalTheme = {
  background: '#1e1e1e',
  foreground: '#d4d4d4',
  cursor: '#d4d4d4',
  selectionBackground: 'rgba(255, 255, 255, 0.3)',
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
  brightWhite: '#ffffff'
}

export const BUILTIN_LIGHT: TerminalTheme = {
  background: '#ffffff',
  foreground: '#383a42',
  cursor: '#383a42',
  selectionBackground: 'rgba(0, 0, 0, 0.2)',
  black: '#383a42',
  red: '#e45649',
  green: '#50a14f',
  yellow: '#c18401',
  blue: '#0184bc',
  magenta: '#a626a4',
  cyan: '#0997b3',
  white: '#fafafa',
  brightBlack: '#8a8f98',
  brightRed: '#f0746a',
  brightGreen: '#6fbf67',
  brightYellow: '#e0ab32',
  brightBlue: '#3ea6d8',
  brightMagenta: '#c264c2',
  brightCyan: '#38b7c9',
  brightWhite: '#ffffff'
}

export const AUTO_SCHEME_ID = 'auto'

export const BUILTIN_SCHEMES: TerminalColorScheme[] = [
  { id: AUTO_SCHEME_ID, name: '跟随应用主题', source: 'builtin', theme: BUILTIN_DARK },
  { id: 'builtin-dark', name: '内置深色', source: 'builtin', theme: BUILTIN_DARK },
  { id: 'builtin-light', name: '内置浅色', source: 'builtin', theme: BUILTIN_LIGHT }
]

export function isBuiltinScheme(id: string): boolean {
  return BUILTIN_SCHEMES.some(s => s.id === id)
}

/** 合并缺省键：xterm 的 theme 对象必须给全 16 色，否则未给的会渲染成默认黑 */
export function normalizeTheme(partial?: Partial<TerminalTheme> | null, base: TerminalTheme = BUILTIN_DARK): TerminalTheme {
  return { ...base, ...(partial || {}) }
}

export function resolveTheme(
  schemeId: string,
  schemes: TerminalColorScheme[],
  dark: boolean
): TerminalTheme {
  if (schemeId === AUTO_SCHEME_ID) return dark ? BUILTIN_DARK : BUILTIN_LIGHT
  const scheme = schemes.find(s => s.id === schemeId)
  if (!scheme) return dark ? BUILTIN_DARK : BUILTIN_LIGHT
  return normalizeTheme(scheme.theme, dark ? BUILTIN_DARK : BUILTIN_LIGHT)
}

const ANSI_KEYS: Array<keyof TerminalTheme> = [
  'black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white',
  'brightBlack', 'brightRed', 'brightGreen', 'brightYellow',
  'brightBlue', 'brightMagenta', 'brightCyan', 'brightWhite'
]

/** 把 `.xcs` 里的颜色值解析成 `#rrggbb`；解析不了返回 null */
export function parseColor(value: string): string | null {
  const v = value.trim().replace(/^"|"$/g, '')
  if (!v) return null
  const hex = /^#?([0-9a-f]{6})$/i.exec(v) || /^0x([0-9a-f]{6})$/i.exec(v)
  if (hex) return `#${hex[1].toLowerCase()}`
  const short = /^#?([0-9a-f]{3})$/i.exec(v)
  if (short) return `#${short[1].split('').map(c => c + c).join('').toLowerCase()}`
  const rgb = v.match(/-?\d+/g)
  if (rgb && rgb.length >= 3) {
    const parts = rgb.slice(0, 3).map(n => Math.max(0, Math.min(255, Number(n))))
    if (parts.every(Number.isFinite)) {
      return `#${parts.map(n => n.toString(16).padStart(2, '0')).join('')}`
    }
  }
  return null
}

function keyOf(raw: string): string {
  return raw.trim().toLowerCase().replace(/[\s_-]/g, '')
}

/** 宽松解析 .xcs。返回 null 表示「不是有效的配色文件」，UI 应提示而不是应用一个全黑方案。 */
export function parseXcs(text: string, name: string): TerminalColorScheme | null {
  const theme: Partial<TerminalTheme> = {}
  let hits = 0
  for (const line of text.replace(/^\uFEFF/, '').split(/\r?\n/)) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith(';') || trimmed.startsWith('#') || trimmed.startsWith('[')) continue
    const eq = trimmed.indexOf('=')
    if (eq < 0) continue
    const key = keyOf(trimmed.slice(0, eq))
    const color = parseColor(trimmed.slice(eq + 1))
    if (!color) continue
    const ansi = /^color(\d{1,2})$/.exec(key)
    if (ansi) {
      const idx = Number(ansi[1])
      if (idx >= 0 && idx < ANSI_KEYS.length) {
        theme[ANSI_KEYS[idx]] = color
        hits++
      }
      continue
    }
    const mapped: Record<string, keyof TerminalTheme> = {
      foregroundcolor: 'foreground', foreground: 'foreground', textcolor: 'foreground',
      text: 'foreground', normaltextcolor: 'foreground',
      backgroundcolor: 'background', background: 'background', bgcolor: 'background',
      cursorcolor: 'cursor', cursor: 'cursor', caretcolor: 'cursor',
      selectioncolor: 'selectionBackground', selection: 'selectionBackground',
      boldcolor: 'brightWhite'
    }
    const target = mapped[key]
    if (target) {
      (theme as any)[target] = color
      hits++
    }
  }
  if (hits < 2 || (!theme.foreground && !theme.background)) return null
  return {
    id: `xcs-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
    name: name.replace(/\.(xcs|txt|ini)$/i, ''),
    source: 'imported',
    theme: normalizeTheme(theme)
  }
}

/** 解析我们导出的 JSON，或任意 xterm ITheme JSON */
export function parseSchemeJson(text: string, name: string): TerminalColorScheme | null {
  try {
    const parsed = JSON.parse(text)
    const raw = parsed?.theme && typeof parsed.theme === 'object' ? parsed.theme : parsed
    if (!raw || typeof raw !== 'object') return null
    const theme = normalizeTheme(raw as Partial<TerminalTheme>)
    if (!parsed?.theme && !raw.foreground && !raw.background) return null
    return {
      id: `json-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      name: String(parsed?.name || name || '导入方案').replace(/\.json$/i, ''),
      source: 'imported',
      theme
    }
  } catch {
    return null
  }
}

export function parseSchemeFile(text: string, name: string): TerminalColorScheme | null {
  return /\.xcs$/i.test(name) ? parseXcs(text, name) : parseSchemeJson(text, name)
}

export function exportScheme(scheme: TerminalColorScheme): string {
  return JSON.stringify({
    kind: 'bdevtools-ssh-theme',
    version: 1,
    name: scheme.name,
    theme: scheme.theme
  }, null, 2)
}
