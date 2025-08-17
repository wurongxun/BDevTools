<script setup lang="ts">
import { computed, onMounted, ref, unref } from 'vue';
import { useI18n } from "vue-i18n";
import { useRoute } from 'vue-router';
import BackHeader from "@/components/header/BackHeader.vue";
import { useHomeNavigation } from '@/events/navigation.ts';

const { pages } = useHomeNavigation();
const { t } = useI18n();
const route = useRoute();

const header_height = '35px';
const appName = ref<string>(t('appName'));
</script>

<template>
  <div class="common-layout">
    <header class="app-header">
      <template v-for="item in pages" :key="item.name">
        <BackHeader 
          v-if="$route.name === item.name" 
          :showTitle="item.showTitle" 
          :title="unref(item.title)"
          :showWinMin="item.showWinMin" 
          :showWinChange="item.showWinChange"
          :showWinHide="item.showWinHide"
        />
      </template>
    </header>
    <main class="app-container">
        <!-- 根据路由元信息决定是否缓存 -->
      <keep-alive>
        <router-view v-if="$route.meta.keepAlive" :key="$route.fullPath" />
        <router-view v-else :key="'normal-' + $route.fullPath" />
      </keep-alive>
    </main>
  </div>
</template>

<style lang="scss" scoped>
.common-layout {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.app-header {
  height: v-bind(header_height);
  width: 100%;
  display: flex;
  border-bottom: 1px solid var(--el-border-color);
  background-color: var(--main--header--bg-color);
  flex-shrink: 0;
}

.app-container {
  flex: 1;
  overflow: auto;
  scrollbar-width: none; // Firefox
  -ms-overflow-style: none; // IE 10+
  
  // WebKit browsers
  &::-webkit-scrollbar {
    width: 0;
    height: 0;
    background: transparent;
  }
}

// 全局滚动条隐藏样式
::-webkit-scrollbar {
  width: 0;
  height: 0;
  background: transparent;
}

// 动画效果
.router-view {
  transition: all 0.3s ease;
}
</style>