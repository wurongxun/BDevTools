// 内存版 Redis 引擎（P0/P1 的可运行后端）。
// 让插件在没有真实 Redis、且沙箱页面无法建立 TCP 连接时，依然能完整演示所有功能。
// 真实连接接入后，本文件保留作「离线演示 / 单测 fixture」（见 transport.ts）。

import { byteLen, globToRegExp, parseBound } from './utils'
import type { Reply, RedisType } from './types'

function status(v: string): Reply {
  return { t: 'status', v }
}
function bulk(v: any): Reply {
  return { t: 'bulk', v: v === undefined ? null : v }
}
function integer(v: number): Reply {
  return { t: 'int', v }
}
export function array(v: any[]): Reply {
  return { t: 'array', v }
}
function error(v: string): Reply {
  return { t: 'error', v }
}

interface Entry {
  type: RedisType
  value: any
  expireAt: number | null
}

function newEntry(type: RedisType): Entry {
  let value: any
  if (type === 'list') value = []
  else if (type === 'hash' || type === 'set' || type === 'zset' || type === 'stream') value = Object.create(null)
  else value = ''
  return { type, value, expireAt: null }
}

export class MockEngine {
  dbs: { keys: Record<string, Entry> }[] = []
  db = 0
  private startedAt = Date.now() - 1000 * 60 * 60 * 26 // 假装已运行约 26 小时
  private lastStreamMs = 0
  private lastStreamSeq = 0

  constructor() {
    for (let i = 0; i < 16; i++) this.dbs.push({ keys: Object.create(null) })
  }

  cur(): { keys: Record<string, Entry> } {
    return this.dbs[this.db]
  }

  now(): number {
    return Date.now()
  }

  // 惰性过期：返回 key 是否仍存在
  private expired(name: string): boolean {
    const db = this.cur()
    const e = db.keys[name]
    if (!e) return true
    if (e.expireAt != null && e.expireAt <= this.now()) {
      delete db.keys[name]
      return true
    }
    return false
  }

  ttlOf(name: string): number {
    const e = this.cur().keys[name]
    if (!e) return -2
    if (e.expireAt == null) return -1
    return Math.max(0, Math.ceil((e.expireAt - this.now()) / 1000))
  }

  sizeOf(e: Entry): number {
    let n = 0
    if (e.type === 'string') n = byteLen(e.value)
    else if (e.type === 'hash') for (const f in e.value) n += byteLen(f) + byteLen(e.value[f])
    else if (e.type === 'list') (e.value as string[]).forEach((x: string) => (n += byteLen(x)))
    else if (e.type === 'set') for (const k in e.value) n += byteLen(k)
    else if (e.type === 'zset') for (const k in e.value) n += byteLen(k) + 8
    else if (e.type === 'stream') {
      for (const id in e.value) {
        n += byteLen(id)
        for (const f in e.value[id]) n += byteLen(f) + byteLen(e.value[id][f])
      }
    }
    return n
  }

  totalMemoryBytes(): number {
    let total = 0
    for (const d of this.dbs) for (const k in d.keys) total += this.sizeOf(d.keys[k])
    return total
  }

  // ---------------------------------------------------------- 命令执行入口
  execute(args: string[]): Reply {
    if (!args || !args.length) return error('ERR wrong number of arguments for (empty)')
    const cmd = String(args[0]).toUpperCase()
    const a = args.slice(1)
    try {
      return this.dispatch(cmd, a)
    } catch (e: any) {
      return error('ERR ' + (e && e.message ? e.message : String(e)))
    }
  }

  private dispatch(cmd: string, a: string[]): Reply {
    const self = this
    const db = () => self.cur()
    const requireArg = (i: number) => {
      if (a[i] == null) throw new Error(`wrong number of arguments for '${cmd}'`)
    }

    switch (cmd) {
      // ---- 连接 / 通用 ----
      case 'PING':
        return a.length ? bulk(a[0]) : status('PONG')
      case 'ECHO':
        return a.length ? bulk(a[0]) : error("ERR wrong number of arguments for 'echo'")
      case 'SELECT': {
        const idx = parseInt(a[0], 10)
        if (isNaN(idx) || idx < 0 || idx > 15) return error('ERR DB index is out of range')
        self.db = idx
        return status('OK')
      }
      case 'DBSIZE':
        return integer(Object.keys(db().keys).length)
      case 'FLUSHDB':
        db().keys = Object.create(null)
        return status('OK')
      case 'FLUSHALL':
        for (let i = 0; i < 16; i++) self.dbs[i].keys = Object.create(null)
        return status('OK')
      case 'RANDOMKEY': {
        const ks = Object.keys(db().keys)
        return bulk(ks.length ? ks[Math.floor(Math.random() * ks.length)] : null)
      }

      // ---- key 空间 ----
      case 'EXISTS': {
        let c = 0
        for (const k of a) if (!self.expired(k)) c++
        return integer(c)
      }
      case 'DEL': {
        let d = 0
        for (const k of a) if (db().keys[k]) {
          delete db().keys[k]
          d++
        }
        return integer(d)
      }
      case 'TYPE':
        if (self.expired(a[0])) return status('none')
        return status(db().keys[a[0]].type)
      case 'RENAME': {
        if (self.expired(a[0])) return error('ERR no such key')
        if (a[0] === a[1]) return error('ERR source and destination objects are the same')
        db().keys[a[1]] = db().keys[a[0]]
        delete db().keys[a[0]]
        return status('OK')
      }
      case 'COPY': {
        if (self.expired(a[0])) return integer(0)
        const dest = a[1]
        const opts = a.slice(2)
        const overwrite = opts.indexOf('REPLACE') >= 0
        if (db().keys[dest] && !overwrite) return integer(0)
        db().keys[dest] = JSON.parse(JSON.stringify(db().keys[a[0]]))
        return integer(1)
      }
      case 'TTL':
        return integer(self.ttlOf(a[0]))
      case 'PTTL': {
        const e = db().keys[a[0]]
        if (!e) return integer(-2)
        if (e.expireAt == null) return integer(-1)
        return integer(Math.max(0, e.expireAt - self.now()))
      }
      case 'EXPIRE': {
        if (self.expired(a[0])) return integer(0)
        const sec = parseInt(a[1], 10)
        if (isNaN(sec)) return error('ERR value is not an integer or out of range')
        db().keys[a[0]].expireAt = self.now() + sec * 1000
        return integer(1)
      }
      case 'PERSIST': {
        if (!db().keys[a[0]]) return integer(0)
        db().keys[a[0]].expireAt = null
        return integer(1)
      }
      case 'KEYS': {
        const re = globToRegExp(a[0] || '*')
        const all = Object.keys(db().keys).filter((n) => !self.expired(n) && re.test(n))
        return array(all)
      }
      case 'SCAN': {
        const cur = parseInt(a[0], 10) || 0
        let match = '*'
        let count = 20
        for (let s = 1; s < a.length; s++) {
          const up = String(a[s]).toUpperCase()
          if (up === 'MATCH' && a[s + 1] != null) match = a[++s]
          else if (up === 'COUNT' && a[s + 1] != null) count = parseInt(a[++s], 10) || 20
        }
        const mre = globToRegExp(match)
        const names = Object.keys(db().keys)
          .filter((n) => !self.expired(n) && mre.test(n))
          .sort()
        const end = Math.min(cur + count, names.length)
        const slice = names.slice(cur, end)
        const next = end < names.length ? String(end) : '0'
        return array([next, slice])
      }

      // ---- String ----
      case 'GET':
        return self.expired(a[0]) ? bulk(null) : bulk(db().keys[a[0]].value)
      case 'SET': {
        let val = a[1]
        let ttl: number | null = null
        for (let si = 2; si < a.length; si++) {
          const su = String(a[si]).toUpperCase()
          if (su === 'EX' && a[si + 1] != null) ttl = parseInt(a[++si], 10) * 1000
          else if (su === 'PX' && a[si + 1] != null) ttl = parseInt(a[++si], 10)
        }
        db().keys[a[0]] = { type: 'string', value: val, expireAt: ttl ? self.now() + ttl : null }
        return status('OK')
      }
      case 'MSET': {
        for (let i = 0; i + 1 < a.length; i += 2) {
          db().keys[a[i]] = { type: 'string', value: a[i + 1], expireAt: null }
        }
        return status('OK')
      }
      case 'APPEND': {
        let en = db().keys[a[0]]
        if (!en) en = db().keys[a[0]] = newEntry('string')
        en.value += a[1]
        return integer(byteLen(en.value))
      }
      case 'STRLEN':
        return self.expired(a[0]) ? integer(0) : integer(byteLen(db().keys[a[0]].value))
      case 'GETRANGE': {
        if (self.expired(a[0])) return bulk('')
        const str = db().keys[a[0]].value
        const len = str.length
        let s0 = parseInt(a[1], 10)
        let e0 = parseInt(a[2], 10)
        if (s0 < 0) s0 = len + s0
        if (e0 < 0) e0 = len + e0
        return bulk(str.substring(s0, e0 + 1))
      }
      case 'SETRANGE': {
        let ee = db().keys[a[0]]
        if (!ee) ee = db().keys[a[0]] = newEntry('string')
        const off = parseInt(a[1], 10)
        let pre = ee.value.substring(0, off)
        while (pre.length < off) pre += '\u0000'
        ee.value = pre + a[2] + ee.value.substring(off + a[2].length)
        return integer(byteLen(ee.value))
      }
      case 'INCR':
      case 'INCRBY':
      case 'DECR':
      case 'DECRBY': {
        if (self.expired(a[0])) db().keys[a[0]] = newEntry('string')
        const cur2 = parseInt(db().keys[a[0]].value, 10)
        if (isNaN(cur2)) return error('ERR value is not an integer or out of range')
        const delta = cmd === 'INCR' ? 1 : cmd === 'DECR' ? -1 : parseInt(a[1], 10)
        if (isNaN(delta)) return error('ERR value is not an integer or out of range')
        const nv = cur2 + delta
        db().keys[a[0]].value = String(nv)
        return integer(nv)
      }

      // ---- Hash ----
      case 'HSET': {
        let h = db().keys[a[0]]
        if (!h) h = db().keys[a[0]] = newEntry('hash')
        let added = 0
        for (let hi = 1; hi + 1 < a.length; hi += 2) {
          if (h.value[a[hi]] === undefined) added++
          h.value[a[hi]] = a[hi + 1]
        }
        return integer(added)
      }
      case 'HSETNX': {
        let h2 = db().keys[a[0]]
        if (!h2) h2 = db().keys[a[0]] = newEntry('hash')
        if (h2.value[a[1]] !== undefined) return integer(0)
        h2.value[a[1]] = a[2]
        return integer(1)
      }
      case 'HGET':
        return self.expired(a[0])
          ? bulk(null)
          : bulk(a[1] in db().keys[a[0]].value ? db().keys[a[0]].value[a[1]] : null)
      case 'HMGET': {
        if (self.expired(a[0])) return array(a.slice(1).map(() => null))
        const hm = db().keys[a[0]].value
        return array(a.slice(1).map((f) => (f in hm ? hm[f] : null)))
      }
      case 'HGETALL': {
        if (self.expired(a[0])) return array([])
        const ha = db().keys[a[0]].value
        const out: string[] = []
        for (const fk in ha) {
          out.push(fk)
          out.push(ha[fk])
        }
        return array(out)
      }
      case 'HDEL': {
        if (self.expired(a[0])) return integer(0)
        const hd = db().keys[a[0]].value
        let removed = 0
        for (let d2 = 1; d2 < a.length; d2++) if (a[d2] in hd) {
          delete hd[a[d2]]
          removed++
        }
        return integer(removed)
      }
      case 'HLEN':
        return self.expired(a[0]) ? integer(0) : integer(Object.keys(db().keys[a[0]].value).length)
      case 'HKEYS':
        return self.expired(a[0]) ? array([]) : array(Object.keys(db().keys[a[0]].value))
      case 'HVALS':
        return self.expired(a[0])
          ? array([])
          : array(Object.keys(db().keys[a[0]].value).map((f) => db().keys[a[0]].value[f]))
      case 'HEXISTS':
        return self.expired(a[0]) ? integer(0) : integer(a[1] in db().keys[a[0]].value ? 1 : 0)
      case 'HINCRBY': {
        if (self.expired(a[0])) db().keys[a[0]] = newEntry('hash')
        const hv = db().keys[a[0]].value
        const base = parseInt(hv[a[1]] || '0', 10)
        const inc = parseInt(a[2], 10)
        if (isNaN(base) || isNaN(inc)) return error('ERR value is not an integer or out of range')
        hv[a[1]] = String(base + inc)
        return integer(base + inc)
      }

      // ---- List ----
      case 'LPUSH':
      case 'RPUSH': {
        let l = db().keys[a[0]]
        if (!l) l = db().keys[a[0]] = newEntry('list')
        for (let li = 1; li < a.length; li++) {
          if (cmd === 'LPUSH') l.value.unshift(a[li])
          else l.value.push(a[li])
        }
        return integer(l.value.length)
      }
      case 'LPOP':
      case 'RPOP': {
        if (self.expired(a[0])) return bulk(null)
        const lv = db().keys[a[0]].value
        if (!lv.length) return bulk(null)
        return bulk(cmd === 'LPOP' ? lv.shift() : lv.pop())
      }
      case 'LRANGE': {
        if (self.expired(a[0])) return array([])
        const arr = db().keys[a[0]].value
        const L = arr.length
        let f0 = parseInt(a[1], 10)
        let t0 = parseInt(a[2], 10)
        if (f0 < 0) f0 = L + f0
        if (t0 < 0) t0 = L + t0
        return array(arr.slice(f0, t0 + 1))
      }
      case 'LLEN':
        return self.expired(a[0]) ? integer(0) : integer(db().keys[a[0]].value.length)
      case 'LINDEX': {
        if (self.expired(a[0])) return bulk(null)
        const la = db().keys[a[0]].value
        let idx = parseInt(a[1], 10)
        if (idx < 0) idx = la.length + idx
        return idx >= 0 && idx < la.length ? bulk(la[idx]) : bulk(null)
      }
      case 'LSET': {
        if (self.expired(a[0])) return error('ERR no such key')
        const lb = db().keys[a[0]].value
        let ix = parseInt(a[1], 10)
        if (ix < 0) ix = lb.length + ix
        if (ix < 0 || ix >= lb.length) return error('ERR index out of range')
        lb[ix] = a[2]
        return status('OK')
      }
      case 'LREM': {
        if (self.expired(a[0])) return integer(0)
        const lc = db().keys[a[0]].value
        const cnt = parseInt(a[1], 10)
        const val3 = a[2]
        let removed2 = 0
        const kept: string[] = []
        for (let c = 0; c < lc.length; c++) {
          if (lc[c] === val3 && cnt !== 0 && (cnt > 0 ? removed2 < cnt : removed2 < -cnt)) {
            removed2++
            continue
          }
          kept.push(lc[c])
        }
        db().keys[a[0]].value = kept
        return integer(removed2)
      }
      case 'LTRIM': {
        if (self.expired(a[0])) return status('OK')
        const lt = db().keys[a[0]].value
        const n = lt.length
        let s3 = parseInt(a[1], 10)
        let e3 = parseInt(a[2], 10)
        if (s3 < 0) s3 = n + s3
        if (e3 < 0) e3 = n + e3
        db().keys[a[0]].value = lt.slice(s3, e3 + 1)
        return status('OK')
      }

      // ---- Set ----
      case 'SADD': {
        let st = db().keys[a[0]]
        if (!st) st = db().keys[a[0]] = newEntry('set')
        let add2 = 0
        for (let ai = 1; ai < a.length; ai++) if (st.value[a[ai]] === undefined) {
          st.value[a[ai]] = 1
          add2++
        }
        return integer(add2)
      }
      case 'SREM': {
        if (self.expired(a[0])) return integer(0)
        const sr = db().keys[a[0]].value
        let rm = 0
        for (let ri = 1; ri < a.length; ri++) if (sr[a[ri]] !== undefined) {
          delete sr[a[ri]]
          rm++
        }
        return integer(rm)
      }
      case 'SMEMBERS':
        return self.expired(a[0]) ? array([]) : array(Object.keys(db().keys[a[0]].value))
      case 'SCARD':
        return self.expired(a[0]) ? integer(0) : integer(Object.keys(db().keys[a[0]].value).length)
      case 'SISMEMBER':
        return self.expired(a[0]) ? integer(0) : integer(a[1] in db().keys[a[0]].value ? 1 : 0)

      // ---- ZSet ----
      case 'ZADD': {
        let z = db().keys[a[0]]
        if (!z) z = db().keys[a[0]] = newEntry('zset')
        let added3 = 0
        for (let zi = 1; zi + 1 < a.length; zi += 2) {
          const sc = parseFloat(a[zi])
          if (isNaN(sc)) return error('ERR value is not a valid float')
          if (z.value[a[zi + 1]] === undefined) added3++
          z.value[a[zi + 1]] = sc
        }
        return integer(added3)
      }
      case 'ZREM': {
        if (self.expired(a[0])) return integer(0)
        const zr = db().keys[a[0]].value
        let rm3 = 0
        for (let zri = 1; zri < a.length; zri++) if (zr[a[zri]] !== undefined) {
          delete zr[a[zri]]
          rm3++
        }
        return integer(rm3)
      }
      case 'ZCARD':
        return self.expired(a[0]) ? integer(0) : integer(Object.keys(db().keys[a[0]].value).length)
      case 'ZSCORE': {
        if (self.expired(a[0])) return bulk(null)
        const zs = db().keys[a[0]].value
        return a[1] in zs ? bulk(String(zs[a[1]])) : bulk(null)
      }
      case 'ZRANGE': {
        if (self.expired(a[0])) return array([])
        const za = db().keys[a[0]].value
        const sorted = Object.keys(za)
          .map((m) => [m, za[m]] as [string, number])
          .sort((x, y) => x[1] - y[1] || (x[0] < y[0] ? -1 : 1))
        let withScore = false
        for (let w = 2; w < a.length; w++) if (String(a[w]).toUpperCase() === 'WITHSCORES') withScore = true
        let start = parseInt(a[1], 10)
        let stop = parseInt(a[2], 10)
        const N = sorted.length
        if (start < 0) start = N + start
        if (stop < 0) stop = N + stop
        const slice3 = sorted.slice(start, stop + 1)
        const out3: string[] = []
        slice3.forEach((p) => {
          out3.push(p[0])
          if (withScore) out3.push(String(p[1]))
        })
        return array(out3)
      }
      case 'ZRANGEBYSCORE': {
        if (self.expired(a[0])) return array([])
        const zb = db().keys[a[0]].value
        const min = parseBound(a[1], -Infinity)
        const max = parseBound(a[2], Infinity)
        const out4: [string, number][] = []
        Object.keys(zb).forEach((m) => {
          const sc = zb[m]
          if (sc >= min && sc <= max) out4.push([m, sc])
        })
        out4.sort((x, y) => x[1] - y[1])
        return array(out4.map((p) => p[0]))
      }

      // ---- Stream ----
      case 'XADD': {
        requireArg(0)
        let key = a[0]
        let i = 1
        const upper = (s: string) => s.toUpperCase()
        if (upper(a[i]) === 'NOMKSTREAM') i++
        const idArg = a[i++]
        const e = db().keys[key] || (db().keys[key] = newEntry('stream'))
        if (e.type !== 'stream') return error('ERR Existing key has wrong value type')
        const fields: Record<string, string> = {}
        for (; i + 1 < a.length; i += 2) fields[a[i]] = a[i + 1]
        const id = this.genStreamId(idArg)
        e.value[id] = fields
        return bulk(id)
      }
      case 'XLEN':
        return self.expired(a[0]) ? integer(0) : integer(Object.keys(db().keys[a[0]].value).length)
      case 'XDEL': {
        if (self.expired(a[0])) return integer(0)
        const sv = db().keys[a[0]].value
        let d = 0
        for (let k = 1; k < a.length; k++) if (a[k] in sv) {
          delete sv[a[k]]
          d++
        }
        return integer(d)
      }
      case 'XRANGE':
      case 'XREVRANGE': {
        if (self.expired(a[0])) return array([])
        const sv = db().keys[a[0]].value
        const ids = Object.keys(sv)
        let start = this.normStreamId(a[1])
        let stop = this.normStreamId(a[2])
        let count = 0
        for (let s = 3; s < a.length; s++) if (a[s].toUpperCase() === 'COUNT') count = parseInt(a[s + 1], 10) || 0
        ids.sort()
        if (cmd === 'XREVRANGE') {
          ids.reverse()
          const t = start
          start = stop
          stop = t
        }
        const out: any[] = []
        let n = 0
        for (const id of ids) {
          if (id < start || id > stop) continue
          const fv = sv[id]
          const flat: string[] = [id]
          for (const f in fv) {
            flat.push(f)
            flat.push(fv[f])
          }
          out.push(flat)
          if (count && ++n >= count) break
        }
        return array(out)
      }
      case 'XREAD': {
        // XREAD [COUNT n] STREAMS key [key ...] id [id ...]
        let count = 0
        let i = 0
        while (i < a.length && a[i].toUpperCase() !== 'STREAMS') {
          if (a[i].toUpperCase() === 'COUNT') count = parseInt(a[i + 1], 10) || 0
          i++
        }
        i++ // skip STREAMS
        const keys: string[] = []
        while (i < a.length && a[i] !== '>') {
          // 收集 key，直到遇到第一个 id（非 key 通常以数字开头或包含 '-'）
          if (a[i].indexOf('-') >= 0 || /^\d+$/.test(a[i])) break
          keys.push(a[i])
          i++
        }
        const ids = a.slice(i)
        const out: any[] = []
        keys.forEach((k, idx) => {
          if (self.expired(k)) return
          const sv = db().keys[k].value
          const all = Object.keys(sv).sort()
          const last = ids[idx]
          const from = last === '$' ? '' : last
          const entries = all.filter((id) => (from ? id > from : true))
          if (!entries.length) return
          const slice = count ? entries.slice(0, count) : entries
          const flat: any[] = [k]
          slice.forEach((id) => {
            const fv = sv[id]
            const e: string[] = [id]
            for (const f in fv) {
              e.push(f)
              e.push(fv[f])
            }
            flat.push(e)
          })
          out.push(flat)
        })
        return out.length ? array(out) : bulk(null)
      }

      // ---- 内存 / 对象 ----
      case 'MEMORY': {
        // 仅支持 MEMORY USAGE key
        if (String(a[0]).toUpperCase() === 'USAGE') {
          return self.expired(a[1]) ? integer(0) : integer(self.sizeOf(db().keys[a[1]]))
        }
        return error('ERR unknown subcommand')
      }
      case 'RCSIZE':
        return self.expired(a[0]) ? integer(0) : integer(self.sizeOf(db().keys[a[0]]))

      // ---- 监控（mock 合成；真实连接由 transport 转发到 server）----
      case 'INFO':
        return bulk(this.synthesizeInfo())
      case 'SLOWLOG':
        return this.slowlog(a)
      case 'CLIENT':
        if (String(a[0]).toUpperCase() === 'LIST') return bulk(this.synthesizeClientList())
        return error('ERR unknown subcommand')

      // ---- P2 分析（mock 合成；真实连接由后端扩展命令支持）----
      case 'RCANALYZE':
        return bulk(this.analyzeMemory())
      case 'RCSLOW':
        return bulk(this.analyzeSlow())
      case 'RCCLUSTER':
        return bulk(this.clusterTopology())
      case 'RCREPL':
        return bulk(this.replicationInfo())
      case 'RCSENTINEL': {
        const sub = String(a[0] || 'MASTERS').toUpperCase()
        if (sub === 'GET-MASTER-ADDR-BY-NAME' && a[1]) return bulk(this.sentinelGetMasterAddr(a[1]))
        return array(this.sentinelMasters())
      }

      default:
        return error(`ERR unknown command '${cmd}'`)
    }
  }

  private genStreamId(idArg?: string): string {
    if (!idArg || idArg === '*') {
      const ms = Date.now()
      if (ms === this.lastStreamMs) this.lastStreamSeq++
      else {
        this.lastStreamMs = ms
        this.lastStreamSeq = 0
      }
      return `${ms}-${this.lastStreamSeq}`
    }
    return idArg
  }

  private normStreamId(id?: string): string {
    if (!id || id === '-' || id === '+' || id === '$') return id === '-' ? '' : '9'.repeat(24)
    return id
  }

  private keyExpiresCount(): number {
    let n = 0
    for (const k in this.cur().keys) {
      const e = this.cur().keys[k]
      if (e.expireAt != null) n++
    }
    return n
  }

  private synthesizeInfo(): string {
    const used = this.totalMemoryBytes()
    const max = 256 * 1024 * 1024 // 模拟 256MB 上限
    const dbKeys = Object.keys(this.cur().keys).length
    const expires = this.keyExpiresCount()
    const uptime = Math.floor((this.now() - this.startedAt) / 1000)
    const ops = 800 + Math.floor(Math.sin(this.now() / 3000) * 200) + Math.floor(Math.random() * 80)
    const lines: string[] = []
    const section = (name: string, fields: [string, string | number][]) => {
      lines.push(`# ${name}`)
      fields.forEach(([k, v]) => lines.push(`${k}:${v}`))
      lines.push('')
    }
    section('Server', [
      ['redis_version', '7.2.0'],
      ['redis_mode', 'standalone'],
      ['arch_bits', '64'],
      ['uptime_in_seconds', uptime],
      ['uptime_in_days', Math.floor(uptime / 86400)],
      ['run_id', 'mock_run_id_00000000000000000000000000000000']
    ])
    section('Clients', [
      ['connected_clients', 2],
      ['blocked_clients', 0],
      ['maxclients', 10000]
    ])
    section('Memory', [
      ['used_memory', used],
      ['used_memory_human', (used / 1024 / 1024).toFixed(2) + 'M'],
      ['used_memory_rss', used + 1024 * 1024],
      ['maxmemory', max],
      ['maxmemory_policy', 'noeviction'],
      ['mem_fragmentation_ratio', '1.02']
    ])
    section('Persistence', [
      ['loading', 0],
      ['rdb_changes_since_last_save', 12],
      ['rdb_last_save_time', Math.floor(this.now() / 1000) - 45],
      ['aof_enabled', 0]
    ])
    section('Stats', [
      ['total_connections_received', 18],
      ['total_commands_processed', 12453],
      ['instantaneous_ops_per_sec', ops],
      ['expired_keys', 7],
      ['evicted_keys', 0],
      ['keyspace_hits', 9321],
      ['keyspace_misses', 412]
    ])
    section('Replication', [['role', 'master'], ['connected_slaves', 0]])
    section('CPU', [
      ['used_cpu_sys', (uptime * 0.02).toFixed(2)],
      ['used_cpu_user', (uptime * 0.05).toFixed(2)]
    ])
    section('Keyspace', [
      [`db${this.db}`, `keys=${dbKeys},expires=${expires},avg_ttl=${expires ? Math.floor(60000 + Math.random() * 40000) : 0}`]
    ])
    return lines.join('\n')
  }

  private slowlog(a: string[]): Reply {
    // SLOWLOG GET [n] | SLOWLOG LEN
    const sub = String(a[0] || 'GET').toUpperCase()
    if (sub === 'LEN') return integer(4)
    const cmds = [
      'KEYS *',
      'HGETALL user:session:large',
      'SORT mylist ALPHA',
      'LRANGE big:list 0 -1'
    ]
    const out: any[] = []
    for (let i = 0; i < 4; i++) {
      out.push([
        i,
        Math.floor(this.now() / 1000) - i * 137,
        1200 + i * 340,
        cmds[i].split(' ')
      ])
    }
    return array(out)
  }

  private synthesizeClientList(): string {
    const lines = [
      'id=11 addr=127.0.0.1:51234 fd=8 name=web-app age=342 idle=0 flags=N db=0 sub=0 psub=0 multi=-1',
      'id=12 addr=10.0.0.7:48120 fd=9 name=worker-1 age=120 idle=27 flags=N db=2 sub=0 psub=0 multi=-1',
      'id=13 addr=127.0.0.1:51240 fd=10 name= age=12 idle=12 flags=S db=0 sub=0 psub=0 multi=-1'
    ]
    return lines.join('\n')
  }

  // ---------------------------------------------------------- P2：内存分析
  analyzeMemory(): any {
    const dbKeys = this.cur().keys
    const items: { name: string; type: RedisType; size: number; ttl: number }[] = []
    for (const name in dbKeys) {
      const ttl = this.ttlOf(name)
      if (ttl === -2) continue
      items.push({ name, type: dbKeys[name].type, size: this.sizeOf(dbKeys[name]), ttl })
    }
    const totalBytes = items.reduce((s, x) => s + x.size, 0)
    const totalKeys = items.length
    const bigKeys = items
      .slice()
      .sort((a, b) => b.size - a.size)
      .slice(0, 20)
    const prefixMap: Record<string, { count: number; size: number }> = {}
    for (const it of items) {
      const p = it.name.includes(':') ? it.name.split(':')[0] : '(root)'
      const m = prefixMap[p] || (prefixMap[p] = { count: 0, size: 0 })
      m.count++
      m.size += it.size
    }
    const prefixes = Object.keys(prefixMap)
      .map((p) => ({
        prefix: p,
        count: prefixMap[p].count,
        size: prefixMap[p].size,
        pct: totalBytes ? Math.round((prefixMap[p].size / totalBytes) * 100) : 0
      }))
      .sort((a, b) => b.size - a.size)
    const typeMap: Record<string, { count: number; size: number }> = {}
    for (const it of items) {
      const m = typeMap[it.type] || (typeMap[it.type] = { count: 0, size: 0 })
      m.count++
      m.size += it.size
    }
    const types = Object.keys(typeMap)
      .map((t) => ({
        type: t as RedisType,
        count: typeMap[t].count,
        size: typeMap[t].size,
        pct: totalBytes ? Math.round((typeMap[t].size / totalBytes) * 100) : 0
      }))
      .sort((a, b) => b.size - a.size)
    return { totalBytes, totalKeys, bigKeys, prefixes, types }
  }

  // ---------------------------------------------------------- P2：慢查询分析
  analyzeSlow(): any {
    const templates = [
      'KEYS *',
      'HGETALL user:session:large',
      'SORT mylist ALPHA',
      'LRANGE big:list 0 -1',
      'SMEMBERS tags:redis',
      'ZRANGE leaderboard 0 -1 WITHSCORES',
      'HGETALL user:1',
      'GET app:config',
      'XREAD COUNT 100 STREAMS stream:events $',
      'MGET cache:hit cache:miss',
      'HSCAN user:session:large 0 COUNT 500',
      'DEBUG SLEEP 1'
    ]
    const durations = [14500, 8200, 6300, 12400, 2100, 9800, 1500, 300, 4500, 800, 7600, 19000]
    const now = Math.floor(this.now() / 1000)
    const items = templates.map((cmd, i) => ({
      id: 100 - i,
      timestamp: now - i * 173,
      durationUs: durations[i],
      command: cmd,
      pattern: this.normalizeSlowCommand(cmd)
    }))
    const durs = items.map((x) => x.durationUs).sort((a, b) => a - b)
    const count = items.length
    const maxUs = durs[durs.length - 1]
    const minUs = durs[0]
    const avgUs = Math.round(durs.reduce((s, x) => s + x, 0) / count)
    const p95Idx = Math.min(durs.length - 1, Math.ceil(0.95 * durs.length) - 1)
    const p95Us = durs[p95Idx]
    const byCmdMap: Record<string, { count: number; totalUs: number; maxUs: number }> = {}
    for (const it of items) {
      const g = byCmdMap[it.pattern] || (byCmdMap[it.pattern] = { count: 0, totalUs: 0, maxUs: 0 })
      g.count++
      g.totalUs += it.durationUs
      g.maxUs = Math.max(g.maxUs, it.durationUs)
    }
    const byCommand = Object.keys(byCmdMap)
      .map((p) => ({ pattern: p, count: byCmdMap[p].count, totalUs: byCmdMap[p].totalUs, maxUs: byCmdMap[p].maxUs }))
      .sort((a, b) => b.totalUs - a.totalUs)
    return { items, count, maxUs, minUs, avgUs, p95Us, byCommand }
  }

  private normalizeSlowCommand(cmd: string): string {
    const parts = cmd.split(/\s+/)
    if (parts.length === 1) return parts[0]
    const tail = parts.slice(1).map((token) => {
      if (token === '*' || token === '$' || /^-?\d+$/.test(token)) return token
      if (token.includes(':')) return token.split(':')[0] + ':*'
      return token
    })
    return parts[0] + ' ' + tail.join(' ')
  }

  // ---------------------------------------------------------- P2：集群 / 复制 / Sentinel（mock 合成）
  clusterTopology(): any {
    const slotRanges = ['0-5460', '5461-10922', '10923-16383']
    const addrs = ['10.0.0.11:6379', '10.0.0.12:6379', '10.0.0.13:6379']
    const nodes: any[] = []
    const myselfId = 'a1b2c3d4e5f60001'
    for (let i = 0; i < 3; i++) {
      nodes.push({
        id: 'a1b2c3d4e5f6000' + (i + 1),
        role: i === 0 ? 'myself' : 'master',
        addr: addrs[i],
        flags: 'master',
        masterId: null,
        slots: slotRanges[i],
        linkState: 'connected'
      })
      nodes.push({
        id: 'b' + (i + 1) + 'c3d4e5f6000' + (i + 1),
        role: 'slave',
        addr: '10.0.0.2' + (i + 1) + ':6379',
        flags: 'slave',
        masterId: 'a1b2c3d4e5f6000' + (i + 1),
        slots: '-',
        linkState: 'connected'
      })
    }
    return { mode: 'cluster', nodeCount: nodes.length, slotsCovered: 16384, myselfId, nodes }
  }

  replicationInfo(): any {
    return {
      role: 'master',
      connectedSlaves: 2,
      masterReplOffset: 38940123,
      slaves: [
        { id: 'slave0', addr: '10.0.0.21:6379', offset: 38940098, lag: 0, state: 'online' },
        { id: 'slave1', addr: '10.0.0.22:6379', offset: 38939950, lag: 1, state: 'online' }
      ]
    }
  }

  sentinelMasters(): any[] {
    return [
      { name: 'mymaster', ip: '10.0.0.11', port: 6379, flags: 'master', numSlaves: 2, numOtherSentinels: 2, quorum: 2 }
    ]
  }

  sentinelGetMasterAddr(_name: string): string {
    return '10.0.0.11:6379'
  }


  // ---------------------------------------------------------- 演示数据（默认连接注入）
  loadSeedDemo(): void {
    const demo: string[][] = [
      ['SET', 'app:name', 'BackendDevTools'],
      ['SET', 'app:config', '{"theme":"dark","lang":"zh","features":["redis","ssh"]}'],
      ['SET', 'greeting', 'hello redis'],
      ['SET', 'big:text', this.bigText()],
      ['HSET', 'user:1', 'name', 'Alice', 'age', '30', 'role', 'admin'],
      ['HSET', 'user:2', 'name', 'Bob', 'age', '25', 'role', 'dev'],
      ['LPUSH', 'queue:tasks', 'task-3', 'task-2', 'task-1'],
      ['RPUSH', 'list:logs', 'line-1', 'line-2', 'line-3', 'line-4', 'line-5'],
      ['SADD', 'tags:redis', 'cache', 'nosql', 'kv', 'pubsub'],
      ['ZADD', 'leaderboard', '100', 'player:A', '250', 'player:B', '180', 'player:C'],
      ['XADD', 'stream:events', '*', 'event', 'login', 'user', 'alice'],
      ['XADD', 'stream:events', '*', 'event', 'click', 'page', 'home'],
      ['SET', 'session:abc', 'token-xyz', 'EX', '120']
    ]
    demo.forEach((c) => this.execute(c))
    // 大 Key 演示数据（用于内存分析/大 Key 排名）
    this.execute(['SET', 'big:string', this.bigText()])
    const bigHash: string[] = ['HSET', 'big:hash']
    for (let i = 0; i < 60; i++) bigHash.push('field' + i, 'value-' + i + '-' + 'x'.repeat(24))
    this.execute(bigHash)
    const bigList: string[] = ['RPUSH', 'big:list']
    for (let i = 0; i < 200; i++) bigList.push('item-' + i + '-' + 'y'.repeat(16))
    this.execute(bigList)
    const bigZ: string[] = ['ZADD', 'big:zset']
    for (let i = 0; i < 40; i++) bigZ.push(String(i), 'member-' + i)
    this.execute(bigZ)
    // 前缀聚合演示：多个 session 与 cache 键
    for (let i = 0; i < 12; i++) this.execute(['SET', 'session:u' + i, 'tok-' + 'a'.repeat(24)])
    for (let i = 0; i < 8; i++) this.execute(['SET', 'cache:item:' + i, 'v' + i + '-' + 'z'.repeat(32)])
    // 多库演示
    this.db = 1
    this.execute(['SET', 'cache:hit', '42'])
    this.db = 0
  }

  private bigText(): string {
    const lines: string[] = []
    for (let i = 0; i < 400; i++) lines.push(`line ${i + 1}: The quick brown fox jumps over the lazy dog. 中文测试内容 ${i + 1}`)
    return lines.join('\n')
  }
}
