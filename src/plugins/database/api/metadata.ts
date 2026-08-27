/**
 * 元数据查询 API
 */
import { invoke } from '@/api/rpc'
import type { DatabaseInfo, TableInfo, ColumnInfo, IndexInfo, ApiResponse } from '../types'

/** 获取数据库列表 */
export function dbListDatabases(configId: string): Promise<ApiResponse<DatabaseInfo[]>> {
  return invoke('db_list_databases', { configId })
}

/** 获取表列表 */
export function dbListTables(configId: string, database: string): Promise<ApiResponse<TableInfo[]>> {
  return invoke('db_list_tables', { configId, database })
}

/** 获取表字段信息 */
export function dbTableColumns(configId: string, database: string, table: string): Promise<ApiResponse<ColumnInfo[]>> {
  return invoke('db_table_columns', { configId, database, table })
}

/** 获取表索引信息 */
export function dbTableIndexes(configId: string, database: string, table: string): Promise<ApiResponse<IndexInfo[]>> {
  return invoke('db_table_indexes', { configId, database, table })
}

/** 获取建表 DDL */
export function dbTableSchema(configId: string, database: string, table: string): Promise<ApiResponse<string>> {
  return invoke('db_table_schema', { configId, database, table })
}
