<template>
  <div class="connection-panel" v-show="showConnectionPanel">
    <!-- Tab 栏 -->
    <ConnectionTabBar />

    <!-- Tab 内容区域 -->
    <div class="connection-content">
      <template v-for="tab in tabs" :key="tab.id">
        <!-- 终端视图 -->
        <TerminalView
          v-show="tab.id === activeTabId && tab.type === 'terminal'"
          :tab="tab"
          :connection="getConnection(tab.connectionId)!"
        />

        <!-- FTP 文件管理器 -->
        <FtpView
          v-show="tab.id === activeTabId && tab.type === 'ftp'"
          :tab="tab"
          :connection="getConnection(tab.connectionId)!"
        />
      </template>

      <!-- 无 Tab 时的占位 -->
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
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import useStore from '@/store'
import ConnectionTabBar from './ConnectionTabBar.vue'
import TerminalView from './Terminal/TerminalView.vue'
import FtpView from './Ftp/FtpView.vue'
import type { RemoteConnection } from '@/entity/entitys'

const { useConnectionStore } = useStore()

const tabs = computed(() => useConnectionStore.tabs)
const activeTabId = computed(() => useConnectionStore.activeTabId)
const showConnectionPanel = computed(() => useConnectionStore.showConnectionPanel)
const connections = computed(() => useConnectionStore.connections)

function getConnection(connectionId: number): RemoteConnection | null {
  return connections.value.find(c => c.id === connectionId) || null
}

onMounted(() => {
  useConnectionStore.loadConnections()
  useConnectionStore.setupSshOutputListener()
  useConnectionStore.setupTransferProgressListener()
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.connection-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  overflow: hidden;
  background-color: #1e1e1e;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
  }
}

.connection-content {
  flex: 1;
  min-height: 0;
  position: relative;
  overflow: hidden;
}

.empty-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #666;

  .empty-content {
    text-align: center;

    svg {
      margin-bottom: 16px;
      opacity: 0.3;
    }

    p {
      margin: 4px 0;
      font-size: 14px;
    }

    .sub-text {
      font-size: 12px;
      color: #555;
    }
  }
}
</style>
