import { createRouter, createWebHashHistory, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import Layout from '@/Layout/index.vue'
import Common from '@/views/Common/Common.vue'

/**
 * @param path: 路由的访问路径
 * @param component: 对应的映射文件
 * @param redirect: 重定向地址,如果包含了children,那么这个路径是必填且对应children中的路径,否则会影响动态加载
 * @param meta: {
 *    @param icon: 图标
 *    @param title: 标题，对应侧边栏显示和面包屑显示
 *    @param noBreadCrumbs: 是否显示面包屑， 默认显示， 如果false则不会显示改路由的层级
 *    @param keepAlive: 是否开始缓存，默认不缓存
 *    @param noTagView: 是否不显示tagView， 默认为false， 即开启 tagview
 *    @param affix: 是否固定在tagview, 设置 affix: true 则 noTagView 不能为 true
 *    @param activeMenu: 接受一个路由路径，对应左侧激活的菜单栏， 同时也会影响到动态加载的路由
 *    @param hidden: 是否在菜单栏显示
 * }
 * @param children: 子路由，多级路由通过children实现
 */

export const constantRoutes: Array<RouteRecordRaw> = [
  {
    path: '/',
    component: Layout,
    redirect: '/home/homePage',
    meta: { icon: 'House', noBreadCrumbs: true },
    children: [
      {
        name: 'HomePage',
        path: '/home/homePage',
        meta: { title: '首页', icon: 'House', keepAlive: true, affix: true },
        component: () => import('@/views/Home/index.vue')
      }
    ]
  },
  {
    path: '/userInfo',
    component: Layout,
    meta: { hidden: true },
    children: [
      {
        name: 'UserInfo',
        path: '/userInfo',
        meta: { title: '个人中心', keepAlive: false },
        component: () => import('@/views/user/UserInfo.vue')
      },
      {
        name: 'ChangePwd',
        path: '/changePwd',
        meta: { title: '修改密码', keepAlive: false },
        component: () => import('@/views/user/ChangePwd.vue')
      }
    ]
  },
  {
    path: '/login',
    meta: { title: '登录', hidden: true, noTagView: true },
    component: () => import('@/views/login/index.vue')
  },
  {
    path: '/App',
    component: Common,
    meta: { hidden: true },
    children: [
      {
        path: 'setting',
        name: 'AppSetting',
        component: () => import('@/views/app/AppSetting.vue'),
        meta: { title: '设置' }
      },
      {
        path: 'about',
        name: 'AppAbout',
        component: () => import('@/views/app/AppAbout.vue'),
        meta: { title: '关于' }
      },
      {
        path: 'help',
        name: 'AppHelp',
        component: () => import('@/views/app/AppHelp.vue'),
        meta: { title: '帮助' }
      },
      {
        path: 'terminal',
        name: 'AppTerminal',
        component: () => import('@/views/app/AppTerminal.vue'),
        meta: { title: '终端' }
      }
    ]
  }
]


const router = createRouter({
  history: createWebHashHistory(), // 使用 hash 模式
  routes: constantRoutes
})

export default router

// ,
// {
//   path: '/App',
//     component: () => import('@/views/Common/Common.vue'),
//   meta: { title: 'App', icon: 'error-handle', noBreadCrumbs: true },
//   children: [
//     {
//       path: '/AppSetting',
//       name: 'AppSetting',
//       component: () => import('@/views/app/AppSetting.vue'),
//       meta: {
//         title: '设置',
//         noTagView: true,
//         i18n: 'settings.name',
//         transition: 'scroll-x-transition'
//       }
//     },
//     {
//       path: '/AppAbout',
//       name: 'AppAbout',
//       component: () => import('@/views/app/AppAbout.vue'),
//       meta: {
//         title: '关于',
//         noTagView: true,
//         i18n: 'about.name',
//         transition: 'scroll-x-transition'
//       }
//     },
//     {
//       path: '/AppHelp',
//       name: 'AppHelp',
//       component: () => import('@/views/app/AppHelp.vue'),
//       meta: {
//         title: '帮助',
//         i18n: 'help.name',
//         noTagView: true,
//         transition: 'scroll-x-transition'
//       }
//     }
//   ]
// }