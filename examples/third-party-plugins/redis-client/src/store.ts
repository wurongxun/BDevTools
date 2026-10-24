// 全局响应式状态 + 业务动作。所有组件共享同一份 store。

import { reactive, computed } from 'vue'
import { RedisClient } from './client'
import { transports } from './transport'
import { persistence } from './bridge'
import { maskSensitive } from './utils'
import type { AuditEntry, AuditOp, ConnectionConfig, KeyMeta, RedisType, ScanResult } from './types'

let client: RedisClient | null = null
let connSeq = 0

interface DialogState {
  type: string
  props?: any
}

export const store = reactive({
  connections: [] as ConnectionConfig[],
  activeConnId: null as string | null,
  connected: false,
  currentDb: 0,
  dbCounts: [] as number[],
  search: '',
  typeFilter: '',
  groupBy: true,
  keys: [] as KeyMeta[],
  scanCursor: '0',
  hasMoreKeys: false,
  loadingKeys: false,
  selectedKey: null as string | null,
  keyInfo: null as { type: RedisType; ttl: number; size: number } | null,
  keyLoadedAt: 0,
  keyLoadError: null as string | null,
  selectedKeys: [] as string[],
  terminalLines: [] as { kind: string; text: string }[],
  commandHistory: [] as string[],
  historyIndex: -1,
  monitorText: '' as string,
  monitorLoading: false,
  toast: null as { msg: string; type: string } | null,
  dialog: null as DialogState | null,
  nowTick: 0,
  mockMode: true,
  audit: [] as AuditEntry[]
})

// ---- 连接持久化 ----
function saveConnections() {
  persistence.set('rc.connections', JSON.stringify(store.connections))
}
function loadConnections() {
  const raw = persistence.get('rc.connections')
  if (raw) {
    try {
      store.connections = JSON.parse(raw)
    } catch {
      store.connections = []
    }
  }
  const active = persistence.get('rc.active')
  if (active && store.connections.some((c) => c.id === active)) store.activeConnId = active
  if (!store.connections.length) seedDefaultConnection()
}
function seedDefaultConnection() {
  const conn: ConnectionConfig = {
    id: 'local-mock',
    name: '本地 Mock',
    host: '127.0.0.1',
    port: 6379,
    db: 0,
    mode: 'mock',
    seed: true
  }
  store.connections = [conn]
  store.activeConnId = conn.id
  saveConnections()
}

export function ensureInit() {
  if (store.connections.length || persistence.get('rc.connections')) loadConnections()
  else loadConnections()
  loadAudit()
  // TTL / QPS 计时
  setInterval(() => {
    store.nowTick++
  }, 1000)
}

// ---- 操作审计 ----
function loadAudit() {
  const raw = persistence.get('rc.audit')
  if (raw) {
    try {
      store.audit = JSON.parse(raw)
    } catch {
      store.audit = []
    }
  }
}
export function recordAudit(op: AuditOp, detail: string, target = '-', status: 'ok' | 'err' = 'ok') {
  const entry: AuditEntry = { ts: Date.now(), op, detail, target, status }
  store.audit.unshift(entry)
  if (store.audit.length > 500) store.audit.length = 500
  persistence.set('rc.audit', JSON.stringify(store.audit))
}
export function clearAudit() {
  store.audit = []
  persistence.set('rc.audit', JSON.stringify(store.audit))
  toast('审计日志已清空', 'ok')
}
export function exportAudit(fmt: 'json' | 'csv' = 'json'): string {
  if (fmt === 'csv') {
    const head = 'ts,op,target,status,detail'
    const rows = store.audit.map((e) => {
      const t = new Date(e.ts).toISOString()
      const d = e.detail.replace(/"/g, '""').replace(/\n/g, ' ')
      return `"${t}",${e.op},"${e.target}",${e.status},"${d}"`
    })
    return head + '\n' + rows.join('\n')
  }
  return JSON.stringify(store.audit, null, 2)
}

// ---- 连接管理 ----
export async function connectActive() {
  if (!store.activeConnId) return
  await connectById(store.activeConnId)
}

export async function connectById(id: string) {
  const cfg = store.connections.find((c) => c.id === id)
  if (!cfg) return
  if (client) client.disconnect()
  client = new RedisClient(transports[cfg.mode]())
  store.mockMode = cfg.mode === 'mock'
  await client.connect(cfg)
  store.connected = client.isConnected()
  store.activeConnId = id
  store.currentDb = cfg.db || 0
  persistence.set('rc.active', id)
  await afterConnect()
}

async function afterConnect() {
  store.selectedKey = null
  store.keyInfo = null
  store.keys = []
  store.selectedKeys = []
  await Promise.all([loadDbCounts(), loadKeys(true)])
}

export function disconnect() {
  if (client) client.disconnect()
  client = null
  store.connected = false
  store.keys = []
  store.selectedKey = null
  store.keyInfo = null
}

export function addConnection(cfg: ConnectionConfig) {
  store.connections.push(cfg)
  saveConnections()
}
export function updateConnection(cfg: ConnectionConfig) {
  const i = store.connections.findIndex((c) => c.id === cfg.id)
  if (i >= 0) store.connections[i] = cfg
  saveConnections()
}
export function deleteConnection(id: string) {
  store.connections = store.connections.filter((c) => c.id !== id)
  if (store.activeConnId === id) {
    store.activeConnId = store.connections[0]?.id || null
    store.connected = false
  }
  saveConnections()
}
export function newConnId(): string {
  return 'conn-' + Date.now().toString(36) + '-' + connSeq++
}

// ---- 库 / key 浏览 ----
export async function loadDbCounts() {
  if (!client) return
  const counts: number[] = []
  for (let db = 0; db < 16; db++) {
    await client.select(db)
    counts.push(await client.dbsize())
  }
  await client.select(store.currentDb)
  store.dbCounts = counts
}

export async function selectDb(db: number) {
  if (!client) return
  store.currentDb = db
  await client.select(db)
  store.keys = []
  store.scanCursor = '0'
  await loadKeys(true)
}

export async function loadKeys(reset: boolean) {
  if (!client) return
  store.loadingKeys = true
  try {
    if (reset) {
      store.keys = []
      store.scanCursor = '0'
    }
    const res: ScanResult = await client.scanKeysWithMeta({
      cursor: store.scanCursor,
      match: store.search || '*',
      count: 200
    })
    let merged = reset ? res.keys : store.keys.concat(res.keys)
    if (store.typeFilter) merged = merged.filter((k) => k.type === store.typeFilter)
    store.keys = merged
    store.scanCursor = res.cursor
    store.hasMoreKeys = res.cursor !== '0'
  } finally {
    store.loadingKeys = false
  }
}

export async function loadMoreKeys() {
  if (store.loadingKeys || !store.hasMoreKeys) return
  await loadKeys(false)
}

export async function refreshKeys() {
  await loadDbCounts()
  await loadKeys(true)
}

export function setSearch(v: string) {
  store.search = v
  loadKeys(true)
}
export function setTypeFilter(v: string) {
  store.typeFilter = v
  loadKeys(true)
}
export function setGroupBy(v: boolean) {
  store.groupBy = v
}

// ---- key 选中 ----
export function selectKey(key: string) {
  store.selectedKey = key
  reloadKeyInfo()
}
export async function reloadKeyInfo() {
  if (!client || !store.selectedKey) return
  const k = store.selectedKey
  store.keyLoadError = null
  try {
    const [type, ttl, size] = await Promise.all([
      client.type(k),
      client.ttl(k),
      client.keyMemory(k)
    ])
    if (store.selectedKey !== k) return
    store.keyInfo = { type: type as RedisType, ttl, size }
    store.keyLoadedAt = Date.now()
  } catch (e: any) {
    if (store.selectedKey === k) {
      store.keyInfo = null
      store.keyLoadError = e.message
    }
  }
}

// ---- 选中集合（批量）----
export function toggleSelect(key: string) {
  const i = store.selectedKeys.indexOf(key)
  if (i >= 0) store.selectedKeys.splice(i, 1)
  else store.selectedKeys.push(key)
}
export function clearSelection() {
  store.selectedKeys = []
}
export function selectAllVisible() {
  store.selectedKeys = store.keys.map((k) => k.name)
}

// ---- 命令行 ----
export async function runCommand(line: string) {
  if (!client || !store.connected) {
    pushTerminal('err', '(未连接) 请先连接 Redis')
    return
  }
  pushTerminal('cmd', 'redis> ' + line)
  const r = await client.executeLine(line)
  pushTerminal(r.kind, r.text)
  // 审计：记录原始命令（敏感参数脱敏）
  recordAudit('CMD', maskSensitive(line), deriveTarget(line), r.kind === 'err' ? 'err' : 'ok')
  // 若影响了 key 空间，刷新
  const lc = line.trim().toUpperCase().split(/\s+/)[0]
  if (['SET', 'DEL', 'HSET', 'LPUSH', 'RPUSH', 'SADD', 'ZADD', 'XADD', 'RENAME', 'COPY', 'EXPIRE', 'PERSIST', 'FLUSHDB', 'FLUSHALL', 'HDEL', 'LREM', 'SREM', 'ZREM', 'XDEL'].includes(lc)) {
    refreshKeys()
  }
}

function deriveTarget(line: string): string {
  const parts = line.trim().split(/\s+/)
  const cmd = parts[0].toUpperCase()
  if (cmd === 'FLUSHDB' || cmd === 'FLUSHALL') return '(all)'
  if (parts[1]) return parts[1]
  return '-'
}

function pushTerminal(kind: string, text: string) {
  store.terminalLines.push({ kind, text })
  if (store.terminalLines.length > 500) store.terminalLines.shift()
}

export function pushHistory(line: string) {
  if (!line.trim()) return
  store.commandHistory = store.commandHistory.filter((h) => h !== line)
  store.commandHistory.unshift(line)
  if (store.commandHistory.length > 100) store.commandHistory.pop()
  store.historyIndex = -1
}
export function historyPrev(): string | null {
  if (!store.commandHistory.length) return null
  store.historyIndex = Math.min(store.historyIndex + 1, store.commandHistory.length - 1)
  return store.commandHistory[store.historyIndex]
}
export function historyNext(): string | null {
  if (!store.commandHistory.length) return null
  store.historyIndex = Math.max(store.historyIndex - 1, -1)
  return store.historyIndex >= 0 ? store.commandHistory[store.historyIndex] : null
}

// ---- 监控 ----
export async function loadMonitor() {
  if (!client) return
  store.monitorLoading = true
  try {
    store.monitorText = await client.info()
  } finally {
    store.monitorLoading = false
  }
}

// ---- 通用 ----
export function toast(msg: string, type = 'info') {
  store.toast = { msg, type }
  setTimeout(() => {
    if (store.toast && store.toast.msg === msg) store.toast = null
  }, 2600)
}

export function getClient(): RedisClient | null {
  return client
}

export function openDialog(type: string, props?: any) {
  store.dialog = { type, props }
}
export function closeDialog() {
  store.dialog = null
}

// ---- 派生 ----
export const groupedKeys = computed(() => {
  if (!store.groupBy) return null
  const map: Record<string, KeyMeta[]> = {}
  for (const k of store.keys) {
    const prefix = k.name.includes(':') ? k.name.split(':')[0] : '(root)'
    ;(map[prefix] || (map[prefix] = [])).push(k)
  }
  return Object.keys(map)
    .sort()
    .map((g) => ({ group: g, items: map[g] }))
})

export const selectedKeyInfo = computed(() => store.keyInfo)
