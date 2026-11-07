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
import AppStatusDot from '@/components/AppStatusDot.vue'
import registerDirectives from '@/directives/index'
import errorHandler from './errorHandler'
import i18n from '../src/locales'
import { initPlugins } from '@/plugins'
import { appConfigInfo } from '@/store/modules/appConfigInfo'
import { pinia } from '@/store/index'
import { subscribe } from '@/core/runtime'
import { initWindowPresetsSync } from '@/core/windowPresets'
import { setHostLocaleProvider, setHostThemeProvider } from '@/core/plugin/window'
import { initTheme, isDark } from '@/composables/useDark'
import { initHighContrast } from '@/composables/useHighContrast'
import { installNativeContextMenuGuard } from '@/core/nativeContextMenu'

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
  app.component('AppStatusDot', AppStatusDot)
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
  // 高对比度是与明暗正交的第二个开关，同样要在首帧前定下来（否则会闪一下常规配色）
  await initHighContrast()

  // 独立窗口型插件建窗时对齐宿主主题。core/ 不依赖 composables，故把取值函数注入进去
  // （与 initPluginHost(BUILTIN_PLUGINS) 同一思路）。只在建窗那一刻取值，宿主后来
  // 切主题不会同步到已打开的独立窗口（见 core/plugin/window.ts 文件头的「主题」一节）。
  setHostThemeProvider(() => (isDark.value ? 'dark' : 'light'))

  // 与上面同一思路，给独立窗口型插件注入宿主语言：那种窗口没有桥，语言只能拼进 URL 的
  // `?lang=` 作为初值（切语言不会同步到已打开的独立窗口）。注入线漏接时 URL 不带该参数，
  // 插件页应回落到自己的默认语言。
  setHostLocaleProvider(() => appConfig.language)

  // 跨窗口语言同步：任一窗口修改语言后广播，所有窗口即时切换
  await subscribe<{ language: string }>('language-changed', (payload) => {
    if (payload?.language) {
      appConfig.setLanguage(payload.language)
    }
  })

  // 跨窗口窗口预设同步：预设改名后，由该预设打开的窗口同步自己的标题。
  // 删预设时「关哪些窗口」由发起方统一处理（见 core/windowPresets.ts），此处不关窗口。
  await initWindowPresetsSync()

  // 去掉 WebView 的浏览器原生右键菜单（生产构建生效；dev 模式放行以保留 Inspect 入口）。
  // 必须在 mount 之前安装：越早注册，越不会出现「首帧右键还能弹出原生菜单」的窗口期。
  // 覆盖范围为宿主所有窗口（它们都是同一个 index.html + hash 路由）；第三方插件页是
  // 跨源沙箱 iframe，宿主够不到，不在覆盖范围内（详见 core/nativeContextMenu.ts 文件头）。
  installNativeContextMenuGuard()

  app.mount('#app')
}

initializeApp()
