import { NotebookInfo, ResponseData } from '@/Types/entitys'
import NotificationUtil from '@/utils/NotificationUtil'
import request from '@/utils/request/index'
import { tauri_request } from '@/utils/tauri_request'
import { ref } from 'vue'
import { RouteRecordRaw } from 'vue-router'
import Layout from '@/Layout/index.vue'
import { load, Store } from '@tauri-apps/plugin-store'

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
    return get_notebooks_tree_Api(data_, md_working_directory.value)
  } else {
    NotificationUtil.error(res.msg)
    return [
      {
        path: '/notebook2',
        meta: {
          title: '笔记2',
          icon: 'component',
          keepAlive: true,
          folder: md_working_directory.value
        },
        component: Layout,
        children: []
      }
    ]
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
    console.log('get_notebooks_tree_Api notebooks_children', notebooks_children)
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
      let route2: RouteRecordRaw = {
        path: child.path,
        name: child.name,
        redirect: '',
        meta: {
          title: child.name,
          keepAlive: true,
          path: child.path,
          folder: child.folder,
          name: child.name,
          isfile: false,
          type: 'folder'
        },
        children: child_child_val
      }
      children_val.push(route2)
    }
  }
  console.log('get_notebooks_tree_Api children_val', children_val)
  let route = [
    {
      path: '/notebook2',
      name: '笔记2',
      meta: { title: '笔记2', icon: 'component', keepAlive: true, folder: notebooks_data.folder },
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
      let route2: RouteRecordRaw = {
        path: child.path,
        name: child.name,
        redirect: '',
        meta: {
          title: child.name,
          keepAlive: true,
          path: child.path,
          folder: child.folder,
          name: child.name,
          isfile: false,
          type: child.isfile ? 'md' : 'folder'
        },
        children: child.isfile ? [] : child_child_val
      }
      children_val.push(route2)
    }
  }

  return children_val
}
// const route: RouteRecordRaw = {
//   path: '/notebook',
//   meta: { title: '笔记', icon: 'component', keepAlive: true },
//   component: Layout,
//   children: [
//     {
//       path: '/notebook/笔记2025.md',
//       name: '笔记2025.md',
//       meta: {
//         title: '副本.md',
//         keepAlive: true,
//         path: 'C:\\Users\\kun15\\Documents\\notebooks\\notebooks\\笔记2025.md',
//         folder: 'F:\\MdNote\\notebooks\\你好',
//         name: '副本.md',
//         isfile: true,
//         type: 'md'
//       },
//       component: () => import('@/views/utilsView/notebook/NoteBookEdit.vue')
//     },
//     {
//       path: '/level22',
//       name: 'level22',
//       meta: {
//         title: '哈哈哈.md',
//         keepAlive: true,
//         path: 'C:\\Users\\kun15\\Documents\\notebooks\\notebooks\\GO啊.md',
//         folder: 'F:\\MdNote\\notebooks\\你好',
//         name: '哈哈哈.md',
//         isfile: true,
//         type: 'md'
//       },
//       component: () => import('@/views/utilsView/notebook/NoteBookEdit.vue')
//     },
//     {
//       path: '/component/nested',
//       meta: { title: '嵌套菜单示例' },
//       children: [
//         {
//           path: '/level22',
//           name: 'level22',
//           meta: {
//             title: '哈哈哈.md',
//             keepAlive: true,
//             path: 'C:\\Users\\kun15\\Documents\\notebooks\\notebooks\\GO啊.md',
//             folder: 'F:\\MdNote\\notebooks\\你好',
//             name: '哈哈哈.md',
//             isfile: true,
//             type: 'md'
//           },
//           component: () => import('@/views/utilsView/notebook/NoteBookEdit.vue')
//         }
//       ]
//     }
//   ]
// }
