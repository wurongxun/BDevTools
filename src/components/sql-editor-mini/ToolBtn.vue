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
  color: #57606a;
  border-radius: 3px;
  cursor: pointer;
  position: relative;
  transition: all .15s;
  .el-icon { font-size: 15px; }
  .arrow {
    position: absolute;
    right: 1px;
    bottom: 1px;
    font-size: 8px !important;
    color: #8c959f;
  }
  &:hover { background: #eaeef2; color: #0969da; .arrow { color: #0969da; } }
  &:disabled { opacity: .4; cursor: not-allowed; &:hover { background: transparent; color: #57606a; } }
}
</style>
