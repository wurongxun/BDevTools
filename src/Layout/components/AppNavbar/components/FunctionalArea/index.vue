<template>
  <div class="functional-area">
    <!-- 窗口控制 -->
    <WindowControls />
  </div>
</template>

<script lang="ts" setup name="FunctionalArea">
// 说明：主题切换 / 切换侧边栏 / 隐藏-显示插件面板 / 沉浸式 这组按钮已整体迁到菜单左侧的
// `QuickActions` 组件（同一标题栏，位置在 MenuBar 之前）。本组件保留为标题栏右侧的宿主入口位，
// 目前只放窗口控制，后续要加搜索 / 消息等右侧图标时仍挂这里。
import WindowControls from '@/components/WindowControls.vue'
</script>

<style scoped lang="scss">
.functional-area {
  display: flex;
  align-items: center;
  height: 100%;
  gap: 2px;
}
</style>
