<template>
  <div class="settings-host">
    <AppSetting />
  </div>
</template>

<script setup lang="ts" name="SettingsMainPanel">
/**
 * 设置插件主面板：直接内嵌 AppSetting.vue。
 *
 * 复用而非复制——设置内容（应用 / 窗口 / 插件三分区）只有一份实现，
 * 独立窗口入口（`app-window-App-setting`）与插件主面板入口渲染的是同一个组件。
 */
import AppSetting from '@/plugins/settings/app/AppSetting.vue'
</script>

<style scoped lang="scss">
.settings-host {
  width: 100%;
  height: 100%;
  overflow: hidden;
}
</style>
