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
import SvgIcon from '@/components/SvgIcon.vue'
import AppButton from '@/components/AppButton.vue'
import registerDirectives from '@/directives/index'
import errorHandler from './errorHandler'
import i18n from '../src/locales'
import { initPlugins } from '@/plugins'
import { appConfigInfo } from '@/store/modules/appConfigInfo'
import { pinia } from '@/store/index'
import { subscribe } from '@/core/runtime'
import { initWindowPresetsSync } from '@/core/windowPresets'
import { setHostThemeProvider } from '@/core/plugin/window'
import { initTheme, isDark } from '@/composables/useDark'

import ContextMenu from './components/context-menu/ContextMenu.vue'
import ContextMenuTrigger from './components/context-menu/ContextMenuTrigger.vue'
import { useContextMenu, showContextMenu, closeContextMenu } from './components/context-menu/useContextMenu'

export { ContextMenu, ContextMenuTrigger, useContextMenu, showContextMenu, closeContextMenu }

export type { MenuItem } from './components/context-menu/ContextMenu.vue'




async function initializeApp() {
  // 注册内置工具插件 + 加载第三方插件与统一排序（插件工作区 ID 由各窗口加载工作区时通过 setPluginWorkspaceId 设置，不再用默认工作区兜底）
  await initPlugins()

  const app = createApp(App)
  app.use(i18n)
  app.use(piniaStore)
  app.use(ElementPlus, {
    locale: zhCn
  })
  // 全局注册
  app.component('svg-icon', SvgIcon)
  app.component('AppButton', AppButton)
  app.component('ContextMenu', ContextMenu)
  app.component('ContextMenuTrigger', ContextMenuTrigger)
  for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
    app.component(key, component)
  }
  registerDirectives(app)

  app.config.errorHandler = (err) => {
    errorHandler(err)
  }

  // 挂载前恢复持久化设置（语言/主题等），避免首帧语言回退到默认值
  const appConfig = appConfigInfo(pinia)
  await appConfig.initializeApp()
  // 恢复主题偏好（KV 持久化），挂载前 await 避免首帧主题闪烁
  await initTheme()

  // 独立窗口型插件建窗时对齐宿主主题。core/ 不依赖 composables，故把取值函数注入进去
  // （与 initPluginHost(BUILTIN_PLUGINS) 同一思路）。只在建窗那一刻取值，宿主后来
  // 切主题不会同步到已打开的独立窗口（见 core/plugin/window.ts 文件头的「主题」一节）。
  setHostThemeProvider(() => (isDark.value ? 'dark' : 'light'))

  // 跨窗口语言同步：任一窗口修改语言后广播，所有窗口即时切换
  await subscribe<{ language: string }>('language-changed', (payload) => {
    if (payload?.language) {
      appConfig.setLanguage(payload.language)
    }
  })

  // 跨窗口窗口预设同步：预设改名后，由该预设打开的窗口同步自己的标题。
  // 删预设时「关哪些窗口」由发起方统一处理（见 core/windowPresets.ts），此处不关窗口。
  await initWindowPresetsSync()

  app.mount('#app')
}

initializeApp()
