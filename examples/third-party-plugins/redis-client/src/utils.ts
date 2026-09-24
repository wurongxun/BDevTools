// 通用工具：字节长度、glob 匹配、命令解析、字节格式化。

export function byteLen(s: unknown): number {
  if (s == null) return 0
  const str = String(s)
  try {
    // UTF-8 近似字节数
    return unescape(encodeURIComponent(str)).length
  } catch {
    return str.length
  }
}

/** 计算字符串的 UTF-8 字节数组（用于 Hex 视图）。 */
export function toUtf8Bytes(s: string): number[] {
  try {
    const esc = unescape(encodeURIComponent(s))
    const out: number[] = []
    for (let i = 0; i < esc.length; i++) out.push(esc.charCodeAt(i) & 0xff)
    return out
  } catch {
    const out: number[] = []
    for (let i = 0; i < s.length; i++) out.push(s.charCodeAt(i) & 0xff)
    return out
  }
}

/** glob 模式 → RegExp（支持 * ? [abc] [^a]）。 */
export function globToRegExp(pattern: string): RegExp {
  let out = '^'
  for (let i = 0; i < pattern.length; i++) {
    const c = pattern[i]
    if (c === '*') out += '.*'
    else if (c === '?') out += '.'
    else if (c === '[') {
      let j = i + 1
      let neg = false
      let cls = ''
      if (pattern[j] === '^' || pattern[j] === '!') {
        neg = true
        j++
      }
      while (j < pattern.length && pattern[j] !== ']') {
        cls += pattern[j]
        j++
      }
      if (pattern[j] === ']') {
        i = j
        out += '[' + (neg ? '^' : '') + cls + ']'
      } else out += '\\['
    } else if ('.+^${}()|\\/'.indexOf(c) >= 0) out += '\\' + c
    else out += c
  }
  return new RegExp(out + '$')
}

export function formatBytes(n: number): string {
  if (n < 1024) return n + ' B'
  if (n < 1024 * 1024) return (n / 1024).toFixed(1) + ' KB'
  return (n / 1024 / 1024).toFixed(2) + ' MB'
}

/** 把 "GET foo" / HSET user:1 name Alice 拆成参数表（支持双引号与转义）。 */
export function parseCommandLine(line: string): string[] {
  const args: string[] = []
  let cur = ''
  let q: string | null = null
  let i = 0
  while (i < line.length) {
    const c = line[i]
    if (q) {
      if (c === '\\' && i + 1 < line.length) {
        cur += line[i + 1]
        i += 2
        continue
      }
      if (c === q) {
        q = null
        i++
        continue
      }
      cur += c
      i++
      continue
    }
    if (c === '"' || c === "'") {
      q = c
      i++
      continue
    }
    if (c === ' ' || c === '\t') {
      if (cur) {
        args.push(cur)
        cur = ''
      }
      i++
      continue
    }
    cur += c
    i++
  }
  if (cur) args.push(cur)
  return args
}

export function parseBound(s: string | null, def: number): number {
  if (s == null) return def
  let neg = false
  if (s[0] === '(') {
    neg = true
    s = s.substring(1)
  }
  const v = parseFloat(s)
  if (isNaN(v)) return def
  return neg ? v + (v > 0 ? 1e-9 : -1e-9) : v
}

/** 把秒数格式化成人类可读（用于 TTL）。 */
export function formatTtl(ttl: number): string {
  if (ttl === -1) return '永久'
  if (ttl === -2) return '不存在'
  if (ttl < 60) return ttl + ' 秒'
  if (ttl < 3600) return Math.floor(ttl / 60) + ' 分 ' + (ttl % 60) + ' 秒'
  if (ttl < 86400) return Math.floor(ttl / 3600) + ' 时 ' + Math.floor((ttl % 3600) / 60) + ' 分'
  return Math.floor(ttl / 86400) + ' 天 ' + Math.floor((ttl % 86400) / 3600) + ' 时'
}

export function formatUptime(sec: number): string {
  const d = Math.floor(sec / 86400)
  const h = Math.floor((sec % 86400) / 3600)
  const m = Math.floor((sec % 3600) / 60)
  if (d > 0) return `${d}天 ${h}时 ${m}分`
  if (h > 0) return `${h}时 ${m}分`
  return `${m}分`
}

/**
 * 敏感数据脱敏：对命令行 / 审计明细中的密码、token、密钥类内容做掩码。
 * 命中规则：AUTH/CONFIG SET requirepass 等命令的密码参数；key 名含 pass/token/secret 的值。
 */
export function maskSensitive(line: string): string {
  let s = line.trim()
  // 命令级：AUTH <pass>、CONFIG SET requirepass <pass>
  const m = /^(AUTH|CONFIG\s+SET\s+requirepass)\b/i.exec(s)
  if (m) {
    const head = m[0]
    s = head + ' ******'
    return s
  }
  // key 级：若命令第二个 token（key）包含敏感字样，则把后续值替换为 ******
  const parts = s.split(/\s+/)
  if (parts.length >= 2) {
    const key = parts[1]
    if (/pass|token|secret|pwd|password|密钥|口令/i.test(key)) {
      return parts[0] + ' ' + key + ' ******'
    }
  }
  return line
}
