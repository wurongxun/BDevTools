<template>
  <el-dropdown
    @command="handleCommand"
    popper-class="setting-dropdown-popper"
    :popper-options="{
      modifiers: [
        {
          name: 'preventOverflow',
          options: {
            boundary: 'viewport'
          }
        }
      ]
    }"
  >
    <div class="setting-trigger">
      <svg-icon
        class-name="svg-icon setting-icon"
        icon-class="setting"
      />
    </div>
    <template #dropdown>
      <el-dropdown-menu class="setting-dropdown-menu">
        <el-dropdown-item command="setting" class="menu-item">
          <el-icon><Setting /></el-icon>
          <span>{{ $t('settings.name') }}</span>
        </el-dropdown-item>
        <el-dropdown-item command="about" class="menu-item">
          <el-icon><InfoFilled /></el-icon>
          <span>{{ $t('about.name') }}</span>
        </el-dropdown-item>
        <el-dropdown-item command="help" divided class="menu-item">
          <el-icon><QuestionFilled /></el-icon>
          <span>{{ $t('help.name') }}</span>
        </el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>

<script setup lang="ts">import useStore from '@/store/index'
import { ElMessage } from 'element-plus'
import { storeToRefs } from 'pinia'
import { useI18n } from "vue-i18n";
import { Setting, InfoFilled, QuestionFilled } from '@element-plus/icons-vue'
import { openAppWindow } from '@/utils/WindowUtils'

const { t, locale } = useI18n()

const { user } = useStore()
const { userInfo } = storeToRefs(user)

const onStting = () => {
  openAppWindow('/App/setting')
}

const onAbout = () => {
  openAppWindow('/App/about')
}

const onHelp = () => {
  openAppWindow('/App/help')
}

const handleCommand = (command: string | number | object) => {
  switch (command) {
    case 'setting':
      onStting()
      break
    case 'about':
      onAbout()
      break
    case 'help':
      onHelp()
      break

    default:
      break
  }
}
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;

.setting-trigger {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 8px;
  cursor: pointer;
  border-radius: var(--radius-md);
  transition: all 0.2s ease;

  &:hover {
    background-color: rgba(var(--el-color-primary-rgb), 0.1);

    .setting-icon {
      color: $theme-color;
    }
  }
}

.svg-icon {
  width: 20px;
  height: 20px;
  cursor: pointer;
  transition: all 0.2s ease;

  &.setting-icon {
    font-size: var(--font-size-2xl);
  }
}

// 移除默认焦点样式
:deep(.el-dropdown:focus) {
  outline: none !important;
}

:deep(.el-dropdown > .setting-trigger:focus-visible) {
  outline: none !important;
}
</style>

<style lang="scss">
// 全局样式，用于提升下拉菜单层级并优化样式
.setting-dropdown-popper {
  z-index: 99999 !important;

  &.el-popper {
    z-index: 99999 !important;
  }

  .el-dropdown__popper {
    z-index: 99999 !important;
  }

  // 优化下拉菜单样式
  .el-dropdown-menu {
    padding: 6px 0;
    min-width: 160px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15) !important;
    border-radius: var(--radius-xl);
    overflow: hidden;

    .menu-item {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 10px 16px;
      font-size: var(--font-size-lg);
      transition: all 0.2s ease;

      .el-icon {
        font-size: var(--font-size-xl);
        opacity: 0.8;
      }

      span {
        flex: 1;
      }

      &:hover {
        background-color: var(--el-fill-color-light);

        .el-icon {
          opacity: 1;
          color: var(--el-color-primary);
        }
      }

      &.is-divided {
        margin-top: 6px;
        padding-top: 12px;
        border-top: 1px solid var(--el-border-color-lighter);
      }
    }
  }

  // 箭头样式
  .el-popper__arrow {
    z-index: 99999 !important;

    &::before {
      background: white;
      box-shadow: -2px -2px 4px rgba(0, 0, 0, 0.05);
    }
  }
}

// 深色模式适配
html.dark .setting-dropdown-popper {
  .el-dropdown-menu {
    background-color: var(--el-bg-color-overlay);
    border: 1px solid var(--el-border-color);

    .menu-item {
      color: var(--el-text-color-primary);

      &:hover {
        background-color: var(--el-fill-color-dark);
      }

      &.is-divided {
        border-top-color: var(--el-border-color);
      }
    }
  }

  .el-popper__arrow::before {
    background: var(--el-bg-color-overlay);
    box-shadow: -2px -2px 4px rgba(0, 0, 0, 0.3);
  }
}
</style>
