<template>
  <div class="pane-inner">
    <div class="ops-head">
      <button class="btn sm" type="button" @click="load" :disabled="loading">刷新</button>
      <span class="muted">当前库共 {{ analysis?.totalKeys ?? 0 }} 个 key · {{ fmtBytes(analysis?.totalBytes ?? 0) }}</span>
    </div>

    <!-- 大 Key 排名 -->
    <div class="info-section">
      <h4>大 Key 排名（Top {{ bigKeys.length }}）</h4>
      <table class="vtable" v-if="bigKeys.length">
        <thead><tr><th>#</th><th>Key</th><th>类型</th><th>内存</th><th>TTL</th><th style="width:30%">占比</th></tr></thead>
        <tbody>
          <tr v-for="(k, i) in bigKeys" :key="k.name">
            <td class="mono">{{ i + 1 }}</td>
            <td class="mono kname">{{ k.name }}</td>
            <td><span class="type-badge" :style="{ background: color(k.type) }">{{ k.type }}</span></td>
            <td class="mono">{{ fmtBytes(k.size) }}</td>
            <td class="mono">{{ k.ttl === -1 ? '永久' : k.ttl + 's' }}</td>
            <td>
              <div class="bar"><i :style="{ width: pctOf(k.size) + '%', background: color(k.type) }" /></div>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-else class="key-empty">暂无数据</div>
    </div>

    <div class="two-col">
      <!-- 按前缀内存占比 -->
      <div class="info-section">
        <h4>按前缀内存占比</h4>
        <table class="vtable" v-if="prefixes.length">
          <thead><tr><th>前缀</th><th>数量</th><th>内存</th><th>占比</th></tr></thead>
          <tbody>
            <tr v-for="p in prefixes" :key="p.prefix">
              <td class="mono">{{ p.prefix }}</td>
              <td class="mono">{{ p.count }}</td>
              <td class="mono">{{ fmtBytes(p.size) }}</td>
              <td class="mono">{{ p.pct }}%</td>
            </tr>
          </tbody>
        </table>
        <div v-else class="key-empty">暂无数据</div>
      </div>

      <!-- 按类型分布 -->
      <div class="info-section">
        <h4>按类型内存分布</h4>
        <div class="donut-wrap" v-if="types.length">
          <svg viewBox="0 0 120 120" class="donut">
            <circle cx="60" cy="60" r="48" fill="none" stroke="var(--border)" stroke-width="14" />
            <circle
              v-for="(s, i) in donut"
              :key="i"
              cx="60" cy="60" r="48"
              fill="none"
              :stroke="color(s.type)"
              stroke-width="14"
              :stroke-dasharray="`${s.dash} ${C}`"
              :stroke-dashoffset="s.offset"
              transform="rotate(-90 60 60)"
            />
          </svg>
          <div class="legend">
            <div v-for="t in types" :key="t.type" class="legend-row">
              <i class="dot" :style="{ background: color(t.type) }" />
              <span class="mono">{{ t.type }}</span>
              <span class="mono muted">{{ t.pct }}%</span>
              <span class="mono muted">{{ fmtBytes(t.size) }}</span>
            </div>
          </div>
        </div>
        <div v-else class="key-empty">暂无数据</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { getClient, store, toast } from '../store'
import { formatBytes } from '../utils'
import { TYPE_COLORS } from '../types'
import type { MemoryAnalysis, RedisType } from '../types'

const loading = ref(false)
const analysis = ref<MemoryAnalysis | null>(null)

const bigKeys = computed(() => analysis.value?.bigKeys ?? [])
const prefixes = computed(() => analysis.value?.prefixes ?? [])
const types = computed(() => analysis.value?.types ?? [])

const R = 48
const C = 2 * Math.PI * R
const donut = computed(() => {
  let acc = 0
  return types.value.map((t) => {
    const start = acc
    acc += t.pct
    return {
      type: t.type,
      dash: (t.pct / 100) * C,
      offset: -(start / 100) * C
    }
  })
})

function fmtBytes(n: number): string {
  return formatBytes(n)
}
function color(t: RedisType): string {
  return TYPE_COLORS[t] || '#9aa0a6'
}
function pctOf(size: number): number {
  const total = analysis.value?.totalBytes ?? 0
  if (!total) return 0
  return Math.max(2, (size / total) * 100)
}

async function load() {
  const c = getClient()
  if (!c) {
    toast('未连接', 'err')
    return
  }
  loading.value = true
  try {
    analysis.value = await c.memoryAnalyze()
  } catch (e: any) {
    toast('内存分析失败: ' + (e.message || e), 'err')
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
.two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
.type-badge { color: #fff; font-size: 10px; padding: 1px 7px; border-radius: 3px; }
.kname { font-size: 11px; }
.bar { height: 8px; background: var(--code-bg); border-radius: 4px; overflow: hidden; border: 1px solid var(--border); }
.bar > i { display: block; height: 100%; }
.donut-wrap { display: flex; gap: 16px; align-items: center; }
.donut { width: 140px; height: 140px; flex: 0 0 140px; }
.legend { flex: 1; display: flex; flex-direction: column; gap: 6px; }
.legend-row { display: flex; align-items: center; gap: 8px; font-size: 12px; }
.dot { width: 10px; height: 10px; border-radius: 50%; display: inline-block; }
.legend-row .muted { margin-left: auto; }
.mono.muted { margin-left: 6px; }
</style>
