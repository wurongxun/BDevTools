import { defineStore } from "pinia";
import { SettingMap } from "../entity/entitys";

export const appConfigInfo = defineStore("app_Config", {
  state: () => ({
    isDark: true,
    isDarkId: "",
    appConfig: [] as Array<SettingMap>,
  }),
  actions: {
    insetId(id: string) {
      this.isDarkId = id;
    },
    changeTrue() {
      this.isDark = true;
    },
    changeFalse() {
      this.isDark = false;
    },
    setAppConfig(app_Config: Array<SettingMap>) {
      this.appConfig = app_Config;
    },
  },
  getters: {
    //計算屬性
    getIsDark: (state) => state.isDark,
    getisDarkId: (state) => state.isDarkId,
    getLanguage: (state) =>{
      let a:Array<SettingMap>=state.appConfig
      for(let i=0;i<a.length;i++){
        if(a[i].app_setting_item_name=="language"){
          return a[i].app_setting_item_value
        }
      }
    } 
  },
});
