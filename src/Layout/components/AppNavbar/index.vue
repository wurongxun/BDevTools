<template>
  <header class="topbar-box" data-tauri-drag-region>
    <div class="logo-box">
      <el-image class="logo" fit="contain" :src="logoSrc" />
    </div>
    <div class="menu-box"  data-tauri-drag-region>
      <menu-bar></menu-bar>
    </div>
    <div class="functional-area-box">
      <functional-area></functional-area>
    </div>
  </header>
</template>

<script lang="ts" setup name="AppNavbar">import FunctionalArea from './components/FunctionalArea/index.vue'
import MenuBar from './components/MenuBar/index.vue'
import useStore from '@/store'
import logoSrc from '@/assets/logo/64x64.png'
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as variables;
@use '@/styles/theme.scss';

.topbar-box {
  height: $header-height;
  display: flex;
  align-items: center;
  gap: 4px;

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.logo-box {
  user-select: none;
  overflow: hidden;
  height: $header-height;
  display: flex;
  align-items: center;
  cursor: pointer;
  flex-shrink: 0;

  &.is-collapse {
    width: variables.$collapse-width;
  }
}

.logo {
  width: 20px;
  height: 20px;
  padding: 2px;
}

.menu-box {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
}

.functional-area-box {
  flex-shrink: 0;
  display: flex;
  align-items: center;
}
</style>
