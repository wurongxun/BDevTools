<template>
  <div class="ssh-main-panel">
    <ConnectionPanel />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import useStore from '@/store'
import ConnectionPanel from './components/RemoteConnection/ConnectionPanel.vue'
import { useRemoteAutoSave } from './composables/useRemoteAutoSave'

const { useConnectionStore } = useStore()

// 启动远程文件自动保存
useRemoteAutoSave()

onMounted(() => {
  useConnectionStore.loadConnections()
  useConnectionStore.setupSshOutputListener()
  useConnectionStore.setupTransferProgressListener()
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
