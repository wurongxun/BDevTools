// 全局类型定义

export type RedisType = 'string' | 'hash' | 'list' | 'set' | 'zset' | 'stream' | 'none'

export type ReplyType = 'status' | 'bulk' | 'int' | 'array' | 'error'

export interface Reply {
  t: ReplyType
  v: any
}

/** 连接配置（UI 表单 ↔ 存储）。ssh/tls/unixSocket 为预留字段，真实传输接入后启用。 */
export interface ConnectionConfig {
  id: string
  name: string
  host: string
  port: number
  password?: string
  db: number
  mode: TransportMode
  seed?: boolean
  ssh?: {
    host: string
    port: number
    user: string
    authType: 'password' | 'key'
    password?: string
    privateKey?: string
  }
  tls?: boolean
  unixSocket?: string
}

export type TransportMode = 'mock' | 'tcp' | 'proxy'

export interface KeyMeta {
  name: string
  type: RedisType
  ttl: number // 秒；-1 永不过期；-2 不存在
  size?: number // 字节（近似）
}

export interface ScanResult {
  cursor: string
  keys: KeyMeta[]
}

export interface HashField {
  field: string
  value: string
}

export interface ZSetMember {
  member: string
  score: number
}

export interface StreamEntry {
  id: string
  fields: Record<string, string>
}

export interface SlowlogEntry {
  id: number
  timestamp: number
  durationUs: number
  command: string
}

export interface ClientInfo {
  id: number
  addr: string
  name: string
  age: number
  idle: number
  db: number
  flags: string
}

export interface InfoSection {
  name: string
  fields: { k: string; v: string }[]
}

export interface MemoryUsage {
  usedBytes: number
  maxBytes: number
  ratio: number
}

export type ValueFormat = 'text' | 'json' | 'xml' | 'yaml' | 'hex'

export const TYPE_COLORS: Record<RedisType, string> = {
  string: '#2f6feb',
  hash: '#1a7f37',
  list: '#b8860b',
  set: '#8957e5',
  zset: '#c0392b',
  stream: '#0d9488',
  none: '#9aa0a6'
}

// ============================================================ P2：内存分析
export interface BigKey {
  name: string
  type: RedisType
  size: number // 字节
  ttl: number // 秒
}

export interface PrefixStat {
  prefix: string
  count: number
  size: number // 字节
  pct: number // 0-100
}

export interface TypeStat {
  type: RedisType
  count: number
  size: number // 字节
  pct: number // 0-100
}

export interface MemoryAnalysis {
  totalBytes: number
  totalKeys: number
  bigKeys: BigKey[]
  prefixes: PrefixStat[]
  types: TypeStat[]
}

// ============================================================ P2：慢查询分析
export interface SlowQueryStat {
  id: number
  timestamp: number
  durationUs: number
  command: string
  pattern: string // 归一化模式，如 HGETALL user:*
}

export interface SlowCommandGroup {
  pattern: string
  count: number
  totalUs: number
  maxUs: number
}

export interface SlowQueryAnalysis {
  items: SlowQueryStat[]
  count: number
  maxUs: number
  minUs: number
  avgUs: number
  p95Us: number
  byCommand: SlowCommandGroup[]
}

// ============================================================ P2：集群 / 复制 / Sentinel
export interface ClusterNode {
  id: string
  role: 'master' | 'slave' | 'myself'
  addr: string
  flags: string
  masterId: string | null
  slots: string // 槽位范围描述，如 "0-5460"
  linkState: string
}

export interface ClusterTopology {
  mode: 'cluster' | 'standalone' | 'sentinel'
  nodeCount: number
  slotsCovered: number
  myselfId: string
  nodes: ClusterNode[]
}

export interface SlaveInfo {
  id: string
  addr: string
  offset: number
  lag: number
  state: string
}

export interface ReplicationInfo {
  role: 'master' | 'slave'
  connectedSlaves: number
  masterReplOffset: number
  slaves: SlaveInfo[]
  masterAddr?: string
}

export interface SentinelMaster {
  name: string
  ip: string
  port: number
  flags: string
  numSlaves: number
  numOtherSentinels: number
  quorum: number
}

// ============================================================ P2：操作审计
export type AuditOp =
  | 'CMD'
  | 'DEL'
  | 'BATCH_DELETE'
  | 'BATCH_EXPIRE'
  | 'BATCH_PERSIST'
  | 'RENAME'
  | 'COPY'
  | 'EXPIRE'
  | 'PERSIST'
  | 'IMPORT'
  | 'EXPORT'
  | 'FLUSHDB'
  | 'FLUSHALL'
  | 'CONN_ADD'
  | 'CONN_EDIT'
  | 'CONN_DEL'

export interface AuditEntry {
  ts: number
  op: AuditOp
  detail: string
  target: string // key(s) 或 '-'
  status: 'ok' | 'err'
}
