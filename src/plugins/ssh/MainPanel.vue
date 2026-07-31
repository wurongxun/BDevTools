<template>
  <div class="ssh-main-panel">
    <ConnectionPanel />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import useStore from '@/store'
import ConnectionPanel from '@/components/RemoteConnection/ConnectionPanel.vue'

const { useConnectionStore } = useStore()

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
