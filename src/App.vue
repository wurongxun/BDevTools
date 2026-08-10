<template>
  <div id="app">
    <!-- 未登录 → 登录页 -->
    <LoginView v-if="!isLoggedIn" @login-success="handleLoginSuccess" />
    <!-- 辅助窗口（设置/关于/帮助/终端）→ 直接渲染对应组件，不走 Router -->
    <Common v-else-if="windowRole === 'auxiliary'" :page-component="appPageComponent" :page-title="appPageTitle" />
    <!-- 主窗口 / 编辑器窗口 → 使用 Layout -->
    <Layout v-else-if="isLoggedIn" />
  </div>
</template>

<script lang="ts" setup>
import { ref, shallowRef, onMounted, onBeforeUnmount, defineAsyncComponent } from 'vue'
import { load, Store } from '@tauri-apps/plugin-store'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Window } from '@tauri-apps/api/window'
import useStore from '@/store'
import Layout from '@/Layout/index.vue'
import Common from '@/views/Common/Common.vue'
import LoginView from '@/views/login/index.vue'
import type { WindowRole } from '@/Types/workspace'

const { useDocumentManagerStore, user, useWorkspaceStore, useSessionStore, routerNoteStore, useLayoutStore } = useStore()

const isLoggedIn = ref(false)
const windowRole = ref<WindowRole>('main')
/** 关闭拦截递归保护标志 */
const isClosing = ref(false)

// 辅助窗口直接渲染的页面组件
const appPageComponent = shallowRef<any>(null)
const appPageTitle = ref('')

// 页面组件映射（懒加载）
const pageComponentMap: Record<string, { title: string; loader: () => Promise<any> }> = {
  'App-setting': { title: '设置', loader: () => import('@/views/app/AppSetting.vue') },
  'App-about': { title: '关于', loader: () => import('@/views/app/AppAbout.vue') },
  'App-help': { title: '帮助', loader: () => import('@/views/app/AppHelp.vue') },
  'App-terminal': { title: '终端', loader: () => import('@/views/app/AppTerminal.vue') }
}

/**
 * 初始化窗口：根据窗口标签判断角色
 */
async function initWindow() {
  try {
    const win = getCurrentWebviewWindow()
    const label = await win.label

    if (label === 'main') {
      // 主窗口
      windowRole.value = 'main'
    } else if (label.startsWith('editor-')) {
      // 编辑器窗口
      windowRole.value = 'editor'
    } else if (label.startsWith('app-window-')) {
      // 辅助窗口
      windowRole.value = 'auxiliary'
      const pageKey = label.replace('app-window-', '')
      const pageConfig = pageComponentMap[pageKey]
      if (pageConfig) {
        appPageTitle.value = pageConfig.title
        appPageComponent.value = defineAsyncComponent(pageConfig.loader)
      }
    }
  } catch (_) {
    windowRole.value = 'main'
  }
}

/**
 * 初始化编辑器窗口：新窗口完全隔离，不加载旧窗口数据
 * 但会读取预设的布局配置（插件过滤、专注模式等）
 */
async function initEditorWindow() {
  if (windowRole.value !== 'editor') return

  // 读取布局配置（由创建窗口的调用方保存到 settings.json）
  try {
    const win = getCurrentWebviewWindow()
    const label = await win.label
    const settingsStore = await load('settings.json', { autoSave: false, defaults: {} })
    const layoutConfig = await settingsStore.get<{
      enabledPlugins: string[]
      focusMode: boolean
      presetId: string
    }>(`window_layout_${label}`)

    if (layoutConfig) {
      // 应用布局配置到当前窗口的 layout store
      useLayoutStore.applyPreset(layoutConfig)
      // 清理临时配置，避免重复读取
      await settingsStore.delete(`window_layout_${label}`)
      await settingsStore.save()
      console.log(`[App] 已应用布局配置: presetId=${layoutConfig.presetId}, focusMode=${layoutConfig.focusMode}`)
    }
  } catch (e) {
    console.warn('加载窗口布局配置失败:', e)
  }

  // 仅启动自动保存，用于保存当前窗口自身的状态
  useSessionStore.startAutoSave()
}

/**
 * 设置窗口关闭拦截：保存会话快照后真正关闭窗口
 */
function setupCloseHandler() {
  try {
    const win = getCurrentWebviewWindow() as unknown as Window
    win.onCloseRequested(async (event: any) => {
      // 辅助窗口不需要保存会话
      if (windowRole.value === 'auxiliary') return
      // 递归保护：destroy() 触发的二次关闭请求直接放行
      if (isClosing.value) return

      // 阻止默认关闭，等待保存完成
      event.preventDefault()
      isClosing.value = true

      try {
        const sessionStore = useSessionStore
        const docStore = useDocumentManagerStore

        // 保存会话快照
        await sessionStore.saveSnapshot()
        // 保存未保存的文档
        await docStore.saveAllDocuments()
      } catch (e) {
        console.warn('关闭前保存失败:', e)
      }

      // 保存完成，真正关闭窗口
      await win.destroy()
    })
  } catch (_) { /* 忽略 */ }
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
  const docStore = useDocumentManagerStore

  // 编辑器窗口：通过会话恢复加载文件
  if (windowRole.value === 'editor') {
    await initEditorWindow()
    return
  }

  // 主窗口：从 settings.json 初始化 per-window 工作目录
  try {
    const settingsStore = await load('settings.json', {
      autoSave: false,
      defaults: {}
    })
    const mdDir = await settingsStore.get<{ value: string }>('md_working_directory')
    if (mdDir?.value) {
      routerNoteStore.setWorkingDirectory(mdDir.value)
    }
  } catch (e) {
    console.warn('加载工作目录设置失败:', e)
  }

  // 主窗口：从 openfile.json 加载文件（向后兼容）
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
          docStore.openDocument(s)
        }
      }
    }
  } catch (e) {
    console.warn('加载 openfile.json 失败:', e)
  }

  // 主窗口加载默认工作区（Layout 挂载前就绪）
  await useWorkspaceStore.loadCurrentWorkspace(null)

  // 主窗口启动自动保存定时器（60s 定时备份）
  useSessionStore.startAutoSave()

  // 主窗口：加载布局配置（插件过滤、专注模式等）
  try {
    const settingsStore = await load('settings.json', { autoSave: false, defaults: {} })
    const layoutConfig = await settingsStore.get<{
      enabledPlugins: string[]
      focusMode: boolean
      presetId: string
    }>('window_layout_main')
    if (layoutConfig) {
      useLayoutStore.applyPreset(layoutConfig)
      console.log(`[App] 主窗口已应用布局配置: presetId=${layoutConfig.presetId}, focusMode=${layoutConfig.focusMode}`)
    }
  } catch (e) {
    console.warn('加载主窗口布局配置失败:', e)
  }
}

onMounted(async () => {
  await initWindow()
  checkLoginState()
  if (isLoggedIn.value) {
    await initApp()
  }
  // 设置窗口关闭拦截
  setupCloseHandler()
})

onBeforeUnmount(() => {
  useSessionStore.stopAutoSave()
})
</script>
