<script setup lang="ts">
import { computed, onMounted, ref, unref } from 'vue';
import { useI18n } from "vue-i18n";
import { useRoute } from 'vue-router';
import { useHomeNavigation } from '@/events/navigation';
const { pages } = useHomeNavigation();
const { t } = useI18n();
const route = useRoute();

const appName = ref<string>(t('appName'));
</script>

<template>
  <div class="common-layout">
    <main class="app-container">
      <keep-alive>
        <router-view v-if="route.meta.keepAlive" :key="route.fullPath" />
        <router-view v-else :key="'normal-' + route.fullPath" />
      </keep-alive>
    </main>
  </div>
</template>

<style lang="scss" scoped>
@use '@/styles/variables.scss';
@use '@/styles/theme.scss';

.common-layout {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.app-container {
  flex: 1;
  overflow: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;

  &::-webkit-scrollbar {
    width: 0;
    height: 0;
    background: transparent;
  }
}

::-webkit-scrollbar {
  width: 0;
  height: 0;
  background: transparent;
}

.router-view {
  transition: all 0.3s ease;
}
</style>
