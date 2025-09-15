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
<!-- 
<style scoped lang="scss">
@import '@/styles/variables.scss';

.main-box {
  position: relative;
  width: calc(100vw);
  height: calc(100vh);

  .side-box {
    position: fixed;
    left: 0;
    top: 0;
    height: 100%;
    width: $--menu-width;
    flex-shrink: 0;
    // overflow: auto;
    border-right: 1px solid #e4e7ed;

    &.is-collapse {
      width: $--collapse-width;
    }
  }

  .app-content {
    width: calc(100vw - $--menu-width);
    height: 100%;
    overflow: hidden;
    margin-left: $--menu-width;
    transition: all 0.36s;

    &.is-collapse {
      width: calc(100vw - $--collapse-width);
      margin-left: $--collapse-width;
    }
  }
}
</style> -->

<style scoped lang="scss">
@import '@/styles/variables.scss';

.main-box {
  position: relative;
  width: calc(100vw);
  height: calc(100vh);
  .side-box {
    position: fixed;
    left: 0;
    top: 0;
    height: 100%;
    width: $--menu-width;
    flex-shrink: 0;
    // overflow: auto;
    border-right: 1px solid #e4e7ed;
    transition: transform 0.36s;
    z-index: 1000;

    &.is-collapse {
      width: $--collapse-width;
      transform: translateX(-($--menu-width - $--collapse-width));
    }
  }

  .app-content {
    width: calc(100vw - $--menu-width);
    height: 100%;
    overflow: hidden;
    margin-left: $--menu-width;
    transition: all 0.36s;

    &.is-collapse {
      width: calc(100vw);
      margin-left: 0;
    }
  }
}
</style>