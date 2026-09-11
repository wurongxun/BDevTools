<template>
  <div class="sidebar-container">
    <!-- 活动栏（左侧图标列）：hideActivityBar 时隐藏（专注模式）
         语义：垂直 tablist + roving tabindex，支持 ↑↓/Home/End 键盘切换（WCAG 2.1.1 / 4.1.2） -->
    <div
      v-show="!hideActivityBar"
      class="activity-bar"
      role="tablist"
      aria-label="插件面板"
      aria-orientation="vertical"
    >
      <button
        v-for="(plugin, index) in displayPlugins"
        :key="plugin.id"
        :ref="(el) => setItemRef(el, index)"
        class="activity-item"
        type="button"
        role="tab"
        :class="{ 'activity-item-active': pluginManager.activeToolId.value === plugin.id }"
        :aria-selected="pluginManager.activeToolId.value === plugin.id"
        :tabindex="pluginManager.activeToolId.value === plugin.id ? 0 : -1"
        :title="plugin.label"
        :aria-label="plugin.label"
        @click="onActivityClick(plugin.id)"
        @keydown="onActivityKeydown($event, index)"
      >
        <component :is="plugin.icon" class="activity-icon" aria-hidden="true" />
      </button>
    </div>

    <!-- 面板区域：动态渲染当前激活插件的侧边栏面板（KeepAlive 常驻：切走 deactivated 缓存实例、卸载 DOM，切回 activated 恢复，不重建）
         插件面板收起（sidebarPanelCollapsed）时隐藏；专注模式（hideActivityBar）下始终显示 -->
    <div v-show="hideActivityBar || !sidebarPanelCollapsed" class="panel-area">
      <template v-for="plugin in displayPlugins" :key="plugin.id">
        <KeepAlive>
          <component :is="plugin.sidebarPanel" v-if="isPanelVisible(plugin.id)" />
        </KeepAlive>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts" name="AppSidebar">
import { computed, ref, nextTick } from 'vue'
import { pluginManager } from '@/core/pluginManager'
import useStore from '@/store/index'

const { useLayoutStore } = useStore()

const props = withDefaults(defineProps<{
  /** 是否隐藏活动栏（专注模式时只隐藏图标列，保留插件面板） */
  hideActivityBar?: boolean
}>(), {
  hideActivityBar: false
})

/** 当前窗口显示的插件列表（根据 layout store 的 enabledPlugins 过滤） */
const displayPlugins = computed(() => useLayoutStore.filteredPlugins)

/** 插件面板是否收起（仅隐藏面板区，保留活动栏） */
const sidebarPanelCollapsed = computed(() => useLayoutStore.sidebarPanelCollapsed)

/**
 * 点击活动栏图标：切换插件；若面板当前处于收起状态则自动展开（否则点了看不到面板）
 */
function onActivityClick(pluginId: string) {
  pluginManager.switchTool(pluginId)
  if (useLayoutStore.sidebarPanelCollapsed) {
    useLayoutStore.setSidebarPanelCollapsed(false)
  }
}

/**
 * 键盘导航：↑↓ 循环切换、Home/End 跳首尾（垂直 tablist 标准交互）
 * 切换后立即把焦点移到新项上，保持 roving tabindex 的焦点跟随
 */
function onActivityKeydown(e: KeyboardEvent, index: number) {
  const count = displayPlugins.value.length
  if (count === 0) return

  let next = -1
  switch (e.key) {
    case 'ArrowDown':
      next = (index + 1) % count
      break
    case 'ArrowUp':
      next = (index - 1 + count) % count
      break
    case 'Home':
      next = 0
      break
    case 'End':
      next = count - 1
      break
    default:
      return
  }

  e.preventDefault()
  onActivityClick(displayPlugins.value[next].id)
  nextTick(() => activityItemRefs.value[next]?.focus())
}

/** 收集活动栏按钮 DOM，供方向键导航后移动焦点 */
const activityItemRefs = ref<HTMLElement[]>([])
function setItemRef(el: unknown, index: number) {
  if (el) activityItemRefs.value[index] = el as HTMLElement
}

/**
 * 判断插件面板是否可见
 * - 活动栏隐藏时（专注模式）：显示第一个插件的面板
 * - 正常模式：显示当前激活插件的面板
 */
function isPanelVisible(pluginId: string): boolean {
  if (props.hideActivityBar) {
    // 专注模式：自动显示第一个插件的面板
    return displayPlugins.value.length > 0 && displayPlugins.value[0].id === pluginId
  }
  return pluginManager.activeToolId.value === pluginId
}
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss' as theme;

.sidebar-container {
  display: flex;
  height: 100%;
  overflow: hidden;
}

// 活动栏（左侧图标列）
.activity-bar {
  display: flex;
  flex-direction: column;
  width: 48px;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('activityBarBg');
    border-right: 1px solid theme.getVar('borderColor');
  }
}

.activity-item {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 48px;
  cursor: pointer;
  transition: all 0.15s;
  border-left: 2px solid transparent;
  // 语义化 <button> 需要抹平浏览器默认外观
  appearance: none;
  border-top: none;
  border-right: none;
  border-bottom: none;
  padding: 0;
  font-family: inherit;

  @include theme.useTheme {
    background-color: theme.getVar('activityBarBg');
    color: theme.getVar('textSecondary');

    &:hover {
      color: theme.getVar('textColor');
      background-color: theme.getVar('activityBarHover');
    }
  }
}

.activity-item-active {
  @include theme.useTheme {
    border-left-color: theme.getVar('hoverColor');
    color: theme.getVar('activityBarActive') !important;
    background-color: theme.getVar('activityBarHover');
  }
}

.activity-icon {
  width: 22px;
  height: 22px;
}

// 面板区域
.panel-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
  }
}

.panel-content {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
</style>
