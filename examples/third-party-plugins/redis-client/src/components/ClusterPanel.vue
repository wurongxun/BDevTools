<template>
  <div class="pane-inner">
    <div class="ops-head">
      <button class="btn sm" type="button" @click="load" :disabled="loading">刷新</button>
      <span class="mode-badge" :class="topo?.mode">{{ modeLabel }}</span>
      <span class="muted">以上为 Mock 合成的集群/复制演示数据</span>
    </div>

    <!-- 复制 -->
    <div class="info-section">
      <h4>主从复制</h4>
      <div class="kv-grid" v-if="repl">
        <div class="kv"><span>角色</span><b>{{ repl.role }}</b></div>
        <div class="kv"><span>已连接从节点</span><b>{{ repl.connectedSlaves }}</b></div>
        <div class="kv"><span>复制偏移</span><b class="mono">{{ repl.masterReplOffset }}</b></div>
      </div>
      <table class="vtable" v-if="repl && repl.slaves.length">
        <thead><tr><th>ID</th><th>地址</th><th>偏移</th><th>延迟(s)</th><th>状态</th></tr></thead>
        <tbody>
          <tr v-for="s in repl.slaves" :key="s.id">
            <td class="mono">{{ s.id }}</td>
            <td class="mono">{{ s.addr }}</td>
            <td class="mono">{{ s.offset }}</td>
            <td class="mono">{{ s.lag }}</td>
            <td><span class="ok-badge">{{ s.state }}</span></td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 集群拓扑 -->
    <div class="info-section">
      <h4>集群拓扑（{{ topo?.nodeCount ?? 0 }} 节点 · {{ topo?.slotsCovered ?? 0 }} 槽位）</h4>
      <table class="vtable" v-if="topo">
        <thead><tr><th>节点</th><th>角色</th><th>地址</th><th>槽位</th><th>主节点</th><th>状态</th></tr></thead>
        <tbody>
          <tr v-for="n in topo.nodes" :key="n.id">
            <td class="mono">{{ short(n.id) }}{{ n.role === 'myself' ? ' ★' : '' }}</td>
            <td><span class="role-badge" :class="n.role">{{ n.role }}</span></td>
            <td class="mono">{{ n.addr }}</td>
            <td class="mono">{{ n.slots }}</td>
            <td class="mono">{{ n.masterId ? short(n.masterId) : '—' }}</td>
            <td><span class="ok-badge">{{ n.linkState }}</span></td>
          </tr>
        </tbody>
      </table>
      <!-- 槽位分布条 -->
      <div class="slot-bar" v-if="topo">
        <div
          v-for="(n, i) in masterNodes"
          :key="n.id"
          class="slot-seg"
          :style="{ flex: slotWidth(n.slots), background: slotColor(i) }"
          :title="n.addr + ' ' + n.slots"
        />
      </div>
      <div class="legend" v-if="topo">
        <span v-for="(n, i) in masterNodes" :key="n.id" class="legend-row">
          <i class="dot" :style="{ background: slotColor(i) }" />{{ n.addr }} ({{ n.slots }})
        </span>
      </div>
    </div>

    <!-- Sentinel -->
    <div class="info-section">
      <h4>Sentinel 监控的主节点</h4>
      <table class="vtable" v-if="sentinels.length">
        <thead><tr><th>名称</th><th>地址</th><th>从节点</th><th>其他 Sentinel</th><th>Quorum</th></tr></thead>
        <tbody>
          <tr v-for="m in sentinels" :key="m.name">
            <td class="mono">{{ m.name }}</td>
            <td class="mono">{{ m.ip }}:{{ m.port }}</td>
            <td class="mono">{{ m.numSlaves }}</td>
            <td class="mono">{{ m.numOtherSentinels }}</td>
            <td class="mono">{{ m.quorum }}</td>
          </tr>
        </tbody>
      </table>
      <div v-else class="key-empty">无 Sentinel 数据</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { getClient, store, toast } from '../store'
import type { ClusterTopology, ReplicationInfo, SentinelMaster } from '../types'

const loading = ref(false)
const topo = ref<ClusterTopology | null>(null)
const repl = ref<ReplicationInfo | null>(null)
const sentinels = ref<SentinelMaster[]>([])

const modeLabel = computed(() => {
  const m = topo.value?.mode
  return m === 'cluster' ? 'cluster 模式' : m === 'sentinel' ? 'sentinel 模式' : 'standalone'
})
const masterNodes = computed(() => (topo.value?.nodes ?? []).filter((n) => n.role !== 'slave'))

function short(id: string): string {
  return id.slice(0, 8)
}
function slotWidth(range: string): number {
  if (range === '-' || !range.includes('-')) return 1
  const [a, b] = range.split('-').map((x) => parseInt(x, 10))
  return b - a + 1
}
const SLOT_COLORS = ['#2f6feb', '#1a7f37', '#b8860b', '#8957e5', '#c0392b', '#0d9488']
function slotColor(i: number): string {
  return SLOT_COLORS[i % SLOT_COLORS.length]
}

async function load() {
  const c = getClient()
  if (!c) {
    toast('未连接', 'err')
    return
  }
  loading.value = true
  try {
    const [t, r, s] = await Promise.all([c.clusterTopology(), c.replicationInfo(), c.sentinelMasters()])
    topo.value = t
    repl.value = r
    sentinels.value = s
  } catch (e: any) {
    toast('集群信息加载失败: ' + (e.message || e), 'err')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  if (store.connected) load()
})
</script>

<style scoped>
.ops-head { display: flex; align-items: center; gap: 12px; margin-bottom: 12px; }
.muted { color: var(--text-2); font-size: 12px; }
.mode-badge { font-size: 11px; padding: 2px 8px; border-radius: 10px; border: 1px solid var(--accent); color: var(--accent); }
.kv-grid { display: flex; gap: 16px; flex-wrap: wrap; margin-bottom: 10px; }
.kv { display: inline-flex; gap: 6px; align-items: center; font-size: 12px; color: var(--text-2); }
.kv b { color: var(--text); }
.ok-badge { color: var(--ok); font-size: 11px; }
.role-badge { font-size: 10px; padding: 1px 7px; border-radius: 3px; color: #fff; background: var(--text-2); }
.role-badge.master { background: var(--accent); }
.role-badge.slave { background: var(--ok); }
.role-badge.myself { background: var(--warn); }
.slot-bar { display: flex; height: 16px; border-radius: 4px; overflow: hidden; border: 1px solid var(--border); margin-top: 8px; gap: 1px; }
.slot-seg { min-width: 2px; }
.legend { display: flex; flex-wrap: wrap; gap: 12px; margin-top: 8px; font-size: 11px; color: var(--text-2); }
.dot { width: 10px; height: 10px; border-radius: 50%; display: inline-block; margin-right: 4px; }
</style>
