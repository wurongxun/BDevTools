<template>
  <el-tooltip :content="title" placement="bottom">
    <button class="tool-btn" :disabled="disabled" @click="emit('click')">
      <el-icon><component :is="icon" /></el-icon>
      <el-icon v-if="withArrow" class="arrow"><ArrowDown /></el-icon>
    </button>
  </el-tooltip>
</template>

<script setup lang="ts">
import { ArrowDown } from '@element-plus/icons-vue'
interface Props {
  icon: any
  title?: string
  disabled?: boolean
  withArrow?: boolean
}
withDefaults(defineProps<Props>(), { title: '', disabled: false, withArrow: false })
const emit = defineEmits<{ (e: 'click'): void }>()
</script>

<style scoped lang="scss">
.tool-btn {
  width: 28px;
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--textSecondary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  position: relative;
  transition: all .15s;
  .el-icon { font-size: 15px; }
  .arrow {
    position: absolute;
    right: 1px;
    bottom: 1px;
    font-size: 8px !important;
    color: var(--textSecondary);
  }
  &:hover { background: var(--hoverColor); color: var(--el-color-primary); .arrow { color: var(--el-color-primary); } }
  &:disabled { opacity: .4; cursor: not-allowed; &:hover { background: transparent; color: var(--textDisabled); } }
}
</style>
