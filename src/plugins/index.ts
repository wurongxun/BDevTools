/**
 * plugins/index.ts
 *
 * Automatically included in `./src/main.ts`
 */

// Plugins
import i18n from "../locales";
import { store } from "../store";
import router from "../router";
import ElementPlus from "element-plus";
import "element-plus/dist/index.css";
import "element-plus/theme-chalk/dark/css-vars.css";
import zhCn from "element-plus/es/locale/lang/zh-cn";
import "dayjs/locale/zh-cn"; //日期和时间本地化
import "dayjs/locale/zh-tw"; //日期和时间本地化
import contextmenu from "v-contextmenu";
import * as ElementPlusIconsVue from '@element-plus/icons-vue'

// Types
import type { App } from "vue";

export function registerPlugins(app: App) {
  for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component)
}
  // loadFonts() use(vuetify)
  //.use(i18n)
  app.use(i18n).use(router).use(store).use(contextmenu).use(ElementPlus, {
    locale: zhCn,
  });
}
