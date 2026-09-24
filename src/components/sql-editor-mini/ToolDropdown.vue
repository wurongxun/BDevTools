<template>
  <el-tooltip :content="title" placement="bottom">
    <el-dropdown trigger="click" @command="handleCmd">
      <button class="tool-btn">
        <el-icon><component :is="icon" /></el-icon>
        <el-icon class="arrow"><ArrowDown /></el-icon>
      </button>
      <template #dropdown>
        <slot />
      </template>
    </el-dropdown>
  </el-tooltip>
</template>

<script setup lang="ts">
import { ArrowDown } from '@element-plus/icons-vue'
interface Props { icon: any; title?: string }
withDefaults(defineProps<Props>(), { title: '' })
const emit = defineEmits<{ (e: 'command', cmd: string): void }>()
const handleCmd = (cmd: string) => emit('command', cmd)
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
  padding: 0;
  .el-icon { font-size: 15px; }
  .arrow {
    position: absolute;
    right: 1px;
    bottom: 1px;
    font-size: 8px !important;
    color: #8c959f;
  }
  &:hover { background: #eaeef2; color: #0969da; .arrow { color: #0969da; } }
}
</style>
