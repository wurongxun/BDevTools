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
import logoSrc from '@/assets/logo/128x128@2x.png'
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as variables;
@use '@/styles/theme.scss';

.topbar-box {
  height: $header-height;
  display: flex;
  align-items: center;
  gap: 4px;
  padding-left: 8px;

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.logo-box {
  user-select: none;
  overflow: hidden;
  width: 24px;
  height: $header-height;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  flex-shrink: 0;
}

// 固定尺寸：百分比写法在收缩布局中会按原图尺寸撑开，破坏标题栏比例
.logo {
  width: 20px;
  height: 20px;
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
  // 与窗口控制按钮拉开距离，避免图标贴边
  margin-right: 4px;
}
</style>
