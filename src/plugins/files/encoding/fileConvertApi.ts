/**
 * 编码 / EOL / BOM 显式转换与撤销清单探测的后端契约（M4 / E4+E5 遗留）。
 *
 * 走 `@/api/rpc` 的 `call`，payload 键 = Rust 形参的 camelCase。
 * 命令定义见 `src-tauri/src/commands/files.rs`。
 */
import { call, type RpcResponse } from '@/api/rpc'
import type { UndoEntry } from '../search/fileSearchApi'

export interface ConvertOutcome {
  path: string
  /** 空串＝目标与现状一致、未写盘，不要入撤销清单 */
  backup_path: string
  encoding: string
  eol: string
  bom: string
  mtime_ms: string
  size: number
  from_encoding: string
  from_eol: string
  from_bom: string
}

export interface ConvertParams {
  path: string
  targetEncoding: string
  targetEol: string
  targetBom: string
  expectedMtimeMs?: number
  expectedSize?: number
}

export function convertEncodingEol(
  p: ConvertParams
): Promise<RpcResponse<ConvertOutcome>> {
  return call('files_convert_encoding_eol', {
    path: p.path,
    targetEncoding: p.targetEncoding,
    targetEol: p.targetEol,
    targetBom: p.targetBom,
    expectedMtimeMs: p.expectedMtimeMs,
    expectedSize: p.expectedSize
  })
}

/** 重启恢复：只保留备份文件仍存在的条目。 */
export function probeUndoEntries(
  entries: UndoEntry[]
): Promise<RpcResponse<UndoEntry[]>> {
  return call('files_undo_probe', { entries })
}

/** 主动丢弃撤销清单：删掉对应 .bak，返回删除失败的路径说明。 */
export function discardUndoEntries(
  entries: UndoEntry[]
): Promise<RpcResponse<string[]>> {
  return call('files_undo_discard', { entries })
}
