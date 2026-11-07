<template>
  <div class="ssh-main-panel">
    <ConnectionPanel />
    <!-- P3 设置中心：全局单例，由终端工具栏的 window 事件唤起 -->
    <SshSettingsDialog />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useConnectionStore } from './store'
import ConnectionPanel from './components/RemoteConnection/ConnectionPanel.vue'
import SshSettingsDialog from './components/RemoteConnection/settings/SettingsDialog.vue'
import { useRemoteAutoSave } from './composables/useRemoteAutoSave'

const connectionStore = useConnectionStore()

// 启动远程文件自动保存
useRemoteAutoSave()

onMounted(() => {
  // 连接列表由 store 创建时立即加载（ensureLoaded），这里只挂全局事件监听
  connectionStore.setupSshOutputListener()
  connectionStore.setupSshClosedListener()
  connectionStore.setupTransferProgressListener()
})
</script>

<style scoped>
.ssh-main-panel {
  width: 100%;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
</style>
