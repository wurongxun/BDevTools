<template>
  <div class="ops">
    <div class="monitor-tabs">
      <button class="tab" :class="{ active: tab === 'memory' }" type="button" @click="tab = 'memory'">内存分析</button>
      <button class="tab" :class="{ active: tab === 'slow' }" type="button" @click="tab = 'slow'">慢查询分析</button>
      <button class="tab" :class="{ active: tab === 'cluster' }" type="button" @click="tab = 'cluster'">集群/复制</button>
      <button class="tab" :class="{ active: tab === 'audit' }" type="button" @click="tab = 'audit'">操作审计</button>
      <span class="spacer" />
      <span class="mock-badge">P2 · Mock 演示</span>
    </div>
    <div class="ops-body">
      <MemoryAnalysisPanel v-if="tab === 'memory'" />
      <SlowQueryPanel v-else-if="tab === 'slow'" />
      <ClusterPanel v-else-if="tab === 'cluster'" />
      <AuditPanel v-else-if="tab === 'audit'" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import MemoryAnalysisPanel from './MemoryAnalysisPanel.vue'
import SlowQueryPanel from './SlowQueryPanel.vue'
import ClusterPanel from './ClusterPanel.vue'
import AuditPanel from './AuditPanel.vue'

const tab = ref('memory')
</script>

<style scoped>
.ops { display: flex; flex-direction: column; height: 100%; }
.ops-body { flex: 1; min-height: 0; overflow: auto; padding: 14px; }
.mock-badge { font-size: 10px; color: var(--text-2); border: 1px solid var(--border); padding: 2px 8px; border-radius: 10px; }
</style>
