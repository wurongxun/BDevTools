<script setup lang="ts">
import WindowControls from '@/components/WindowControls.vue'

defineProps<{
  pageComponent: any
  pageTitle: string
}>()
</script>

<template>
  <div class="common-layout">
    <header class="common-topbar" data-tauri-drag-region>
      <span class="common-topbar-title">{{ pageTitle }}</span>
      <div class="common-topbar-spacer" data-tauri-drag-region></div>
      <WindowControls />
    </header>
    <main class="app-container">
      <component :is="pageComponent" v-if="pageComponent" />
    </main>
  </div>
</template>

<style lang="scss" scoped>
@use '@/styles/variables' as *;
@use '@/styles/theme';

.common-layout {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.common-topbar {
  height: $header-height;
  display: flex;
  align-items: center;
  flex-shrink: 0;
  gap: 8px;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.common-topbar-title {
  font-size: var(--font-size-md);
  font-weight: 500;
  user-select: none;
  padding: 0 12px;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.common-topbar-spacer {
  flex: 1;
  min-width: 0;
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
</style>
