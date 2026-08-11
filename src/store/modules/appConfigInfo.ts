import { defineStore } from 'pinia'
import { SettingMap } from '@/entity/entitys'
import { load, Store } from '@tauri-apps/plugin-store';

export const appConfigInfo = defineStore('app_Config', {
  state: () => ({
    isDark: false,
    isDarkId: '',
    appConfig: [] as Array<SettingMap>,
    language: 'zh-CN',
    md_working_directory: '',
    read_lines: 50000,
    theme: 'light',
    autoStart: false,
    update_reminder: false
  }),
  actions: {
    async initializeApp() {

      let store: Store | undefined = undefined;
      store = await load('settings.json', {
        autoSave: false,
        defaults: {}
      });

      let val_auto_start = await store.get<{ value: boolean }>('auto_start');
      if (val_auto_start !== undefined) {
        
        this.setAutoStart(val_auto_start.value);
      }
      let language_val = await store.get<{ value: string }>('language');
      if (language_val !== undefined) {
        
        this.setLanguage(language_val.value);
      }
      let md_working_directory_val = await store.get<{ value: string }>('md_working_directory');
      if (md_working_directory_val !== undefined) {
       
        this.setMdWorkingDirectory(md_working_directory_val.value);
      }
      let read_lines_val = await store.get<{ value: number }>('read_lines');
      if (read_lines_val !== undefined) {
        this.setReadLines(Number(read_lines_val.value))
      }
      let update_reminder_val = await store.get<{ value: boolean }>('update_reminder');
      if (update_reminder_val !== undefined) {
       // update_reminder.value = update_reminder_val.value;
       this.setUpdateReminder(update_reminder_val.value)
      }
      let theme_val = await store.get<{ value: string }>('theme');
      if (theme_val !== undefined) {
        
        this.setTheme(theme_val.value);
      }
      let dark_val = await store.get<{ value: boolean }>('dark');
      if (dark_val !== undefined) {

        this.setDark(dark_val.value);
      }
    },
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
    setReadLines(read_lines: number) {
      this.read_lines = read_lines
    },
    setTheme(theme: string) {
      this.theme = theme
    },
    setAutoStart(autoStart: boolean) {
      this.autoStart = autoStart
    },
    setDark(isDark: boolean) {
      this.isDark = isDark
    },
    setUpdateReminder(update_reminder: boolean){
      this.update_reminder = update_reminder
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
