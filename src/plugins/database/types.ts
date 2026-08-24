/**
 * 数据库插件类型定义
 */

/** 数据库类型 */
export type DbType = 'mysql' | 'postgresql'

/** 数据库连接配置 */
export interface DbConnectionConfig {
  id?: string
  name: string
  db_type: DbType
  host: string
  port: number
  username: string
  password: string
  database_name?: string
  ssh_enabled: boolean
  ssh_host?: string
  ssh_port?: number
  ssh_username?: string
  ssh_password?: string
  ssh_key_path?: string
  created_at?: string
  updated_at?: string
}

/** SQL 执行结果 */
export interface SqlResult {
  columns: string[]
  rows: any[][]
  affected_rows: number
  execution_time_ms: number
  is_query: boolean
  /** 是否还有下一页数据（分页查询） */
  has_more?: boolean
}

/** 表数据查询结果 */
export interface TableDataResult {
  columns: ColumnInfo[]
  rows: any[][]
  total: number
  page: number
  page_size: number
}

/** 字段信息 */
export interface ColumnInfo {
  name: string
  data_type: string
  is_nullable: boolean
  is_primary_key: boolean
  default_value?: string
  comment?: string
}

/** 数据库信息 */
export interface DatabaseInfo {
  name: string
  charset?: string
  collation?: string
}

/** 表信息 */
export interface TableInfo {
  name: string
  engine?: string
  row_count?: number
  comment?: string
  data_length?: number
}

/** 索引信息 */
export interface IndexInfo {
  name: string
  column_name: string
  is_unique: boolean
  is_primary: boolean
  index_type?: string
}

/** 导出结果 */
export interface ExportResult {
  file_path: string
  row_count: number
}

/** 导入结果 */
export interface ImportResult {
  success_count: number
  error_count: number
  errors: string[]
}

/** 编辑器 Tab 页 */
export interface EditorTab {
  id: string
  title: string
  type: 'sql' | 'table' | 'metadata'
  configId?: string
  database?: string
  table?: string
  content?: string
  dirty?: boolean
  filePath?: string
}

/** 树节点 */
export interface TreeNode {
  id: string
  label: string
  type: 'connection' | 'database' | 'table-group' | 'table' | 'column'
  children?: TreeNode[]
  isLeaf?: boolean
  data?: any
}

/** 查询文件的默认连接/数据库绑定（存于 scripts/index.json） */
export interface ScriptFileBinding {
  configId?: string
  database?: string
  updatedAt?: number
}

/** 查询文件树节点（scripts 目录） */
export interface ScriptFileNode {
  name: string
  path: string
  isDir: boolean
  children: ScriptFileNode[]
}

/** 统一响应 */
export interface ApiResponse<T> {
  code: number
  data: T
  msg: string
}
