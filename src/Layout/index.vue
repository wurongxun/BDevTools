<template>
  <div class="main-box">
    <AppNavbar />
    <div class="main-area" :class="{ 'is-resizing': isDragging }">
      <aside ref="sideBoxRef" class="side-box" :class="{ 'is-collapse': isCollapse }"
        :style="!isCollapse ? { width: sidebarWidth + 'px', minWidth: sidebarWidth + 'px', maxWidth: sidebarWidth + 'px', transition: isDragging ? 'none' : '' } : {}">
        <app-sidebar :hideActivityBar="isFocusMode" />
      </aside>
      <div v-show="!isCollapse && !useLayoutStore.sidebarPanelCollapsed" class="resize-handle" :class="{ 'is-dragging': isDragging }" @mousedown="onResizeStart" role="separator" aria-orientation="vertical" tabindex="0" :aria-valuenow="sidebarWidth" :aria-valuemin="activeSidebarCfg.min" :aria-valuemax="activeSidebarCfg.max" aria-label="调整侧边栏宽度" @keydown="onResizeKeydown">
        <div class="resize-handle-line"></div>
      </div>
      <main ref="mainContentRef" class="app-content"
        :class="{ 'is-collapse': isCollapse || isFocusMode, 'has-sidebar': !isCollapse && !isFocusMode && windowWidth >= 1024 }">
        <div class="editor-wrapper">
          <app-editor-main></app-editor-main>
        </div>
      </main>
    </div>
    <AppStatusBar />
  </div>
</template>

<script setup lang="ts" name="Layout">
import AppNavbar from './components/AppNavbar/index.vue'
import AppEditorMain from './components/AppEditorMain/index.vue'
import AppSidebar from './components/AppSidebar/index.vue'
import AppStatusBar from './components/AppStatusBar/index.vue'
import useStore from '@/store/index'
import { pluginManager } from '@/core/pluginManager'
import { storeToRefs } from 'pinia'
import { watch, nextTick, onMounted, onUnmounted, ref, reactive, computed } from 'vue'
import { useResizeObserver } from '@vueuse/core'

const { system, useWorkspaceStore, useLayoutStore } = useStore()
const { isCollapse } = storeToRefs(system)

/** 当前窗口是否处于专注模式 */
const isFocusMode = computed(() => useLayoutStore.focusMode)

// const isEditorRoute = computed(() => {
//   return !route.path.startsWith('/App')
// })





const sideBoxRef = ref<HTMLElement | null>(null)
const mainContentRef = ref<HTMLElement | null>(null)
const windowWidth = ref(window.innerWidth)

const SIDEBAR_MIN_WIDTH = 60
const SIDEBAR_MAX_WIDTH = 700
const SIDEBAR_DEFAULT_WIDTH = 220
/** 活动栏（activity-bar）固定宽度：插件面板收起时侧边栏收缩到该宽度 */
const ACTIVITY_BAR_WIDTH = 48

/** 当前激活插件的侧边栏布局配置（插件可声明 sidebarMinWidth/MaxWidth/DefaultWidth，未声明用全局默认） */
const activeSidebarCfg = computed(() => {
  const plugin = pluginManager.getPlugin(pluginManager.activeToolId.value)
  return {
    min: plugin?.sidebarMinWidth ?? SIDEBAR_MIN_WIDTH,
    max: plugin?.sidebarMaxWidth ?? SIDEBAR_MAX_WIDTH,
    def: plugin?.sidebarDefaultWidth ?? SIDEBAR_DEFAULT_WIDTH,
  }
})

/** 每个插件独立的侧边栏宽度（持久化存储） */
const pluginSidebarWidths: Record<string, number> = {}

/** 当前侧边栏宽度（直接驱动 DOM） */
const sidebarWidth = ref(SIDEBAR_DEFAULT_WIDTH)

/** 插件切换时恢复该插件记住的宽度：无动画、宽度相同则跳过；切换后即时通知一次布局更新（消除延迟链） */
watch(() => pluginManager.activeToolId.value, async (newId) => {
  // 插件面板处于收起状态：保持活动栏宽度，不恢复插件记住的宽度
  if (useLayoutStore.sidebarPanelCollapsed) {
    sidebarWidth.value = ACTIVITY_BAR_WIDTH
    await nextTick()
    triggerLayoutUpdate('plugin-switch')
    return
  }
  const target = pluginSidebarWidths[newId] ?? activeSidebarCfg.value.def
  if (sidebarWidth.value !== target) {
    const asideEl = sideBoxRef.value
    // 无动画恢复，避免 300ms transition 造成「切换后布局慢慢变化」的滞后感
    asideEl?.classList.add('no-transition')
    sidebarWidth.value = target
    requestAnimationFrame(() => asideEl?.classList.remove('no-transition'))
  }
  // 切换后立即（非 setTimeout 延迟）通知子组件按新容器重新测量布局（Monaco/xterm 等）
  await nextTick()
  triggerLayoutUpdate('plugin-switch')
})

/** 插件面板收起/展开：收起时缩到活动栏宽度，展开时恢复该插件记住的宽度 */
watch(() => useLayoutStore.sidebarPanelCollapsed, async (collapsed) => {
  const activeId = pluginManager.activeToolId.value
  if (collapsed) {
    // 记住收起前的宽度，展开时恢复
    pluginSidebarWidths[activeId] = sidebarWidth.value
    sidebarWidth.value = ACTIVITY_BAR_WIDTH
  } else {
    sidebarWidth.value = pluginSidebarWidths[activeId] ?? activeSidebarCfg.value.def
  }
  await nextTick()
  triggerLayoutUpdate('panel-toggle')
})
const isDragging = ref(false)

const onResizeStart = (e: MouseEvent) => {
  e.preventDefault()
  isDragging.value = true
  const startX = e.clientX
  const startWidth = sidebarWidth.value
  const asideEl = sideBoxRef.value
  if (!asideEl) return
  // 拖拽钳制范围取当前激活插件的声明值
  const { min, max } = activeSidebarCfg.value

  let currentWidth = startWidth

  const onMouseMove = (moveEvent: MouseEvent) => {
    currentWidth = Math.min(max, Math.max(min, startWidth + (moveEvent.clientX - startX)))
    // 拖拽期间只操作 DOM，不触发 Vue 响应式更新
    asideEl.style.width = currentWidth + 'px'
    asideEl.style.minWidth = currentWidth + 'px'
    asideEl.style.maxWidth = currentWidth + 'px'
  }

  const onMouseUp = () => {
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    // 同步更新 ref，Vue 重渲染后 :style 绑定会接管宽度
    sidebarWidth.value = currentWidth
    pluginSidebarWidths[pluginManager.activeToolId.value] = currentWidth
    isDragging.value = false
    triggerLayoutUpdate('drag-resize')
  }

  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}

let prevWindowWidth = window.innerWidth

// 触发自定义事件的辅助函数
const triggerLayoutUpdate = (reason: string, collapseState?: boolean) => {
  const event = new CustomEvent('layout-update', {
    detail: {
      isCollapse: collapseState !== undefined ? collapseState : isCollapse.value,
      timestamp: Date.now(),
      reason: reason,
      sidebarWidth: sideBoxRef.value?.offsetWidth || 0,
      mainWidth: mainContentRef.value?.offsetWidth || 0,
      windowWidth: windowWidth.value
    }
  })
  window.dispatchEvent(event)
}

// 监听窗口大小变化，跨过 720px 阈值时自动折叠/展开侧边栏（仅按方向切换，避免缩放抖动）
const handleWindowResize = () => {
  const w = window.innerWidth
  windowWidth.value = w
  if (w < 720 && !isCollapse.value && prevWindowWidth >= 720) {
    system.packCollapse()
  } else if (w >= 720 && isCollapse.value && prevWindowWidth < 720) {
    system.expandCollapse()
  }
  prevWindowWidth = w

  // 触发重新计算
  setTimeout(() => {
    triggerLayoutUpdate('window-resize')
  }, 100)
}

/** 键盘调节侧边栏宽度（WCAG 2.1.1/2.1.2）：分隔条聚焦后 ← → ±16px，Home/End 跳到最小/最大 */
const onResizeKeydown = (e: KeyboardEvent) => {
  const { min, max } = activeSidebarCfg.value
  let next = sidebarWidth.value
  if (e.key === 'ArrowLeft') next = Math.max(min, sidebarWidth.value - 16)
  else if (e.key === 'ArrowRight') next = Math.min(max, sidebarWidth.value + 16)
  else if (e.key === 'Home') next = min
  else if (e.key === 'End') next = max
  else return
  e.preventDefault()
  sidebarWidth.value = next
  pluginSidebarWidths[pluginManager.activeToolId.value] = next
  triggerLayoutUpdate('keyboard-resize')
}

// 监听侧边栏折叠状态变化
watch(isCollapse, async (newValue) => {
  await nextTick()

  // 等待过渡动画完成后再触发事件
  setTimeout(() => {
    triggerLayoutUpdate('collapse-change', newValue)
  }, newValue ? 400 : 350)
})

// 使用 ResizeObserver 监听侧边栏实际宽度变化
useResizeObserver(sideBoxRef, async (entries) => {
  if (isDragging.value) return
  for (const entry of entries) {
    if (entry.target === sideBoxRef.value) {
      await nextTick()

      setTimeout(() => {
        triggerLayoutUpdate('sidebar-resize')
      }, 100)

      console.log('[Layout] Sidebar width changed:', entry.contentRect.width)
      break
    }
  }
})

// 组件挂载时触发初始化事件并设置初始状态
onMounted(async () => {
  await nextTick()

  // 加载工作区列表（用于菜单显示，currentWorkspace 由 App.vue 负责加载）
  await useWorkspaceStore.loadAllWorkspaces()

  // 初始窗口宽度小于 720px 时自动隐藏侧边栏，跨过阈值再展开
  if (windowWidth.value < 720 && !isCollapse.value) {
    system.packCollapse()
  } else if (windowWidth.value >= 720 && isCollapse.value) {
    system.expandCollapse()
  }

  setTimeout(() => {
    triggerLayoutUpdate('initial-mount')
  }, 200)

  // 监听窗口大小变化（仅保留一个 resize 处理，避免双份监听器互相覆盖，审查报告 #33）
  window.addEventListener('resize', handleWindowResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleWindowResize)
})
</script>


<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss';

.main-box {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;
  box-sizing: border-box;

  .main-area {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;

    &.is-resizing {

      .side-box,
      .app-content {
        transition: none !important;
      }
    }

    .side-box {
      flex-shrink: 0;
      height: 100%;
      transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1),
        min-width 0.3s cubic-bezier(0.4, 0, 0.2, 1),
        max-width 0.3s cubic-bezier(0.4, 0, 0.2, 1),
        opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      z-index: 1000;
      box-sizing: border-box;
      overflow: hidden;

      // 插件切换时无动画恢复宽度（避免切换后布局慢慢变化）
      &.no-transition {
        transition: none !important;
      }

      @include theme.useTheme {
        background-color: theme.getVar('bgColor');
        border-right: none;
      }

      &.is-collapse {
        width: 0;
        min-width: 0;
        max-width: 0;
        opacity: 0;
      }
    }

    .resize-handle {
      flex-shrink: 0;
      width: var(--resize-handle-v-line-width);
      cursor: col-resize;
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 1001;
      position: relative;
      background-color: var(--resize-handle-color);
      transition: background-color 0.2s;

      // 扩大命中热区（透明、不占布局）：1px 的线太难精确点中，
      // 向左右各扩 hit-area 让鼠标更容易触发 hover / mousedown。
      &::before {
        content: '';
        position: absolute;
        top: 0;
        bottom: 0;
        left: calc(-1 * var(--resize-handle-hit-area));
        right: calc(-1 * var(--resize-handle-hit-area));
      }

      &:hover,
      &.is-dragging {
        background-color: var(--resize-handle-color-active);
      }

      .resize-handle-line {
        width: var(--resize-handle-v-line-width);
        height: var(--resize-handle-v-line-height);
        border-radius: var(--radius-xs);
        background-color: var(--resize-handle-color-active);
        opacity: 0;
        transition: width 0.2s ease, opacity 0.2s ease;
      }

      // 悬停 / 拖拽中：中线增粗（1px → active 宽度）并显现，提示「此处可拖拽」
      &:hover .resize-handle-line,
      &.is-dragging .resize-handle-line {
        width: var(--resize-handle-v-line-width-active);
        opacity: 1;
      }
    }

    .app-content {
      flex: 1;
      min-width: 0;
      display: flex;
      flex-direction: column;
      height: 100%;
      overflow: hidden;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      box-sizing: border-box;

      &.has-sidebar {
        margin-left: 0;
      }

      .editor-wrapper,
      .page-wrapper {
        flex: 1;
        min-height: 0;
        display: flex;
        flex-direction: column;
        height: 100%;
        overflow: hidden;
      }
    }
  }
}
</style>