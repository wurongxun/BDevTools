<template>
  <div class="container">
    <search-menu />
    <theme-switcher />
    <!-- <fullscreen /> -->
    <message />
    <!-- <error-handler /> -->
     <app-setting />
    <hide />
  </div>
</template>

<script lang="ts" setup name="FunctionalArea">
import Fullscreen from './Fullscreen.vue'
import Message from './Message.vue'
import SearchMenu from './SearchMenu.vue'
import ThemeSwitcher from './ThemeSwitcher.vue'
import Hide from './Hide.vue'
import AppSetting from './Setting.vue'
</script>

<style scoped lang="scss">
.container {
  font-size: 20px;
  display: flex;
  align-items: center;
}
</style>
