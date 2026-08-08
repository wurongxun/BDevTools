import { defineStore } from 'pinia'
import type { RouteRecordRaw } from 'vue-router'
import { getMenu } from '@/api/user'
import { get_file_Api } from '@/api/file'
import useStore from '@/store/index'
import { constantRoutes } from '@/router/index'
import router from '@/router/index'
import VerifyUtils from '@/utils/VerifyUtils'
import ToolUtils from '@/utils/ToolUtils'
import { TreeNode } from '@/Types/entitys'
/**
 *
 * @param { RouteRecordRaw[] } routes
 * @param { unknown } parentName
 * @description 数组扁平化， 同时修改路由name为[父元素name -> name]
 * @returns flattenRoutes
 */
function flattenRoutes(routes: RouteRecordRaw[], parentName: unknown = '') {
  let flattenedRoutes: RouteRecordRaw[] = []
  routes.forEach((route) => {
    if ((route.meta && route.meta.hidden) || (route.meta && !route.meta.title)) return

    // clone object 避免修改原始对象
    const flatRoute = JSON.parse(JSON.stringify(route))
    // 获取新的名称，父级 + '->' + 自身名称
    flatRoute.meta.title = parentName
      ? `${parentName} -> ${flatRoute.meta?.title}`
      : flatRoute.meta.title
    if (flatRoute.children) {
      // 递归扁平化子路由，传递新的父级名称
      const childRoutes = flattenRoutes(flatRoute.children, flatRoute!.meta!.title)
      // 移除原始路由中的子路由
      delete flatRoute.children
      // 将子路由添加到扁平化的路由数组中
      flattenedRoutes = flattenedRoutes.concat(childRoutes)
    }
    // 将当前路由添加到扁平化的路由数组中
    flattenedRoutes.push(flatRoute)
  })
  return flattenedRoutes
}

function getResDataPaths(item: RouteRecordRaw, result: Array<string>) {
  if (VerifyUtils.isArray(item.children) && item.children?.length) {
    result.push(item.path)
    item.children.map((i) => getResDataPaths(i, result))
  } else {
    result.push(item.path)
  }
}

function hasPermission(paths: Array<string>, item: RouteRecordRaw): boolean {
  return (
    paths.includes(item.path) ||
    paths.includes(item.meta?.activeMenu as string) ||
    paths.includes(item.redirect as string)
  )
}

function getChildrenData(
  paths: Array<string>,
  children: Array<RouteRecordRaw>
): Array<RouteRecordRaw> {
  const result: Array<RouteRecordRaw> = []
  children.map((item) => {
    if (hasPermission(paths, item)) {
      if (VerifyUtils.isArray(item.children) && item.children?.length) {
        const tmp = ToolUtils.deepClone(item)
        result.push({ ...item, ...{ children: getChildrenData(paths, item.children) } })
      } else {
        result.push({ ...item })
      }
    } else {
      if (VerifyUtils.isArray(item.children) && item.children?.length) {
        result.push({ ...item, ...{ children: getChildrenData(paths, item.children) } })
      }
    }
  })

  return result
}


export const routerNoteStore = defineStore('routerNoteStore', {
  state: () => {
    return {
      tree_node: [] as TreeNode[],
      search_data: [] as TreeNode[],
      routes_loading: false,
      /** 当前窗口绑定的工作目录（per-window 隔离） */
      workingDirectory: ''
    }
  },
  getters: {
    getTreeNode: (state) => state.tree_node,
    getRoutesLoading: (state) => state.routes_loading
  },
  actions: {
    /** 设置当前窗口的工作目录 */
    setWorkingDirectory(dir: string) {
      this.workingDirectory = dir
    },
    /**
     * @description 数组扁平化， 同时修改路由name为[父元素name -> name]
     * @returns RouteRecordRaw[]
     */
    async setRoutes() {
      try {
        this.routes_loading = true

        this.tree_node = []
        // 异步调用接口，主线程无需等待
        get_file_Api()
          .then((red_note: RouteRecordRaw[]) => {
            // 处理获取到的路由数据
            console.log('setRoutes ===============bred_note', red_note)

          })
          .catch((error) => {
            console.error('获取路由信息失败:', error)
          })
      } catch (error) {
        console.error('获取路由信息失败:', error)
      } finally {
        this.routes_loading = false
      }
    },
    async addTreeNode(item: TreeNode[]) {
      this.tree_node = item
    },
    async addSearchData(item: TreeNode[]) {
      this.search_data = item
    },
    /**
     * @description 路由重置
     */
    resetRoutes() {
      this.$reset()
    }
  }
})
export default routerNoteStore
