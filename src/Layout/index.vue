<template>
  <div class="main-box">
    <aside ref="sideBoxRef" class="side-box" :class="{ 'is-collapse': isCollapse }">
      <app-sidebar />
    </aside>
    <main ref="mainContentRef" class="app-content" :class="{ 'is-collapse': isCollapse, 'has-sidebar': !isCollapse && windowWidth >= 1024 }">
      <app-navbar class="app-navbar"></app-navbar>
      <app-main></app-main>
    </main>
  </div>
</template>

<script setup lang="ts" name="Layout">
import AppMain from './components/AppMain/index.vue'
import AppNavbar from './components/AppNavbar/index.vue'
import AppSidebar from './components/AppSidebar/index.vue'
import useStore from '@/store/index'
import { storeToRefs } from 'pinia'
import { watch, nextTick, onMounted, ref, onUnmounted } from 'vue'
import { useResizeObserver } from '@vueuse/core'

const { system } = useStore()
const { isCollapse } = storeToRefs(system)

const sideBoxRef = ref<HTMLElement | null>(null)
const mainContentRef = ref<HTMLElement | null>(null)
const windowWidth = ref(window.innerWidth)

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

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss';

.main-box {
  display: flex;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  box-sizing: border-box;

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
      border-right: 1px solid theme.getVar('borderColor');
    }

    &.is-collapse {
      width: 0;
      min-width: 0;
      max-width: 0;
      opacity: 0;
      border-right: none;
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
  }
}
</style>
