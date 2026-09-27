<template>
  <el-config-provider :locale="epLocale">
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
        <app-button size="small" @click="tempConfirmAction = 'cancel'; showTempWorkspaceConfirm = false">取消</app-button>
        <app-button size="small" type="danger" plain @click="tempConfirmAction = 'discard'; showTempWorkspaceConfirm = false">不保存</app-button>
        <app-button size="small" type="primary" @click="tempConfirmAction = 'save'; showTempWorkspaceConfirm = false">保存</app-button>
      </template>
    </GlobalDialog>

    <!-- 草稿冲突：文件在关闭后被外部修改，逐个让用户决策 -->
    <GlobalDialog
      v-model="showDraftConflict"
      title="草稿冲突"
      width="460px"
    >
      <div v-if="currentConflict" class="temp-ws-message">
        文件在上次关闭后已被外部修改，与未保存草稿冲突：
        <div class="temp-ws-sub conflict-path">{{ currentConflict.path }}</div>
        <div class="temp-ws-sub">
          草稿时间：{{ formatTs(currentConflict.draftSavedAt) }}，
          文件修改：{{ formatTs(currentConflict.fileMtime * 1000) }}
          （剩余 {{ sessionConflicts.length }} 个）
        </div>
      </div>
      <template #footer>
        <app-button size="small" type="danger" plain @click="handleConflictAction('discard')">放弃草稿</app-button>
        <app-button size="small" @click="handleConflictAction('saveAs')">草稿另存为副本</app-button>
        <app-button size="small" type="primary" @click="handleConflictAction('restore')">恢复草稿</app-button>
      </template>
    </GlobalDialog>
    </div>
  </el-config-provider>
</template>

<script lang="ts" setup>
import { ref, shallowRef, computed, onMounted, onBeforeUnmount, defineAsyncComponent } from 'vue'
import { load, getCurrentWebviewWindow } from '@/core/runtime'
import type { Window } from '@/core/runtime'
import { useI18n } from 'vue-i18n'
import elZhCn from 'element-plus/es/locale/lang/zh-cn'
import elZhTw from 'element-plus/es/locale/lang/zh-tw'
import elEn from 'element-plus/es/locale/lang/en'
import useStore from '@/store'
import Layout from '@/Layout/index.vue'
import Common from '@/components/Common/Common.vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import WorkspaceSelectDialog from '@/components/WorkspaceSelectDialog.vue'
import type { WindowRole } from '@/Types/workspace'
import { useDocumentManagerStore } from '@/plugins/files/store/documents'
import { windowService } from '@/services/windowService'
import { workspaceService } from '@/services/workspaceService'
import NotificationUtil from '@/utils/NotificationUtil'
import type { DraftConflict } from '@/store/modules/session'

const { useWorkspaceStore, useSessionStore, fileTreeStore, useLayoutStore } = useStore()
const docStore = useDocumentManagerStore()

// ===== Element Plus 组件语言跟随 i18n（日期选择器/分页/弹窗按钮等内置文案） =====
const { locale } = useI18n()
const epLocale = computed(() => {
  const lang = locale.value
  if (lang === 'en') return elEn
  if (lang === 'zh_TW' || lang === 'zh_HK') return elZhTw
  return elZhCn
})

const windowRole = ref<WindowRole>('main')
/** 关闭拦截递归保护标志 */
const isClosing = ref(false)

// ===== 新建窗口：工作区选择弹窗 =====
const showWorkspaceSelect = ref(false)

// 编辑器窗口：启动后需弹窗选择工作区。
// 必须在 setup 阶段同步置位：父组件 onMounted 晚于子组件，
// 否则 MenuBar/AppStatusBar 的 onMounted 会先自动分配兜底工作区
try {
  const label = getCurrentWebviewWindow().label
  // 编辑器窗口与**主窗口**都支持「未绑定工作区 → 打开时选择」：必须在子组件挂载前
  // 置位 pending，否则 MenuBar / AppStatusBar 的 onMounted 会抢先自动分配兜底工作区。
  if (label.startsWith('editor-') || label === 'main') {
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

// ===== 草稿冲突（文件在关闭后被外部修改）=====
const sessionConflicts = ref<DraftConflict[]>([])
const showDraftConflict = computed(() => sessionConflicts.value.length > 0)
const currentConflict = computed(() => sessionConflicts.value[0] ?? null)

/** 时间戳格式化（弹窗展示用） */
function formatTs(ts: number): string {
  if (!ts) return '未知'
  return new Date(ts).toLocaleString('zh-CN', {
    month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit'
  })
}

/** 处理当前冲突项后推进队列 */
async function handleConflictAction(action: 'restore' | 'saveAs' | 'discard') {
  const conflict = currentConflict.value
  const wsId = useWorkspaceStore.currentWorkspace?.id
  if (!conflict || !wsId) {
    sessionConflicts.value = []
    return
  }
  await useSessionStore.resolveDraftConflict(wsId, conflict, action)
  sessionConflicts.value = sessionConflicts.value.slice(1)
}

/** 窗口工作区就绪后尝试恢复会话，冲突交给弹窗处理 */
async function restoreSessionIfAny() {
  const wsId = useWorkspaceStore.currentWorkspace?.id
  if (!wsId) return
  const conflicts = await useSessionStore.tryRestoreSession(wsId).catch((e) => {
    console.warn('会话恢复失败:', e)
    return [] as DraftConflict[]
  })
  if (conflicts.length > 0) {
    sessionConflicts.value = conflicts
    NotificationUtil.warn(`已恢复上次会话，${conflicts.length} 个文件的草稿与外部修改冲突，请处理`)
  }
}

// 辅助窗口直接渲染的页面组件
const appPageComponent = shallowRef<any>(null)
const appPageTitle = ref('')

// 页面组件映射（懒加载）
const pageComponentMap: Record<string, { title: string; loader: () => Promise<any> }> = {
  'App-setting': { title: '设置', loader: () => import('@/plugins/settings/app/AppSetting.vue') },
  'App-about': { title: '关于', loader: () => import('@/plugins/settings/app/AppAbout.vue') },
  'App-help': { title: '帮助', loader: () => import('@/plugins/settings/app/AppHelp.vue') }
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
  let layoutWorkspaceId = ''
  try {
    const win = getCurrentWebviewWindow()
    const label = await win.label
    const settingsStore = await load('settings.json', { autoSave: false, defaults: {} })
    const layoutConfig = await settingsStore.get<{
      enabledPlugins: string[]
      focusMode: boolean
      presetId: string
      workspaceId?: string
    }>(`window_layout_${label}`)

    if (layoutConfig) {
      // 应用布局配置到当前窗口的 layout store
      useLayoutStore.applyPreset(layoutConfig)
      layoutWorkspaceId = layoutConfig.workspaceId || ''
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

  // 优先从注册表 meta 读取创建时绑定的工作区（比布局 JSON 更权威）
  let boundMetaWorkspaceId = ''
  try {
    const win = getCurrentWebviewWindow()
    const list = await windowService.getWindowList()
    const meta = list.find((m) => m.label === win.label)
    boundMetaWorkspaceId = meta?.workspace_id ?? ''
  } catch (e) {
    console.warn('读取窗口绑定工作区失败:', e)
  }

  // 已绑定工作区（注册表 meta 或布局配置）：未被占用则直接加载，不弹选择窗；
  // 已被占用（同一工作区不能同时打开）：提示后降级弹窗选择
  const boundWorkspaceId = boundMetaWorkspaceId || layoutWorkspaceId
  if (boundWorkspaceId) {
    const holder = await workspaceService.lockStatus(boundWorkspaceId).catch(() => null)
    if (!holder) {
      useWorkspaceStore.pendingWorkspaceSelection = false
      await useWorkspaceStore.loadCurrentWorkspace(boundWorkspaceId)
      if (useWorkspaceStore.currentWorkspace) {
        // 工作区就绪：恢复上次会话（含草稿指纹校验）
        await restoreSessionIfAny()
        return
      }
    } else {
      NotificationUtil.warn(`绑定的工作区已在窗口 [${holder}] 中打开，请重新选择`)
    }
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
  if (result.success) {
    // 切换到的工作区可能存在上次会话（含草稿）
    await restoreSessionIfAny()
  } else if (!useWorkspaceStore.currentWorkspace) {
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

        // 保存会话快照（脏文档内容同步写入草稿）
        await sessionStore.saveSnapshot()
        // 保存未保存的文档（内容落盘后不再需要草稿）
        await docStore.saveAllDocuments()
        // 清理已落盘文档的草稿，避免堆积；未保存文档的草稿保留接续保护
        const wsId = useWorkspaceStore.currentWorkspace?.id
        if (wsId) {
          await sessionStore.cleanupSavedDrafts(wsId)
        }
      } catch (e) {
        console.warn('关闭前保存失败:', e)
      }

      // 保存完成，真正关闭窗口。
      //
      // 主窗口例外：它的「关闭」＝**隐藏到托盘**（close-to-tray），之后靠托盘图标 / 菜单
      // 「显示窗口」唤回（Rust 侧同样拦截它的 CloseRequested，见
      // `window_manager::main_window::should_hide_instead_of_close`）。
      // 这里必须走 hide() 而不是 destroy()：destroy() **绕过** CloseRequested，会真把主窗口
      // 销毁掉，只剩「托盘重建」这条兜底路径可走。
      if (windowRole.value === 'main') {
        await win.hide()
        // 复位拦截标志：下次再点关闭仍要走「保存会话 → 隐藏」这一整套
        isClosing.value = false
        return
      }

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

  // 主窗口：从 openfile.json 恢复「上次未关闭的 tab」（失效记录会在恢复时清理）。
  // 注意：这里只还原未关闭的 tab，不是「最近打开」列表；「最近打开」是用户主动打开的历史，
  // 不参与启动恢复（详见 docStore.restoreOpenFiles / addRecentFile 的注释）。
  await docStore.restoreOpenFiles()

  // 主窗口：读取主窗口预设绑定的工作区（主窗口现在也可绑定；未绑定则「打开时选择」）
  let mainBoundWorkspaceId = ''
  try {
    const wsSettings = await load('settings.json', { autoSave: false, defaults: {} })
    const mainVal = await wsSettings.get<{ workspaceId?: string }>('main_window_preset')
    if (mainVal?.workspaceId) mainBoundWorkspaceId = mainVal.workspaceId
  } catch (e) {
    console.warn('读取主窗口预设失败:', e)
  }

  let mainWorkspaceResolved = false
  if (mainBoundWorkspaceId) {
    const holder = await workspaceService.lockStatus(mainBoundWorkspaceId).catch(() => null)
    if (!holder) {
      await useWorkspaceStore.loadCurrentWorkspace(mainBoundWorkspaceId)
      if (useWorkspaceStore.currentWorkspace) mainWorkspaceResolved = true
    } else {
      NotificationUtil.warn(`绑定的工作区已在窗口 [${holder}] 中打开，请重新选择`)
    }
  }
  // 已绑定且成功加载 / 未绑定 / 被占用 ⇒ 打开时选择工作区
  if (!mainWorkspaceResolved) {
    useWorkspaceStore.pendingWorkspaceSelection = true
    showWorkspaceSelect.value = true
  } else {
    useWorkspaceStore.pendingWorkspaceSelection = false
  }

  // 主窗口：恢复上次会话（含草稿指纹校验）
  await restoreSessionIfAny()

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

.conflict-path {
  word-break: break-all;
  font-size: var(--font-size-base);
}

//#app {
//  border-radius: var(--radius-xl);
//  box-shadow: 0 1px 5px rgba(0,0,0,0.15);
//  overflow: hidden;
//}
</style>
