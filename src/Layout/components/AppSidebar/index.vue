<template>
  <div class="sidebar-container">
    <logo></logo>
    <div class="app-sidebar">
      <FileTree></FileTree>
    </div>
  </div>
</template>

<script setup lang="ts" name="AppSidebar">
import logo from './components/Logo.vue'
import FileTree from './components/FileTree.vue'

</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;

.sidebar-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

.app-sidebar {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;

  // 隐藏滚动条但保持滚动功能
  scrollbar-width: thin;

  &::-webkit-scrollbar {
    width: 6px;
  }

  &::-webkit-scrollbar-thumb {
    background-color: rgba(155, 155, 155, 0.3);
    border-radius: 3px;

    &:hover {
      background-color: rgba(155, 155, 155, 0.5);
    }
  }
  &::-webkit-scrollbar-track {
    background: transparent;
  }
}
</style>
