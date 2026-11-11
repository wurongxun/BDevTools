<template>
  <div class="connection-panel">
    <!--
      所有 tab 组件始终挂载在同一个绝对定位容器里，只切 v-show 与几何位置。
      分屏切换若把组件搬到另一个父节点，Vue 会销毁重建 ⇒ xterm 与 SSH 会话全丢，
      这是这里不用「两个 pane 各渲染一份」的根本原因。
    -->
    <div class="connection-content" :class="{ 'is-split': splitEnabled }">
      <div
        v-for="tab in tabs"
        :key="tab.id"
        v-show="isVisible(tab.id)"
        class="tab-slot"
        :class="{ 'slot-active': tab.id === activeTabId }"
        :style="slotStyle(tab.id)"
      >
        <template v-if="getConnection(tab.connectionId)">
          <div v-if="splitEnabled" class="pane-caption">
            {{ tab.id === secondaryTabId ? '副窗格' : '主窗格' }} · {{ tab.title }}
          </div>
          <div class="pane-body">
            <TerminalView
              v-if="tab.type === 'terminal'"
              :tab="tab"
              :connection="getConnection(tab.connectionId)!"
            />
            <FtpView
              v-else-if="tab.type === 'ftp'"
              :tab="tab"
              :connection="getConnection(tab.connectionId)!"
            />
            <TunnelPanel
              v-else
              :tab="tab"
              :connection="getConnection(tab.connectionId)!"
            />
          </div>
        </template>
      </div>

      <div v-if="splitEnabled" class="split-divider" :class="`split-${splitOrientation}`" />

      <div v-if="tabs.length === 0" class="empty-placeholder">
        <div class="empty-content">
          <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1">
            <path d="M4 17l6-6-6-6" />
            <line x1="12" y1="19" x2="20" y2="19" />
          </svg>
          <p>从左侧连接列表中选择连接</p>
          <p class="sub-text">或点击右键新建连接</p>
        </div>
      </div>
    </div>

    <ComposeBar v-if="composeVisible" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useConnectionStore } from '../../store'
import TerminalView from './Terminal/TerminalView.vue'
import FtpView from './Ftp/FtpView.vue'
import TunnelPanel from './TunnelPanel.vue'
import ComposeBar from './ComposeBar.vue'
import type { RemoteConnection } from '../../types'

const connectionStore = useConnectionStore()

const tabs = computed(() => connectionStore.tabs)
const activeTabId = computed(() => connectionStore.activeTabId)
const secondaryTabId = computed(() => connectionStore.secondaryTabId)
const splitEnabled = computed(() => connectionStore.splitEnabled)
const splitOrientation = computed(() => connectionStore.splitOrientation)
const composeVisible = computed(() => connectionStore.composeVisible)
const connections = computed(() => connectionStore.connections)

function getConnection(connectionId: number): RemoteConnection | null {
  return connections.value.find(c => c.id === connectionId) || null
}

function isVisible(tabId: string): boolean {
  return tabId === activeTabId.value || (splitEnabled.value && tabId === secondaryTabId.value)
}

/** 主窗格恒占左/上半，副窗格占右/下半；未显示的 tab 给主窗格几何（反正 v-show=false） */
function slotStyle(tabId: string): Record<string, string> {
  if (!splitEnabled.value || tabId !== secondaryTabId.value) {
    return {
      left: '0', top: '0',
      width: splitEnabled.value && splitOrientation.value === 'vertical' ? '50%' : '100%',
      height: splitEnabled.value && splitOrientation.value === 'horizontal' ? '50%' : '100%'
    }
  }
  return splitOrientation.value === 'vertical'
    ? { left: '50%', top: '0', width: '50%', height: '100%' }
    : { left: '0', top: '50%', width: '100%', height: '50%' }
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.connection-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  overflow: hidden;
  @include theme.useTheme { background-color: theme.getVar('appMainBgColor'); }
}

.connection-content {
  flex: 1;
  min-height: 0;
  position: relative;
  overflow: hidden;
}

.tab-slot {
  position: absolute;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  @include theme.useTheme { background-color: theme.getVar('appMainBgColor'); }
}

.pane-caption {
  flex-shrink: 0;
  padding: 2px 10px;
  font-size: theme.getVar('fontSizeSm');
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  @include theme.useTheme {
    color: theme.getVar('textSecondary');
    background-color: theme.getVar('sidebarBg');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.pane-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  > * { flex: 1; min-height: 0; }
}

.split-divider {
  position: absolute;
  z-index: 3;
  pointer-events: none;
  @include theme.useTheme { background-color: theme.getVar('borderStrong'); }
  &.split-vertical { left: 50%; top: 0; width: 2px; height: 100%; transform: translateX(-1px); }
  &.split-horizontal { left: 0; top: 50%; width: 100%; height: 2px; transform: translateY(-1px); }
}

.empty-placeholder {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  @include theme.useTheme { color: theme.getVar('textSecondary'); }

  .empty-content {
    text-align: center;
    svg { margin-bottom: 16px; opacity: 0.3; }
    p { margin: 4px 0; font-size: theme.getVar('fontSize'); }
    .sub-text {
      font-size: theme.getVar('fontSizeSmall');
      @include theme.useTheme { color: theme.getVar('textDisabled'); }
    }
  }
}
</style>
