<template>
  <logo></logo>
  <div class="app-sidebar">
    <!-- <side-menu></side-menu> -->
    <FileTreeTest></FileTreeTest>
  </div>

</template>

<script setup lang="ts" name="AppSidebar">
import SideMenu from './components/SideMenu.vue'
import logo from './components/Logo.vue'
import FileTreeTest from './components/FileTree.vue'

</script>

<style scoped lang="scss">
.app-sidebar {
  overflow: auto;
  height: calc(100vh - 60px);
  /* 减去 logo 高度 */
  max-height: 100vh;
}
</style>
