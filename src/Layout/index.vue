<template>
  <div class="main-box">
    <aside class="side-box" :class="{ 'is-collapse': isCollapse }">
      <app-sidebar />
    </aside>
    <main class="app-content" :class="{ 'is-collapse': isCollapse }">
      <app-navbar></app-navbar>
      <app-main></app-main>
    </main>
  </div>
</template>

<script setup lang="ts" name="Layout">
import AppMain from './components/AppMain/index.vue'
import AppNavbar from './components/AppNavbar/index.vue'
import AppSidebar from './components/AppSidebar/index.vue'
import useStore from '@/store/index'
import { storeToRefs } from 'pinia'

const { system } = useStore()
const { isCollapse } = storeToRefs(system)
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss';

.main-box {
  display: flex;
  width: 100vw;
  height: 100vh;
  overflow: hidden;

  .side-box {
    flex-shrink: 0;
    width: $menu-width;
    height: 100%;
    transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    z-index: 1000;

    @include theme.useTheme {
      background-color: theme.getVar('bgColor');
      border-right: 1px solid theme.getVar('borderColor');
    }

    &.is-collapse {
      width: $collapse-width;
    }
  }

  .app-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }
}

// 平板端适配
@media screen and (max-width: 1024px) and (min-width: 769px) {
  .main-box {
    .side-box {
      position: absolute;
      left: 0;
      top: 0;
      height: 100%;
      box-shadow: 2px 0 8px rgba(0, 0, 0, 0.1);

      &.is-collapse {
        transform: translateX(-$menu-width);
      }
    }

    .app-content {
      width: 100%;
      margin-left: 0;

      &.is-collapse {
        margin-left: 0;
      }
    }
  }
}

// 移动端适配
@media screen and (max-width: 768px) {
  .main-box {
    .side-box {
      position: absolute;
      left: 0;
      top: 0;
      height: 100%;
      width: $menu-width;
      box-shadow: 2px 0 8px rgba(0, 0, 0, 0.15);
      transform: translateX(-$menu-width);

      &.is-collapse {
        transform: translateX(0);
      }
    }

    .app-content {
      width: 100%;
      margin-left: 0;
    }
  }
}
</style>
