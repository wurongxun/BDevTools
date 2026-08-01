<template>
  <div class="app-main" :style="{ height: appMainHeight }">
    <el-header class="app-header">
      <!-- 动态渲染所有插件的 Tab 栏（v-show 保持存活，切换不重建） -->
      <template v-for="plugin in pluginManager.plugins" :key="plugin.id">
        <component
          :is="plugin.tabBar"
          v-show="pluginManager.activeToolId.value === plugin.id && plugin.tabBar"
        />
      </template>
    </el-header>
    <el-main class="app-main-content">
      <!-- 动态渲染所有插件的主面板（v-show 保持存活，切换不重建） -->
      <template v-for="plugin in pluginManager.plugins" :key="plugin.id">
        <component
          :is="plugin.mainPanel"
          v-show="pluginManager.activeToolId.value === plugin.id"
        />
      </template>
    </el-main>
  </div>
</template>


<script lang="ts" setup name="AppEditorMain">
import { pluginManager } from '@/core/pluginManager'
import { ref, onMounted, onUnmounted, nextTick } from 'vue'

// 动态计算主内容区域高度
const appMainHeight = ref('calc(100vh - var(--header-height) - var(--tage-view-height))')

const updateHeight = () => {
  const navbar = document.querySelector('.topbar-box')
  let headerHeight = 32

  if (navbar instanceof HTMLElement) {
    headerHeight = navbar.offsetHeight
  }

  appMainHeight.value = `calc(100vh - ${headerHeight}px)`
}

// 窗口 resize 处理函数（带防抖）
let resizeTimer: ReturnType<typeof setTimeout> | null = null

const handleResize = async () => {
  if (resizeTimer) {
    clearTimeout(resizeTimer)
  }

  resizeTimer = setTimeout(async () => {
    updateHeight()
    await nextTick()

    const event = new CustomEvent('window-resize', {
      detail: {
        timestamp: Date.now(),
        width: window.innerWidth,
        height: window.innerHeight,
        reason: 'window-resize'
      }
    })
    window.dispatchEvent(event)
  }, 200)
}

// 处理侧边栏切换事件
const handleLayoutUpdate = async (event: Event) => {
  const customEvent = event as CustomEvent

  await nextTick()
  updateHeight()

  const resizeEvent = new CustomEvent('window-resize', {
    detail: {
      timestamp: Date.now(),
      reason: customEvent.detail?.reason || 'layout-update',
      isCollapse: customEvent.detail?.isCollapse,
      sidebarWidth: customEvent.detail?.sidebarWidth,
      mainWidth: customEvent.detail?.mainWidth
    }
  })
  window.dispatchEvent(resizeEvent)
}

onMounted(() => {
  updateHeight()
  window.addEventListener('resize', handleResize)
  window.addEventListener('orientationchange', handleResize)
  window.addEventListener('layout-update', handleLayoutUpdate)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  window.removeEventListener('orientationchange', handleResize)
  window.removeEventListener('layout-update', handleLayoutUpdate)

  if (resizeTimer) {
    clearTimeout(resizeTimer)
  }
})
</script>

<style scoped lang="scss">@use '@/styles/variables' as *;
@use '@/styles/theme';

.app-main {
  width: 100%;
  height: 100%;
  margin: 0;
  padding: 0;
  overflow: hidden;
  box-sizing: border-box;
  position: relative;
  display: flex;
  flex-direction: column;

  @include theme.useTheme {
    background-color: theme.getVar('appMainBgColor');
  }
}

.app-header {
  margin: 0;
  padding: 0;
  height: auto !important;
  min-height: unset;
  flex-shrink: 0;
}

.app-main-content {
  margin: 0;
  padding: 0;
  overflow: hidden;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
</style>
