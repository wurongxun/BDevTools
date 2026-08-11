import { defineStore } from 'pinia'

import ToolUtils from '@/utils/ToolUtils'
import { TreeNode } from '@/Types/entitys'


export const fileTreeStore = defineStore('fileTreeStore', {
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
    async setFileTree() {
      try {
        this.routes_loading = true
        this.tree_node = []
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
export default fileTreeStore
