<template>
  <Modal title="值对比" @close="$emit('close')">
    <div class="cmp-form">
      <div class="field-row">
        <div class="field">
          <label>Key A</label>
          <input class="input" v-model="keyA" placeholder="左侧 key" />
        </div>
        <div class="field">
          <label>Key B</label>
          <input class="input" v-model="keyB" placeholder="右侧 key" />
        </div>
      </div>
      <button class="btn primary" type="button" @click="run" :disabled="busy || !keyA || !keyB">开始对比</button>
    </div>

    <div v-if="result" class="cmp-result">
      <div class="cmp-head">{{ result.mode === 'json' ? 'JSON 结构差异' : '文本行差异' }}</div>

      <div v-if="result.mode === 'text'" class="diff-box">
        <div v-for="(l, i) in result.lines" :key="i" :class="'diff-' + l.kind">
          {{ l.kind === 'add' ? '+ ' : l.kind === 'del' ? '- ' : '  ' }}{{ l.text }}
        </div>
      </div>

      <div v-else class="diff-box">
        <div v-for="(c, i) in (result.changes || [])" :key="i" :class="'diff-' + (c.type === 'changed' ? 'chg' : c.type === 'added' ? 'add' : 'del')">
          <b>{{ c.path }}</b> [{{ c.type }}]
          <template v-if="c.type === 'changed'"> {{ jsonVal(c.from) }} → {{ jsonVal(c.to) }}</template>
          <template v-else-if="c.type === 'added'"> = {{ jsonVal(c.to) }}</template>
          <template v-else> (已移除) {{ jsonVal(c.from) }}</template>
        </div>
        <div v-if="!(result.changes && result.changes.length)" class="diff-eq">两个值完全一致 ✓</div>
      </div>
    </div>
  </Modal>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import Modal from './Modal.vue'
import { store, getClient, toast } from '../store'

defineEmits<{ (e: 'close'): void }>()

const keyA = ref('')
const keyB = ref('')
const busy = ref(false)
const result = ref<null | { mode: 'text' | 'json'; lines?: any[]; changes?: any[] }>(null)

// 初始化：若对话框携带 keyA 则预填
if (store.dialog?.props?.keyA) keyA.value = store.dialog.props.keyA

function jsonVal(v: any): string {
  if (v === undefined) return ''
  if (typeof v === 'string') return JSON.stringify(v)
  return JSON.stringify(v)
}

async function run() {
  const c = getClient()
  if (!c) {
    toast('未连接', 'err')
    return
  }
  busy.value = true
  try {
    result.value = (await c.diffValues(keyA.value, keyB.value)) as any
  } catch (e: any) {
    toast('对比失败: ' + (e.message || e), 'err')
  } finally {
    busy.value = false
  }
}
</script>

<style scoped>
.cmp-form { display: flex; flex-direction: column; gap: 10px; }
.cmp-result { margin-top: 6px; }
.cmp-head { font-size: 12px; color: var(--text-2); margin-bottom: 6px; }
.diff-eq { color: var(--ok); padding: 4px 0; }
</style>
