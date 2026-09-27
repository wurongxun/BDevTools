import { ResponseData, TreeNode } from '@/Types/entitys'

/** 笔记目录信息（仅本模块使用） */
class NotebookInfo {
  public id: string = ''
  public parent_id: string = ''
  public path: string = ''
  public folder: string = ''
  public name: string = ''
  public isfile: boolean = false
  public data_type: string = ''
  public children: Array<NotebookInfo> = []
}
import NotificationUtil from '@/utils/NotificationUtil'
import { tauri_request } from '@/utils/tauri_request'
import { ref } from 'vue'
import {
  note_comm_router_notebookMd,
  note_comm_router_notebookPdf
} from '@/setting'
import useStore from '@/store/index'

export const get_file_Api = async () => {
  const { fileTreeStore } = useStore()
  // 从当前窗口的 Pinia 状态读取工作目录（per-window 隔离）
  const workingDir = fileTreeStore.workingDirectory || ''

  let notebooks_data = ref<NotebookInfo | null>(null)
  // 将 per-window 工作目录传给 Rust 命令，不再从全局 settings.json 读取
  let res = await tauri_request<ResponseData<NotebookInfo>>('get_document_notebooks', {
    workingDirectory: workingDir || null
  })
  if (res && typeof res === 'object' && 'code' in res && res.code == 200) {
    let data_: NotebookInfo = typeof res.data === 'string' ? JSON.parse(res.data) : res.data
    notebooks_data.value = data_
    FileTreeNodeHandler(data_, workingDir)
    return []
  } else {
    NotificationUtil.error(res.msg)
    return []
  }
}

const FileTreeNodeHandler = (notebooks_data: NotebookInfo, workingDirectory: string = ''): TreeNode[] => {
  let root: Array<TreeNode> = new Array<TreeNode>()
  let searchData_val: Array<TreeNode> = new Array<TreeNode>()
  let notebooks_children: NotebookInfo[] = notebooks_data.children
  let children_val: Array<TreeNode> = []

  if (notebooks_children && notebooks_children.length > 0) {
    for (let i = 0; i < notebooks_children.length; i++) {
      let child: NotebookInfo = notebooks_children[i]
      let child_child: Array<NotebookInfo> = child.children
      let child_child_val: Array<TreeNode> = []
      if (child_child && child_child.length > 0) {
        let children: Array<TreeNode> = File_treeNode_child(child_child, searchData_val)
        for (let j = 0; j < children.length; j++) {
          let child_item = children[j]
          child_child_val.push(child_item)
        }
      }
      let path: string = '/notebook2/'
      if (child.isfile && child.data_type == 'md') {
        path = note_comm_router_notebookMd + child.id
      } else if (child.isfile && child.data_type == 'pdf') {
        path = note_comm_router_notebookPdf + child.id
      } else if (child.isfile && child.data_type == 'txt') {
        path = note_comm_router_notebookPdf + child.id
      }
      let route2: TreeNode = {
        id: child.id,
        label: child.name,
        isFolder: child.isfile ? false : true,
        type: child.data_type as TreeNode['type'],
        routerPath: path,
        folder: child.folder,
        filePath: child.path,
        isfile: child.isfile,
        value: child.name,
        name: child.name,
        children: child.isfile ? [] : child_child_val
      }
      if (child.isfile) {
        searchData_val.push(route2)
      }
      children_val.push(route2)
    }
  }

  // 使用工作区完整路径作为根节点标签
  let rootLabel = workingDirectory || notebooks_data.path || notebooks_data.name || '工作区'

  let root_data: TreeNode = {
    id: notebooks_data.id,
    label: rootLabel,
    isFolder: true,
    type: notebooks_data.data_type as TreeNode['type'],
    routerPath: notebooks_data.path,
    folder: workingDirectory || notebooks_data.folder,
    filePath: notebooks_data.path,
    isfile: false,
    value: notebooks_data.name,
    name: notebooks_data.name,
    children: children_val,
    isRoot: true
  }
  root.push(root_data)

  const { fileTreeStore } = useStore()
  fileTreeStore.addTreeNode(root)
  fileTreeStore.addSearchData(searchData_val)
  return root
}

const File_treeNode_child = (
  notebooks_children: Array<NotebookInfo>,
  searchData_val: Array<TreeNode>
): Array<TreeNode> => {
  let children_val: Array<TreeNode> = new Array<TreeNode>()
  if (notebooks_children && notebooks_children.length > 0) {
    console.log('get_notebooks_tree_child notebooks_children', notebooks_children)
    for (let i = 0; i < notebooks_children.length; i++) {
      let child: NotebookInfo = notebooks_children[i]
      let child_child: Array<NotebookInfo> = child.children
      let child_child_val: Array<TreeNode> = new Array<TreeNode>()
      if (child_child && child_child.length > 0) {
        let children: Array<TreeNode> = File_treeNode_child(child_child, searchData_val)
        for (let j = 0; j < children.length; j++) {
          let child_item = children[j]
          child_child_val.push(child_item)
        }
      }
      let route2: TreeNode = {
        id: child.id,
        label: child.name,
        isFolder: child.isfile ? false : true,
        type: child.data_type as TreeNode['type'],
        folder: child.folder,
        filePath: child.path,
        isfile: child.isfile,
        value: child.name,
        name: child.name,
        children: child.isfile ? [] : child_child_val
      }
      if (child.isfile) {
        searchData_val.push(route2)
      }

      children_val.push(route2)
    }
  }

  return children_val
}
