<template>
  <div class="app-main" :style="{ height: appMainHeight }">
    <el-header class="app-header">
      <!-- 动态渲染当前窗口启用插件的 Tab 栏（v-show 保持存活，切换不重建） -->
      <template v-for="plugin in displayPlugins" :key="plugin.id">
        <component
          :is="plugin.tabBar"
          v-show="isTabVisible(plugin.id)"
        />
      </template>
    </el-header>
    <el-main class="app-main-content">
      <!-- 动态渲染当前窗口启用插件的主面板（v-show 保持存活，切换不重建） -->
      <template v-for="plugin in displayPlugins" :key="plugin.id">
        <component
          :is="plugin.mainPanel"
          v-show="isPanelVisible(plugin.id)"
        />
      </template>
    </el-main>
  </div>
</template>


<script lang="ts" setup name="AppEditorMain">
import { pluginManager } from '@/core/pluginManager'
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import useStore from '@/store/index'

const { useLayoutStore } = useStore()

/** 当前窗口是否处于专注模式 */
const isFocusMode = computed(() => useLayoutStore.focusMode)

/** 当前窗口显示的插件列表（根据 layout store 的 enabledPlugins 过滤） */
const displayPlugins = computed(() => useLayoutStore.filteredPlugins)

/**
 * 专注模式下 activeToolId 可能不在 displayPlugins 中，
 * 自动将其同步为第一个启用插件，确保主面板可见
 */
watch(displayPlugins, (plugins) => {
  if (isFocusMode.value && plugins.length > 0) {
    if (!plugins.some(p => p.id === pluginManager.activeToolId.value)) {
      pluginManager.switchTool(plugins[0].id)
    }
  }
}, { immediate: true })

watch(isFocusMode, (focus) => {
  if (focus) {
    const plugins = displayPlugins.value
    if (plugins.length > 0 && !plugins.some(p => p.id === pluginManager.activeToolId.value)) {
      pluginManager.switchTool(plugins[0].id)
    }
  }
})

/** 判断 Tab 栏是否可见 */
function isTabVisible(pluginId: string): boolean {
  return pluginManager.activeToolId.value === pluginId
}

/** 判断主面板是否可见 */
function isPanelVisible(pluginId: string): boolean {
  return pluginManager.activeToolId.value === pluginId
}

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
