<template>
  <div class="main-box">
    <AppNavbar />
    <div class="main-area" :class="{ 'is-resizing': isDragging }">
      <aside
        ref="sideBoxRef"
        class="side-box"
        :class="{ 'is-collapse': isCollapse }"
        :style="!isCollapse ? { width: sidebarWidth + 'px', minWidth: sidebarWidth + 'px', maxWidth: sidebarWidth + 'px' } : {}"
      >
        <app-sidebar :hideActivityBar="isFocusMode" />
      </aside>
      <div
        v-show="!isCollapse"
        class="resize-handle"
        :class="{ 'is-dragging': isDragging }"
        @mousedown="onResizeStart"
      >
        <div class="resize-handle-line"></div>
      </div>
      <main ref="mainContentRef" class="app-content" :class="{ 'is-collapse': isCollapse || isFocusMode, 'has-sidebar': !isCollapse && !isFocusMode && windowWidth >= 1024 }">
        <div class="editor-wrapper">
          <app-editor-main></app-editor-main>
        </div>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts" name="Layout">
import AppNavbar from './components/AppNavbar/index.vue'
import AppEditorMain from './components/AppEditorMain/index.vue'
import AppHeader from './components/AppHeader/index.vue'
import AppSidebar from './components/AppSidebar/index.vue'
import useStore from '@/store/index'
import { storeToRefs } from 'pinia'
import { watch, nextTick, onMounted, onUnmounted, ref, computed } from 'vue'
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

const SIDEBAR_MIN_WIDTH =60
const SIDEBAR_MAX_WIDTH = 700
const SIDEBAR_DEFAULT_WIDTH = 220

const sidebarWidth = ref(SIDEBAR_DEFAULT_WIDTH)
const isDragging = ref(false)

const onResizeStart = (e: MouseEvent) => {
  e.preventDefault()
  isDragging.value = true
  const startX = e.clientX
  const startWidth = sidebarWidth.value
  const asideEl = sideBoxRef.value
  if (!asideEl) return

  asideEl.style.transition = 'none'

  let rafId: number | null = null
  let pendingWidth = startWidth

  const onMouseMove = (moveEvent: MouseEvent) => {
    const delta = moveEvent.clientX - startX
    pendingWidth = Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, startWidth + delta))
    if (rafId === null) {
      rafId = requestAnimationFrame(() => {
        asideEl.style.width = pendingWidth + 'px'
        asideEl.style.minWidth = pendingWidth + 'px'
        asideEl.style.maxWidth = pendingWidth + 'px'
        rafId = null
      })
    }
  }

  const onMouseUp = () => {
    if (rafId !== null) {
      cancelAnimationFrame(rafId)
    }
    sidebarWidth.value = pendingWidth
    asideEl.style.transition = ''
    isDragging.value = false
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    triggerLayoutUpdate('drag-resize')
  }

  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}

let windowWidth_2 = ref<number>(window.innerWidth)
let lastWindowWidth = ref<number>(window.innerWidth)
let windowWidthshrink = ref<Boolean>(false)

const handleResize=() => {
  updateWindowWidth();
}
const updateWindowWidth=() => {
  windowWidth_2.value = window.innerWidth;
  if (windowWidth_2.value < lastWindowWidth.value) {
    console.log('窗口宽度在变小');
    windowWidthshrink.value = true;
  } else {
    console.log('窗口宽度在变大或保持不变')
    windowWidthshrink.value = false;
  }
  lastWindowWidth.value = windowWidth_2.value;
}

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
  console.log('[Layout] Layout update triggered:', reason, '| isCollapse:', isCollapse.value, '| windowWidth:', windowWidth.value)
}

// 监听窗口大小变化，自动隐藏/显示侧边栏
const handleWindowResize = () => {
  windowWidth.value = window.innerWidth

  // 窗口宽度小于 720px 时自动隐藏侧边栏
  if (windowWidth.value < 720 && !isCollapse.value&&windowWidthshrink.value) {
    system.packCollapse()
    console.log('[Layout] Auto-hiding sidebar (width < 720px):', windowWidth.value)
  }else if (windowWidth.value >= 720 && isCollapse.value&&!windowWidthshrink.value) {
    system.expandCollapse()
    console.log('[Layout] Auto-show sidebar (width >= 720px):', windowWidth.value)
  }

  // 触发重新计算
  setTimeout(() => {
    triggerLayoutUpdate('window-resize')
  }, 100)
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

  // 如果初始窗口宽度小于 720px，自动隐藏侧边栏
  if (windowWidth.value < 720 && !isCollapse.value) {
    system.packCollapse()
    console.log('[Layout] Initial auto-hide sidebar (width < 720px):', windowWidth.value)
  }else if (windowWidth.value >= 720 && isCollapse.value) {
    system.expandCollapse()
    console.log('[Layout] Auto-show sidebar (width >= 720px):', windowWidth.value)
  }

  setTimeout(() => {
    triggerLayoutUpdate('initial-mount')
  }, 200)

  // 监听窗口大小变化
  window.addEventListener('resize', handleWindowResize)
  updateWindowWidth();
  window.addEventListener('resize', handleResize);

  console.log('[Layout] Mounted, initial sidebar state:', isCollapse.value, '| windowWidth:', windowWidth.value)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleWindowResize)
  window.removeEventListener('resize', handleResize);
})
</script>


<style scoped lang="scss">@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss';

.main-box {
  display: flex;
  flex-direction: column;
  width: 100vw;
  height: 100vh;
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
      width: $menu-width;
      min-width: $menu-width;
      max-width: $menu-width;
      height: 100%;
      transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1),
      min-width 0.3s cubic-bezier(0.4, 0, 0.2, 1),
      max-width 0.3s cubic-bezier(0.4, 0, 0.2, 1),
      opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      z-index: 1000;
      box-sizing: border-box;
      overflow: hidden;

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
      width: 1px;
      cursor: col-resize;
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 1001;
      position: relative;
      transition: background-color 0.2s;

      @include theme.useTheme {
        background-color: theme.getVar('borderColor');
      }

      &:hover,
      &.is-dragging {
        @include theme.useTheme {
          background-color: $theme-color;
        }
      }

      .resize-handle-line {
        width: 2px;
        height: 30px;
        border-radius: 1px;
        opacity: 0;
        transition: opacity 0.2s;

        @include theme.useTheme {
          background-color: $theme-color;
        }
      }

      &:hover .resize-handle-line,
      &.is-dragging .resize-handle-line {
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