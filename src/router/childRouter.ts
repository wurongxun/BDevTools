import { createRouter, createWebHashHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import Common from '@/views/Common/Common.vue'

export const childRoutes: Array<RouteRecordRaw> = [
  {
    path: '/App',
    component: Common,
    children: [
      {
        path: '/AppSetting',
        name: 'AppSetting',
        component: () => import('@/views/app/AppSetting.vue'),
        meta: { title: '设置' }
      },
      {
        path: '/AppAbout',
        name: 'AppAbout',
        component: () => import('@/views/app/AppAbout.vue'),
        meta: { title: '关于' }
      },
      {
        path: '/AppHelp',
        name: 'AppHelp',
        component: () => import('@/views/app/AppHelp.vue'),
        meta: { title: '帮助' }
      },
      {
        path: '/AppTerminal',
        name: 'AppTerminal',
        component: () => import('@/views/app/AppTerminal.vue'),
        meta: { title: '终端' }
      }
    ]
  }
]

const childRouter = createRouter({
  history: createWebHashHistory(),
  routes: childRoutes
})

childRouter.beforeEach((to, from, next) => {
  next()
})

export default childRouter
