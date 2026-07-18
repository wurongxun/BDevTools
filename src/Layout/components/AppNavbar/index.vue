<template>
  <header class="topbar-box" data-tauri-drag-region>
    <div class="logo-box">
      <el-image class="logo" fit="contain" :src="logoSrc" />
    </div>
    <div class="menu-box">
      <menu-bar></menu-bar>
    </div>
    <div class="functional-area-box">
      <functional-area></functional-area>
    </div>
  </header>
</template>

<script lang="ts" setup name="AppNavbar">
import FunctionalArea from './components/FunctionalArea/index.vue'
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
  gap: 8px;

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }

  .bread-crumb-box {
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }
}

@media screen and (max-width: 768px) {
  .topbar-box {
    padding-left: 12px;
    padding-right: 8px;
    gap: 6px;
  }
}

.logo-box {
  user-select: none;
  overflow: hidden;
  height: $header-height;
  display: flex;
  align-items: center;
  cursor: pointer;
  margin-bottom: 1px;

  &.is-collapse {
    width: variables.$collapse-width;
  }

  .icon {
    margin: 15px;
    flex-shrink: 0;
  }

  .title {
    font-size: 16px;
    white-space: nowrap;

    &.shrink-title {
      display: none;
    }
  }
}

.logo {
  width: 30px;
  height: 30px;
  padding: 2px 2px;
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
