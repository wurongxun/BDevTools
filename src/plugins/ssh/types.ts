/**
 * SSH/FTP 插件专属类型定义
 * 从 @/entity/entitys 中抽离，仅与本插件组件和 store 耦合
 */

// 连接类型
export enum ConnectionType {
  SSH = 'ssh',
  FTP = 'ftp',
  SFTP = 'sftp'
}

// 连接状态
export enum ConnectionStatus {
  DISCONNECTED = 'disconnected',
  CONNECTING = 'connecting',
  CONNECTED = 'connected',
  ERROR = 'error'
}

// Tab 内容类型（P2 新增隧道面板）
export type TabContentType = 'terminal' | 'ftp' | 'tunnel'

/** 代理配置：kind = none | http | socks5 | socks4 */
export interface SshProxyConfig {
  kind: string
  host: string
  port: number
  username: string
  password: string
}

/** 一级跳板机（按数组顺序链式跳转） */
export interface SshJumpHost {
  host: string
  port: number
  username: string
  password: string
  authType: string
  privateKeyPath: string
  passphrase: string
}

/**
 * P2 高级连接选项。整体以 JSON 文本存在 SQLite 的 `advanced_options` 列，
 * 后端 `options.rs` 用 camelCase 反序列化；新增字段不需要再改表结构。
 * `mfaCode` 是一次性值，刻意不入库。
 */
export interface SshAdvancedOptions {
  proxy: SshProxyConfig
  jumpHosts: SshJumpHost[]
  compress: boolean
  ciphers: string
  macs: string
  kex: string
  hostKeyAlgo: string
  keyboardInteractive: boolean
  agentForward: boolean
  x11Display: string
}

export function defaultAdvancedOptions(): SshAdvancedOptions {
  return {
    proxy: { kind: 'none', host: '', port: 1080, username: '', password: '' },
    jumpHosts: [],
    compress: false,
    ciphers: '',
    macs: '',
    kex: '',
    hostKeyAlgo: '',
    keyboardInteractive: false,
    agentForward: false,
    x11Display: ''
  }
}

/** 端口转发规则（前端持久化，后端按 id 起停任务） */
export type ForwardKind = 'local' | 'remote' | 'dynamic'

export interface ForwardRule {
  id: string
  connectionId: number
  kind: ForwardKind
  bindHost: string
  bindPort: number
  targetHost: string
  targetPort: number
  label: string
}

/** 后端 `ssh_tunnel_list` 返回的运行态 */
export interface ForwardInfo {
  id: string
  kind: ForwardKind
  label: string
  bindHost: string
  bindPort: number
  targetHost: string
  targetPort: number
  status: 'running' | 'stopped' | 'error'
  error: string
  connections: number
  bytesUp: number
  bytesDown: number
  startedAt: number
}

// 远程连接配置
export interface RemoteConnection {
  id: number
  name: string
  address: string
  port: number
  username: string
  password: string
  agreement: ConnectionType
  favorites_name: string
  affix: boolean
  auth_type: 'password' | 'pubkey' | 'agent'
  private_key_path: string
  /** 加密私钥的口令，与登录密码分开 */
  passphrase: string
  /** 终端字符编码，交给前端 TextDecoder(stream) 解码后端透传的原始字节 */
  encoding: string
  /** SSH keepalive 间隔（秒），0 关闭 */
  keepalive_interval: number
  /** PTY 请求的终端类型 */
  term_type: string
  /** 会话备注 */
  description: string
  /** P2 高级连接选项（JSON 文本，见 SshAdvancedOptions） */
  advanced_options: string
  /** P3 颜色标签（#RRGGBB，空串为不标色）；分组支持 `A/B/C` 多级路径 */
  color: string
  status?: ConnectionStatus
}

/** TextDecoder 支持的 label；值为空/未知时一律回落 UTF-8 */
export const ENCODING_OPTIONS = [
  { label: 'UTF-8', value: 'utf-8' },
  { label: 'GBK', value: 'gbk' },
  { label: 'GB18030', value: 'gb18030' },
  { label: 'Big5', value: 'big5' },
  { label: 'Shift_JIS', value: 'shift_jis' }
]

export const TERM_TYPE_OPTIONS = [
  { label: 'xterm-256color', value: 'xterm-256color' },
  { label: 'xterm', value: 'xterm' },
  { label: 'vt100', value: 'vt100' },
  { label: 'linux', value: 'linux' }
]

/** 老数据 / 导入文件缺字段时的兜底值 */
export const DEFAULT_CONNECTION_OPTIONS = {
  passphrase: '',
  encoding: 'utf-8',
  keepalive_interval: 30,
  term_type: 'xterm-256color',
  description: '',
  advanced_options: '',
  color: '',
  auth_type: 'password' as RemoteConnection['auth_type'],
  private_key_path: ''
}

export const PROXY_KIND_OPTIONS = [
  { label: '不使用代理', value: 'none' },
  { label: 'HTTP CONNECT', value: 'http' },
  { label: 'SOCKS5', value: 'socks5' },
  { label: 'SOCKS4 / 4a', value: 'socks4' }
]

export const FORWARD_KIND_OPTIONS = [
  { label: '本地转发 -L', value: 'local' },
  { label: '远程转发 -R', value: 'remote' },
  { label: '动态 SOCKS -D', value: 'dynamic' }
] as const

// 连接 Tab 页签
export interface ConnectionTab {
  id: string
  connectionId: number
  title: string
  icon: string
  type: TabContentType
  closable: boolean
  status: ConnectionStatus
}

// FTP 文件项
export interface FtpFileItem {
  name: string
  path: string
  isDirectory: boolean
  size: number
  modifiedTime: string
  permissions: string
}

// 传输任务状态（paused 与 cancelled 的区别：前者保留 pause 语义，重试时自动续传）
export type TransferStatus =
  | 'pending' | 'transferring' | 'paused'
  | 'completed' | 'failed' | 'cancelled'

/**
 * 入队项。`sourcePath` / `targetPath` 都是**完整路径**——目录递归由
 * FtpView 展开成扁平列表后再入队，store 不再自己拼 `${dir}/${name}`，
 * 否则嵌套目录会被摊平到同一层。
 */
export interface TransferItem {
  fileName: string
  sourcePath: string
  targetPath: string
  size: number
}

// 传输任务
export interface TransferTask {
  id: string
  tabId: string
  /** 调度器重跑任务要用，不能只靠组件持有的 sftpSessionId */
  sessionId: string
  type: 'upload' | 'download'
  fileName: string
  remotePath: string
  localPath: string
  totalSize: number
  transferredSize: number
  progress: number
  status: TransferStatus
  speed: string
  error: string
  startTime: number
  /** 重试时置 true，后端按目标端已有大小续传 */
  resume: boolean
  /** 用户点的是「暂停」而不是「取消」，决定 499 回来后的落点状态 */
  pauseRequested: boolean
}

// FTP 面板状态
export interface FtpPanelState {
  tabId: string
  remotePath: string
  localPath: string
  remoteFiles: FtpFileItem[]
  localFiles: FtpFileItem[]
  selectedRemoteFiles: string[]
  selectedLocalFiles: string[]
  transferTasks: TransferTask[]
}


// ==================== P3：自动化与生态 ====================

/** 关键字高亮规则 */
export interface HighlightRule {
  id: string
  pattern: string
  regex: boolean
  caseSensitive: boolean
  wholeWord: boolean
  foreground: string
  background: string
  bold: boolean
  enabled: boolean
}

/** 高亮集：Xshell 允许按场景启用多个集合 */
export interface HighlightSet {
  id: string
  name: string
  enabled: boolean
  rules: HighlightRule[]
}

/** xterm ITheme 子集；所有导入方案都归一化到这个结构 */
export interface TerminalTheme {
  background: string
  foreground: string
  cursor: string
  cursorAccent?: string
  selectionBackground: string
  black: string
  red: string
  green: string
  yellow: string
  blue: string
  magenta: string
  cyan: string
  white: string
  brightBlack: string
  brightRed: string
  brightGreen: string
  brightYellow: string
  brightBlue: string
  brightMagenta: string
  brightCyan: string
  brightWhite: string
}

export interface TerminalColorScheme {
  id: string
  name: string
  /** imported 来自 .xcs / JSON 导入；builtin 跟随 IDE 明暗主题 */
  source: 'builtin' | 'imported'
  theme: TerminalTheme
}

/** 自定义键盘映射：action=send 时 payload 是发给远端的字符串（支持 \r \n 转义写成真实字符） */
export type KeyAction =
  | 'copy' | 'paste' | 'search' | 'selectAll' | 'clear'
  | 'send' | 'runMacro' | 'toggleRecord' | 'reconnect'

export interface KeyBinding {
  id: string
  label: string
  key: string
  ctrl: boolean
  alt: boolean
  shift: boolean
  meta: boolean
  action: KeyAction
  payload: string
  enabled: boolean
}

/** 自动登录：期望-发送规则 */
export interface ExpectRule {
  id: string
  name: string
  /** 正则表达式文本 */
  pattern: string
  /** 发送内容，支持 ${password} / ${passphrase} / ${username} / ${host} */
  send: string
  /** 每次连接只触发一次 */
  once: boolean
  enabled: boolean
  /** 含口令，写入时不记日志、界面不回显 */
  sensitive: boolean
}

export type MacroStepKind = 'send' | 'wait' | 'expect'

/** 宏步骤：声明式，刻意不使用 eval，避免配置变成任意代码执行 */
export interface MacroStep {
  id: string
  kind: MacroStepKind
  value: string
  /** wait 的毫秒数 / expect 的超时毫秒数 */
  arg: number
  /** send 时自动追加回车 */
  newline: boolean
}

export interface MacroScript {
  id: string
  name: string
  steps: MacroStep[]
  /** 连接成功后自动运行（Xshell 登录脚本） */
  autoRun: boolean
  enabled: boolean
}

export type MacroRunState = 'idle' | 'running' | 'done' | 'failed' | 'stopped'

/** 会话日志录制偏好 */
export interface LogSettings {
  /** 全局开关 */
  enabled: boolean
  /** 连接建立后自动开始 */
  autoRecord: boolean
  /** 每行加 [yyyy-MM-dd HH:mm:ss] */
  timestamp: boolean
  /** 记录用户输入（敏感值会被掩码） */
  logInput: boolean
  /** 用 *** 替换会话密码 / 私钥口令 */
  maskSensitive: boolean
}

/** 分组颜色标签（路径式分组，如 `生产/华东`） */
export interface GroupTag {
  path: string
  color: string
}

export interface VaultState {
  enabled: boolean
  unlocked: boolean
  path: string
}

/** 会话导入导出的统一包裹（P3 起加入转发规则与偏好，可选字段保持向后兼容） */
export interface SessionBundle {
  kind: 'bdevtools-ssh-sessions'
  version: number
  connections: RemoteConnection[]
  groups?: string[]
  groupTags?: GroupTag[]
  forwardRules?: ForwardRule[]
  highlightSets?: HighlightSet[]
  colorSchemes?: TerminalColorScheme[]
  macros?: MacroScript[]
  expectRules?: ExpectRule[]
  keyBindings?: KeyBinding[]
}

export const HIGHLIGHT_PALETTE = [
  { label: '红字', foreground: '#ff6b6b', background: '' },
  { label: '黄底黑字', foreground: '#1e1e1e', background: '#ffd166' },
  { label: '绿字', foreground: '#4ade80', background: '' },
  { label: '蓝底白字', foreground: '#ffffff', background: '#2563eb' },
  { label: '紫底白字', foreground: '#ffffff', background: '#7c3aed' },
  { label: '橙底黑字', foreground: '#1e1e1e', background: '#fb923c' }
]

export const SESSION_COLORS = [
  { label: '无', value: '' },
  { label: '红', value: '#ef4444' },
  { label: '橙', value: '#f97316' },
  { label: '黄', value: '#eab308' },
  { label: '绿', value: '#22c55e' },
  { label: '青', value: '#06b6d4' },
  { label: '蓝', value: '#3b82f6' },
  { label: '紫', value: '#a855f7' }
]
