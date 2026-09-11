/**
 * 日志工作台持久化
 *
 * 存储位置：后端 KV 命名空间 plugins/logs.json（新增插件无需改后端，见 core/runtime.ts）。
 * 按工作区隔离：每一个工作区保存各自的高亮规则 / 日志源列表 / 筛选条件 / 当前激活源，
 * 切换工作区时自动保存旧工作区状态、清空视图、加载目标工作区状态（"一键切换逻辑"）。
 */
import { load, type Store } from '@/core/runtime'
import { DEFAULT_FILTER, normalizeFilter, type FilterState } from './filter'
import type { HighlightRule, LogSource } from '../types'

const FILE = 'plugins/logs.json'
const PREFIX = 'ws:'
const K_RULES = 'highlightRules'
const K_SOURCES = 'sources'
const K_FILTER = 'filter'
const K_ACTIVE = 'activeSource'
const K_EXPLORER_DIR = 'explorerDir'
const K_FILE_TYPES = 'fileTypes'
const K_REMOTE_CONNS = 'remoteConnections'
const K_STREAM_CONNS = 'streamConnections'
/**
 * 远程连接（SSH / SFTP 日志源）是「用户级配置」，不按工作区分桶：
 * 服务器清单跨工作区共享，走不带 ws: 前缀的普通键（同 plugins/<id>.json 的用户级约定）。
 *
 * 之所以不复用 ssh 插件的 connections 表：该表字段（SshSubMenu）没有 remotePath，
 * 且 (favorites_name, name) 唯一约束要求 name 不重复——同一台主机只能存一条，
 * 无法保存多个不同日志路径，也不该污染 SSH 插件自己的连接列表。
 */

/**
 * 默认过滤文件类型（日志浏览器）：
 * - 常见日志扩展名 .log/.txt/.out
 * - 结构化日志 .json/.ndjson
 * - '' 表示「无后缀名文件」也纳入（需求要求）
 */
export const DEFAULT_FILE_TYPES: string[] = ['.log', '.txt', '.out', '.json', '.ndjson', '']

/** 兼容迁移：旧全局键（实现"一键切换"前的单例数据） */
const LEGACY_KEYS: Record<string, string> = {
  [K_RULES]: K_RULES,
  [K_SOURCES]: K_SOURCES,
  [K_FILTER]: K_FILTER,
}

function wsKey(wsId: string, kind: string): string {
  return `${PREFIX}${wsId}:${kind}`
}

/** 读取：优先按工作区键；缺失时回退旧全局键（仅迁移首跑用一次） */
async function readKey<T>(s: Store, key: string, legacyKey?: string): Promise<T | undefined> {
  const v = await s.get<T | null>(key)
  if (v !== null && v !== undefined) return v
  if (legacyKey) {
    const lv = await s.get<T | null>(legacyKey)
    if (lv !== null && lv !== undefined) return lv
  }
  return undefined
}

/**
 * 内置高亮规则（常见异常关键词）
 * 背景使用半透明色，明暗主题下都不遮挡文字；用户可禁用、修改或删除。
 */
export const DEFAULT_RULES: HighlightRule[] = [
  {
    id: 'builtin-npe',
    name: '空指针异常',
    pattern: 'NullPointerException|NoneType|Cannot read propert|undefined is not',
    useRegex: true,
    ignoreCase: false,
    color: 'rgba(147, 112, 219, 0.38)',
    enabled: true,
  },
  {
    id: 'builtin-timeout',
    name: '超时',
    pattern: '\\b(?:Timeout|TimedOut|timed out|ReadTimeout|ConnectTimeout|SocketTimeout)\\b',
    useRegex: true,
    ignoreCase: true,
    color: 'rgba(255, 153, 0, 0.34)',
    enabled: true,
  },
  {
    id: 'builtin-oom',
    name: '内存溢出',
    pattern: 'OutOfMemory|MemoryError|OutOfDirectMemory|GC overhead',
    useRegex: true,
    ignoreCase: false,
    color: 'rgba(220, 38, 38, 0.36)',
    enabled: true,
  },
]

// ==================== 高亮规则 ====================
export async function loadRules(wsId: string): Promise<HighlightRule[]> {
  const s = await getStore()
  const saved = await readKey<HighlightRule[]>(s, wsKey(wsId, K_RULES), LEGACY_KEYS[K_RULES])
  if (!Array.isArray(saved)) return [...DEFAULT_RULES]
  return saved
}

export async function saveRules(rules: HighlightRule[], wsId: string): Promise<void> {
  const s = await getStore()
  await s.set(wsKey(wsId, K_RULES), rules)
}

// ==================== 日志源列表 ====================
export async function loadSources(wsId: string): Promise<LogSource[]> {
  const s = await getStore()
  const saved = await readKey<LogSource[]>(s, wsKey(wsId, K_SOURCES), LEGACY_KEYS[K_SOURCES])
  if (!Array.isArray(saved)) return []
  return saved
}

export async function saveSources(sources: LogSource[], wsId: string): Promise<void> {
  const s = await getStore()
  await s.set(wsKey(wsId, K_SOURCES), sources)
}

// ==================== 筛选条件 ====================
export async function loadFilter(wsId: string): Promise<FilterState> {
  const s = await getStore()
  const saved = await readKey<FilterState>(s, wsKey(wsId, K_FILTER), LEGACY_KEYS[K_FILTER])
  if (!saved || typeof saved !== 'object') return { ...DEFAULT_FILTER }
  return normalizeFilter(saved)
}

export async function saveFilter(filter: FilterState, wsId: string): Promise<void> {
  const s = await getStore()
  await s.set(wsKey(wsId, K_FILTER), filter)
}

// ==================== 当前激活源 ====================
export async function loadActiveSource(wsId: string): Promise<string> {
  const s = await getStore()
  const saved = await s.get<string | null>(wsKey(wsId, K_ACTIVE))
  return saved ?? ''
}

export async function saveActiveSource(id: string, wsId: string): Promise<void> {
  const s = await getStore()
  await s.set(wsKey(wsId, K_ACTIVE), id)
}

// ==================== 远程连接（SSH / SFTP） ====================
/** 一条保存的远程日志连接 */
export interface RemoteConnRecord {
  id: string
  /** 显示名（默认取远程路径的文件名，可在对话框中编辑） */
  name: string
  host: string
  port: number
  username: string
  /** 明文保存，与 ssh 插件现有做法一致（本机数据目录） */
  password: string
  authType: 'password' | 'pubkey'
  privateKeyPath: string
  /** 远程日志绝对路径（ssh 连接表无此字段，故自行持久化） */
  remotePath: string
  savedAt: number
}

export async function loadRemoteConns(): Promise<RemoteConnRecord[]> {
  const s = await getStore()
  const saved = await s.get<RemoteConnRecord[] | null>(K_REMOTE_CONNS)
  return Array.isArray(saved) ? saved : []
}

export async function saveRemoteConns(list: RemoteConnRecord[]): Promise<void> {
  const s = await getStore()
  await s.set(K_REMOTE_CONNS, list)
}

// ==================== 实时流连接（WebSocket / TCP） ====================
/** 一条保存的实时日志流连接（与远程连接同理，用户级、不按工作区分桶） */
export interface StreamConnRecord {
  id: string
  /** 显示名（留空时由 url / host:port 自动推导） */
  name: string
  protocol: 'ws' | 'tcp'
  /** protocol = ws 时有效 */
  url: string
  /** protocol = tcp 时有效 */
  host: string
  port: string
  savedAt: number
}

export async function loadStreamConns(): Promise<StreamConnRecord[]> {
  const s = await getStore()
  const saved = await s.get<StreamConnRecord[] | null>(K_STREAM_CONNS)
  return Array.isArray(saved) ? saved : []
}

export async function saveStreamConns(list: StreamConnRecord[]): Promise<void> {
  const s = await getStore()
  await s.set(K_STREAM_CONNS, list)
}

// ==================== 日志浏览器目录 ====================
/** 读取单独设置的浏览器根目录（空串表示回退到工作区设置的目录） */
export async function loadExplorerDir(wsId: string): Promise<string> {
  const s = await getStore()
  const v = await s.get<string | null>(wsKey(wsId, K_EXPLORER_DIR))
  return v ?? ''
}

export async function saveExplorerDir(dir: string, wsId: string): Promise<void> {
  const s = await getStore()
  await s.set(wsKey(wsId, K_EXPLORER_DIR), dir)
}

// ==================== 过滤文件类型 ====================
/** 读取过滤文件类型（持久化为规范化后的数组：'.ext' 或 '' 表示无后缀） */
export async function loadFileTypes(wsId: string): Promise<string[]> {
  const s = await getStore()
  const v = await s.get<string[] | null>(wsKey(wsId, K_FILE_TYPES))
  if (!Array.isArray(v) || v.length === 0) return [...DEFAULT_FILE_TYPES]
  return v
}

export async function saveFileTypes(types: string[], wsId: string): Promise<void> {
  const s = await getStore()
  await s.set(wsKey(wsId, K_FILE_TYPES), types)
}

/**
 * 将用户输入（逗号 / 空格 / 换行分隔）解析为规范化文件类型数组。
 * - 空输入 → 返回默认类型
 * - 'log' / 'txt' / '.log' → '.log' / '.txt'（统一带点小写）
 * - 单独一项为空（或显式 'none'）→ 以 '' 表示「无后缀名文件」
 */
export function parseFileTypes(input: string): string[] {
  const raw = input
    .split(/[\s,;]+/)
    .map(t => t.trim())
    .filter(Boolean)
  if (raw.length === 0) return [...DEFAULT_FILE_TYPES]
  const out = new Set<string>()
  for (const t of raw) {
    const lower = t.toLowerCase()
    if (lower === 'none' || lower === '无后缀' || lower === '无扩展名') {
      out.add('')
    } else if (lower.startsWith('.')) {
      out.add(lower)
    } else {
      out.add('.' + lower)
    }
  }
  return [...out]
}

/** 判断文件名是否匹配给定的文件类型集合（types 含 '' 表示无后缀文件也匹配） */
export function matchFileType(name: string, types: string[]): boolean {
  const dot = name.lastIndexOf('.')
  const hasExt = dot > 0 && dot < name.length - 1
  const ext = hasExt ? name.slice(dot).toLowerCase() : ''
  if (ext === '') return types.includes('')
  return types.includes(ext)
}

/** 将规范化文件类型数组拼回展示文本（'' 显示为 'none'） */
export function formatFileTypes(types: string[]): string {
  return types.map(t => (t === '' ? 'none' : t)).join(' ')
}

// ==================== Store 句柄（单例） ====================
let storeRef: Store | null = null

async function getStore(): Promise<Store> {
  if (!storeRef) storeRef = await load(FILE)
  return storeRef
}
