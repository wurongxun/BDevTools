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
  name: string,
  value?: string,
  isRoot?: boolean
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

export {
  ResponseData,
  TreeNode,
  NoteDataStore
}
