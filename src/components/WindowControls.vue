<template>
  <div class="window-controls">
    <el-tooltip content="最小化" placement="bottom">
      <el-icon class="ctrl-btn" @click="handleMinimize">
        <Minus />
      </el-icon>
    </el-tooltip>
    <el-tooltip :content="isMaximized ? '还原' : '最大化'" placement="bottom">
      <el-icon class="ctrl-btn" @click="handleToggleMaximize">
        <WindowRestoreIcon v-if="isMaximized" />
        <WindowMaximizationIcon v-else />
      </el-icon>
    </el-tooltip>
    <el-tooltip content="关闭" placement="bottom">
      <el-icon class="ctrl-btn ctrl-close" @click="handleClose">
        <Close />
      </el-icon>
    </el-tooltip>
  </div>
</template>

<script lang="ts" setup>
import { ref } from 'vue'
import { Close, Minus } from '@element-plus/icons-vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import WindowMaximizationIcon from '@/components/Icon/WindowMaximizationIcon.vue'
import WindowRestoreIcon from '@/components/Icon/WindowRestoreIcon.vue'

const isMaximized = ref(false)

async function handleMinimize() {
  try {
    await getCurrentWindow().minimize()
  } catch (e) {
    console.error('最小化失败:', e)
  }
}

async function handleToggleMaximize() {
  try {
    await getCurrentWindow().toggleMaximize()
    isMaximized.value = !isMaximized.value
  } catch (e) {
    console.error('最大化切换失败:', e)
  }
}

async function handleClose() {
  try {
    await getCurrentWindow().close()
  } catch (e) {
    console.error('关闭窗口失败:', e)
  }
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.window-controls {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  margin-left: 2px;
}

.ctrl-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  cursor: pointer;
  border-radius: 4px;
  transition: background-color 0.15s;
  flex-shrink: 0;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &:hover {
    @include theme.useTheme {
      color: theme.getVar('textColor');
      background-color: theme.getVar('activityBarHover');
    }
  }
}

.ctrl-close:hover {
  color: #e81123 !important;
  background-color: rgba(232, 17, 35, 0.1) !important;
}
</style>
