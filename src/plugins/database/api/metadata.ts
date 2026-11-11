/**
 * 元数据查询 API
 *
 * 命令名与后端 `catalog` 抽象一一对应：`db_catalog_capabilities` 先拉一次能力声明，
 * 前端只渲染该数据库真正支持的节点；其余接口在不支持时返回 code=300 + 明确提示。
 */
import { invoke } from '@/api/rpc'
import type {
  DatabaseInfo,
  TableInfo,
  ColumnInfo,
  IndexInfo,
  ApiResponse,
  CatalogCapabilities,
  DatabaseIndexInfo,
  DatabaseObjectInfo,
  DbAdminInfo,
  DbUserInfo,
  KeyValueRow,
  ObjectSourceKind,
} from '../types'

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

// ==================== 目录能力声明 ====================

/**
 * 获取当前连接支持哪些目录能力。
 *
 * 建立连接后调用一次并缓存；未连接 / 查询失败时返回全 false，调用方应据此隐藏可选分组。
 */
export async function dbCatalogCapabilities(configId: string): Promise<CatalogCapabilities> {
  const res = await invoke<ApiResponse<CatalogCapabilities>>('db_catalog_capabilities', { configId })
  if (res?.code === 200 && res.data) return res.data
  return {
    views: false,
    procedures: false,
    functions: false,
    triggers: false,
    events: false,
    database_indexes: false,
    users: false,
    admins: false,
    session_status: false,
    global_status: false,
    session_variables: false,
    global_variables: false,
    object_source: false,
  }
}

// ==================== 库级对象 ====================

/** 获取视图列表 */
export function dbListViews(configId: string, database: string): Promise<ApiResponse<DatabaseObjectInfo[]>> {
  return invoke('db_list_views', { configId, database })
}

/** 获取存储过程列表 */
export function dbListProcedures(configId: string, database: string): Promise<ApiResponse<DatabaseObjectInfo[]>> {
  return invoke('db_list_procedures', { configId, database })
}

/** 获取函数列表 */
export function dbListFunctions(configId: string, database: string): Promise<ApiResponse<DatabaseObjectInfo[]>> {
  return invoke('db_list_functions', { configId, database })
}

/**
 * 取对象源码（视图定义 / 存储过程体 / 函数体 / 触发器体）。
 *
 * 与 `dbTableSchema` 不是一回事：后者返回「可读的 DDL」（部分库是按列拼出来的），
 * 这里返回数据库自己承认的源文本。
 */
export function dbObjectSource(
  configId: string,
  database: string,
  kind: ObjectSourceKind,
  name: string
): Promise<ApiResponse<string>> {
  return invoke('db_object_source', { configId, database, kind, name })
}

/** 获取触发器列表 */
export function dbListTriggers(configId: string, database: string): Promise<ApiResponse<DatabaseObjectInfo[]>> {
  return invoke('db_list_triggers', { configId, database })
}

/** 获取事件列表（仅 MySQL 支持） */
export function dbListEvents(configId: string, database: string): Promise<ApiResponse<DatabaseObjectInfo[]>> {
  return invoke('db_list_events', { configId, database })
}

/** 获取库级索引聚合 */
export function dbListDatabaseIndexes(configId: string, database: string): Promise<ApiResponse<DatabaseIndexInfo[]>> {
  return invoke('db_list_database_indexes', { configId, database })
}

// ==================== 实例级 ====================

/** 获取用户列表 */
export function dbListUsers(configId: string): Promise<ApiResponse<DbUserInfo[]>> {
  return invoke('db_list_users', { configId })
}

/** 获取管理员列表 */
export function dbListAdmins(configId: string): Promise<ApiResponse<DbAdminInfo[]>> {
  return invoke('db_list_admins', { configId })
}

/** 获取运行状态（scope: session | global） */
export function dbShowStatus(configId: string, scope: 'session' | 'global'): Promise<ApiResponse<KeyValueRow[]>> {
  return invoke('db_show_status', { configId, scope })
}

/** 获取系统变量（scope: session | global） */
export function dbShowVariables(configId: string, scope: 'session' | 'global'): Promise<ApiResponse<KeyValueRow[]>> {
  return invoke('db_show_variables', { configId, scope })
}
