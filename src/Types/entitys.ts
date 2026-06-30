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
class NotebookInfo {
  public id: string = ''
  public parent_id: string = ''
  public path: string = ''
  public folder: string = ''
  public name: string = ''
  public isfile: boolean = false
  public data_type: string = ''
  public children: Array<NotebookInfo> = []
  //添加get set 方法
}
// 定义节点数据结构
interface TreeNode {
  id: string
  label: string
  isFolder: boolean
  type?: 'folder' | 'markdown' | 'pdf'
  children?: TreeNode[]
  folder: string,
  filePath: string,
  isfile: boolean,
  value?: string
  [key: string]: any
}
// 定义笔记数据接口
interface NoteDataStore {
  id: string
  title: string
  content: string
  filePath?: string
  tempContent?: string
  createdAt: Date
  updatedAt: Date
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
  NotebookInfo,
  TreeNode,
  NoteDataStore
}
