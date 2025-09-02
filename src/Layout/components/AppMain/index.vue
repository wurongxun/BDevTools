<template>
  <div class="app-main">
    <router-view v-slot="{ Component, route }">
      <keep-alive :include="cacheList">
        <component :is="Component" :key="route.path" />
      </keep-alive>
    </router-view>
  </div>
</template>

<script lang="ts" setup name="AppMain">
import useStore from '@/store'
import { storeToRefs } from 'pinia'

const { tagview } = useStore()
const { cacheList } = storeToRefs(tagview)

</script>

<style scoped lang="scss">
@import '@/styles/variables.scss';

.app-main {
  width: 100%;
  height: calc(100vh - 51px - 40px);
  overflow: auto;
  box-sizing: border-box;
  @include scrollbar;

  @include useTheme {
    background-color: getVar('appMainBgColor');
  }
}
</style>
