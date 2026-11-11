/**
 * P3 `.xsh` 会话文件导入（Xshell 兼容层）。
 *
 * 现实约束：Xshell 把 `Password=` 用自己的算法加密，且不同版本不兼容，
 * **没有任何可靠的还原方式**。因此这里只导入非机密字段，并把「请重新填写密码」
 * 作为 warning 交给 UI 明确提示，绝不假装导入完整。
 *
 * `.xsh` 常见为 UTF-16LE（BOM FF FE），也有 ANSI/UTF-8；先按 BOM 挑 TextDecoder。
 */
import { ConnectionType, DEFAULT_CONNECTION_OPTIONS } from '../types'
import type { RemoteConnection } from '../types'

export interface XshImportResult {
  connection: Omit<RemoteConnection, 'id'>
  warnings: string[]
}

export function decodeXshBuffer(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer)
  const utf16le = bytes[0] === 0xff && bytes[1] === 0xfe
  const utf16be = bytes[0] === 0xfe && bytes[1] === 0xff
  try {
    if (utf16le) return new TextDecoder('utf-16le').decode(bytes.slice(2))
    if (utf16be) {
      const swapped = new Uint8Array(bytes.length - 2)
      for (let i = 2; i + 1 < bytes.length; i += 2) {
        swapped[i - 2] = bytes[i + 1]
        swapped[i - 1] = bytes[i]
      }
      return new TextDecoder('utf-16le').decode(swapped)
    }
  } catch {
    // 落到 UTF-8，坏文件由 parseXsh 返回空字段 + warning
  }
  // 无 BOM 但文本里夹着 NUL，基本可以断定是 UTF-16LE
  if (bytes.length > 4 && bytes[1] === 0 && bytes[3] === 0) {
    try { return new TextDecoder('utf-16le').decode(bytes) } catch { /* ignore */ }
  }
  return new TextDecoder('utf-8').decode(bytes)
}

type Fields = Record<string, string>

function readIni(text: string): Fields {
  const out: Fields = {}
  for (const raw of text.replace(/^\uFEFF/, '').split(/\r?\n/)) {
    const line = raw.trim()
    if (!line || line.startsWith(';') || line.startsWith('#') || line.startsWith('[')) continue
    const eq = line.indexOf('=')
    if (eq < 0) continue
    const key = line.slice(0, eq).trim().toLowerCase()
    const value = line.slice(eq + 1).trim().replace(/^"|"$/g, '')
    // 同名键以最后一个为准（Xshell 的 AUTHENTICATION 段会覆盖 CONNECTION 段的 UserName）
    if (key && value) out[key] = value
  }
  return out
}

function first(fields: Fields, ...keys: string[]): string {
  for (const key of keys) {
    if (fields[key]) return fields[key]
  }
  return ''
}

/** 单个 .xsh 文件 → 一条会话草稿（id 由导入流程分配，这里恒为 0 之外由调用方覆盖） */
export function parseXsh(text: string, fallbackName: string): XshImportResult {
  const f = readIni(text)
  const warnings: string[] = []
  const host = first(f, 'host', 'hostname', 'address', 'hostaddress')
  const port = Number(first(f, 'port', 'portnumber')) || 22
  const username = first(f, 'username', 'user', 'loginname', 'userid') || ''
  const protocol = first(f, 'protocol', 'connectiontype', 'type').toLowerCase()
  const folder = first(f, 'folder', 'group', 'foldername')
  const nameFromField = first(f, 'name', 'sessionname', 'title')

  if (f.password || f.passphrase) {
    warnings.push('Xshell 的密码为专有加密格式，无法还原，请导入后手动填写密码 / 私钥口令。')
  }
  if (!host) warnings.push('未识别到 Host 字段，请导入后手动补全主机地址。')
  if (protocol && !protocol.includes('ssh')) {
    warnings.push(`协议为 ${protocol}，本插件仅支持 SSH，已按 SSH 导入。`)
  }
  if (folder) warnings.push(`原分组：${folder}（已按多级路径导入分组字段）。`)

  const privateKeyPath = first(f, 'privatekey', 'userkey', 'identityfile')
  const connection: Omit<RemoteConnection, 'id'> = {
    ...DEFAULT_CONNECTION_OPTIONS,
    name: nameFromField || fallbackName.replace(/\.xsh$/i, ''),
    address: host,
    port,
    username,
    password: '',
    agreement: ConnectionType.SSH,
    favorites_name: folder || '默认分组',
    affix: false,
    private_key_path: privateKeyPath,
    auth_type: privateKeyPath ? 'pubkey' : 'password',
    encoding: first(f, 'encoding', 'charset') || DEFAULT_CONNECTION_OPTIONS.encoding,
    term_type: first(f, 'terminaltype', 'termtype') || DEFAULT_CONNECTION_OPTIONS.term_type,
    description: first(f, 'description', 'comment', 'memo')
  }
  return { connection, warnings }
}
