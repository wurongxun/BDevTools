<template>
  <div class="app-main">
    <router-view v-slot="{ Component }">
      <keep-alive :include="cacheList">
        <component :is="Component" />
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
@use '@/styles/variables.scss';
@use '@/styles/theme.scss';

.app-main {
  width: 100%;
  height: calc(100vh - $header-height - 42px);
  overflow: auto;
  box-sizing: border-box;
  @include scrollbar; // 保留自定义滚动条样式

  @include useTheme {
    background-color: getVar('appMainBgColor');
  }

  // 添加以下样式来隐藏滚动条但保留滚动功能
  &::-webkit-scrollbar {
    display: none; // 隐藏滚动条但保留滚动功能 (WebKit浏览器)
  }

  -ms-overflow-style: none; // 隐藏滚动条但保留滚动功能 (IE/Edge)
  scrollbar-width: none;
}
</style>
