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
import AppButton from '@/components/AppButton.vue'
import registerDirectives from '@/directives/index'
import errorHandler from './errorHandler'
import i18n from '../src/locales'
import { registerBuiltinPlugins } from '@/plugins'
import { appConfigInfo } from '@/store/modules/appConfigInfo'
import { pinia } from '@/store/index'
import { subscribe } from '@/core/runtime'
import { initTheme } from '@/composables/useDark'

async function initializeApp() {
  // 注册内置工具插件（插件工作区 ID 由各窗口加载工作区时通过 setPluginWorkspaceId 设置，不再用默认工作区兜底）
  registerBuiltinPlugins()

  const app = createApp(App)
  app.use(i18n)
  app.use(piniaStore)
  app.use(ElementPlus, {
    locale: zhCn
  })
  app.component('svg-icon', SvgIcon)
  app.component('AppButton', AppButton)
  for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
    app.component(key, component)
  }
  registerDirectives(app)

  app.config.errorHandler = (err, vm, info) => {
    errorHandler(err, vm, info)
  }

  // 挂载前恢复持久化设置（语言/主题等），避免首帧语言回退到默认值
  const appConfig = appConfigInfo(pinia)
  await appConfig.initializeApp()
  // 恢复主题偏好（KV 持久化），挂载前 await 避免首帧主题闪烁
  await initTheme()

  // 跨窗口语言同步：任一窗口修改语言后广播，所有窗口即时切换
  await subscribe<{ language: string }>('language-changed', (payload) => {
    if (payload?.language) {
      appConfig.setLanguage(payload.language)
    }
  })

  app.mount('#app')
}

initializeApp()
