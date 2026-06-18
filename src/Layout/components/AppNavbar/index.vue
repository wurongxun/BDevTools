<template>
  <header class="topbar-box" data-tauri-drag-region>
    <toggle-switch></toggle-switch>
    <div class="bread-crumb-box">
      <bread-crumb></bread-crumb>
    </div>
    <functional-area></functional-area>
    <!-- <avatar></avatar> -->
  </header>
<!--  <tag-view></tag-view>-->
</template>

<script lang="ts" setup name="AppNavbar">
import ToggleSwitch from './components/ToggleSwitch.vue'
import BreadCrumb from './components/BreadCrumb.vue'
import avatar from './components/Avatar.vue'
import TagView from './components/TagView.vue'
import FunctionalArea from './components/FunctionalArea/index.vue'
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss';

.topbar-box {
  height: $header-height;
  display: flex;
  align-items: center;
  padding-left: 12px;
  padding-right: 12px;
  gap: 8px;

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }

  .bread-crumb-box {
    flex: 1;
    min-width: 0; // 防止面包屑溢出
    overflow: hidden;
  }
}

// 移动端适配
@media screen and (max-width: 768px) {
  .topbar-box {
    padding-left: 12px;
    padding-right: 8px;
    gap: 6px;
  }
}
</style>
