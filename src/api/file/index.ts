import { NotebookInfo, ResponseData, TreeNode } from '@/Types/entitys'
import NotificationUtil from '@/utils/NotificationUtil'
import request from '@/utils/request/index'
import { tauri_request } from '@/utils/tauri_request'
import { defineAsyncComponent, ref } from 'vue'
import { RouteRecordRaw, RouteComponent } from 'vue-router'
import { load, Store } from '@tauri-apps/plugin-store'
import {
  note_comm_router,
  note_comm_router_notebookMd,
  note_comm_router_notebookPdf
} from '@/setting'
import { id } from 'element-plus/es/locales.mjs'
import useStore from '@/store/index'
import router from '@/router/index'

export const get_file_Api = async () => {
  let notebooks_data = ref<NotebookInfo | null>(null)
  let md_working_directory = ref<string>('')
  let store: Store | undefined = undefined
  store = await load('settings.json', {
    autoSave: false,
    defaults: {}
  })
  //获取文件信息
  let md_working_directory_val = await store.get<{ value: string }>('md_working_directory')
  if (md_working_directory_val !== undefined) {
    md_working_directory.value = md_working_directory_val.value
  }
  let res = await tauri_request<ResponseData<NotebookInfo>>('get_document_notebooks', {})
  if (res && typeof res === 'object' && 'code' in res && res.code == 200) {
    //值 返回ResponseData 对象，包含data、code、msg参数 例如：data {"id":"1752761254929","parent_id":"","path":"C:\\Users\\kun15\\Documents\\notebooks","name":"notebooks","isfile":false,"children":[]}
    let data_: NotebookInfo = typeof res.data === 'string' ? JSON.parse(res.data) : res.data
    notebooks_data.value = data_
    FileTreeNodeHandler(data_, md_working_directory.value)
    let data: RouteRecordRaw[] = [] as RouteRecordRaw[]
    return data
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

  const { routerNoteStore } = useStore()
  routerNoteStore.addTreeNode(root)
  routerNoteStore.addSearchData(searchData_val)
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
