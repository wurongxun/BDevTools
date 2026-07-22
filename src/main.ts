import { createApp } from 'vue'
import App from './App.vue'
import piniaStore from '@/store/index'
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
import useStore from '@/store'

function isChildWindow(): boolean {
  const hash = window.location.hash
  return hash.startsWith('#/App')
}

async function initializeApp() {
  const app = createApp(App)
  app.use(i18n)

  if (isChildWindow()) {
    const { default: childRouter } = await import('./router/childRouter')
    app.use(childRouter)
  } else {
    const { default: router } = await import('./router/index')
    app.use(router)
    await import('./permission')
  }

  app.use(piniaStore)
  app.use(ElementPlus, {
    locale: zhCn
  })
  app.component('svg-icon', SvgIcon)
  for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
    app.component(key, component)
  }
  registerDirectives(app)

  const { appConfig } = useStore()
  await appConfig.initializeApp()
  app.mount('#app')

  app.config.errorHandler = (err, vm, info) => {
    errorHandler(err, vm, info)
  }
}

initializeApp()
