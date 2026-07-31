export class SystemInfo {
  public memory_total: number | undefined
  public memory_used: number | undefined
  public cpu_used: string | undefined
  public hostname: string | undefined
  public cpus: number | undefined
}

export class TerminalInfo {
  public term: string = ''
  public socketUri: string = ''
  public socket: WebSocket
  public accessToken: string = ''
  // 构造函数
  constructor(term: string, socketUri: string, socket: WebSocket, accessToken: string) {
    this.term = term
    this.socketUri = socketUri
    this.socket = socket
    this.accessToken = accessToken
  }
}

class Message {
  public message_type: string = ''
  public data: string = ''
}

class SshLinkInfo {
  public name: string = ''
  public icon: string = ''
  public path: string = ''
  public children: SshLinkInfo[] = []
  public parent: SshLinkInfo | null = null
  public id: number = 0
  public type: string = ''
  public isShow: boolean = false
  public isHide: boolean = false
  public isDisabled: boolean = false
  public isDisabledChildren: boolean = false
  public isDisabledParent: boolean = false
}

class SettingMap {
  public id: string = ''
  public app_setting_item_name: string = ''
  public app_setting_item_value: string = ''
}
class ResponseData<T> {
  public code: number
  public data: T
  public msg: string = ''
  constructor(code: number, data: T, msg: string) {
    this.code = code
    this.data = data
    this.msg = msg
  }
}

class SettingIdValueName<T> {
  public id: string
  public app_setting_item_name: string
  public app_setting_item_value: T
  constructor(id: string, app_setting_item_name: string, app_setting_item_value: T) {
    this.id = id
    this.app_setting_item_name = app_setting_item_name
    this.app_setting_item_value = app_setting_item_value
  }
}
class SshLinkInfoTree {
  public favorites_name: string = ''
  public subMenu: SshSubMenu[] = []
}
class SshSubMenu {
  public id: number = 0
  public name: string = ''
  public address: string = ''
  public port: number = 22
  public username: string = ''
  public password: string = ''
  public affix?: boolean = false
  public agreement: string = ''
  public favorites_name: string = ''
}
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

class NotebookInfo {
  public id: string = ''
  public parent_id: string = ''
  public path: string = ''
  public folder: string = ''
  public name: string = ''
  public isfile: boolean = false
  public children: Array<NotebookInfo> = []
  //添加get set 方法
}

//  {"id":"1752831345744","parent_id":"","path":"C:\\Users\\rongxun.wu01\\Documents\\notebooks","name":"notebooks","isfile":false,"children":[{"id":"Jabalpur","parent_id":"1752831345744","path":"C:\\Users\\rongxun.wu01\\Documents\\notebooks\\Jabalpur","name":"Jabalpur","isfile":false,"children":[{"id":"OA.md","parent_id":"Jabalpur","path":"C:\\Users\\rongxun.wu01\\Documents\\notebooks\\Jabalpur\\OA.md","name":"OA.md","isfile":true,"children":[]},{"id":"OA24.md","parent_id":"Jabalpur","path":"C:\\Users\\rongxun.wu01\\Documents\\notebooks\\Jabalpur\\OA24.md","name":"OA24.md","isfile":true,"children":[]}]}]}
export {
  Message,
  SshLinkInfo,
  SettingMap,
  ResponseData,
  SettingIdValueName,
  SshLinkInfoTree,
  SshSubMenu,
  NotebookInfo
}

