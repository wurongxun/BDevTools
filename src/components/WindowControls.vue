<template>
  <div class="window-controls">
    <el-tooltip content="最小化" placement="bottom">
      <button type="button" class="ctrl-btn" aria-label="最小化窗口" @click="handleMinimize">
        <el-icon><Minus /></el-icon>
      </button>
    </el-tooltip>
    <el-tooltip :content="isMaximized ? '还原' : '最大化'" placement="bottom">
      <button
        type="button"
        class="ctrl-btn"
        :aria-label="isMaximized ? '还原窗口' : '最大化窗口'"
        @click="handleToggleMaximize"
      >
        <el-icon>
          <WindowRestoreIcon v-if="isMaximized" />
          <WindowMaximizationIcon v-else />
        </el-icon>
      </button>
    </el-tooltip>
    <el-tooltip content="关闭" placement="bottom">
      <button type="button" class="ctrl-btn ctrl-close" aria-label="关闭窗口" @click="handleClose">
        <el-icon><Close /></el-icon>
      </button>
    </el-tooltip>
  </div>
</template>

<script lang="ts" setup>
import { ref } from 'vue'
import { Close, Minus } from '@element-plus/icons-vue'
import { getCurrentWindow } from '@/core/runtime'
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
  gap: 4px;
  flex-shrink: 0;
  margin-right: 6px;
}

// 紧凑方形按钮，与 FunctionalArea 的 .fa-icon 规格一致，在 32px 标题栏内垂直居中
// 命中区 32×26：宽度按桌面窗口控制惯例取 32px，高度受限于 32px 标题栏
.ctrl-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 26px;
  cursor: pointer;
  border-radius: var(--radius-md);
  font-size: var(--font-size-base);
  transition: background-color 0.15s;
  flex-shrink: 0;
  // 语义化 <button> 需要抹平浏览器默认外观
  appearance: none;
  border: none;
  padding: 0;
  background: transparent;
  font-family: inherit;

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
  color: #fff !important;
  background-color: #e81123 !important;
}
</style>
