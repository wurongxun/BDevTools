<template>
  <div class="monitor">
    <div class="monitor-tabs">
      <button class="tab" :class="{ active: tab === 'overview' }" type="button" @click="tab = 'overview'">概览</button>
      <button class="tab" :class="{ active: tab === 'info' }" type="button" @click="tab = 'info'">INFO</button>
      <button class="tab" :class="{ active: tab === 'slowlog' }" type="button" @click="tab = 'slowlog'">慢日志</button>
      <button class="tab" :class="{ active: tab === 'clients' }" type="button" @click="tab = 'clients'">客户端</button>
      <span class="spacer" />
      <button class="btn sm" type="button" @click="load" :disabled="loading">刷新</button>
    </div>

    <div class="monitor-body">
      <!-- 概览 -->
      <div v-if="tab === 'overview'" class="pane-inner">
        <div class="monitor-grid">
          <div class="metric-card">
            <div class="label">内存占用</div>
            <div class="value">{{ mem.usedText }}</div>
            <div class="mem-bar"><i :style="{ width: mem.pct + '%' }" /></div>
            <div class="sub">上限 {{ mem.maxText }} · {{ mem.pct }}%</div>
          </div>
          <div class="metric-card">
            <div class="label">运行时长</div>
            <div class="value">{{ uptime }}</div>
          </div>
          <div class="metric-card">
            <div class="label">客户端连接</div>
            <div class="value">{{ connectedClients }}</div>
          </div>
          <div class="metric-card">
            <div class="label">实时 QPS</div>
            <div class="value">{{ ops }}</div>
          </div>
          <div class="metric-card">
            <div class="label">当前库 key 数</div>
            <div class="value">{{ dbKeys }}</div>
            <div class="sub">过期 {{ expires }}</div>
          </div>
          <div class="metric-card">
            <div class="label">Redis 版本</div>
            <div class="value" style="font-size:16px">{{ version }}</div>
          </div>
        </div>
        <div class="info-section">
          <h4>Keyspace</h4>
          <table class="kv-table">
            <tr v-for="(row, i) in keyspaceRows" :key="i">
              <td>{{ row.k }}</td>
              <td>{{ row.v }}</td>
            </tr>
            <tr v-if="!keyspaceRows.length"><td colspan="2">无数据</td></tr>
          </table>
        </div>
      </div>

      <!-- INFO 全文分区 -->
      <div v-else-if="tab === 'info'" class="pane-inner">
        <div v-for="sec in sections" :key="sec.name" class="info-section">
          <h4>{{ sec.name }}</h4>
          <table class="kv-table">
            <tr v-for="(f, i) in sec.fields" :key="i">
              <td>{{ f.k }}</td>
              <td>{{ f.v }}</td>
            </tr>
          </table>
        </div>
        <div v-if="!sections.length" class="key-empty">暂无数据，点击刷新</div>
      </div>

      <!-- 慢日志 -->
      <div v-else-if="tab === 'slowlog'" class="pane-inner">
        <table class="vtable">
          <thead><tr><th>#</th><th>时间</th><th>耗时(μs)</th><th>命令</th></tr></thead>
          <tbody>
            <tr v-for="s in slowlog" :key="s.id">
              <td>{{ s.id }}</td>
              <td class="mono">{{ fmtTime(s.timestamp) }}</td>
              <td class="mono">{{ s.durationUs }}</td>
              <td class="mono">{{ s.command }}</td>
            </tr>
            <tr v-if="!slowlog.length"><td colspan="4" class="key-empty">无慢日志</td></tr>
          </tbody>
        </table>
      </div>

      <!-- 客户端 -->
      <div v-else-if="tab === 'clients'" class="pane-inner">
        <table class="vtable">
          <thead><tr><th>ID</th><th>地址</th><th>名称</th><th>库</th><th>时长</th><th>空闲</th><th>标志</th></tr></thead>
          <tbody>
            <tr v-for="c in clients" :key="c.id">
              <td class="mono">{{ c.id }}</td>
              <td class="mono">{{ c.addr }}</td>
              <td>{{ c.name || '—' }}</td>
              <td>{{ c.db }}</td>
              <td class="mono">{{ fmtDur(c.age) }}</td>
              <td class="mono">{{ fmtDur(c.idle) }}</td>
              <td class="mono">{{ c.flags }}</td>
            </tr>
            <tr v-if="!clients.length"><td colspan="7" class="key-empty">无客户端</td></tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { getClient, store, toast } from '../store'
import { parseInfo, computeMemoryUsage, infoField } from '../monitor'
import { formatBytes, formatUptime } from '../utils'
import type { InfoSection, SlowlogEntry, ClientInfo } from '../types'

const tab = ref('overview')
const loading = ref(false)
const infoText = ref('')
const sections = ref<InfoSection[]>([])
const slowlog = ref<SlowlogEntry[]>([])
const clients = ref<ClientInfo[]>([])

const mem = computed(() => {
  const m = computeMemoryUsage(infoText.value)
  return {
    usedText: formatBytes(m.usedBytes),
    maxText: m.maxBytes > 0 ? formatBytes(m.maxBytes) : '无限制',
    pct: Math.round(m.ratio * 100)
  }
})
const uptime = computed(() => {
  const v = infoField(infoText.value, 'Server', 'uptime_in_seconds')
  return v != null ? formatUptime(parseInt(v, 10)) : '—'
})
const version = computed(() => infoField(infoText.value, 'Server', 'redis_version') || '—')
const connectedClients = computed(() => infoField(infoText.value, 'Clients', 'connected_clients') || '—')
const ops = computed(() => infoField(infoText.value, 'Stats', 'instantaneous_ops_per_sec') || '—')
const dbKeys = computed(() => {
  const k = infoField(infoText.value, 'Keyspace', 'db' + store.currentDb)
  if (!k) return 0
  const m = /keys=(\d+)/.exec(k)
  return m ? m[1] : '—'
})
const expires = computed(() => {
  const k = infoField(infoText.value, 'Keyspace', 'db' + store.currentDb)
  if (!k) return 0
  const m = /expires=(\d+)/.exec(k)
  return m ? m[1] : '0'
})
const keyspaceRows = computed(() => {
  const sec = sections.value.find((s) => s.name === 'Keyspace')
  return sec ? sec.fields : []
})

function fmtTime(ts: number): string {
  const d = new Date(ts * 1000)
  return d.toLocaleString()
}
function fmtDur(sec: number): string {
  if (sec < 60) return sec + 's'
  if (sec < 3600) return Math.floor(sec / 60) + 'm'
  return Math.floor(sec / 3600) + 'h'
}

async function load() {
  const c = getClient()
  if (!c) {
    toast('未连接，无法加载监控', 'err')
    return
  }
  loading.value = true
  try {
    const [info, sl, cl] = await Promise.all([c.info(), c.slowlog(20), c.clientList()])
    infoText.value = info
    sections.value = parseInfo(info)
    slowlog.value = sl
    clients.value = cl
  } catch (e: any) {
    toast('监控加载失败: ' + (e.message || e), 'err')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  if (store.connected) load()
})
</script>

<style scoped>
.monitor { display: flex; flex-direction: column; height: 100%; }
.monitor-body { flex: 1; overflow: auto; padding: 14px; }
.pane-inner { display: block; }
.mono { font-family: var(--mono); font-size: 12px; }
</style>
