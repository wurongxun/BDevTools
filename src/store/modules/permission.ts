import { defineStore } from 'pinia'

const permissionStore = defineStore('permissionStore', {
  state: () => {
    return {
      permissions: [] as Array<string>
    }
  },
  actions: {
    /**
     * @description 初始化权限（桌面工具无需服务端权限）
     */
    async getPermissions() {
      // 桌面工具本地运行，无需服务端权限验证
      this.permissions = ['*']
    }
  }
})

export default permissionStore
