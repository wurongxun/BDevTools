/**
 * P2 连接配置适配层：前端 snake_case 会话 ↔ 后端 camelCase ConnectTarget。
 *
 * SSH 会话配置在 SQLite 里是扁平列，而 exec / 端口转发两条新命令只收一个 JSON 串。
 * 这里集中做转换，避免每个调用点各自拼一遍字段（漏一个字段就是「转发连不上」）。
 */
import { defaultAdvancedOptions } from '../types'
import type { RemoteConnection, SshAdvancedOptions, SshJumpHost } from '../types'

/** 老数据 / 坏 JSON 一律回落默认值，不能让一个坏字段把连接堵死（与后端 parse_options 同口径） */
export function parseAdvancedOptions(raw?: string): SshAdvancedOptions {
  const base = defaultAdvancedOptions()
  if (!raw || !raw.trim()) return base
  try {
    const parsed = JSON.parse(raw)
    if (!parsed || typeof parsed !== 'object') return base
    const jumps: SshJumpHost[] = Array.isArray(parsed.jumpHosts)
      ? parsed.jumpHosts.map((j: any) => ({
          host: String(j?.host ?? ''),
          port: Number(j?.port) || 22,
          username: String(j?.username ?? ''),
          password: String(j?.password ?? ''),
          authType: String(j?.authType ?? 'password'),
          privateKeyPath: String(j?.privateKeyPath ?? ''),
          passphrase: String(j?.passphrase ?? '')
        }))
      : []
    return {
      ...base,
      ...parsed,
      proxy: { ...base.proxy, ...(parsed.proxy || {}) },
      jumpHosts: jumps
    }
  } catch {
    return base
  }
}

export function serializeAdvancedOptions(options: SshAdvancedOptions): string {
  return JSON.stringify(options)
}

/**
 * 把一次性值（MFA 验证码、信任主机）合并进 options JSON。
 * 验证码是临时的，绝不写回 SQLite。
 */
export function withConnectOverrides(
  raw: string | undefined,
  overrides: { mfaCode?: string; trustHost?: boolean }
): string {
  const adv = parseAdvancedOptions(raw) as SshAdvancedOptions & Record<string, unknown>
  if (overrides.mfaCode !== undefined) adv.mfaCode = overrides.mfaCode
  return JSON.stringify(adv)
}

/** 生成后端 `options::ConnectTarget` 的 JSON 字符串 */
export function buildSshTarget(
  conn: RemoteConnection,
  overrides?: { mfaCode?: string; trustHost?: boolean; options?: string }
): string {
  return JSON.stringify({
    host: conn.address,
    port: conn.port || 22,
    username: conn.username,
    password: conn.password,
    authType: conn.auth_type || 'password',
    privateKeyPath: conn.private_key_path || '',
    passphrase: conn.passphrase || '',
    trustHost: overrides?.trustHost ?? false,
    keepaliveInterval: conn.keepalive_interval ?? 30,
    termType: conn.term_type || 'xterm-256color',
    advancedOptions: parseAdvancedOptions(overrides?.options ?? conn.advanced_options),
    mfaCode: overrides?.mfaCode || ''
  })
}
