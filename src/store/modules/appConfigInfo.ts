import { defineStore } from 'pinia'
import { SettingMap } from '@/Types/entitys'

export const appConfigInfo = defineStore('app_Config', {
  state: () => ({
    isDark: false,
    isDarkId: '',
    appConfig: [] as Array<SettingMap>,
    language: 'zh-CN',
    md_working_directory: '',
    theme: 'light',
    autoStart: false
  }),
  actions: {
    insetId(id: string) {
      this.isDarkId = id
    },
    changeTrue() {
      this.isDark = true
    },
    changeFalse() {
      this.isDark = false
    },
    setAppConfig(app_Config: Array<SettingMap>) {
      this.appConfig = app_Config
    },
    setLanguage(language: string) {
      this.language = language
    },
    setMdWorkingDirectory(md_working_directory: string) {
      this.md_working_directory = md_working_directory
    },
    setTheme(theme: string) {
      this.theme = theme
    },
    setAutoStart(autoStart: boolean) {
      this.autoStart = autoStart
    },
    setDark(isDark: boolean) {
      this.isDark = isDark
    }
  },
  getters: {
    //計算屬性
    getIsDark: (state) => state.isDark,
    getisDarkId: (state) => state.isDarkId,
    getLanguage: (state) => {
      let a: Array<SettingMap> = state.appConfig
      for (let i = 0; i < a.length; i++) {
        if (a[i].app_setting_item_name == 'language') {
          return a[i].app_setting_item_value
        }
      }
    },
    getMdWorkingDirectory: (state) => {
      return state.md_working_directory
    },
    getTheme: (state) => {
      return state.theme
    },
    getAutoStart: (state) => {
      return state.autoStart
    }
  }
})
export default appConfigInfo
