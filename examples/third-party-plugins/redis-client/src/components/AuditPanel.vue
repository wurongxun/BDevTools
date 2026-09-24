<template>
  <div class="pane-inner">
    <div class="ops-head">
      <span class="muted">共 {{ store.audit.length }} 条操作记录（按时间倒序）</span>
      <span class="spacer" />
      <select class="select small" v-model="filter">
        <option value="">全部操作</option>
        <option v-for="op in ops" :key="op" :value="op">{{ op }}</option>
      </select>
      <select class="select small" v-model="expFmt">
        <option value="json">JSON</option>
        <option value="csv">CSV</option>
      </select>
      <button class="btn sm" type="button" @click="doExport">导出</button>
      <button class="btn sm danger" type="button" @click="clearAudit">清空</button>
    </div>

    <div v-if="filtered.length" class="audit-list">
      <div v-for="(e, i) in filtered" :key="i" class="audit-row" :class="e.status">
        <span class="ts mono">{{ fmtTime(e.ts) }}</span>
        <span class="op" :class="'op-' + e.op">{{ e.op }}</span>
        <span class="target mono" v-if="e.target !== '-'">{{ e.target }}</span>
        <span class="detail">{{ e.detail }}</span>
        <span class="st" :class="e.status">{{ e.status === 'ok' ? '✓' : '✗' }}</span>
      </div>
    </div>
    <div v-else class="key-empty">暂无审计记录，执行任意命令或操作后此处会出现记录</div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { store, clearAudit, exportAudit, toast } from '../store'
import type { AuditOp } from '../types'

const filter = ref('')
const expFmt = ref<'json' | 'csv'>('json')
const ops: AuditOp[] = [
  'CMD', 'DEL', 'BATCH_DELETE', 'BATCH_EXPIRE', 'BATCH_PERSIST',
  'RENAME', 'COPY', 'EXPIRE', 'PERSIST', 'IMPORT', 'EXPORT'
]

const filtered = computed(() => (filter.value ? store.audit.filter((e) => e.op === filter.value) : store.audit))

function fmtTime(ts: number): string {
  const d = new Date(ts)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
}

function doExport() {
  try {
    const text = exportAudit(expFmt.value)
    const blob = new Blob([text], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `redis-audit.${expFmt.value}`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    setTimeout(() => URL.revokeObjectURL(url), 1000)
    toast('审计日志已导出', 'ok')
  } catch (e: any) {
    toast('导出失败: ' + (e.message || e), 'err')
  }
}
</script>

<style scoped>
.ops-head { display: flex; align-items: center; gap: 8px; margin-bottom: 12px; }
.spacer { flex: 1; }
.muted { color: var(--text-2); font-size: 12px; }
.audit-list { display: flex; flex-direction: column; gap: 2px; font-size: 12px; }
.audit-row {
  display: flex; align-items: center; gap: 10px;
  padding: 6px 8px; border-radius: 4px; background: var(--panel);
  border-left: 3px solid var(--border);
}
.audit-row.err { border-left-color: var(--danger); }
.ts { color: var(--text-2); flex: 0 0 auto; }
.op {
  flex: 0 0 auto; font-size: 10px; font-weight: 600; padding: 1px 7px; border-radius: 3px;
  background: var(--accent-soft); color: var(--accent);
}
.op-BATCH_DELETE, .op-DEL { background: color-mix(in srgb, var(--danger) 18%, transparent); color: var(--danger); }
.op-IMPORT, .op-EXPORT { background: color-mix(in srgb, var(--ok) 18%, transparent); color: var(--ok); }
.target { flex: 0 0 auto; max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--accent); }
.detail { flex: 1; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.st { flex: 0 0 auto; font-weight: 700; }
.st.ok { color: var(--ok); }
.st.err { color: var(--danger); }
.mono { font-family: var(--mono); }
</style>
