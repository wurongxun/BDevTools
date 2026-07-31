<template>
  <div class="conn-tab-bar">
    <div class="conn-tab-list">
      <div
        v-for="tab in tabs"
        :key="tab.id"
        class="conn-tab-item"
        :class="{
          'conn-tab-active': tab.id === activeTabId,
          'conn-tab-connected': tab.status === 'connected'
        }"
        @click="handleTabClick(tab.id)"
      >
        <!-- 连接状态指示点 -->
        <span
          class="conn-status-dot"
          :class="`status-${tab.status}`"
        />

        <!-- Tab 图标 -->
        <span class="conn-tab-icon">
          <svg v-if="tab.type === 'terminal'" width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M2 3l5 5-5 5" stroke="currentColor" stroke-width="1.5" fill="none"/>
            <line x1="8" y1="13" x2="14" y2="13" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <svg v-else width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M2 3h12v10H2V3z" fill="none" stroke="currentColor" stroke-width="1"/>
            <line x1="2" y1="6" x2="14" y2="6" stroke="currentColor" stroke-width="1"/>
            <circle cx="4" cy="4.5" r="0.5" fill="currentColor"/>
            <circle cx="6" cy="4.5" r="0.5" fill="currentColor"/>
          </svg>
        </span>

        <!-- Tab 标题 -->
        <span class="conn-tab-label">{{ tab.title }}</span>

        <!-- 关闭按钮 -->
        <button
          v-if="tab.closable"
          class="conn-tab-close"
          @click.stop="handleTabClose(tab.id)"
          title="关闭连接"
        >
          <svg width="10" height="10" viewBox="0 0 12 12" fill="currentColor">
            <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.5"/>
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import useStore from '@/store'

const { useConnectionStore } = useStore()

const tabs = computed(() => useConnectionStore.tabs)
const activeTabId = computed(() => useConnectionStore.activeTabId)

function handleTabClick(tabId: string) {
  useConnectionStore.switchTab(tabId)
}

function handleTabClose(tabId: string) {
  useConnectionStore.closeTab(tabId)
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.conn-tab-bar {
  display: flex;
  align-items: stretch;
  height: 34px;
  background-color: #1e1e1e;
  border-bottom: 1px solid #2d2d2d;
  user-select: none;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-bottom-color: theme.getVar('borderColor');
  }
}

.conn-tab-list {
  flex: 1;
  display: flex;
  align-items: stretch;
  overflow-x: auto;
  scrollbar-width: none;

  &::-webkit-scrollbar {
    display: none;
  }
}

.conn-tab-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
  min-width: 100px;
  max-width: 200px;
  height: 100%;
  flex-shrink: 0;
  background-color: #252526;
  border-right: 1px solid #1e1e1e;
  color: #888;
  font-size: 12px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
  white-space: nowrap;
  position: relative;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-right-color: theme.getVar('borderColor');
    color: theme.getVar('textColor');
  }

  &:hover {
    background-color: #2a2d2e;
    color: #ccc;

    @include theme.useTheme {
      background-color: rgba(255, 255, 255, 0.05);
    }
  }
}

.conn-tab-active {
  background-color: #1e1e1e !important;
  color: #fff !important;
  border-bottom: 2px solid #007acc;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor') !important;
    color: theme.getVar('textColor') !important;
    border-bottom-color: #007acc;
  }
}

.conn-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;

  &.status-disconnected {
    background-color: #888;
  }
  &.status-connecting {
    background-color: #e2b714;
    animation: pulse 1s infinite;
  }
  &.status-connected {
    background-color: #4ec9b0;
  }
  &.status-error {
    background-color: #f44747;
  }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.conn-tab-icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  color: inherit;
}

.conn-tab-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
}

.conn-tab-close {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  border-radius: 3px;
  color: #888;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s, background-color 0.15s;

  &:hover {
    background-color: rgba(255, 255, 255, 0.1);
    color: #fff;
  }
}

.conn-tab-item:hover .conn-tab-close {
  opacity: 1;
}
</style>
