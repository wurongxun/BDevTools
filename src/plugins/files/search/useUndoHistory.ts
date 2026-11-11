/**
 * 写盘操作的撤销清单（M4：收口 M2 遗留「撤销仅会话内有效」）。
 *
 * 真源是 `settings.json` 的 `files_undo_history` 键，形状沿用项目统一的 `{ value: T }`。
 * 为什么要落盘：`.bak` 本来就在磁盘上，但清单只在内存里，重启后备份成了孤儿 —— 用户
 * 明明有备份却没有入口恢复。落盘后还必须做两件事：
 *  ① 启动/首次使用时用 `files_undo_probe` 过滤掉备份已不存在的条目（否则按钮假亮）；
 *  ② 批次有上限，超出时删掉最老批次的 `.bak`，不让隐藏备份无限堆积。
 *
 * 批量替换与「编码/EOL 转换」共用这一份清单，因为二者的备份契约完全一致：
 * 都是 `UndoEntry { path, backup_path }`，恢复时按原始字节覆盖，不经过任何编码转换。
 */
import { computed, ref, watch } from 'vue'
import NotificationUtil from '@/utils/NotificationUtil'
import { load, type Store } from '@/core/runtime'
import { useDocumentManagerStore } from '@/store/modules/documents'
import { undoReplace, type UndoEntry } from './fileSearchApi'
import { discardUndoEntries, probeUndoEntries } from '../encoding/fileConvertApi'

const STORE_KEY = 'files_undo_history'
/** 最多保留 8 批；每批上限 200 条，防止 IPC / settings 被撑爆 */
const MAX_BATCHES = 8
const MAX_ENTRIES_PER_BATCH = 200

export interface UndoBatch {
  id: string
  label: string
  createdAt: number
  entries: UndoEntry[]
}

export const undoBatches = ref<UndoBatch[]>([])
export const undoHistoryVisible = ref(false)
export const canUndo = computed(() => undoBatches.value.length > 0)

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
let initPromise: Promise<void> | null = null

function sanitize(raw: unknown): UndoBatch[] {
  if (!Array.isArray(raw)) return []
  const out: UndoBatch[] = []
  for (const item of raw) {
    if (!item || typeof item !== 'object') continue
    const b = item as Partial<UndoBatch>
    if (typeof b.id !== 'string' || !Array.isArray(b.entries)) continue
    const entries = b.entries
      .filter(
        (e): e is UndoEntry =>
          !!e && typeof e === 'object' && typeof e.path === 'string' && typeof e.backup_path === 'string'
      )
      .slice(0, MAX_ENTRIES_PER_BATCH)
    if (!entries.length) continue
    out.push({
      id: b.id,
      label: typeof b.label === 'string' ? b.label : '写盘操作',
      createdAt: typeof b.createdAt === 'number' ? b.createdAt : Date.now(),
      entries
    })
  }
  return out.slice(-MAX_BATCHES)
}

/** 懒初始化：面板首次用到才读盘并探测，避免插件加载阶段就发 RPC。 */
export function initUndoHistory(): Promise<void> {
  if (initPromise) return initPromise
  initPromise = (async () => {
    if (initialized) return
    initialized = true
    try {
      const s = await store()
      const saved = await s.get<{ value: unknown }>(STORE_KEY)
      const batches = sanitize(saved?.value)
      if (batches.length) {
        const flat = batches.flatMap((b) => b.entries)
        const res = await probeUndoEntries(flat)
        const alive = new Set((res?.data || []).map((e) => `${e.path}::${e.backup_path}`))
        undoBatches.value = batches
          .map((b) => ({
            ...b,
            entries: b.entries.filter((e) => alive.has(`${e.path}::${e.backup_path}`))
          }))
          .filter((b) => b.entries.length > 0)
      }
    } catch (e) {
      console.warn('[files/undo] 读取撤销清单失败，本次会话仍可正常撤销', e)
    }
    watch(undoBatches, scheduleSave, { deep: true })
  })()
  return initPromise
}

function scheduleSave() {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(async () => {
    saveTimer = null
    try {
      const s = await store()
      await s.set(STORE_KEY, { value: JSON.parse(JSON.stringify(undoBatches.value)) })
    } catch (e) {
      console.warn('[files/undo] 保存撤销清单失败', e)
    }
  }, 250)
}

function newBatchId(): string {
  return `undo-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
}

/**
 * 记入一批写盘备份。超出上限时把最老批次的 `.bak` 一起删掉：
 * 清单被裁剪后用户再也点不到那些备份，留在磁盘上就是纯垃圾。
 */
export async function pushUndoBatch(label: string, entries: UndoEntry[]): Promise<void> {
  await initUndoHistory()
  const valid = entries
    .filter((e) => !!e?.path && !!e?.backup_path)
    .slice(0, MAX_ENTRIES_PER_BATCH)
  if (!valid.length) return
  const next: UndoBatch[] = [
    ...undoBatches.value,
    { id: newBatchId(), label, createdAt: Date.now(), entries: valid }
  ]
  while (next.length > MAX_BATCHES) {
    const dropped = next.shift()
    if (dropped) void discardUndoEntries(dropped.entries)
  }
  undoBatches.value = next
}

/**
 * 替换/撤销写盘后刷新已打开的文档（M2 遗留 1）。
 * - 干净文档：直接 reloadDocument，并刷新 mtime/编码/EOL 基线
 * - 脏文档：**绝不覆盖用户未保存的内容**，只置 externalModified，保存时走 409 三选一
 */
export async function refreshOpenDocuments(paths: string[]): Promise<void> {
  const store = useDocumentManagerStore()
  let reloaded = 0
  let conflicted = 0
  for (const path of new Set(paths)) {
    const doc = store.documents.get(path)
    if (!doc) continue
    if (doc.isDirty) {
      doc.externalModified = true
      conflicted++
      continue
    }
    await store.reloadDocument(path)
    reloaded++
  }
  if (conflicted > 0) {
    NotificationUtil.warn(`${conflicted} 个已打开文档有未保存更改，未自动刷新；保存时会弹出冲突处理`)
  } else if (reloaded > 0) {
    NotificationUtil.info(`已刷新 ${reloaded} 个打开的文档`)
  }
}

/** 恢复指定批次（默认最新一批）：按原始字节覆盖，成功后删备份并移除清单条目。 */
export async function restoreUndoBatch(id?: string): Promise<boolean> {
  await initUndoHistory()
  const target = id
    ? undoBatches.value.find((b) => b.id === id)
    : undoBatches.value[undoBatches.value.length - 1]
  if (!target) return false
  try {
    const res = await undoReplace(target.entries)
    if (res?.code !== 200) {
      NotificationUtil.error(res?.msg || '撤销失败')
      return false
    }
    const failed = res.data.failed || []
    // 部分失败时只移除成功恢复的条目，其余保留。后端失败文案是「{path}: {原因}」，
    // Windows 路径形如 C:\...，绝不能用 split(':')[0] 取路径（会得到 "C"）。
    const remaining = target.entries.filter((e) =>
      failed.some((f) => f.startsWith(`${e.path}:`))
    )
    undoBatches.value = undoBatches.value
      .map((b) => (b.id === target.id ? { ...b, entries: remaining } : b))
      .filter((b) => b.entries.length > 0)
    void refreshOpenDocuments(target.entries.map((e) => e.path))
    if (failed.length) {
      NotificationUtil.warn(`已恢复 ${res.data.restored} 个文件，${failed.length} 个失败：${failed[0]}`)
    } else {
      NotificationUtil.success(`已恢复 ${res.data.restored} 个文件`)
    }
    return true
  } catch (e: any) {
    NotificationUtil.error(e?.message || '撤销异常')
    return false
  }
}

/** 主动丢弃一批（不再撤销）：删 `.bak` 并移出清单。 */
export async function discardUndoBatch(id: string): Promise<void> {
  const target = undoBatches.value.find((b) => b.id === id)
  if (!target) return
  try {
    const res = await discardUndoEntries(target.entries)
    if (res?.data?.length) {
      NotificationUtil.warn(`${res.data.length} 个备份删除失败：${res.data[0]}`)
    }
  } catch (e) {
    console.warn('[files/undo] 删除备份失败', e)
  }
  undoBatches.value = undoBatches.value.filter((b) => b.id !== id)
}

export async function clearUndoHistory(): Promise<void> {
  const all = undoBatches.value.flatMap((b) => b.entries)
  if (all.length) {
    try {
      await discardUndoEntries(all)
    } catch (e) {
      console.warn('[files/undo] 清空备份失败', e)
    }
  }
  undoBatches.value = []
}

export function formatBatchTime(ts: number): string {
  try {
    return new Date(ts).toLocaleString()
  } catch {
    return String(ts)
  }
}
