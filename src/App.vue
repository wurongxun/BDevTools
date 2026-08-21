<template>
  <div id="app">
    <!-- 辅助窗口（设置/关于/帮助/终端）→ 直接渲染对应组件，不走 Router -->
    <Common v-if="windowRole === 'auxiliary'" :page-component="appPageComponent" :page-title="appPageTitle" />
    <!-- 主窗口 / 编辑器窗口 → 使用 Layout -->
    <Layout v-else />

    <!-- 新建编辑器窗口：选择工作区或新建临时工作区 -->
    <WorkspaceSelectDialog
      v-model="showWorkspaceSelect"
      @select="handleWorkspaceSelected"
      @create-temp="handleCreateTempWorkspace"
      @close="handleWorkspaceSelectClosed"
    />

    <!-- 关闭确认：临时工作区是否保留 -->
    <GlobalDialog
      v-model="showTempWorkspaceConfirm"
      title="临时工作区"
      width="400px"
      @close="handleTempConfirmClosed"
    >
      <div class="temp-ws-message">
        当前窗口使用的是临时工作区「{{ tempWorkspaceName }}」，关闭窗口前是否保存该工作区？
        <div class="temp-ws-sub">不保存将在关闭后清除。</div>
      </div>
      <template #footer>
        <el-button size="small" @click="tempConfirmAction = 'cancel'; showTempWorkspaceConfirm = false">取消</el-button>
        <el-button size="small" type="danger" plain @click="tempConfirmAction = 'discard'; showTempWorkspaceConfirm = false">不保存</el-button>
        <el-button size="small" type="primary" @click="tempConfirmAction = 'save'; showTempWorkspaceConfirm = false">保存</el-button>
      </template>
    </GlobalDialog>
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
import GlobalDialog from '@/components/GlobalDialog.vue'
import WorkspaceSelectDialog from '@/components/WorkspaceSelectDialog.vue'
import type { WindowRole } from '@/Types/workspace'
import { useDocumentManagerStore } from '@/plugins/files/store/documents'
import { windowService } from '@/services/windowService'

const { useWorkspaceStore, useSessionStore, fileTreeStore, useLayoutStore } = useStore()
const docStore = useDocumentManagerStore()

const windowRole = ref<WindowRole>('main')
/** 关闭拦截递归保护标志 */
const isClosing = ref(false)

// ===== 新建窗口：工作区选择弹窗 =====
const showWorkspaceSelect = ref(false)

// 编辑器窗口：启动后需弹窗选择工作区。
// 必须在 setup 阶段同步置位：父组件 onMounted 晚于子组件，
// 否则 MenuBar/AppStatusBar 的 onMounted 会先自动分配兜底工作区
try {
  if (getCurrentWebviewWindow().label.startsWith('editor-')) {
    useWorkspaceStore.pendingWorkspaceSelection = true
  }
} catch { /* 非 Tauri 环境忽略 */ }
// ===== 关闭确认：临时工作区保留/清除 =====
const showTempWorkspaceConfirm = ref(false)
/** 用户在临时工作区确认弹窗中的选择（save/discard/cancel） */
const tempConfirmAction = ref<'save' | 'discard' | 'cancel'>('cancel')
const tempWorkspaceName = ref('')
/** 确认弹窗关闭后待执行的收尾动作 */
let tempConfirmResolve: ((action: 'save' | 'discard' | 'cancel') => void) | null = null

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

  // 创建时已绑定工作区（注册表 meta）：直接加载，不弹选择窗
  try {
    const win = getCurrentWebviewWindow()
    const list = await windowService.getWindowList()
    const meta = list.find((m) => m.label === win.label)
    if (meta?.workspace_id) {
      useWorkspaceStore.pendingWorkspaceSelection = false
      await useWorkspaceStore.loadCurrentWorkspace(meta.workspace_id)
      if (useWorkspaceStore.currentWorkspace) return
    }
  } catch (e) {
    console.warn('读取窗口绑定工作区失败:', e)
  }

  // 未绑定工作区：弹窗让用户选择或新建临时工作区（pending 标志已在 setup 阶段置位）
  if (!useWorkspaceStore.currentWorkspace) {
    showWorkspaceSelect.value = true
  } else {
    useWorkspaceStore.pendingWorkspaceSelection = false
  }
}

/** 选中已有工作区：绑定到当前窗口（内含锁保护） */
async function handleWorkspaceSelected(workspaceId: string) {
  useWorkspaceStore.pendingWorkspaceSelection = false
  const result = await useWorkspaceStore.switchWorkspace(workspaceId)
  if (!result.success && !useWorkspaceStore.currentWorkspace) {
    // 锁定失败且当前无工作区，兜底创建临时工作区
    await useWorkspaceStore.createTempWorkspace()
  }
}

/** 新建临时工作区并绑定到当前窗口 */
async function handleCreateTempWorkspace() {
  useWorkspaceStore.pendingWorkspaceSelection = false
  await useWorkspaceStore.createTempWorkspace()
}

/** 选择弹窗被直接关闭（未选择）：兜底创建临时工作区，保证窗口有工作区可用 */
async function handleWorkspaceSelectClosed() {
  if (useWorkspaceStore.pendingWorkspaceSelection) {
    useWorkspaceStore.pendingWorkspaceSelection = false
    if (!useWorkspaceStore.currentWorkspace) {
      await useWorkspaceStore.createTempWorkspace()
    }
  }
}

/** 以 Promise 形式等待用户在临时工作区确认弹窗中作出选择 */
function askTempWorkspaceConfirm(name: string): Promise<'save' | 'discard' | 'cancel'> {
  return new Promise((resolve) => {
    tempWorkspaceName.value = name
    tempConfirmAction.value = 'cancel'
    tempConfirmResolve = resolve
    showTempWorkspaceConfirm.value = true
  })
}

/** 确认弹窗关闭（按钮或右上角 ✕）：兑现 Promise */
function handleTempConfirmClosed() {
  const resolve = tempConfirmResolve
  tempConfirmResolve = null
  resolve?.(tempConfirmAction.value)
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

      // 新窗口尚在工作区选择阶段且未绑定工作区：无状态可保存，直接关闭
      if (useWorkspaceStore.pendingWorkspaceSelection && !useWorkspaceStore.currentWorkspace) {
        await win.destroy()
        return
      }

      // 临时工作区：关闭前询问是否保存，不保存则清除
      if (useWorkspaceStore.isTemporary(useWorkspaceStore.currentWorkspace)) {
        const tempWs = useWorkspaceStore.currentWorkspace!
        const action = await askTempWorkspaceConfirm(tempWs.name)
        if (action === 'cancel') {
          // 取消关闭：复位拦截标志，放行后续关闭请求
          isClosing.value = false
          return
        }
        if (action === 'save') {
          await useWorkspaceStore.saveTempWorkspace(tempWs.id)
        } else {
          // 不保存：清除临时工作区（先置空再删，避免快照/监听引用已删工作区）
          await useWorkspaceStore.deleteWorkspace(tempWs.id)
        }
      }

      try {
        const sessionStore = useSessionStore

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
 * 初始化应用数据
 */
async function initApp() {
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
      fileTreeStore.setWorkingDirectory(mdDir.value)
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
  await initApp()
  // 设置窗口关闭拦截
  setupCloseHandler()
})

onBeforeUnmount(() => {
  useSessionStore.stopAutoSave()
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.temp-ws-message {
  line-height: 1.6;
}

.temp-ws-sub {
  margin-top: 6px;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}
</style>
