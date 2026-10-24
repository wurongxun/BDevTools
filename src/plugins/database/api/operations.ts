/**
 * 数据库操作 API
 *
 * 写库只有一条路：`dbApplyEdits`（批量、单事务）—— 若干条 UPDATE/INSERT/DELETE
 *   要么全成功要么全回滚，且逐条返回受影响行数（0 行 = 冲突，不是「成功但没改」）。
 *   `dbPreviewEdits` 与它共用同一份 SQL 生成逻辑，所以**预览看到的语句就是即将执行的语句**。
 */
import { invoke } from '@/api/rpc'
import type {
  SqlResult,
  TableDataResult,
  ApiResponse,
  EditOp,
  EditBatchResult,
  EditPreview,
} from '../types'

/** 执行 SQL（分页：默认 200 行/页；pageSize 不设上限，offset 用于翻页） */
export function dbExecuteSql(
  configId: string,
  sql: string,
  database?: string,
  pageSize?: number,
  offset?: number
): Promise<ApiResponse<SqlResult>> {
  return invoke('db_execute_sql', { configId, sql, database, pageSize, offset })
}

/** 分页查询表数据 */
export function dbQueryTableData(
  configId: string,
  database: string,
  table: string,
  page: number,
  pageSize: number,
  whereClause?: string,
  orderBy?: string
): Promise<ApiResponse<TableDataResult>> {
  return invoke('db_query_table_data', { configId, database, table, page, pageSize, whereClause, orderBy })
}

/**
 * 批量写回（一个事务）。
 *
 * 前端**不需要**自己拼 SQL —— 语句由后端按方言生成（标识符引用、字面量转义都不止一种写法），
 * 这里只负责描述「哪些行、哪些列、改成什么」。
 */
export function dbApplyEdits(
  configId: string,
  database: string,
  ops: EditOp[]
): Promise<ApiResponse<EditBatchResult>> {
  return invoke('db_apply_edits', { configId, database, ops })
}

/** 预览批量写将要执行的 SQL（不落地；失败 reason 逐条给出） */
export function dbPreviewEdits(
  configId: string,
  database: string,
  ops: EditOp[]
): Promise<ApiResponse<EditPreview>> {
  return invoke('db_preview_edits', { configId, database, ops })
}
