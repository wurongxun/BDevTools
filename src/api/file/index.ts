import { NotebookInfo, ResponseData, TreeNode } from '@/Types/entitys'
import NotificationUtil from '@/utils/NotificationUtil'
import request from '@/utils/request/index'
import { tauri_request } from '@/utils/tauri_request'
import { defineAsyncComponent, ref } from 'vue'
import { RouteRecordRaw, RouteComponent } from 'vue-router'
import Layout from '@/Layout/index.vue'
import { load, Store } from '@tauri-apps/plugin-store'
import { note_comm_router, note_comm_router_notebookMd, note_comm_router_notebookPdf } from '@/setting'
import { id } from 'element-plus/es/locales.mjs'
import useStore from '@/store/index'
import router from '@/router/index'

/**
 * @description 获取控件权限
 */
export const get_notebooks_Api = async () => {
  let notebooks_data = ref<NotebookInfo | null>(null)
  let md_working_directory = ref<string>('')
  let store: Store | undefined = undefined
  store = await load('settings.json', {
    autoSave: false,
    defaults: {}
  })
  let md_working_directory_val = await store.get<{ value: string }>('md_working_directory')
  if (md_working_directory_val !== undefined) {
    md_working_directory.value = md_working_directory_val.value
  }
  let res = await tauri_request<ResponseData<NotebookInfo>>('get_document_notebooks', {})
  if (res && typeof res === 'object' && 'code' in res && res.code == 200) {
    //值 返回ResponseData 对象，包含data、code、msg参数 例如：data {"id":"1752761254929","parent_id":"","path":"C:\\Users\\kun15\\Documents\\notebooks","name":"notebooks","isfile":false,"children":[]}
    let data_: NotebookInfo = typeof res.data === 'string' ? JSON.parse(res.data) : res.data
    notebooks_data.value = data_
    console.log('get_notebooks_Api', md_working_directory.value)
    console.log('get_notebooks_Api', data_)
    FileTreeNodeHandler(data_)
    let data: RouteRecordRaw[] = get_notebooks_tree_Api(data_, md_working_directory.value)
    return data
  } else {
    NotificationUtil.error(res.msg)
    return []
  }
}
//将NotebookInfo结构转换为RouteRecordRaw
const get_notebooks_tree_Api = (
  notebooks_data: NotebookInfo,
  md_working_directory: string
): RouteRecordRaw[] => {
  let children_val: Array<RouteRecordRaw> = new Array<RouteRecordRaw>()
  let notebooks_children: NotebookInfo[] = notebooks_data.children
  if (notebooks_children && notebooks_children.length > 0) {
    for (let i = 0; i < notebooks_children.length; i++) {
      let child: NotebookInfo = notebooks_children[i]
      let child_child: Array<NotebookInfo> = child.children
      let child_child_val: Array<RouteRecordRaw> = new Array<RouteRecordRaw>()
      if (child_child && child_child.length > 0) {
        let children: Array<RouteRecordRaw> = get_notebooks_tree_child(child_child)
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
      }
      let route2: RouteRecordRaw = {
        path: path,
        name: child.id + child.name,
        redirect: '',
        meta: {
          title: child.name,
          affix: child.isfile,
          keepAlive: true,
          filePath: child.path,
          folder: child.folder,
          name: child.id + child.name,
          isfile: child.isfile,
          type: child.data_type,
          id: child.id
        },
        children: child.isfile ? [] : child_child_val
      }
      // 只有当是文件时才添加component属性
      if (child.isfile) {
        route2.component = get_component(child.data_type)
      }
      children_val.push(route2)
    }
  }
  console.log('get_notebooks_tree_Api children_val', children_val)
  let route = [
    {
      path: '/notebook2/',
      name: '笔记',
      meta: {
        id: 'notebook2',
        title: '笔记',
        icon: 'component',
        keepAlive: true,
        folder: notebooks_data.folder,
        isFile: false
      },
      component: Layout,
      children: children_val
    }
  ]
  return route
}

const get_notebooks_tree_child = (
  notebooks_children: Array<NotebookInfo>
): Array<RouteRecordRaw> => {
  let children_val: Array<RouteRecordRaw> = new Array<RouteRecordRaw>()
  if (notebooks_children && notebooks_children.length > 0) {
    console.log('get_notebooks_tree_child notebooks_children', notebooks_children)
    for (let i = 0; i < notebooks_children.length; i++) {
      let child: NotebookInfo = notebooks_children[i]
      let child_child: Array<NotebookInfo> = child.children
      let child_child_val: Array<RouteRecordRaw> = new Array<RouteRecordRaw>()
      if (child_child && child_child.length > 0) {
        let children: Array<RouteRecordRaw> = get_notebooks_tree_child(child_child)
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
      }
      let route2: RouteRecordRaw = {
        path: path,
        name: child.id + child.name,
        redirect: '',
        meta: {
          title: child.name,
          affix: child.isfile,
          keepAlive: true,
          filePath: child.path,
          folder: child.folder,
          name: child.id + child.name,
          noTagView: child.isfile ? false : true,
          isfile: child.isfile,
          type: child.data_type,
          id: child.id
        },
        children: child.isfile ? [] : child_child_val
      }
      // 只有当是文件时才添加component属性
      if (child.isfile) {
        route2.component = get_component(child.data_type)
      }
      children_val.push(route2)
    }
  }

  return children_val
}
const get_component = (str: string): (() => Promise<any>) | null => {
  if (str == 'md') {
    return () => import('@/views/utilsView/markDown/mdPreViewAndEdit.vue')
  } else if (str == 'pdf') {
    return () => import('@/views/utilsView/pdf/pdfPreview.vue')
  } else {
    // 返回一个默认组件，避免返回null
    return () => import('@/views/utilsView/markDown/mdPreViewAndEdit.vue')
  }
}

const FileTreeNodeHandler = (notebooks_data: NotebookInfo): TreeNode[] => {
  let children_val: Array<TreeNode> = new Array<TreeNode>()
  let searchData_val: Array<TreeNode> = new Array<TreeNode>()
  let notebooks_children: NotebookInfo[] = notebooks_data.children
  if (notebooks_children && notebooks_children.length > 0) {
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
      let path: string = '/notebook2/'
      if (child.isfile && child.data_type == 'md') {
        path = note_comm_router_notebookMd + child.id
      } else if (child.isfile && child.data_type == 'pdf') {
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
        children: child.isfile ? [] : child_child_val
      }
      // 由于两个数组包含的是对同一对象的引用，所以：
      // ✅ 清空数组操作是独立的
      // ❌ 修改对象属性会影响两个数组中的对象
      // ✅ 删除数组元素操作是独立的
      if (child.isfile) {
        searchData_val.push(route2)
      }
      children_val.push(route2)
    }
  }
  const { routerNoteStore } = useStore()
  routerNoteStore.addTreeNode(children_val)
  routerNoteStore.addSearchData(searchData_val)
  return children_val
}

const File_treeNode_child = (notebooks_children: Array<NotebookInfo>, searchData_val: Array<TreeNode>): Array<TreeNode> => {
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
      let path: string = '/notebook2/'
      if (child.isfile && child.data_type == 'md') {
        path = note_comm_router_notebookMd + child.id
      } else if (child.isfile && child.data_type == 'pdf') {
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
