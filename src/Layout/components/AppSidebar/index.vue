<template>
  <div class="sidebar-container">
    <!-- 活动栏（左侧图标列）：hideActivityBar 时隐藏（专注模式） -->
    <div v-show="!hideActivityBar" class="activity-bar">
      <div
        v-for="plugin in displayPlugins"
        :key="plugin.id"
        class="activity-item"
        :class="{ 'activity-item-active': pluginManager.activeToolId.value === plugin.id }"
        :title="plugin.label"
        @click="pluginManager.switchTool(plugin.id)"
      >
        <component :is="plugin.icon" class="activity-icon" />
      </div>
    </div>

    <!-- 面板区域：动态渲染当前激活插件的侧边栏面板 -->
    <div class="panel-area">
      <template v-for="plugin in displayPlugins" :key="plugin.id">
        <div class="panel-content" v-show="isPanelVisible(plugin.id)">
          <component :is="plugin.sidebarPanel" />
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts" name="AppSidebar">
import { computed } from 'vue'
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

  @include theme.useTheme {
    color: theme.getVar('textSecondary');

    &:hover {
      color: theme.getVar('textColor');
      background-color: theme.getVar('activityBarHover');
    }
  }
}

.activity-item-active {
  border-left-color: #007acc;

  @include theme.useTheme {
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
