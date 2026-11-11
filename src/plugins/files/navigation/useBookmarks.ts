/**
 * 行书签（F1）。按文件路径存行号数组，持久化到 settings.json 的 files_bookmarks。
 *
 * 行号是「文本行」而非 Monaco decoration id：文件被外部改动后行号可能漂移，
 * 这里不做重定位（Notepad++ 同样如此），书签面板始终展示原始行号，避免猜错位置。
 */
import { computed, ref, watch } from 'vue'
import { load, type Store } from '@/core/runtime'

const STORE_KEY = 'files_bookmarks'

const bookmarks = ref<Record<string, number[]>>({})
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

export async function initBookmarks(): Promise<void> {
  if (initialized) return
  initialized = true
  try {
    const s = await store()
    const saved = await s.get<{ value: Record<string, number[]> }>(STORE_KEY)
    if (saved?.value && typeof saved.value === 'object') {
      const clean: Record<string, number[]> = {}
      for (const [path, lines] of Object.entries(saved.value)) {
        if (Array.isArray(lines)) {
          clean[path] = [...new Set(lines.filter((n) => Number.isInteger(n) && n > 0))].sort(
            (a, b) => a - b
          )
        }
      }
      bookmarks.value = clean
    }
  } catch (e) {
    console.warn('[files/bookmarks] 读取书签失败', e)
  }
  watch(bookmarks, scheduleSave, { deep: true })
}

function scheduleSave() {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(async () => {
    saveTimer = null
    try {
      const s = await store()
      await s.set(STORE_KEY, { value: JSON.parse(JSON.stringify(bookmarks.value)) })
    } catch (e) {
      console.warn('[files/bookmarks] 保存书签失败', e)
    }
  }, 300)
}

function linesFor(path: string): number[] {
  return bookmarks.value[path] || []
}

function has(path: string, line: number): boolean {
  return linesFor(path).includes(line)
}

function toggle(path: string, line: number): boolean {
  if (!path || !Number.isInteger(line) || line < 1) return false
  const next = { ...bookmarks.value }
  const current = next[path] || []
  if (current.includes(line)) {
    const rest = current.filter((n) => n !== line)
    if (rest.length) next[path] = rest
    else delete next[path]
  } else {
    next[path] = [...current, line].sort((a, b) => a - b)
  }
  bookmarks.value = next
  return !current.includes(line)
}

function clear(path: string) {
  if (!bookmarks.value[path]) return
  const next = { ...bookmarks.value }
  delete next[path]
  bookmarks.value = next
}

function next(path: string, fromLine: number): number | null {
  const lines = linesFor(path)
  return lines.find((n) => n > fromLine) ?? lines[0] ?? null
}

function prev(path: string, fromLine: number): number | null {
  const lines = linesFor(path)
  return [...lines].reverse().find((n) => n < fromLine) ?? lines[lines.length - 1] ?? null
}

export function useBookmarks() {
  return {
    bookmarks: computed(() => bookmarks.value),
    linesFor,
    has,
    toggle,
    clear,
    next,
    prev
  }
}
