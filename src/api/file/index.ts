import { NotebookInfo, ResponseData } from '@/Types/entitys';
import NotificationUtil from '@/utils/NotificationUtil';
import request from '@/utils/request/index'
import { tauri_request } from "@/utils/tauri_request";
import { ref } from 'vue';
import { RouteRecordRaw } from 'vue-router';
import Layout from '@/Layout/index.vue'

/**
 * @description 获取控件权限
 */
export const get_notebooks_Api = async () => {
  let notebooks_data = ref<NotebookInfo | null>(null);

  let res = await tauri_request<ResponseData<String[]>>("get_document_notebooks", {})
  if (res && typeof res === "object" && "code" in res && res.code == 200) {
    //值 返回ResponseData 对象，包含data、code、msg参数 例如：data {"id":"1752761254929","parent_id":"","path":"C:\\Users\\kun15\\Documents\\notebooks","name":"notebooks","isfile":false,"children":[]}
    let data_: NotebookInfo =
      typeof res.data === "string" ? JSON.parse(res.data) : res.data;
    notebooks_data.value = data_;
    //转换为RouteRecordRaw 树结构
  let route: RouteRecordRaw = {
    path: '/notebook',
    meta: { title: '笔记', icon: 'component', keepAlive: true },
    component: Layout,
    children: []
  };
   
  } else {
    NotificationUtil.error(res.msg);
  }
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