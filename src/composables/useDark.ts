import { useDark, useToggle } from '@vueuse/core'
import { watch } from 'vue'
import { load } from '@/core/runtime'

/**
 * 主题状态：不再写 localStorage，改为项目自研 KV（settings.json 的 `appearance` 键）持久化，
 * 符合「应用数据走 KV、不用 localStorage 存关键数据」的约定（见 CODEBUDDY.md）。
 */
export const isDark = useDark({
  valueDark: 'dark',
  valueLight: 'light',
})

export const toggleDark = useToggle(isDark)

const SETTINGS_FILE = 'settings.json'
const THEME_KEY = 'appearance'
let initialized = false

/**
 * 应用启动时调用一次：从 KV 读取主题偏好；无偏好则兼容旧版 localStorage、再回退到系统配色。
 * 在 main.ts 挂载前 await，保证首帧即是正确主题，无闪烁。
 */
export async function initTheme() {
  if (initialized) return
  initialized = true
  let pref: string | null = null
  try {
    const store = await load(SETTINGS_FILE)
    pref = (await store.get<string>(THEME_KEY)) ?? null
  } catch {
    pref = null
  }
  // 兼容升级前写入 localStorage 的旧偏好
  if (pref !== 'dark' && pref !== 'light' && typeof localStorage !== 'undefined') {
    pref = localStorage.getItem('theme-appearance')
  }
  if (pref === 'dark' || pref === 'light') {
    isDark.value = pref === 'dark'
  } else if (typeof window !== 'undefined' && window.matchMedia) {
    // 首次运行：跟随系统配色（WCAG 相关的桌面应用惯例）
    isDark.value = window.matchMedia('(prefers-color-scheme: dark)').matches
  }
}

// 主题变化后防抖持久化到 KV（初始化阶段不会重复写回）
let persistTimer: ReturnType<typeof setTimeout> | null = null
watch(isDark, (val) => {
  if (persistTimer) clearTimeout(persistTimer)
  persistTimer = setTimeout(() => {
    load(SETTINGS_FILE)
      .then((store) => store.set(THEME_KEY, val ? 'dark' : 'light'))
      .catch(() => {})
  }, 200)
})
