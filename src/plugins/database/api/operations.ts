/**
 * 数据库操作 API
 */
import { invoke } from '@/api/rpc'
import type { SqlResult, TableDataResult, ApiResponse } from '../types'

/** 执行 SQL（分页：默认 200 行/页，后端上限 1000；offset 用于翻页） */
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

/** 插入行 */
export function dbInsertRow(
  configId: string,
  database: string,
  table: string,
  data: Record<string, any>
): Promise<ApiResponse<boolean>> {
  return invoke('db_insert_row', { configId, database, table, data })
}

/** 更新行 */
export function dbUpdateRow(
  configId: string,
  database: string,
  table: string,
  primaryKey: string,
  primaryValue: any,
  data: Record<string, any>
): Promise<ApiResponse<boolean>> {
  return invoke('db_update_row', { configId, database, table, primaryKey, primaryValue, data })
}

/** 删除行 */
export function dbDeleteRow(
  configId: string,
  database: string,
  table: string,
  primaryKey: string,
  primaryValue: any
): Promise<ApiResponse<boolean>> {
  return invoke('db_delete_row', { configId, database, table, primaryKey, primaryValue })
}
