import { defineStore } from 'pinia'

import ToolUtils from '@/utils/ToolUtils'
import { TreeNode } from '@/Types/entitys'
import { get_file_Api } from '@/api/file'


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
     * 构建当前工作目录的文件树（真实加载：调用 get_document_notebooks 并写入 tree_node）
     * 工作目录未就绪时清空树（此时无目录可加载）
     */
    async setFileTree() {
      if (!this.workingDirectory) {
        this.tree_node = []
        return
      }
      this.routes_loading = true
      try {
        await get_file_Api()
      } catch (error) {
        console.error('获取文件树失败:', error)
        this.tree_node = []
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
