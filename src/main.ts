import { createApp } from 'vue'
import App from './App.vue'
import piniaStore from '@/store/index'
import router from '@/router'
import ElementPlus from 'element-plus'
import zhCn from 'element-plus/dist/locale/zh-cn.mjs'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import '../mock'
import 'element-plus/dist/index.css'
import 'normalize.css/normalize.css'
import '@/styles/index.scss'
import IndexUtils from '@/utils/index'
import SvgIcon from '@/components/SvgIcon.vue'
import registerDirectives from '@/directives/index'
import errorHandler from './errorHandler'
import i18n from '../src/locales'
import { registerBuiltinPlugins } from '@/plugins'
import '@/permission'

async function initializeApp() {
  // 注册内置工具插件
  registerBuiltinPlugins()

  const app = createApp(App)
  app.use(i18n)
  app.use(piniaStore)
  app.use(router)
  app.use(ElementPlus, {
    locale: zhCn
  })
  app.component('svg-icon', SvgIcon)
  for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
    app.component(key, component)
  }
  registerDirectives(app)

  app.config.errorHandler = (err, vm, info) => {
    errorHandler(err, vm, info)
  }

  app.mount('#app')
}

initializeApp()
