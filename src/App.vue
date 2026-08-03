<template>
  <div id="app">
    <!-- 未登录 → 登录页 -->
    <LoginView v-if="!isLoggedIn" @login-success="handleLoginSuccess" />
    <!-- 已登录 → 根据路由判断渲染组件 -->
    <Common v-else-if="isAppWindow" />
    <Layout v-else />
  </div>
</template>

<script lang="ts" setup>
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { load, Store } from '@tauri-apps/plugin-store'
import useStore from '@/store'
import Layout from '@/Layout/index.vue'
import Common from '@/views/Common/Common.vue'
import LoginView from '@/views/login/index.vue'

const route = useRoute()
const { useDocumentManagerStore, user } = useStore()

const isLoggedIn = ref(false)

// 判断当前路由是否为 App 窗口路由
const isAppWindow = computed(() => {
  const path = route.path
  return path === '/App/setting' || path === '/App/about' || path === '/App/help' || path === '/App/terminal'
})

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

onMounted(() => {
  checkLoginState()
  if (isLoggedIn.value) {
    initApp()
  }
})
</script>
