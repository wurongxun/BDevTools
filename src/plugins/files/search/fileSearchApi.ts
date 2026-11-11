/**
 * files 跨文件搜索 / 批量替换的后端契约。
 *
 * 全部走 `@/api/rpc` 的 `call`（唯一 RPC 入口），不直接 import tauri。
 * 命令名与 `src-tauri/src/commands/files.rs` 一一对应；payload 键用 Rust 形参的 camelCase。
 * 返回字段保持 Rust 侧的 snake_case（与 logs 插件的既有契约一致）。
 */
import { call, type RpcResponse } from '@/api/rpc'

export type SearchMode = 'text' | 'regex'

export interface SearchHit {
  line: number
  text: string
  start: number
  end: number
}

export interface FileHits {
  path: string
  rel_path: string
  hits: SearchHit[]
  encoding: string
  eol: string
  bom: string
  mtime_ms: number
  size: number
}

export interface SearchOutcome {
  files: FileHits[]
  total_matches: number
  files_scanned: number
  files_skipped: number
  truncated: boolean
  cancelled: boolean
}

/** 替换目标直接复用搜索结果里的 mtime/size，后端据此做 409 冲突检测。 */
export interface ReplaceTarget {
  path: string
  mtime_ms: number
  size: number
}

export interface UndoEntry {
  path: string
  backup_path: string
}

export interface ReplaceSample {
  line: number
  before: string
  after: string
}

export interface ReplaceItem {
  path: string
  rel_path: string
  code: number
  msg: string
  replacements: number
  samples: ReplaceSample[]
  backup_path: string
  encoding: string
  eol: string
}

export interface ReplaceOutcome {
  dry_run: boolean
  items: ReplaceItem[]
  total_replacements: number
  files_changed: number
  cancelled: boolean
  backups: UndoEntry[]
}

export interface UndoOutcome {
  restored: number
  failed: string[]
}

export interface CommonParams {
  keyword: string
  replacement: string
  mode: SearchMode
  ignoreCase: boolean
  taskId: string
}

export interface SearchParams extends CommonParams {
  rootPath: string
  include: string
  exclude: string
}

export function searchInFiles(p: SearchParams): Promise<RpcResponse<SearchOutcome>> {
  return call('files_search_in_files', {
    rootPath: p.rootPath,
    keyword: p.keyword,
    mode: p.mode,
    ignoreCase: p.ignoreCase,
    include: p.include,
    exclude: p.exclude,
    taskId: p.taskId
  })
}

export function replaceInFiles(
  p: CommonParams & { targets: ReplaceTarget[]; dryRun: boolean }
): Promise<RpcResponse<ReplaceOutcome>> {
  return call('files_replace_in_files', {
    keyword: p.keyword,
    replacement: p.replacement,
    mode: p.mode,
    ignoreCase: p.ignoreCase,
    targets: p.targets,
    dryRun: p.dryRun,
    taskId: p.taskId
  })
}

export function undoReplace(entries: UndoEntry[]): Promise<RpcResponse<UndoOutcome>> {
  return call('files_replace_undo', { entries })
}

export function cancelSearch(taskId: string): Promise<RpcResponse<boolean>> {
  return call('files_search_cancel', { taskId })
}

export function toReplaceTargets(files: FileHits[]): ReplaceTarget[] {
  return files.map((f) => ({ path: f.path, mtime_ms: f.mtime_ms, size: f.size }))
}
