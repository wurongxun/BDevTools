<template>
  <div class="app-main" :style="{ height: appMainHeight }">
    <el-header class="app-header">
      <!-- 动态渲染当前窗口启用插件的 Tab 栏（KeepAlive 常驻：切走 deactivated 缓存实例、卸载 DOM，切回 activated 恢复，不重建） -->
      <template v-for="item in tabBarItems" :key="item.id">
        <ErrorBoundary v-slot="{ retryKey }" :scope="`${panelLabel(item.pluginId)} · Tab 栏`">
          <KeepAlive>
            <component
              :is="item.payload.component"
              v-if="isTabVisible(item.pluginId)"
              :key="`${item.pluginId}-${retryKey}`"
            />
          </KeepAlive>
        </ErrorBoundary>
      </template>
    </el-header>
    <el-main class="app-main-content">
      <!-- 动态渲染当前窗口启用插件的主面板（KeepAlive 常驻：切走 deactivated 缓存实例、卸载 DOM，切回 activated 恢复，不重建）
           只声明停靠面板的插件没有主面板，它的面板由 Layout 停靠在主区边缘 -->
      <template v-for="item in mainItems" :key="item.id">
        <ErrorBoundary v-slot="{ retryKey }" :scope="`${panelLabel(item.pluginId)} · 主面板`">
          <KeepAlive>
            <component
              :is="item.payload.component"
              v-if="isPanelVisible(item.pluginId)"
              :key="`${item.pluginId}-${retryKey}`"
            />
          </KeepAlive>
        </ErrorBoundary>
      </template>
    </el-main>
  </div>
</template>


<script lang="ts" setup name="AppEditorMain">
import ErrorBoundary from '@/components/ErrorBoundary.vue'
import { pluginManager } from '@/core/pluginManager'
import { contributionRegistry } from '@/core/registry'
import type { ViewPayload } from '@/core/registry'
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import useStore from '@/store/index'

const { useLayoutStore } = useStore()

/** 当前窗口是否处于专注模式 */
const isFocusMode = computed(() => useLayoutStore.focusMode)

/**
 * 当前窗口可见的插件 ID。
 *
 * 可见性是**窗口策略**（窗口预设的显示清单），注册表只回答「系统里有什么」，
 * 两者在这里相交 —— 顺序与组件都来自注册表。
 */
const visibleIds = computed(() => new Set(useLayoutStore.filteredPlugins.map(p => p.id)))

/**
 * 本窗口可见的插件（只用于「激活哪个插件」的判定）。
 *
 * 刻意用插件列表而不是 `mainItems`：专注模式下该切到**第一个可见插件**而不是第一个
 * 有主面板的插件 —— 侧栏（AppSidebar）与宽度（Layout）都是按「第一个可见插件」算的，
 * 这里若换一套口径，主区与侧栏就会落到两个不同插件上。
 */
const visiblePlugins = computed(() => useLayoutStore.filteredPlugins)

/** 面板出错时的可读名字（告诉用户崩掉的是哪个插件的哪一块） */
function panelLabel(pluginId: string): string {
  return pluginManager.getPlugin(pluginId)?.label ?? pluginId
}

/** Tab 栏条目 */
const tabBarItems = computed(() =>
  contributionRegistry.views<ViewPayload>('tabBar').filter(e => visibleIds.value.has(e.pluginId)),
)

/** 主面板条目 */
const mainItems = computed(() =>
  contributionRegistry.views<ViewPayload>('main').filter(e => visibleIds.value.has(e.pluginId)),
)

/**
 * 专注模式下 activeToolId 可能不在 displayPlugins 中，
 * 自动将其同步为第一个启用插件，确保主面板可见
 */
watch(visiblePlugins, (plugins) => {
  if (isFocusMode.value && plugins.length > 0) {
    if (!plugins.some(p => p.id === pluginManager.activeToolId.value)) {
      pluginManager.switchTool(plugins[0].id)
    }
  }
}, { immediate: true })

watch(isFocusMode, (focus) => {
  if (focus) {
    const plugins = visiblePlugins.value
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
