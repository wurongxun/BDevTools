/**
 * 数据库插件类型定义
 */
import type { RpcResponse } from '@/api/rpc'

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

/**
 * 一次执行的结果落到哪个子结果标签。
 * - `reuse`：同语句已经有子结果就**复用**它（不新增标签），没有才追加 —— 「执行 SQL 语句」。
 * - `new-tab`：**总是**追加新标签 —— 「在新标签页中执行」。
 */
export type SqlExecuteMode = 'reuse' | 'new-tab'

/** 表数据查询结果 */
export interface TableDataResult {
  columns: ColumnInfo[]
  rows: any[][]
  total: number
  page: number
  page_size: number
}

/**
 * 字段信息（结果网格编辑依赖后半段的元数据）
 * - `ordinal_position`：列在表中的序号，用于把「结果列」与「表列」按位置对齐
 * - `is_auto_increment` / `is_generated`：自增与计算列 —— 写入通常没有意义 / 直接报错
 * - `numeric_precision/scale`、`character_max_length`：前端粗校验与选编辑器类型的依据
 *
 * 各库查不到时按「未知」处理（0 / None），**宁可少一次校验，也不要把可写的列判成只读**。
 */
export interface ColumnInfo {
  name: string
  data_type: string
  is_nullable: boolean
  is_primary_key: boolean
  default_value?: string
  comment?: string
  ordinal_position?: number
  is_auto_increment?: boolean
  is_generated?: boolean
  numeric_precision?: number | null
  numeric_scale?: number | null
  character_max_length?: number | null
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
  functions: boolean
  triggers: boolean
  events: boolean
  database_indexes: boolean
  users: boolean
  admins: boolean
  session_status: boolean
  global_status: boolean
  session_variables: boolean
  global_variables: boolean
  /** 能否取到对象源码（视图定义 / 存储过程体 …）。决定要不要渲染「源码」页签 */
  object_source: boolean
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
export type ObjectListType =
  | 'user'
  | 'admin'
  | 'view'
  | 'index'
  | 'procedure'
  | 'function'
  | 'trigger'
  | 'event'

/**
 * 对象源码类型（与后端 `ObjectSourceKind` 一一对应）。
 * 列表里「有这个对象」≠「能取到它的源码」，所以二者分开。
 */
export type ObjectSourceKind = 'view' | 'procedure' | 'function' | 'trigger'

/** 例程查看器（routine 标签）支持的三种对象 */
export type RoutineKind = Extract<ObjectSourceKind, 'procedure' | 'function' | 'trigger'>

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

// ==================== 结果网格编辑（批量写 / 事务） ====================

/** 列名 + 值。`column` 用**表里的原始列名**（后端负责按方言加引号） */
export interface EditKeyValue {
  column: string
  /** null 表示 SQL NULL；其余用原始标量（字符串 / 数字 / 布尔） */
  value: unknown
}

/**
 * 一次写操作。
 * - `update`：`keys` 定位一行（可多列复合键），`data` 是要改的字段
 * - `insert` / `delete`：新增与删除行
 *
 * 刻意用数组而不是对象承载 data：对象会在 JSON 序列化时丢掉字段顺序，
 * 同一份改动预览与执行的 SQL 就可能对不上。
 */
export type EditOp =
  | {
      op: 'update'
      table: string
      keys: EditKeyValue[]
      data: EditKeyValue[]
      /**
       * 要求「恰好命中一行」，否则整批回滚。
       * 用于**没有主键也没有唯一索引**的表：那种行的身份只能靠一堆列的原值拼，
       * 命中 0 行 = 数据已被改过、命中多行 = 这条 WHERE 根本定位不了一行。
       */
      expect_one?: boolean
    }
  | { op: 'insert'; table: string; data: EditKeyValue[] }
  | { op: 'delete'; table: string; keys: EditKeyValue[]; expect_one?: boolean }

/** 单个子操作的结果（受影响 0 行 ≠ 成功：通常是行已被别处删掉） */
export interface EditOutcome {
  index: number
  ok: boolean
  affected: number
  error?: string | null
}

export interface EditBatchResult {
  outcomes: EditOutcome[]
  /** true = 全在一个事务里（要么全成功要么全回滚） */
  transactional: boolean
  committed: number
}

export interface EditStatement {
  index: number
  sql: string
  error?: string | null
  /** 执行时是否要求「恰好命中一行」（预览里要标出来：这种 WHERE 是拼出来的） */
  expect_one?: boolean
}

export interface EditPreview {
  statements: EditStatement[]
}

/** 编辑器 Tab 页 */
export interface EditorTab {
  id: string
  title: string
  /**
   * - sql：SQL 查询编辑器
   * - table：表数据网格
   * - metadata：表结构/元数据
   * - view：视图查看器（数据 / 字段 / 源码）
   * - routine：例程查看器（存储过程 / 函数 / 触发器 的属性与源码）
   * - object-list：通用对象列表（用户/管理员/视图/索引/存储过程/函数/触发器/事件）
   * - system-info：系统信息（会话/全局 状态与变量）
   */
  type: 'sql' | 'table' | 'metadata' | 'view' | 'routine' | 'object-list' | 'system-info'
  configId?: string
  database?: string
  table?: string
  /**
   * view / routine 两类标签承载的**对象名**（视图名 / 例程名）。
   *
   * 不复用 `table`：只有表才有 table 的语义，用一个字段兼职两种对象，
   * 去重逻辑和 props 透传都会跟着变难懂。
   */
  objectName?: string
  content?: string
  dirty?: boolean
  filePath?: string
  /**
   * object-list：具体对象类型
   * routine：具体例程类型（procedure / function / trigger），决定取哪种源码
   */
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
    | 'function'
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

/**
 * 统一响应：与核心 `@/api/rpc` 的 `RpcResponse` 是同一份契约。
 *
 * 用别名而不是再定义一次：响应包裹是通信契约，核心服务也要用，各写一份就会出现
 * 「同一个东西两个名字」，改动时漏改一边。
 */
export type ApiResponse<T> = RpcResponse<T>
