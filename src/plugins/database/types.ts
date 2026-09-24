/**
 * 数据库插件类型定义
 */

/** 数据库类型 */
export type DbType = 'mysql' | 'postgresql' | 'sqlserver' | 'oracle'

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
  /** 查询结果总行数（仅纯 SELECT 首页统计；DML 或无法包装的语句为 null） */
  total?: number | null
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

/** 库级索引聚合（比表级索引多一个所属表） */
export interface DatabaseIndexInfo {
  name: string
  table_name: string
  column_name: string
  is_unique: boolean
  is_primary: boolean
  index_type?: string
}

/** 目录能力声明：后端按数据库类型给出，前端据此决定渲染哪些树节点 */
export interface CatalogCapabilities {
  views: boolean
  procedures: boolean
  triggers: boolean
  events: boolean
  database_indexes: boolean
  users: boolean
  admins: boolean
  session_status: boolean
  global_status: boolean
  session_variables: boolean
  global_variables: boolean
}

/** 键值行：状态 / 变量统一用这个结构（名-值两列） */
export interface KeyValueRow {
  name: string
  value: string
}

/** 用户（账号）信息 */
export interface DbUserInfo {
  user: string
  /** MySQL 是 host；其余无主机概念的库为空串 */
  host: string
  plugin?: string
  account_locked?: boolean
  password_expired?: boolean
  /** 原始权限标记，供人工核对 */
  privileges?: string
}

/** 管理员信息（判定用各库原生语义，reason 说明命中了哪条依据） */
export interface DbAdminInfo {
  user: string
  host: string
  plugin?: string
  reason: string
  privileges?: string
}

/** 数据库对象（视图 / 存储过程 / 触发器 / 事件） */
export interface DatabaseObjectInfo {
  name: string
  object_type?: string
  database?: string
  definer?: string
  created?: string
  modified?: string
  comment?: string
  status?: string
  /** 触发器：触发事件 */
  event?: string
  /** 触发器：所属表 */
  table_name?: string
}

/**
 * 对象列表类型：决定 ObjectListViewer 调哪个接口、渲染哪些列
 * - user / admin 是连接级（不需要 database）
 * - view / index / procedure / trigger / event 是库级（需要 database）
 */
export type ObjectListType = 'user' | 'admin' | 'view' | 'index' | 'procedure' | 'trigger' | 'event'

/**
 * 系统信息子类型：决定 SystemInfoViewer 调哪个接口
 */
export type SystemInfoSubType =
  | 'session-status'
  | 'global-status'
  | 'session-variables'
  | 'global-variables'

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
  /**
   * - sql：SQL 查询编辑器
   * - table：表数据网格
   * - metadata：表结构/元数据
   * - object-list：通用对象列表（用户/管理员/视图/索引/存储过程/触发器/事件）
   * - system-info：系统信息（会话/全局 状态与变量）
   */
  type: 'sql' | 'table' | 'metadata' | 'object-list' | 'system-info'
  configId?: string
  database?: string
  table?: string
  content?: string
  dirty?: boolean
  filePath?: string
  /** object-list：具体对象类型 */
  objectType?: ObjectListType
  /** system-info：具体子类型 */
  subType?: SystemInfoSubType
}

/** 树节点 */
export interface TreeNode {
  id: string
  label: string
  /**
   * - connection：连接根节点
   * - connection-group：连接级分组（数据库 / 用户 / 管理员 / 系统信息）
   * - database：库节点
   * - database-group：库级分组（表 / 视图 / 索引 / 存储过程 / 触发器 / 事件）
   * - table-group：表分组（兼容旧结构）
   * - 其余为具体对象叶子节点
   */
  type:
    | 'connection'
    | 'connection-group'
    | 'database'
    | 'database-group'
    | 'table-group'
    | 'table'
    | 'column'
    | 'user'
    | 'admin'
    | 'view'
    | 'index'
    | 'procedure'
    | 'trigger'
    | 'event'
    | 'status-group'
    | 'variable-group'
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

/** 查询文件树节点（scripts 目录；isSourceRoot 为源根节点——默认源或外部挂载源） */
export interface ScriptFileNode {
  name: string
  path: string
  isDir: boolean
  children: ScriptFileNode[]
  /** 是否为源根节点（默认源或外部挂载源，顶层节点） */
  isSourceRoot?: boolean
  /** 外部源标识：default 表示默认脚本目录，external 表示用户挂载的外部目录 */
  sourceKind?: 'default' | 'external'
}

/** 统一响应 */
export interface ApiResponse<T> {
  code: number
  data: T
  msg: string
}
