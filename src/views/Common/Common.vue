<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from "vue-i18n";
import { useRoute } from 'vue-router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state';
import { Close, Minus } from '@element-plus/icons-vue';
import WindowMaximizationIcon from '@/components/Icon/WindowMaximizationIcon.vue'
import WindowRestoreIcon from '@/components/Icon/WindowRestoreIcon.vue'

const route = useRoute();
const { t } = useI18n();

const pageTitles: Record<string, string> = {
  '/AppSetting': '设置',
  '/AppAbout': '关于',
  '/AppHelp': '帮助',
  '/AppTerminal': '终端'
}

const pageTitle = ref(pageTitles[route.path] || '')
const isMaximized = ref(false)

const handleMinimize = async () => {
  try {
    await getCurrentWindow().minimize()
  } catch (e) {
    console.error('最小化失败:', e)
  }
}

const handleToggleMaximize = async () => {
  try {
    const win = getCurrentWindow()
    await win.toggleMaximize()
    isMaximized.value = !isMaximized.value
  } catch (e) {
    console.error('最大化切换失败:', e)
  }
}

const handleClose = async () => {
  try {
    await getCurrentWindow().close()
  } catch (e) {
    console.error('关闭窗口失败:', e)
  }
}
</script>

<template>
  <div class="common-layout">
    <header class="common-topbar" data-tauri-drag-region>
      <span class="common-topbar-title">{{ pageTitle }}</span>
      <div class="common-topbar-spacer" data-tauri-drag-region></div>
      <div class="common-window-controls">
        <el-tooltip content="最小化" placement="bottom">
          <el-icon class="common-ctrl-btn" @click="handleMinimize">
            <Minus />
          </el-icon>
        </el-tooltip>
        <el-tooltip :content="isMaximized ? '还原' : '最大化'" placement="bottom">
          <el-icon class="common-ctrl-btn" @click="handleToggleMaximize">
            <WindowRestoreIcon v-if="isMaximized" />
            <WindowMaximizationIcon v-else />
          </el-icon>
        </el-tooltip>
        <el-tooltip content="关闭" placement="bottom">
          <el-icon class="common-ctrl-btn common-ctrl-close" @click="handleClose">
            <Close />
          </el-icon>
        </el-tooltip>
      </div>
    </header>
    <main class="app-container">
      <keep-alive>
        <router-view v-if="route.meta.keepAlive" :key="route.fullPath" />
        <router-view v-else :key="'normal-' + route.fullPath" />
      </keep-alive>
    </main>
  </div>
</template>

<style lang="scss" scoped>
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss';

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
  font-size: 13px;
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

.common-window-controls {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.common-ctrl-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  margin-right: 12px;
  cursor: pointer;
  color: inherit;
  transition: color 0.2s;
  flex-shrink: 0;

  &:hover {
    color: $theme-color;
  }
}

.common-ctrl-close:hover {
  color: #e81123;
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
