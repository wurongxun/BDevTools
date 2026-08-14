/**
 * 数据库连接管理 API
 */
import { invoke } from '@tauri-apps/api/core'
import type { DbConnectionConfig, ApiResponse } from '../types'

/** 创建连接配置 */
export function dbConnectionCreate(config: DbConnectionConfig): Promise<ApiResponse<string>> {
  return invoke('db_connection_create', { config })
}

/** 删除连接配置 */
export function dbConnectionDelete(id: string): Promise<ApiResponse<boolean>> {
  return invoke('db_connection_delete', { id })
}

/** 更新连接配置 */
export function dbConnectionUpdate(config: DbConnectionConfig): Promise<ApiResponse<boolean>> {
  return invoke('db_connection_update', { config })
}

/** 获取连接列表 */
export function dbConnectionList(): Promise<ApiResponse<DbConnectionConfig[]>> {
  return invoke('db_connection_list')
}

/** 测试连接 */
export function dbConnectionTest(config: DbConnectionConfig): Promise<ApiResponse<boolean>> {
  return invoke('db_connection_test', { config })
}

/** 建立连接 */
export function dbConnect(configId: string): Promise<ApiResponse<boolean>> {
  return invoke('db_connect', { configId })
}

/** 断开连接 */
export function dbDisconnect(configId: string): Promise<ApiResponse<boolean>> {
  return invoke('db_disconnect', { configId })
}
