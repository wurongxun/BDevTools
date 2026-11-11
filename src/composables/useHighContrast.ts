import { ref, watch } from 'vue'
import { load } from '@/core/runtime'

/**
 * 高对比度模式：与「暗黑模式」**正交**的第二个开关（浅/深 × 普通/高对比，共 4 种组合）。
 *
 * 刻意不做成第三套主题：它改的只是**色值**（见 styles/contrast.scss 的变量覆盖），
 * 不是结构，做成独立主题会与日后主主题的样式改动脱节。
 *
 * 与暗黑模式的边界：
 *   - 暗黑状态由 `useDark.ts` 的全局单例负责（vueuse 管理 `<html class="light|dark">`）
 *   - 这里只负责 `<html class="hc">` 的挂摘 + KV 读写，两者互不干扰
 * 持久化：settings.json 的 `high_contrast` 键（与 `appearance` 同文件、不同键）。
 */
export const isHighContrast = ref(false)

export function toggleHighContrast() {
  isHighContrast.value = !isHighContrast.value
}

const SETTINGS_FILE = 'settings.json'
const HC_KEY = 'high_contrast'
let initialized = false

/** 应用启动时调用一次（main.ts 挂载前 await，保证首帧即正确、无闪烁） */
export async function initHighContrast() {
  if (initialized) return
  initialized = true
  try {
    const store = await load(SETTINGS_FILE)
    const saved = await store.get<{ value: boolean }>(HC_KEY)
    isHighContrast.value = saved?.value === true
  } catch {
    isHighContrast.value = false
  }
}

// 状态 → <html class="hc">（immediate 保证初始化读取后立刻同步一次）
watch(
  isHighContrast,
  (val) => {
    if (typeof document === 'undefined') return
    document.documentElement.classList.toggle('hc', val)
  },
  { immediate: true }
)

// 状态 → KV（防抖，与 useDark 的持久化节奏一致）
let persistTimer: ReturnType<typeof setTimeout> | null = null
watch(isHighContrast, (val) => {
  if (persistTimer) clearTimeout(persistTimer)
  persistTimer = setTimeout(() => {
    load(SETTINGS_FILE)
      .then((store) => store.set(HC_KEY, { value: val }))
      .catch(() => {})
  }, 200)
})
