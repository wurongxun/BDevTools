<template>
  <el-breadcrumb class="breadcrumb-container" separator=">" data-tauri-drag-region>
    <el-breadcrumb-item>
      {{ name }}
    </el-breadcrumb-item>
  </el-breadcrumb>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import useStore from '@/store'
const { useDocumentManagerStore } = useStore()
const name = computed(() => {
  console.log("4444444444444444444"+useDocumentManagerStore.activeDocument)
  return useDocumentManagerStore.activeDocument?.name || ''
})
</script>

<style lang="scss" scoped>
.breadcrumb-container {
  display: flex;
  align-items: center;
  user-select: none; // 添加这一行

  :deep(.el-breadcrumb__inner, .el-breadcrumb__item) {
    display: inline-flex;
    align-items: center;
    margin: 1px;
  }

  .menu-icon {
    padding-right: 5px;
  }
}
</style>
