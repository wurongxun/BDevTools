<template>
  <div id="app">
    <!-- 未登录 → 登录页 -->
    <LoginView v-if="!isLoggedIn" @login-success="handleLoginSuccess" />
    <!-- 子窗口 → 直接渲染对应组件，不走 Router -->
    <Common v-else-if="isChildWindow" :page-component="appPageComponent" :page-title="appPageTitle" />
    <!-- 主窗口 → 使用 Router -->
    <Layout v-else />
  </div>
</template>

<script lang="ts" setup>
import { ref, shallowRef, onMounted, defineAsyncComponent } from 'vue'
import { load, Store } from '@tauri-apps/plugin-store'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import useStore from '@/store'
import Layout from '@/Layout/index.vue'
import Common from '@/views/Common/Common.vue'
import LoginView from '@/views/login/index.vue'

const { useDocumentManagerStore, user } = useStore()

const isLoggedIn = ref(false)
const isChildWindow = ref(false)

// 子窗口直接渲染的页面组件
const appPageComponent = shallowRef<any>(null)
const appPageTitle = ref('')

// 页面组件映射（懒加载）
const pageComponentMap: Record<string, { title: string; loader: () => Promise<any> }> = {
  'App-setting': { title: '设置', loader: () => import('@/views/app/AppSetting.vue') },
  'App-about': { title: '关于', loader: () => import('@/views/app/AppAbout.vue') },
  'App-help': { title: '帮助', loader: () => import('@/views/app/AppHelp.vue') },
  'App-terminal': { title: '终端', loader: () => import('@/views/app/AppTerminal.vue') }
}

// 通过 Tauri 窗口标签判断窗口类型并加载对应组件
async function initWindow() {
  try {
    const win = getCurrentWebviewWindow()
    const label = await win.label
    // 主窗口标签为 'main'，子窗口标签为 'app-window-App-xxx'
    if (label.startsWith('app-window-')) {
      isChildWindow.value = true
      const pageKey = label.replace('app-window-', '')
      const pageConfig = pageComponentMap[pageKey]
      if (pageConfig) {
        appPageTitle.value = pageConfig.title
        appPageComponent.value = defineAsyncComponent(pageConfig.loader)
      }
    }
  } catch (_) {
    isChildWindow.value = false
  }
}

/**
 * 检查登录状态（从 Cookie 读取 token）
 */
function checkLoginState() {
  isLoggedIn.value = !!user.hasToken()
}

/**
 * 登录成功回调
 */
function handleLoginSuccess() {
  isLoggedIn.value = true
  initApp()
}

/**
 * 初始化应用数据
 */
async function initApp() {
  // 加载 store 中保存的文件路径
  let store: Store | undefined = undefined
  try {
    store = await load('openfile.json', {
      autoSave: false,
      defaults: {}
    })
    if (store) {
      const entries = await store.entries()
      for (const [, value] of entries) {
        const s = String(value)
        if (s) {
          useDocumentManagerStore.openDocument(s)
        }
      }
    }
  } catch (e) {
    console.warn('加载 openfile.json 失败:', e)
  }
}

onMounted(async () => {
  await initWindow()
  checkLoginState()
  if (isLoggedIn.value) {
    initApp()
  }
})
</script>
