<template>
  <header class="topbar-box" data-tauri-drag-region>
    <toggle-switch></toggle-switch>
    <div class="bread-crumb-box">
      <bread-crumb></bread-crumb>
    </div>
    <functional-area></functional-area>
    <!-- <avatar></avatar> -->
  </header>
  <tag-view></tag-view>
</template>

<script lang="ts" setup name="AppNavbar">
import ToggleSwitch from './components/ToggleSwitch.vue'
import BreadCrumb from './components/BreadCrumb.vue'
import avatar from './components/Avatar.vue'
import TagView from './components/TagView.vue'
import FunctionalArea from './components/FunctionalArea/index.vue'
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss';
@use '@/styles/theme.scss';
.topbar-box {
  height: 51px;
 // margin-bottom: 1px;
  display: flex;
  align-items: center;

  @include useTheme {
    border-bottom: 1px solid getVar('borderColor');
  }

  .bread-crumb-box {
    flex: 1;
  }
}
</style>
