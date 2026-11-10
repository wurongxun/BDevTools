/**
 * 编辑器内部的一次性事件通道。
 *
 * 用 window CustomEvent 而不是 runtime.publish：后者是 Tauri emit（跨窗口广播 + IPC），
 * 适合低频「语言切换/布局更新」，不适合「点击搜索结果跳一行」这种高频本地联动。
 * TextView.vue 已有 window CustomEvent 范式（window-resize / encoding-change），沿用一致。
 */
import { LARGE_REFRESH_EVENT } from '@/store/modules/documents'

const REVEAL_LINE = 'files:reveal-line'

export interface RevealLineDetail {
  path: string
  line: number
  column?: number
}

/** 请求把某个已打开文档滚到第 N 行（CodeEditor 与 TextView 各自监听）。 */
export function revealLine(path: string, line: number, column?: number): void {
  if (!path || !Number.isFinite(line) || line < 1) return
  window.dispatchEvent(
    new CustomEvent<RevealLineDetail>(REVEAL_LINE, {
      detail: { path, line: Math.floor(line), column }
    })
  )
}

export function onRevealLine(handler: (detail: RevealLineDetail) => void): () => void {
  const listener = (e: Event) => {
    const detail = (e as CustomEvent<RevealLineDetail>).detail
    if (detail && typeof detail.path === 'string') handler(detail)
  }
  window.addEventListener(REVEAL_LINE, listener)
  return () => window.removeEventListener(REVEAL_LINE, listener)
}

// ===== 行操作（A6）：命令面板在组件外，只能通过事件把意图交给持有 editor 的 CodeEditor =====
const LINE_OP = 'files:line-op'

export type LineOp =
  | 'copyDown'
  | 'copyUp'
  | 'moveUp'
  | 'moveDown'
  | 'delete'
  | 'sortAsc'
  | 'sortDesc'
  | 'dedup'
  | 'removeEmpty'
  | 'upper'
  | 'lower'

export interface LineOpDetail {
  path: string
  op: LineOp
}

export function requestLineOp(path: string, op: LineOp): void {
  if (!path) return
  window.dispatchEvent(new CustomEvent<LineOpDetail>(LINE_OP, { detail: { path, op } }))
}

export function onLineOp(handler: (detail: LineOpDetail) => void): () => void {
  const listener = (e: Event) => {
    const detail = (e as CustomEvent<LineOpDetail>).detail
    if (detail && typeof detail.path === 'string') handler(detail)
  }
  window.addEventListener(LINE_OP, listener)
  return () => window.removeEventListener(LINE_OP, listener)
}

// ===== 大文件行编辑（M4 / E7）：保存 / 外部刷新后由 documents.ts 派发，TextView 失效块缓存 =====
export interface LargeRefreshDetail {
  path: string
}

/** 测试与其它视图也可派发；事件名只认 store 导出的那一个常量，杜绝两边字符串漂移。 */
export function emitLargeRefresh(path: string): void {
  if (!path) return
  window.dispatchEvent(
    new CustomEvent<LargeRefreshDetail>(LARGE_REFRESH_EVENT, { detail: { path } })
  )
}

export function onLargeRefresh(handler: (detail: LargeRefreshDetail) => void): () => void {
  const listener = (e: Event) => {
    const detail = (e as CustomEvent<LargeRefreshDetail>).detail
    if (detail && typeof detail.path === 'string') handler(detail)
  }
  window.addEventListener(LARGE_REFRESH_EVENT, listener)
  return () => window.removeEventListener(LARGE_REFRESH_EVENT, listener)
}
