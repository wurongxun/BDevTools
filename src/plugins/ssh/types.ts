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

// Tab 内容类型
export type TabContentType = 'terminal' | 'ftp'

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
  auth_type: 'password' | 'pubkey'
  private_key_path: string
  status?: ConnectionStatus
}

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

// 传输任务状态
export type TransferStatus = 'pending' | 'transferring' | 'completed' | 'failed' | 'cancelled'

// 传输任务
export interface TransferTask {
  id: string
  tabId: string
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
