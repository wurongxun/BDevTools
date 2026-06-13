<template>
  <div class="main-box">
    <div class="side-box" :class="{ 'is-collapse': isCollapse }">
      <app-sidebar />
    </div>
    <div class="app-content" :class="{ 'is-collapse': isCollapse }">
      <app-navbar></app-navbar>
      <app-main></app-main>
    </div>
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
  position: relative;
  width: 100vw;
  height: 100vh;

  .side-box {
    position: fixed;
    left: 0;
    top: 0;
    height: 100%;
    width: $menu-width;
    flex-shrink: 0;
    @include theme.useTheme {
      border-right: 1px solid theme.getVar('borderColor');
    }
    transition: transform 0.3s ease;
    z-index: 1000;

    &.is-collapse {
      width: $collapse-width;
      transform: translateX(-($menu-width - $collapse-width));
    }
  }

  .app-content {
    width: calc(100vw - #{$menu-width});
    height: 100%;
    overflow: hidden;
    margin-left: $menu-width;
    transition: all 0.3s ease;

    &.is-collapse {
      width: 100vw;
      margin-left: 0;
    }
  }
}

@media screen and (max-width: 768px) {
  .main-box {
    .side-box {
      transform: translateX(-$menu-width);

      &.is-collapse {
        transform: translateX(0);
      }
    }

    .app-content {
      width: 100vw;
      margin-left: 0;

      &.is-collapse {
        margin-left: 0;
      }
    }
  }
}
</style>