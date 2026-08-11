<template>
  <div class="ssh-main-panel">
    <ConnectionPanel />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useConnectionStore } from './store'
import ConnectionPanel from './components/RemoteConnection/ConnectionPanel.vue'
import { useRemoteAutoSave } from './composables/useRemoteAutoSave'

const connectionStore = useConnectionStore()

// 启动远程文件自动保存
useRemoteAutoSave()

onMounted(() => {
  connectionStore.loadConnections()
  connectionStore.setupSshOutputListener()
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
