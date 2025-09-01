import Layout from '@/Layout/index.vue'
import type { RouteRecordRaw } from 'vue-router'

const route: RouteRecordRaw = {
  path: '/universal',
  meta: { title: '通用页面', icon: 'Warning', keepAlive: false },
  redirect: '/universal/success',
  component: Layout,
  children: [
    {
      path: '/universal/success',
      meta: { title: '成功页面', keepAlive: false },
      component: () => import('@/views/universal/Success.vue')
    },
    {
      path: '/universal/fail',
      meta: { title: '失败页面', keepAlive: false },
      component: () => import('@/views/universal/Fail.vue')
    }
  ]
}

export default route
