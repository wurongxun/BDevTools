<template>
  <Modal :title="mode === 'import' ? '导入 Key' : '导出 Key'" @close="$emit('close')">
    <div class="ie-head">
      <div class="fmt-tabs">
        <button
          v-for="f in formats"
          :key="f.v"
          type="button"
          :class="{ active: fmt === f.v }"
          @click="fmt = f.v"
        >
          {{ f.label }}
        </button>
      </div>
      <span class="spacer" />
      <span class="meta">{{ scopeLabel }}</span>
    </div>

    <!-- 导出 -->
    <template v-if="mode === 'export'">
      <button class="btn primary" type="button" @click="doExport" :disabled="busy">生成导出内容</button>
      <textarea class="ie-box" v-model="output" readonly spellcheck="false"></textarea>
      <div class="row-actions">
        <button class="mini-btn" type="button" @click="copyOut">复制</button>
        <button class="mini-btn" type="button" @click="download">下载文件</button>
      </div>
    </template>

    <!-- 导入 -->
    <template v-else>
      <textarea class="ie-box" v-model="input" placeholder="在此粘贴 json / csv / redis 命令脚本" spellcheck="false"></textarea>
      <button class="btn primary" type="button" @click="doImport" :disabled="busy || !input.trim()">开始导入</button>
      <div v-if="importResult" class="import-result">
        <div :class="importResult.errors.length ? 'diff-chg' : 'diff-add'">
          成功导入 {{ importResult.count }} 个 key
        </div>
        <div v-for="(e, i) in importResult.errors" :key="i" class="diff-del">{{ e }}</div>
      </div>
    </template>
  </Modal>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import Modal from './Modal.vue'
import { store, getClient, toast, refreshKeys, recordAudit } from '../store'

defineEmits<{ (e: 'close'): void }>()

const dlg = computed(() => store.dialog?.props || {})
const mode = computed<'import' | 'export'>(() => dlg.value.mode === 'import' ? 'import' : 'export')
const scopeLabel = computed(() => {
  if (mode.value === 'import') return ''
  return dlg.value.scope === 'selected' ? `已选 ${store.selectedKeys.length} 个` : '当前库全部'
})

const formats: { v: 'json' | 'csv' | 'redis'; label: string }[] = [
  { v: 'json', label: 'JSON' },
  { v: 'csv', label: 'CSV' },
  { v: 'redis', label: 'Redis 命令' }
]
const fmt = ref<'json' | 'csv' | 'redis'>('json')

const output = ref('')
const input = ref('')
const busy = ref(false)
const importResult = ref<null | { count: number; errors: string[] }>(null)

async function doExport() {
  const c = getClient()
  if (!c) {
    toast('未连接', 'err')
    return
  }
  const keys = dlg.value.scope === 'selected' ? [...store.selectedKeys] : store.keys.map((k) => k.name)
  if (!keys.length) {
    toast('没有可导出的 key', 'err')
    return
  }
  busy.value = true
  try {
    output.value = await c.exportKeys(keys, fmt.value)
    recordAudit('EXPORT', `导出 ${keys.length} 个 key (${fmt.value})`, keys.join(','))
    toast('已生成导出内容', 'ok')
  } catch (e: any) {
    toast('导出失败: ' + (e.message || e), 'err')
  } finally {
    busy.value = false
  }
}

async function doImport() {
  const c = getClient()
  if (!c) {
    toast('未连接', 'err')
    return
  }
  busy.value = true
  try {
    const r = await c.importKeys(input.value, fmt.value)
    importResult.value = r
    if (r.count) {
      recordAudit('IMPORT', `导入 ${r.count} 个 key (${fmt.value})`, r.errors.length ? `失败 ${r.errors.length}` : '-')
      refreshKeys()
    } else {
      recordAudit('IMPORT', `导入 0 个 key (${fmt.value})`, '无有效条目')
    }
  } catch (e: any) {
    toast('导入失败: ' + (e.message || e), 'err')
  } finally {
    busy.value = false
  }
}

function copyOut() {
  if (!output.value) return
  const ta = document.createElement('textarea')
  ta.value = output.value
  document.body.appendChild(ta)
  ta.select()
  try {
    document.execCommand('copy')
    toast('已复制到剪贴板', 'ok')
  } catch {
    toast('复制失败，请手动选择', 'err')
  }
  document.body.removeChild(ta)
}

function download() {
  if (!output.value) return
  try {
    const blob = new Blob([output.value], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `redis-export.${fmt.value === 'redis' ? 'txt' : fmt.value}`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    setTimeout(() => URL.revokeObjectURL(url), 1000)
  } catch (e: any) {
    toast('下载失败: ' + (e.message || e), 'err')
  }
}
</script>

<style scoped>
.ie-head { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }
.meta { font-size: 11px; color: var(--text-2); }
.ie-box {
  width: 100%; height: 220px; resize: vertical;
  font-family: var(--mono); font-size: 11px; padding: 8px;
  border: 1px solid var(--border-strong); border-radius: var(--radius);
  background: var(--code-bg); color: var(--text);
}
.import-result { margin-top: 8px; font-size: 12px; }
</style>
