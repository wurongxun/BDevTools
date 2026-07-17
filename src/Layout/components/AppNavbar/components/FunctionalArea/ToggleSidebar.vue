<script setup lang="ts">
import useStore from '@/store'
import { storeToRefs } from 'pinia'
const { system } = useStore()
const { isCollapse } = storeToRefs(system)
function toggleSidebar() {
  if (isCollapse.value) {
    system.expandCollapse()
  } else {
    system.packCollapse()
  }
}
</script>

<template>
  <div
    class="layout-toggle-btn"
    :class="{ 'is-active': !isCollapse }"
    @click="toggleSidebar"
    :title="isCollapse ? '显示侧边栏' : '隐藏侧边栏'"
  >
    <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
      <rect x="3" y="3" width="18" height="18" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/>
      <rect x="3" y="3" width="6" height="18" rx="1" :fill="!isCollapse ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="1.2" opacity="0.9"/>
    </svg>
  </div>
</template>

<style scoped lang="scss">
@use '@/styles/variables.scss' as variables;

.layout-toggle-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  margin-right: 12px;
  margin-bottom: 2px;
  cursor: pointer;
  color: inherit;
  transition: color 0.2s;

  &:hover {
    color: variables.$theme-color;
  }

  &.is-active {
    color: variables.$theme-color;
  }
}
</style>
