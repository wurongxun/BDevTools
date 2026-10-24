<template>
  <div class="pane-inner">
    <div class="ops-head">
      <button class="btn sm" type="button" @click="load" :disabled="loading">刷新</button>
      <span class="muted">采样 {{ analysis?.count ?? 0 }} 条慢查询（μs = 微秒）</span>
    </div>

    <div class="monitor-grid">
      <div class="metric-card">
        <div class="label">慢查询数</div>
        <div class="value">{{ analysis?.count ?? 0 }}</div>
      </div>
      <div class="metric-card">
        <div class="label">最大耗时</div>
        <div class="value danger">{{ analysis?.maxUs ?? 0 }}<small>μs</small></div>
      </div>
      <div class="metric-card">
        <div class="label">平均耗时</div>
        <div class="value">{{ analysis?.avgUs ?? 0 }}<small>μs</small></div>
      </div>
      <div class="metric-card">
        <div class="label">P95 耗时</div>
        <div class="value warn">{{ analysis?.p95Us ?? 0 }}<small>μs</small></div>
      </div>
    </div>

    <!-- 命令热点 -->
    <div class="info-section">
      <h4>命令热点（按累计耗时）</h4>
      <table class="vtable" v-if="byCommand.length">
        <thead><tr><th>命令模式</th><th>次数</th><th>最大μs</th><th>累计μs</th><th style="width:34%">占比</th></tr></thead>
        <tbody>
          <tr v-for="g in byCommand" :key="g.pattern">
            <td class="mono">{{ g.pattern }}</td>
            <td class="mono">{{ g.count }}</td>
            <td class="mono">{{ g.maxUs }}</td>
            <td class="mono">{{ g.totalUs }}</td>
            <td>
              <div class="bar"><i :style="{ width: pctOfTotal(g.totalUs) + '%' }" /></div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 明细 -->
    <div class="info-section">
      <h4>慢查询明细</h4>
      <table class="vtable" v-if="items.length">
        <thead><tr><th>#</th><th>时间</th><th>耗时μs</th><th>原始命令</th><th>归一化模式</th></tr></thead>
        <tbody>
          <tr v-for="s in items" :key="s.id" :class="{ hot: s.durationUs >= 10000 }">
            <td class="mono">{{ s.id }}</td>
            <td class="mono">{{ fmtTime(s.timestamp) }}</td>
            <td class="mono" :class="s.durationUs >= 10000 ? 'danger' : ''">{{ s.durationUs }}</td>
            <td class="mono">{{ s.command }}</td>
            <td class="mono muted">{{ s.pattern }}</td>
          </tr>
        </tbody>
      </table>
      <div v-else class="key-empty">暂无慢查询</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { getClient, store, toast } from '../store'
import type { SlowQueryAnalysis } from '../types'

const loading = ref(false)
const analysis = ref<SlowQueryAnalysis | null>(null)

const items = computed(() => analysis.value?.items ?? [])
const byCommand = computed(() => analysis.value?.byCommand ?? [])

function pctOfTotal(us: number): number {
  const total = byCommand.value.reduce((s, g) => s + g.totalUs, 0)
  if (!total) return 0
  return Math.max(2, (us / total) * 100)
}
function fmtTime(ts: number): string {
  const d = new Date(ts * 1000)
  return d.toLocaleTimeString()
}

async function load() {
  const c = getClient()
  if (!c) {
    toast('未连接', 'err')
    return
  }
  loading.value = true
  try {
    analysis.value = await c.slowQueryAnalysis(20)
  } catch (e: any) {
    toast('慢查询分析失败: ' + (e.message || e), 'err')
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
.metric-card .value { font-size: 20px; font-family: var(--mono); font-weight: 600; }
.metric-card .value small { font-size: 11px; color: var(--text-2); margin-left: 3px; }
.value.danger { color: var(--danger); }
.value.warn { color: var(--warn); }
.bar { height: 8px; background: var(--code-bg); border-radius: 4px; overflow: hidden; border: 1px solid var(--border); }
.bar > i { display: block; height: 100%; background: var(--accent); }
.danger { color: var(--danger); }
tr.hot { background: color-mix(in srgb, var(--danger) 8%, transparent); }
</style>
