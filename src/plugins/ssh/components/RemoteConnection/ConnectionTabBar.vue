<template>
  <TabBarContainer>
    <TabBarItem
      v-for="tab in tabs"
      :key="tab.id"
      :active="tab.id === activeTabId"
      @click="handleTabClick(tab.id)"
      @close="handleTabClose(tab.id)"
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
      <TabCloseButton v-if="tab.closable" @click="handleTabClose(tab.id)" title="关闭连接" />
    </TabBarItem>
  </TabBarContainer>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { TabBarContainer, TabBarItem, TabCloseButton } from '@/components/TabBar'
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
/* SSH 插件特有样式：连接状态指示点 */
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
</style>
