import { defineStore } from 'pinia'
import type { IUser } from '@/Types/User'

const userStore = defineStore('userStore', {
  state: () => {
    return {
      userInfo: {
        account: '',
        nickName: '',
        phone: '',
        avatar: '',
        email: '',
        postTitle: ''
      } as IUser
    }
  },
  actions: {
    /**
     * @description 重置用户信息
     */
    resetUserInfo(): void {
      this.userInfo = {
        account: '',
        nickName: '',
        phone: ''
      }
    },
    hasUserInfo(): boolean {
      if (this.userInfo.account) {
        return true
      }
      return false
    }
  }
})

export default userStore
