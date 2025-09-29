import { createApp } from 'vue'
import App from './App.vue'
import router from './router/index'
import piniaStore from '@/store/index'
import ElementPlus from 'element-plus'
import zhCn from 'element-plus/dist/locale/zh-cn.mjs'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import '../mock'
import './permission'
import 'element-plus/dist/index.css'
import 'normalize.css/normalize.css'
import '@/styles/index.scss'
import IndexUtils from '@/utils/index'
import SvgIcon from '@/components/SvgIcon.vue'
import registerDirectives from '@/directives/index'
import errorHandler from './errorHandler'
// import VuePdfEmbed from 'vue-pdf-embed'
import i18n from '../src/locales'
import useStore from '@/store'
async function initializeApp() {
  const app = createApp(App)
  app.use(i18n)
  app.use(router)
  app.use(piniaStore)
  app.use(ElementPlus, {
    locale: zhCn
  })
  // app.component('vue-pdf-embed', VuePdfEmbed)
  app.component('svg-icon', SvgIcon)
  // 注册指令
  for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
    app.component(key, component)
  }
  registerDirectives(app)
  // 挂载前初始化
  const { appConfig } = useStore()
  await appConfig.initializeApp()
  app.mount('#app')

  // 全局错误监听
  app.config.errorHandler = (err, vm, info) => {
    errorHandler(err, vm, info)
  }
}

initializeApp()
