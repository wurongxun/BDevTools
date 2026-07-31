<template>
  <div class="sidebar-container">
    <!-- 活动栏（左侧图标列）：动态渲染所有已注册插件 -->
    <div class="activity-bar">
      <div
        v-for="plugin in pluginManager.plugins"
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
      <template v-for="plugin in pluginManager.plugins" :key="plugin.id">
        <div class="panel-content" v-show="pluginManager.activeToolId.value === plugin.id">
          <component :is="plugin.sidebarPanel" />
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts" name="AppSidebar">
import { pluginManager } from '@/core/pluginManager'
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
  background-color: #252525;
  border-right: 1px solid #2d2d2d;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    border-right-color: theme.getVar('borderColor');
  }
}

.activity-item {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 48px;
  cursor: pointer;
  color: #888;
  transition: all 0.15s;
  border-left: 2px solid transparent;

  &:hover {
    color: #ccc;
  }

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.activity-item-active {
  color: #fff !important;
  border-left-color: #007acc;

  @include theme.useTheme {
    color: theme.getVar('textColor') !important;
    border-left-color: #007acc;
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
}

.panel-content {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
</style>
