/**
 * 导入导出 API
 */
import { invoke } from '@/api/rpc'
import type { ExportResult, ImportResult, ApiResponse } from '../types'

/** 导出为 CSV */
export function dbExportCsv(configId: string, database: string, table: string, filePath: string): Promise<ApiResponse<ExportResult>> {
  return invoke('db_export_csv', { configId, database, table, filePath })
}

/** 导出为 Excel */
export function dbExportExcel(configId: string, database: string, table: string, filePath: string): Promise<ApiResponse<ExportResult>> {
  return invoke('db_export_excel', { configId, database, table, filePath })
}

/** 按任意 SQL 语句导出结果为 CSV */
export function dbExportSqlCsv(configId: string, sql: string, filePath: string): Promise<ApiResponse<ExportResult>> {
  return invoke('db_export_sql_csv', { configId, sql, filePath })
}

/** 按任意 SQL 语句导出结果为 Excel */
export function dbExportSqlExcel(configId: string, sql: string, filePath: string): Promise<ApiResponse<ExportResult>> {
  return invoke('db_export_sql_excel', { configId, sql, filePath })
}

/** 从 CSV 导入 */
export function dbImportCsv(configId: string, database: string, table: string, filePath: string, hasHeader: boolean): Promise<ApiResponse<ImportResult>> {
  return invoke('db_import_csv', { configId, database, table, filePath, hasHeader })
}

/** 从 Excel 导入 */
export function dbImportExcel(configId: string, database: string, table: string, filePath: string): Promise<ApiResponse<ImportResult>> {
  return invoke('db_import_excel', { configId, database, table, filePath })
}
