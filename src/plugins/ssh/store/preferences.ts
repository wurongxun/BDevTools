/**
 * P3 偏好中心：高亮集 / 配色方案 / 键盘映射 / 自动登录 / 宏 / 日志 / 分组标签 / 保险库。
 *
 * 这些数据是**用户偏好**，与会话配置（后端 SQLite）刻意分开，统一存
 * `localStorage:ssh-preferences-v1`；因此不参与插件裁剪、也不受会话保险库加密影响。
 * 遵守 `ensureLoaded` 约定：store 创建即读盘，写盘由 deep watch 自动完成。
 */
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { invoke } from '@/api/rpc'
import { isDark } from '@/composables/useDark'
import type {
  ExpectRule, GroupTag, HighlightRule, HighlightSet, KeyBinding, LogSettings,
  MacroScript, TerminalColorScheme, VaultState
} from '../types'
import { AUTO_SCHEME_ID, BUILTIN_SCHEMES, resolveTheme } from '../utils/theme'
import { defaultExpectRules } from '../utils/automation'

const STORAGE_KEY = 'ssh-preferences-v1'

export function uid(prefix: string): string {
  return `${prefix}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`
}

function defaultHighlightRules(): HighlightRule[] {
  return [
    {
      id: 'hl-error', pattern: '(ERROR|FATAL|Exception|失败|错误)', regex: true,
      caseSensitive: false, wholeWord: false,
      foreground: '#ff6b6b', background: '', bold: true, enabled: true
    },
    {
      id: 'hl-warn', pattern: '(WARN|WARNING|警告)', regex: true,
      caseSensitive: false, wholeWord: false,
      foreground: '#1e1e1e', background: '#ffd166', bold: false, enabled: true
    },
    {
      id: 'hl-ok', pattern: '(SUCCESS|OK|成功|已完成)', regex: true,
      caseSensitive: false, wholeWord: false,
      foreground: '#4ade80', background: '', bold: false, enabled: true
    }
  ]
}

function defaultHighlightSets(): HighlightSet[] {
  return [{ id: 'hs-default', name: '默认高亮集', enabled: true, rules: defaultHighlightRules() }]
}

function defaultKeyBindings(): KeyBinding[] {
  return [
    {
      id: 'kb-clear', label: '清屏', key: 'c', ctrl: false, alt: true, shift: false, meta: false,
      action: 'clear', payload: '', enabled: true
    },
    {
      id: 'kb-top', label: '发送 top', key: 't', ctrl: false, alt: true, shift: false, meta: false,
      action: 'send', payload: 'top\r', enabled: false
    }
  ]
}

function defaultMacros(): MacroScript[] {
  return [{
    id: 'macro-sysinfo', name: '登录健康检查（示例）', enabled: true, autoRun: false,
    steps: [
      { id: 'ms-1', kind: 'send', value: 'echo "=== $(hostname) ==="; uptime; df -h /', arg: 0, newline: true },
      { id: 'ms-2', kind: 'expect', value: '===', arg: 10000, newline: false },
      { id: 'ms-3', kind: 'wait', value: '', arg: 200, newline: false }
    ]
  }]
}

function defaultLogSettings(): LogSettings {
  return { enabled: true, autoRecord: false, timestamp: true, logInput: false, maskSensitive: true }
}

interface Persisted {
  highlightSets: HighlightSet[]
  colorSchemes: TerminalColorScheme[]
  activeSchemeId: string
  keyBindings: KeyBinding[]
  expectRules: ExpectRule[]
  macros: MacroScript[]
  logSettings: LogSettings
  groupTags: GroupTag[]
  groupViewMode: 'list' | 'tree'
}

function defaults(): Persisted {
  return {
    highlightSets: defaultHighlightSets(),
    colorSchemes: [],
    activeSchemeId: AUTO_SCHEME_ID,
    keyBindings: defaultKeyBindings(),
    expectRules: defaultExpectRules(),
    macros: defaultMacros(),
    logSettings: defaultLogSettings(),
    groupTags: [],
    groupViewMode: 'list'
  }
}

function load(): Persisted {
  const base = defaults()
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return base
    const saved = JSON.parse(raw) as Partial<Persisted>
    return {
      ...base,
      ...saved,
      // 内置方案始终由代码提供，避免旧文件把内置项覆盖成坏数据
      colorSchemes: Array.isArray(saved.colorSchemes)
        ? saved.colorSchemes.filter(s => s && s.theme && s.source === 'imported')
        : [],
      highlightSets: Array.isArray(saved.highlightSets) ? saved.highlightSets : base.highlightSets,
      keyBindings: Array.isArray(saved.keyBindings) ? saved.keyBindings : base.keyBindings,
      expectRules: Array.isArray(saved.expectRules) ? saved.expectRules : base.expectRules,
      macros: Array.isArray(saved.macros) ? saved.macros : base.macros,
      groupTags: Array.isArray(saved.groupTags) ? saved.groupTags : [],
      logSettings: { ...base.logSettings, ...(saved.logSettings || {}) }
    }
  } catch {
    return base
  }
}

export interface VaultResult {
  ok: boolean
  message: string
  count?: number
}

export const useSshPreferencesStore = defineStore('sshPreferences', () => {
  const initial = load()

  const highlightSets = ref<HighlightSet[]>(initial.highlightSets)
  const colorSchemes = ref<TerminalColorScheme[]>(initial.colorSchemes)
  const activeSchemeId = ref(initial.activeSchemeId)
  const keyBindings = ref<KeyBinding[]>(initial.keyBindings)
  const expectRules = ref<ExpectRule[]>(initial.expectRules)
  const macros = ref<MacroScript[]>(initial.macros)
  const logSettings = ref<LogSettings>(initial.logSettings)
  const groupTags = ref<GroupTag[]>(initial.groupTags)
  const groupViewMode = ref<'list' | 'tree'>(initial.groupViewMode)
  const vault = ref<VaultState>({ enabled: false, unlocked: false, path: '' })

  /** 所有启用集合里的启用规则，按集合顺序合并 */
  const activeRules = computed<HighlightRule[]>(() =>
    highlightSets.value.filter(s => s.enabled).flatMap(s => s.rules).filter(r => r.enabled)
  )

  const allSchemes = computed<TerminalColorScheme[]>(() => [...BUILTIN_SCHEMES, ...colorSchemes.value])
  const terminalTheme = computed(() =>
    resolveTheme(activeSchemeId.value, colorSchemes.value, isDark.value)
  )

  // ==================== 自动保存（状态必须对用户可见） ====================

  /** saved = 与本地磁盘一致；saving = debounce 待写；error = 写入失败且改动可能丢失 */
  const saveState = ref<'saved' | 'saving' | 'error'>('saved')
  const lastSavedAt = ref<number | null>(null)
  const saveError = ref('')

  function snapshot(): Persisted {
    return {
      highlightSets: highlightSets.value,
      colorSchemes: colorSchemes.value,
      activeSchemeId: activeSchemeId.value,
      keyBindings: keyBindings.value,
      expectRules: expectRules.value,
      macros: macros.value,
      logSettings: logSettings.value,
      groupTags: groupTags.value,
      groupViewMode: groupViewMode.value
    }
  }

  let saveTimer: ReturnType<typeof setTimeout> | null = null

  /** 真正写盘。失败绝不能静默：用户会以为宏已经保存然后关掉窗口。 */
  function persist(): boolean {
    if (saveTimer) {
      clearTimeout(saveTimer)
      saveTimer = null
    }
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(snapshot()))
      saveState.value = 'saved'
      lastSavedAt.value = Date.now()
      saveError.value = ''
      return true
    } catch (error) {
      saveState.value = 'error'
      saveError.value = error instanceof Error ? error.message : String(error)
      return false
    }
  }

  function scheduleSave() {
    saveState.value = 'saving'
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(persist, 200)
  }

  /** 关闭窗口前调用：把 debounce 里尚未落盘的改动立刻写完 */
  function saveNow(): boolean {
    return persist()
  }

  watch(
    [highlightSets, colorSchemes, activeSchemeId, keyBindings, expectRules, macros, logSettings, groupTags, groupViewMode],
    scheduleSave,
    { deep: true }
  )

  // 首次启动把默认偏好落盘，让初始的「已保存」状态名副其实
  try {
    if (localStorage.getItem(STORAGE_KEY) === null) persist()
  } catch { /* 隐私模式下 localStorage 本身不可用，由后续 persist 报 error */ }

  // 页面隐藏 / 关闭时兜底 flush（store 是全局单例，生命周期等同应用，无需注销监听）
  if (typeof window !== 'undefined') {
    const flush = () => { if (saveTimer) persist() }
    window.addEventListener('pagehide', flush)
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'hidden') flush()
    })
  }

  function groupColor(path: string): string {
    return groupTags.value.find(t => t.path === path)?.color || ''
  }

  function setGroupColor(path: string, color: string) {
    const found = groupTags.value.find(t => t.path === path)
    if (found) found.color = color
    else groupTags.value.push({ path, color })
  }

  // ==================== 保险库 ====================

  async function refreshVaultStatus(): Promise<void> {
    try {
      const res = await invoke<any>('ssh_vault_status')
      if (res?.code === 200 && res.data) vault.value = res.data
    } catch { /* 插件裁剪 / 应用启动中，保持默认状态 */ }
  }

  async function vaultCall(cmd: string, args: Record<string, unknown>): Promise<VaultResult> {
    try {
      const res = await invoke<any>(cmd, args)
      await refreshVaultStatus()
      return { ok: res?.code === 200, message: res?.msg || '', count: res?.data as number | undefined }
    } catch (error) {
      return { ok: false, message: String(error) }
    }
  }

  const setupVault = (password: string, migrate = true) =>
    vaultCall('ssh_vault_setup', { password, migrate })
  const unlockVault = (password: string) => vaultCall('ssh_vault_unlock', { password })
  const lockVault = () => vaultCall('ssh_vault_lock', {})
  const encryptExisting = () => vaultCall('ssh_vault_encrypt_existing', {})
  const disableVault = (password: string) => vaultCall('ssh_vault_disable', { password })

  // 遵守 ensureLoaded：store 创建即拉取一次后端保险库状态
  void refreshVaultStatus()

  /** 导入导出用的偏好子集（会话与转发规则由 connectionStore / 组件补齐） */
  function preferencesBundle() {
    return {
      highlightSets: highlightSets.value,
      colorSchemes: colorSchemes.value,
      macros: macros.value,
      expectRules: expectRules.value,
      keyBindings: keyBindings.value,
      groupTags: groupTags.value
    }
  }

  function applyPreferencesBundle(bundle: Partial<ReturnType<typeof preferencesBundle>>, merge: boolean) {
    const put = <T,>(target: { value: T[] }, incoming: T[] | undefined) => {
      if (!Array.isArray(incoming)) return
      target.value = merge ? [...target.value, ...incoming] : incoming
    }
    put(highlightSets, bundle.highlightSets)
    put(colorSchemes, bundle.colorSchemes)
    put(macros, bundle.macros)
    put(expectRules, bundle.expectRules)
    put(keyBindings, bundle.keyBindings)
    put(groupTags, bundle.groupTags)
  }

  return {
    highlightSets, colorSchemes, activeSchemeId, keyBindings, expectRules, macros,
    logSettings, groupTags, groupViewMode, vault,
    saveState, lastSavedAt, saveError, saveNow,
    activeRules, allSchemes, terminalTheme,
    groupColor, setGroupColor,
    refreshVaultStatus, setupVault, unlockVault, lockVault, encryptExisting, disableVault,
    preferencesBundle, applyPreferencesBundle
  }
})
