import { defineStore } from 'pinia'
import { useDark, useToggle } from '@vueuse/core'

const systemStore = defineStore('systemStore', {
  state: () => {
    return {
      isCollapse: false,
      isDark: localStorage.getItem('theme-appearance') === 'dark' ? true : false
    }
  },
  actions: {
    /**
     * @description 切换侧边栏展开收缩
     */
    changeCollapse() {
      this.isCollapse = !this.isCollapse
    },

    /**
     * @description 收缩侧边栏
     */
    packCollapse() {
      this.isCollapse = true
    },

    /**
     * @description 展开侧边栏
     */
    expandCollapse(){
      this.isCollapse = false
    },

    /**
     * @param { isDark } boolean
     * @description 暗黑，明亮主题切换
     */

    toggleDark(isDark: boolean) {
      console.log("toggleDark============",isDark)
      this.isDark = isDark
      console.log("toggleDark============ this.isDark", this.isDark)
    }
  }
})

export default systemStore
