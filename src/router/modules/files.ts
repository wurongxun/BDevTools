import Layout from '@/Layout/index.vue'
import type { RouteRecordRaw } from 'vue-router'

const route: RouteRecordRaw = {
  path: '/notebook',
  meta: { title: '笔记', icon: 'component', keepAlive: true },
  component: Layout,
  children: [
    {
      path: '/notebook/笔记2025.md',
      name: '笔记2025.md',
      meta: {
        title: '副本.md',
        keepAlive: true,
        path: 'C:\\Users\\kun15\\Documents\\notebooks\\notebooks\\笔记2025.md',
        folder: 'F:\\MdNote\\notebooks\\你好',
        name: '副本.md',
        isfile: true,
        type: 'md'
      },
      component: () => import('@/views/utilsView/markDown/mdPreViewAndEdit.vue')
    },
    {
      path: '/level22',
      name: 'level22',
      meta: {
        title: '哈哈哈.md',
        keepAlive: true,
        filePath: 'C:\\Users\\kun15\\Documents\\notebooks\\notebooks\\GO啊.md',
        folder: 'F:\\MdNote\\notebooks\\你好',
        name: '哈哈哈.md',
        isfile: true,
        type: 'md'
      },
      component: () => import('@/views/utilsView/markDown/mdPreViewAndEdit.vue')
    },
    {
      path: '/component/nested',
      meta: { title: '嵌套菜单示例' },
      children: [
        {
          path: '/level22',
          name: 'level22',
          meta: {
            title: '哈哈哈.md',
            keepAlive: true,
            filePath: 'C:\\Users\\kun15\\Documents\\notebooks\\notebooks\\GO啊.md',
            folder: 'F:\\MdNote\\notebooks\\你好',
            name: '哈哈哈.md',
            isfile: true,
            type: 'md'
          },
          component: () => import('@/views/utilsView/markDown/mdPreViewAndEdit.vue')
        }
      ]
    }
  ]
}

export default route
