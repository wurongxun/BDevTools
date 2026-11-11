/**
 * files 编辑器外观偏好（模块级单例）。
 *
 * 真源是 `settings.json` 的 `files_editor` 键，值形状沿用项目统一的 `{ value: T }`。
 * 读写句柄自己拿而不复用 `useSettingsStore`：那是 settings 插件的内部模块，
 * `verify-plugin-boundaries.mjs` 禁止插件间互相 import。`load()` 本身是即时读写磁盘的
 * 薄 RPC，多一个句柄不会互相覆盖。
 *
 * `pureViewMode` 是 M3 的回退开关：打开后所有增强项失效并强制只读，用于验收时
 * 快速回到 M1 的「纯查看/纯编辑」基线（路线图第 7 节 Go/No-Go 门禁 2）。
 */
import { computed, reactive, ref, watch } from 'vue'
import { load, type Store } from '@/core/runtime'

const STORE_KEY = 'files_editor'

export interface EditorPreferences {
  wordWrap: 'on' | 'off'
  minimap: boolean
  renderWhitespace: 'none' | 'selection' | 'all'
  guidesIndentation: boolean
  bracketPairColorization: boolean
  mouseWheelZoom: boolean
  fontSize: number
  fontFamily: string
  lineNumbers: 'on' | 'off' | 'relative'
  /** 回退开关：为真时忽略全部外观增强并强制只读 */
  pureViewMode: boolean
  /** command id → 组合键字符串，如 "Ctrl+Alt+B"（空串表示不绑定） */
  keybindings: Record<string, string>
}

export const DEFAULT_PREFERENCES: EditorPreferences = {
  wordWrap: 'off',
  minimap: false,
  renderWhitespace: 'none',
  guidesIndentation: true,
  bracketPairColorization: true,
  mouseWheelZoom: true,
  fontSize: 14,
  fontFamily: '',
  lineNumbers: 'on',
  pureViewMode: false,
  keybindings: {
    'files.view.toggleSplit': 'Ctrl+\\',
    'files.bookmark.toggle': 'Ctrl+F2',
    'files.bookmark.next': 'F2',
    'files.bookmark.prev': 'Shift+F2',
    'files.editor.zoomIn': 'Ctrl+=',
    'files.editor.zoomOut': 'Ctrl+-'
  }
}

export const prefs = reactive<EditorPreferences>(
  JSON.parse(JSON.stringify(DEFAULT_PREFERENCES)) as EditorPreferences
)

/** 偏好设置弹窗开关（命令面板 / 状态栏共用）。 */
export const editorPreferencesVisible = ref(false)

let storeHandle: Promise<Store> | null = null
function store(): Promise<Store> {
  if (!storeHandle) {
    storeHandle = load('settings.json', { autoSave: false, defaults: {} }).catch((e: unknown) => {
      storeHandle = null
      throw e
    })
  }
  return storeHandle
}

let initialized = false
let saveTimer: ReturnType<typeof setTimeout> | null = null

export async function initEditorPreferences(): Promise<void> {
  if (initialized) return
  initialized = true
  try {
    const s = await store()
    const saved = await s.get<{ value: Partial<EditorPreferences> }>(STORE_KEY)
    if (saved?.value && typeof saved.value === 'object') {
      const { keybindings, ...rest } = saved.value
      Object.assign(prefs, rest)
      // 键位表做合并而不是整体替换：新增命令在老配置里也能拿到默认键位
      if (keybindings && typeof keybindings === 'object') {
        prefs.keybindings = { ...DEFAULT_PREFERENCES.keybindings, ...keybindings }
      }
      if (typeof prefs.fontSize !== 'number' || !Number.isFinite(prefs.fontSize)) {
        prefs.fontSize = DEFAULT_PREFERENCES.fontSize
      }
      prefs.fontSize = Math.min(28, Math.max(10, prefs.fontSize))
    }
  } catch (e) {
    console.warn('[files/preferences] 读取编辑器偏好失败，使用默认值', e)
  }
  watch(prefs, scheduleSave, { deep: true })
}

function scheduleSave() {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(async () => {
    saveTimer = null
    try {
      const s = await store()
      await s.set(STORE_KEY, { value: JSON.parse(JSON.stringify(prefs)) })
    } catch (e) {
      console.warn('[files/preferences] 保存编辑器偏好失败', e)
    }
  }, 300)
}

/** 传给 MonacoEditor 的 options（纯对象，业务侧不 import monaco）。 */
export const editorOptions = computed<Record<string, unknown>>(() => ({
  wordWrap: prefs.pureViewMode ? 'off' : prefs.wordWrap,
  minimap: { enabled: !prefs.pureViewMode && prefs.minimap },
  renderWhitespace: prefs.pureViewMode ? 'none' : prefs.renderWhitespace,
  guides: {
    indentation: !prefs.pureViewMode && prefs.guidesIndentation,
    bracketPairs: !prefs.pureViewMode && prefs.bracketPairColorization
  },
  bracketPairColorization: { enabled: !prefs.pureViewMode && prefs.bracketPairColorization },
  mouseWheelZoom: !prefs.pureViewMode && prefs.mouseWheelZoom,
  fontSize: prefs.pureViewMode ? DEFAULT_PREFERENCES.fontSize : prefs.fontSize,
  fontFamily: prefs.pureViewMode ? '' : prefs.fontFamily,
  lineNumbers: prefs.pureViewMode ? 'on' : prefs.lineNumbers,
  // 行书签需要 glyph margin；纯查看模式下也保留（书签是导航而非外观增强，且无副作用）
  glyphMargin: true
}))

export function zoomBy(delta: number) {
  prefs.fontSize = Math.min(28, Math.max(10, prefs.fontSize + delta))
}

export function resetPreferences() {
  Object.assign(prefs, JSON.parse(JSON.stringify(DEFAULT_PREFERENCES)))
}

/** Ctrl / Cmd 不区分：Windows 与 macOS 都把 meta 当作 Ctrl 用。 */
export function matchKeybinding(e: KeyboardEvent, combo: string): boolean {
  if (!combo) return false
  const parts = combo.toLowerCase().split('+').map((p) => p.trim()).filter(Boolean)
  if (parts.length === 0) return false
  const key = parts[parts.length - 1]
  const needCtrl = parts.includes('ctrl') || parts.includes('cmd')
  const needShift = parts.includes('shift')
  const needAlt = parts.includes('alt')
  if (needCtrl !== (e.ctrlKey || e.metaKey)) return false
  if (needShift !== e.shiftKey) return false
  if (needAlt !== e.altKey) return false
  const actual = e.key.toLowerCase()
  if (actual === key) return true
  // 数字/符号键：e.key 可能带 shift 变形（如 Ctrl+Shift+8 → '*'），再用 code 兜底
  const codeKey = e.code.replace(/^(Key|Digit)/, '').toLowerCase()
  return codeKey === key
}

/** 找到本次按键对应的命令 id；返回第一个命中（键位表按注册顺序去重）。 */
export function commandForKeybinding(e: KeyboardEvent): string | null {
  for (const [id, combo] of Object.entries(prefs.keybindings)) {
    if (matchKeybinding(e, combo)) return id
  }
  return null
}
