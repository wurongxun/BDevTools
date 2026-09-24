// RedisClient：屏蔽传输细节的高层 API + 命令行解析/执行/补全 + P1 批量/导出/对比/监控。

import { RedisTransport } from './transport'
import { array } from './engine'
import { parseCommandLine, toUtf8Bytes } from './utils'
import { calcLineDiff, calcJsonDiff } from './diff'
import type {
  Reply,
  RedisType,
  KeyMeta,
  ScanResult,
  ZSetMember,
  StreamEntry,
  SlowlogEntry,
  ClientInfo,
  ConnectionConfig,
  ValueFormat,
  MemoryAnalysis,
  SlowQueryAnalysis,
  ClusterTopology,
  ReplicationInfo,
  SentinelMaster
} from './types'

function unwrap(reply: Reply): any {
  if (!reply) return null
  switch (reply.t) {
    case 'status':
      return reply.v
    case 'bulk':
      return reply.v
    case 'int':
      return reply.v
    case 'array':
      return reply.v || []
    case 'error':
      throw new Error(reply.v)
    default:
      return reply.v
  }
}

export class RedisClient {
  transport: RedisTransport

  constructor(transport: RedisTransport) {
    this.transport = transport
  }

  connect(cfg: ConnectionConfig): Promise<{ ok: boolean; mode: string; id: string }> {
    return this.transport.connect(cfg) as Promise<{ ok: boolean; mode: string; id: string }>
  }
  disconnect(): void {
    this.transport.disconnect()
  }
  isConnected(): boolean {
    return this.transport.isConnected()
  }

  private cmd(...args: string[]): Promise<any> {
    return this.transport.exec(args).then(unwrap)
  }

  execRaw(args: string[]): Promise<Reply> {
    return this.transport.exec(args)
  }

  // ---- 连接 / 库 ----
  select(db: number | string): Promise<any> {
    return this.cmd('SELECT', String(db))
  }
  dbsize(): Promise<number> {
    return this.cmd('DBSIZE')
  }
  flushdb(): Promise<any> {
    return this.cmd('FLUSHDB')
  }

  // ---- key 浏览 ----
  async scanKeysWithMeta(opts: { cursor?: string; match?: string; count?: number } = {}): Promise<ScanResult> {
    const args = ['SCAN', String(opts.cursor || '0')]
    if (opts.match) args.push('MATCH', opts.match)
    args.push('COUNT', String(opts.count || 100))
    const raw = await this.transport.exec(args).then(unwrap)
    const names: string[] = raw[1] || []
    const keys: KeyMeta[] = await Promise.all(
      names.map(async (n) => {
        const [type, ttl, size] = await Promise.all([
          this.type(n).catch(() => 'none' as RedisType),
          this.ttl(n).catch(() => -2),
          this.keyMemory(n).catch(() => 0)
        ])
        return { name: n, type: type as RedisType, ttl, size }
      })
    )
    return { cursor: raw[0], keys }
  }

  type(k: string): Promise<RedisType> {
    return this.cmd('TYPE', k)
  }
  ttl(k: string): Promise<number> {
    return this.cmd('TTL', k)
  }
  keyMemory(k: string): Promise<number> {
    return this.cmd('RCSIZE', k).catch(() => this.cmd('MEMORY', 'USAGE', k))
  }
  exists(k: string): Promise<number> {
    return this.cmd('EXISTS', k)
  }
  del(...keys: string[]): Promise<number> {
    return this.cmd('DEL', ...keys)
  }
  rename(k1: string, k2: string): Promise<any> {
    return this.cmd('RENAME', k1, k2)
  }
  copy(k1: string, k2: string, replace = false): Promise<number> {
    return this.cmd('COPY', k1, k2, ...(replace ? ['REPLACE'] : []))
  }
  expire(k: string, sec: number): Promise<number> {
    return this.cmd('EXPIRE', k, String(sec))
  }
  persist(k: string): Promise<number> {
    return this.cmd('PERSIST', k)
  }
  mset(items: Record<string, string>): Promise<any> {
    const args: string[] = []
    for (const k in items) {
      args.push(k, items[k])
    }
    return this.cmd('MSET', ...args)
  }

  // ---- String ----
  get(k: string): Promise<string | null> {
    return this.cmd('GET', k)
  }
  set(k: string, v: string, exSec?: number): Promise<any> {
    if (exSec) return this.cmd('SET', k, v, 'EX', String(exSec))
    return this.cmd('SET', k, v)
  }
  append(k: string, v: string): Promise<number> {
    return this.cmd('APPEND', k, v)
  }
  strlen(k: string): Promise<number> {
    return this.cmd('STRLEN', k)
  }

  // ---- Hash ----
  async hgetall(k: string): Promise<Record<string, string>> {
    const flat = await this.cmd('HGETALL', k)
    const o: Record<string, string> = {}
    for (let i = 0; i + 1 < flat.length; i += 2) o[flat[i]] = flat[i + 1]
    return o
  }
  hset(k: string, field: string, v: string): Promise<number> {
    return this.cmd('HSET', k, field, v)
  }
  hdel(k: string, field: string): Promise<number> {
    return this.cmd('HDEL', k, field)
  }
  hlen(k: string): Promise<number> {
    return this.cmd('HLEN', k)
  }

  // ---- List ----
  lrange(k: string, s: number, e: number): Promise<string[]> {
    return this.cmd('LRANGE', k, String(s), String(e))
  }
  lpush(k: string, v: string): Promise<number> {
    return this.cmd('LPUSH', k, v)
  }
  rpush(k: string, v: string): Promise<number> {
    return this.cmd('RPUSH', k, v)
  }
  lset(k: string, idx: number, v: string): Promise<any> {
    return this.cmd('LSET', k, String(idx), v)
  }
  lrem(k: string, val: string): Promise<number> {
    return this.cmd('LREM', k, '1', val)
  }
  llen(k: string): Promise<number> {
    return this.cmd('LLEN', k)
  }
  lindex(k: string, idx: number): Promise<string | null> {
    return this.cmd('LINDEX', k, String(idx))
  }

  // ---- Set ----
  smembers(k: string): Promise<string[]> {
    return this.cmd('SMEMBERS', k)
  }
  sadd(k: string, v: string): Promise<number> {
    return this.cmd('SADD', k, v)
  }
  srem(k: string, v: string): Promise<number> {
    return this.cmd('SREM', k, v)
  }
  scard(k: string): Promise<number> {
    return this.cmd('SCARD', k)
  }

  // ---- ZSet ----
  async zrangeAll(k: string): Promise<ZSetMember[]> {
    const flat = await this.cmd('ZRANGE', k, '0', '-1', 'WITHSCORES')
    const out: ZSetMember[] = []
    for (let i = 0; i + 1 < flat.length; i += 2) out.push({ member: flat[i], score: parseFloat(flat[i + 1]) })
    return out
  }
  zadd(k: string, member: string, score: number): Promise<number> {
    return this.cmd('ZADD', k, String(score), member)
  }
  zrem(k: string, member: string): Promise<number> {
    return this.cmd('ZREM', k, member)
  }
  zcard(k: string): Promise<number> {
    return this.cmd('ZCARD', k)
  }
  zscore(k: string, member: string): Promise<string | null> {
    return this.cmd('ZSCORE', k, member)
  }

  // ---- Stream ----
  xadd(k: string, fields: Record<string, string>, id = '*'): Promise<string> {
    const args: string[] = ['XADD', k, id]
    for (const f in fields) {
      args.push(f, fields[f])
    }
    return this.cmd(...args)
  }
  xlen(k: string): Promise<number> {
    return this.cmd('XLEN', k)
  }
  async xrange(k: string, start: string, end: string, count?: number): Promise<StreamEntry[]> {
    const args = ['XRANGE', k, start, end]
    if (count) args.push('COUNT', String(count))
    const raw = await this.cmd(...args)
    return this.parseStream(raw)
  }
  async xrevrange(k: string, end: string, start: string, count?: number): Promise<StreamEntry[]> {
    const args = ['XREVRANGE', k, end, start]
    if (count) args.push('COUNT', String(count))
    const raw = await this.cmd(...args)
    return this.parseStream(raw)
  }
  xdel(k: string, id: string): Promise<number> {
    return this.cmd('XDEL', k, id)
  }
  private parseStream(raw: any[]): StreamEntry[] {
    const out: StreamEntry[] = []
    for (const item of raw) {
      const id = item[0]
      const fields: Record<string, string> = {}
      for (let i = 1; i + 1 < item.length; i += 2) fields[item[i]] = item[i + 1]
      out.push({ id, fields })
    }
    return out
  }

  // ---- 监控（mock 合成 / 真实连接均可用）----
  async info(): Promise<string> {
    return this.cmd('INFO')
  }
  async slowlog(n = 10): Promise<SlowlogEntry[]> {
    const raw = await this.cmd('SLOWLOG', 'GET', String(n))
    return (raw || []).map((e: any[]) => ({
      id: e[0],
      timestamp: e[1],
      durationUs: e[2],
      command: Array.isArray(e[3]) ? e[3].join(' ') : String(e[3])
    }))
  }
  async clientList(): Promise<ClientInfo[]> {
    const text = await this.cmd('CLIENT', 'LIST')
    return String(text || '')
      .split('\n')
      .filter((l) => l.trim())
      .map((line) => {
        const o: Record<string, string> = {}
        line.split(' ').forEach((kv) => {
          const i = kv.indexOf('=')
          if (i >= 0) o[kv.slice(0, i)] = kv.slice(i + 1)
        })
        return {
          id: parseInt(o.id || '0', 10),
          addr: o.addr || '',
          name: o.name || '',
          age: parseInt(o.age || '0', 10),
          idle: parseInt(o.idle || '0', 10),
          db: parseInt(o.db || '0', 10),
          flags: o.flags || ''
        }
      })
  }

  // ============================================================ P1：批量操作
  async batchDelete(keys: string[]): Promise<number> {
    if (!keys.length) return 0
    return this.del(...keys)
  }
  async batchExpire(keys: string[], sec: number): Promise<number> {
    let ok = 0
    for (const k of keys) {
      if (await this.expire(k, sec).catch(() => 0)) ok++
    }
    return ok
  }
  async batchPersist(keys: string[]): Promise<number> {
    let ok = 0
    for (const k of keys) {
      if (await this.persist(k).catch(() => 0)) ok++
    }
    return ok
  }

  // ============================================================ P1：导出 / 导入
  async exportKeys(
    keys: string[],
    fmt: 'json' | 'csv' | 'redis'
  ): Promise<string> {
    const items: { key: string; type: RedisType; ttl: number; value: any }[] = []
    for (const k of keys) {
      const type = await this.type(k).catch(() => 'none' as RedisType)
      let value: any = null
      if (type === 'string') value = await this.get(k)
      else if (type === 'hash') value = await this.hgetall(k)
      else if (type === 'list') value = await this.lrange(k, 0, -1)
      else if (type === 'set') value = await this.smembers(k)
      else if (type === 'zset') value = await this.zrangeAll(k)
      else if (type === 'stream') value = await this.xrange(k, '-', '+')
      const ttl = await this.ttl(k).catch(() => -1)
      items.push({ key: k, type, ttl, value })
    }
    if (fmt === 'json') return JSON.stringify(items, null, 2)
    if (fmt === 'csv') {
      const head = 'key,type,ttl,value_json'
      const rows = items.map(
        (it) => `${csvCell(it.key)},${it.type},${it.ttl},"${String(it.value ?? '').replace(/"/g, '""')}"`
      )
      return head + '\n' + rows.join('\n')
    }
    // redis 命令脚本
    const lines: string[] = []
    for (const it of items) {
      if (it.ttl > 0) lines.push(`# TTL ${it.ttl}s`)
      if (it.type === 'string') lines.push(`SET ${it.key} ${quoteArg(it.value ?? '')}${it.ttl > 0 ? ' EX ' + it.ttl : ''}`)
      else if (it.type === 'hash') {
        const fv: string[] = []
        for (const f in it.value) fv.push(quoteArg(f), quoteArg(it.value[f]))
        lines.push(`HSET ${it.key} ${fv.join(' ')}`)
      } else if (it.type === 'list') {
        for (const v of it.value) lines.push(`RPUSH ${it.key} ${quoteArg(v)}`)
      } else if (it.type === 'set') {
        for (const v of it.value) lines.push(`SADD ${it.key} ${quoteArg(v)}`)
      } else if (it.type === 'zset') {
        for (const m of it.value) lines.push(`ZADD ${it.key} ${m.score} ${quoteArg(m.member)}`)
      } else if (it.type === 'stream') {
        for (const e of it.value) {
          const fv: string[] = []
          for (const f in e.fields) fv.push(quoteArg(f), quoteArg(e.fields[f]))
          lines.push(`XADD ${it.key} ${e.id} ${fv.join(' ')}`)
        }
      }
    }
    return lines.join('\n')
  }

  async importKeys(text: string, fmt: 'json' | 'csv' | 'redis'): Promise<{ count: number; errors: string[] }> {
    const errors: string[] = []
    let count = 0
    try {
      if (fmt === 'json') {
        const items = JSON.parse(text)
        for (const it of items) {
          try {
            await this.writeItem(it.key, it.type, it.value, it.ttl)
            count++
          } catch (e: any) {
            errors.push(`${it.key}: ${e.message}`)
          }
        }
      } else if (fmt === 'csv') {
        const rows = text.trim().split('\n').slice(1)
        for (const row of rows) {
          const parts = splitCsv(row)
          if (parts.length < 4) continue
          const [key, type, ttl, valueJson] = parts
          try {
            const value = JSON.parse(valueJson)
            await this.writeItem(key, type as RedisType, value, parseInt(ttl, 10))
            count++
          } catch (e: any) {
            errors.push(`${key}: ${e.message}`)
          }
        }
      } else {
        // redis 脚本：逐行解析简单命令
        for (const line of text.split('\n')) {
          const t = line.trim()
          if (!t || t.startsWith('#')) continue
          const args = parseCommandLine(t)
          if (!args.length) continue
          try {
            await this.transport.exec(args)
            count++
          } catch (e: any) {
            errors.push(`${t}: ${e.message}`)
          }
        }
      }
    } catch (e: any) {
      errors.push('解析失败: ' + e.message)
    }
    return { count, errors }
  }

  private async writeItem(key: string, type: RedisType, value: any, ttl?: number): Promise<void> {
    if (type === 'string') await this.set(key, String(value ?? ''), ttl && ttl > 0 ? ttl : undefined)
    else if (type === 'hash') {
      for (const f in value) await this.hset(key, f, String(value[f]))
    } else if (type === 'list') {
      for (const v of value) await this.rpush(key, String(v))
    } else if (type === 'set') {
      for (const v of value) await this.sadd(key, String(v))
    } else if (type === 'zset') {
      for (const m of value) await this.zadd(key, m.member, m.score)
    } else if (type === 'stream') {
      for (const e of value) await this.xadd(key, e.fields, e.id)
    }
    if (ttl && ttl > 0) await this.expire(key, ttl)
  }

  // ============================================================ P1：值多格式视图
  async readValue(k: string, fmt: ValueFormat): Promise<{ text: string; ok: boolean; error?: string }> {
    const raw = await this.get(k)
    if (raw == null) return { text: '', ok: true }
    if (fmt === 'hex') {
      const bytes = toUtf8Bytes(raw)
      return { text: bytes.map((b) => b.toString(16).padStart(2, '0')).join(' '), ok: true }
    }
    if (fmt === 'json') {
      try {
        return { text: JSON.stringify(JSON.parse(raw), null, 2), ok: true }
      } catch (e: any) {
        return { text: raw, ok: false, error: e.message }
      }
    }
    // text / xml / yaml 均视为纯文本查看（编辑即原始值）
    return { text: raw, ok: true }
  }

  // ============================================================ P1：值对比
  async diffValues(
    a: string,
    b: string
  ): Promise<{ mode: 'text' | 'json'; lines?: any[]; changes?: any[] }> {
    const [ra, rb] = await Promise.all([this.get(a), this.get(b)])
    const va = ra == null ? '' : ra
    const vb = rb == null ? '' : rb
    let ja: any = null
    let jb: any = null
    try {
      ja = JSON.parse(va)
      jb = JSON.parse(vb)
    } catch {
      // ignore
    }
    if (ja !== null && jb !== null) {
      return { mode: 'json', changes: calcJsonDiff(ja, jb) }
    }
    return { mode: 'text', lines: calcLineDiff(va.split('\n'), vb.split('\n')) }
  }

  // ============================================================ P2：内存分析
  async memoryAnalyze(): Promise<MemoryAnalysis> {
    return this.cmd('RCANALYZE')
  }
  async slowQueryAnalysis(n = 20): Promise<SlowQueryAnalysis> {
    return this.cmd('RCSLOW', String(n))
  }
  async clusterTopology(): Promise<ClusterTopology> {
    return this.cmd('RCCLUSTER')
  }
  async replicationInfo(): Promise<ReplicationInfo> {
    return this.cmd('RCREPL')
  }
  async sentinelMasters(): Promise<SentinelMaster[]> {
    return this.cmd('RCSENTINEL', 'MASTERS')
  }
  async sentinelGetMasterAddr(name: string): Promise<string> {
    return this.cmd('RCSENTINEL', 'GET-MASTER-ADDR-BY-NAME', name)
  }

  // ---- 命令行 ----
  executeLine(line: string): Promise<{ kind: string; text: string; raw?: Reply }> {
    const args = parseCommandLine(line.trim())
    if (!args.length) return Promise.resolve({ kind: 'info', text: '' })
    return this.transport
      .exec(args)
      .then((reply) => {
        const isErr = reply && reply.t === 'error'
        return { kind: isErr ? 'err' : 'ok', text: formatReply(reply), raw: reply }
      })
      .catch((e) => ({ kind: 'err', text: e && e.message ? e.message : String(e) }))
  }
}

function quoteArg(s: string): string {
  if (/^[\w\-.:/@]+$/.test(s)) return s
  return '"' + s.replace(/\\/g, '\\\\').replace(/"/g, '\\"') + '"'
}

function csvCell(s: string): string {
  return /[",\n]/.test(s) ? '"' + s.replace(/"/g, '""') + '"' : s
}

function splitCsv(row: string): string[] {
  const out: string[] = []
  let cur = ''
  let q = false
  for (let i = 0; i < row.length; i++) {
    const c = row[i]
    if (q) {
      if (c === '"') {
        if (row[i + 1] === '"') {
          cur += '"'
          i++
        } else q = false
      } else cur += c
    } else if (c === '"') q = true
    else if (c === ',') {
      out.push(cur)
      cur = ''
    } else cur += c
  }
  out.push(cur)
  return out
}

// 回复格式化（命令行输出）
export function formatReply(reply: Reply): string {
  if (!reply) return '(nil)'
  switch (reply.t) {
    case 'status':
      return '+' + reply.v
    case 'bulk':
      if (reply.v === null) return '(nil)'
      return tryPretty(reply.v)
    case 'int':
      return '(integer) ' + reply.v
    case 'array': {
      if (!reply.v || !reply.v.length) return '(empty array)'
      return reply.v
        .map((x: any, i: number) => {
          if (x && x.t === 'bulk') return i + 1 + ') ' + (x.v === null ? '(nil)' : tryPretty(x.v))
          if (x && typeof x === 'object') return i + 1 + ') ' + JSON.stringify(x)
          return i + 1 + ') ' + (x === null ? '(nil)' : String(x))
        })
        .join('\n')
    }
    case 'error':
      return '(error) ' + reply.v
    default:
      return String(reply.v)
  }
}

function tryPretty(s: any): string {
  if (typeof s !== 'string') return String(s)
  const t = s.trim()
  if ((t[0] === '{' && t[t.length - 1] === '}') || (t[0] === '[' && t[t.length - 1] === ']')) {
    try {
      return JSON.stringify(JSON.parse(t), null, 2)
    } catch {
      return s
    }
  }
  return s
}

// 命令字典（补全 + 文档用）
export const COMMANDS: string[] = [
  'PING', 'ECHO', 'SELECT', 'DBSIZE', 'FLUSHDB', 'FLUSHALL', 'RANDOMKEY',
  'EXISTS', 'DEL', 'TYPE', 'RENAME', 'COPY', 'TTL', 'PTTL', 'EXPIRE', 'PERSIST', 'KEYS', 'SCAN',
  'GET', 'SET', 'MSET', 'APPEND', 'STRLEN', 'GETRANGE', 'SETRANGE', 'INCR', 'INCRBY', 'DECR', 'DECRBY',
  'HSET', 'HSETNX', 'HGET', 'HMGET', 'HGETALL', 'HDEL', 'HLEN', 'HKEYS', 'HVALS', 'HEXISTS', 'HINCRBY',
  'LPUSH', 'RPUSH', 'LPOP', 'RPOP', 'LRANGE', 'LLEN', 'LINDEX', 'LSET', 'LREM', 'LTRIM',
  'SADD', 'SREM', 'SMEMBERS', 'SCARD', 'SISMEMBER',
  'ZADD', 'ZREM', 'ZCARD', 'ZSCORE', 'ZRANGE', 'ZRANGEBYSCORE',
  'XADD', 'XLEN', 'XDEL', 'XRANGE', 'XREVRANGE', 'XREAD',
  'INFO', 'SLOWLOG', 'CLIENT', 'MEMORY', 'RCSIZE'
]

export function completeCommand(prefix: string): string[] {
  const p = (prefix || '').toUpperCase()
  return COMMANDS.filter((c) => c.indexOf(p) === 0)
}

export { array }
