<template>
  <div class="placeholder">
    <el-icon class="ph-icon"><Box /></el-icon>
    <h3 class="ph-title">{{ title }}</h3>
    <p class="ph-desc">{{ desc }}</p>
    <p class="ph-tag">该功能在后续迭代中接入</p>
  </div>
</template>

<script setup lang="ts">
import { Box } from '@element-plus/icons-vue'

/** 占位视图：为尚未实现的分析功能提供统一说明面板（扩展位） */
defineProps<{
  title: string
  desc: string
}>()
</script>

<style scoped>
.placeholder {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px;
  text-align: center;
}

.ph-icon {
  font-size: 40px;
  color: var(--el-text-color-placeholder);
}

.ph-title {
  margin: 4px 0 0;
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.ph-desc {
  margin: 0;
  max-width: 520px;
  font-size: var(--font-size-base);
  line-height: 1.7;
  color: var(--el-text-color-secondary);
}

.ph-tag {
  margin: 8px 0 0;
  font-size: var(--font-size-xs);
  padding: 2px 10px;
  border-radius: var(--radius-xl);
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
}
</style>
