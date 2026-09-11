/**
 * 日志工作台 —— 类型定义
 *
 * 该模块为纯类型与常量，不依赖 Vue / Tauri，便于被核心层与 UI 层共同引用，
 * 也便于后续接入后端索引时复用同一套数据结构。
 */
import type { FilterState } from './core/filter'

/** 归一化后的日志级别（固定集合，UI 配色按此映射） */
export type LogLevel = 'FATAL' | 'ERROR' | 'WARN' | 'INFO' | 'DEBUG' | 'TRACE' | 'UNKNOWN'

/** 原始级别词 → 标准级别（覆盖 Java/Logback/Python/系统日志常见写法） */
export const LEVEL_ALIASES: Record<string, LogLevel> = {
  FATAL: 'FATAL',
  CRITICAL: 'FATAL',
  CRIT: 'FATAL',
  SEVERE: 'FATAL',
  PANIC: 'FATAL',
  ERROR: 'ERROR',
  ERR: 'ERROR',
  EXCEPTION: 'ERROR',
  WARNING: 'WARN',
  WARN: 'WARN',
  INFORMATION: 'INFO',
  INFO: 'INFO',
  NOTICE: 'INFO',
  DEBUG: 'DEBUG',
  FINE: 'DEBUG',
  FINER: 'DEBUG',
  TRACE: 'TRACE',
  FINEST: 'TRACE',
  VERBOSE: 'TRACE',
}

/** 级别展示顺序（筛选/图例使用，从重到轻） */
export const LEVEL_ORDER: LogLevel[] = ['FATAL', 'ERROR', 'WARN', 'INFO', 'DEBUG', 'TRACE', 'UNKNOWN']

/** 文本区间（半开区间 [start, end)，相对原始行字符串） */
export interface TextRange {
  start: number
  end: number
}

/** 堆栈片段类型 */
export type StackPartKind = 'cls' | 'method' | 'file' | 'line' | 'message'

/** 堆栈解析结果 */
export interface StackInfo {
  /** 语言归属，决定配色与提示文案 */
  lang: 'java' | 'python' | 'other'
  /** 需要单独着色的片段（位置相对原始行） */
  parts: Array<TextRange & { kind: StackPartKind }>
  /** 摘要（如 Caused by 的异常类名） */
  signature?: string
  /**
   * 异常首行的自身级别（thrown / Caused by 等视为 ERROR）。
   * 堆栈帧行不设置，由 parseLines 继承所属异常首行的级别。
   */
  level?: LogLevel
}

/** 单行日志解析结果 */
export interface ParsedLine {
  /** 全局行号（1-based） */
  index: number
  /** 原始文本（始终保留，渲染以它为准） */
  raw: string
  /** 归一化级别；堆栈行由调用方继承上一行级别 */
  level: LogLevel
  /** 时间戳原文 */
  tsText?: string
  /** 时间戳毫秒值（解析失败时为空） */
  ts?: number
  /** 线程名 */
  thread?: string
  /** logger / 类名 */
  logger?: string
  /** 去掉时间戳/线程/logger 后的正文 */
  message: string
  /** 是否为异常堆栈行（含 Caused by / at ... / File "..."） */
  isStack: boolean
  stack?: StackInfo
  /** 元信息片段位置，供高亮渲染复用（避免二次正则） */
  ranges: {
    ts?: TextRange
    thread?: TextRange
    logger?: TextRange
  }
}

// ==================== 高亮规则 ====================

/** 用户自定义高亮规则（持久化到 plugins/logs.json） */
export interface HighlightRule {
  id: string
  /** 规则名（展示用） */
  name: string
  /** 匹配内容：正则模式 或 普通子串 */
  pattern: string
  /** true = 按正则解析，false = 按子串包含 */
  useRegex: boolean
  ignoreCase: boolean
  /** 背景色（CSS 颜色值） */
  color: string
  /** 前景色（可选，留空则由组件按背景自动取对比色） */
  textColor?: string
  /** 整行着色（true 时高亮整行而非仅命中片段） */
  wholeLine?: boolean
  enabled: boolean
}

// ==================== 搜索 ====================

export type SearchMode = 'text' | 'regex'

export interface SearchOptions {
  keyword: string
  mode: SearchMode
  ignoreCase: boolean
}

/** 单行命中结果 */
export interface LineHits {
  /** 行号（1-based，与 ParsedLine.index 对应） */
  lineIndex: number
  ranges: TextRange[]
}

/** 搜索结果集 */
export interface SearchResult {
  /** 命中行（按行号升序） */
  hits: LineHits[]
  /** 命中总次数（所有行的 range 数量之和） */
  total: number
  /** 命中行号集合，O(1) 判断某行是否命中 */
  hitLines: Set<number>
}

// ==================== 日志源 ====================

export type LogSourceKind = 'local' | 'ssh' | 'stream'

/** 实时流协议类型 */
export type StreamProtocol = 'ws' | 'tcp'

/** 实时流连接描述（持久化用，连接凭据不另存） */
export interface StreamSpec {
  protocol: StreamProtocol
  /** ws：完整 ws:// / wss:// URL；tcp：留空由 host+port 构成 */
  url?: string
  /** tcp 主机 */
  host?: string
  /** tcp 端口 */
  port?: number
}

/** 实时流运行状态（仅 kind==='stream' 有意义） */
export type StreamState = 'connecting' | 'live' | 'paused' | 'error'

/** 一个日志来源（首批仅 local，后续扩展 ssh / stream） */
export interface LogSource {
  id: string
  kind: LogSourceKind
  /** 展示名（文件名或会话名） */
  name: string
  /** 本地文件为绝对路径；远程/流为标识串 */
  path: string
  /** 左侧色标颜色，用于多源区分 */
  color: string
  /** 编码（本地文件读取用） */
  encoding: string
  /** 已加载行数 */
  loadedLines: number
  /** 文件总行数（后端返回；流源为已接收行数） */
  totalLines: number
  /** 实时流描述（kind==='stream' 时存在） */
  stream?: StreamSpec
  /** 实时流运行状态（kind==='stream' 时存在） */
  streamState?: StreamState
  /** 实时流错误信息（streamState==='error' 时存在） */
  streamError?: string
  /** 该源独立记忆的搜索条件（每 tab 互不干扰） */
  search?: SearchOptions
  /** 该源独立记忆的基础筛选条件（每 tab 互不干扰） */
  filter?: FilterState
}
