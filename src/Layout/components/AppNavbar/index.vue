<template>
  <!-- 沉浸式下整条标题栏隐藏（v-show 而非 v-if：保留内部状态与已加载的主题设置，
       空间由 flex 列自动让给主区；拖拽区与窗口控制由 AppImmersiveBar 临时接管） -->
  <header v-show="!immersive" class="topbar-box" data-tauri-drag-region>
    <!-- 快捷操作区（菜单左侧）：主题 / 侧边栏 / 插件面板 / 沉浸式 -->
    <quick-actions></quick-actions>
    <div class="menu-box" data-tauri-drag-region>
      <menu-bar></menu-bar>
    </div>
    <div class="functional-area-box">
      <functional-area></functional-area>
    </div>
  </header>
</template>

<script lang="ts" setup name="AppNavbar">import { storeToRefs } from 'pinia'
import FunctionalArea from './components/FunctionalArea/index.vue'
import MenuBar from './components/MenuBar/index.vue'
import QuickActions from './components/QuickActions/index.vue'
import useStore from '@/store'
import logoSrc from '@/assets/logo/128x128@2x.png'

const store = useStore()

/** 沉浸式：只显示插件界面，整条标题栏（含菜单、快捷操作、窗口控制）隐藏 */
const { immersiveMode: immersive } = storeToRefs(store.useLayoutStore)
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
  height: 100%;
}

.functional-area-box {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  // 与窗口控制按钮拉开距离，避免图标贴边
  margin-right: 1px;
}
</style>
