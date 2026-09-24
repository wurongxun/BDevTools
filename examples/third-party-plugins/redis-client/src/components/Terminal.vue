<template>
  <div class="term">
    <div class="term-output" ref="out" aria-live="polite">
      <div v-for="(l, i) in store.terminalLines" :key="i" :class="'term-line-' + l.kind">{{ l.text }}</div>
      <div v-if="!store.terminalLines.length" class="term-line-info">
        命令行已就绪。输入 Redis 命令，如 <code>GET foo</code> / <code>HGETALL user:1</code> / <code>SCAN 0 MATCH user:*</code>。
        ↑/↓ 浏览历史，Tab 自动补全。
      </div>
    </div>
    <div class="term-input-row">
      <span class="prompt">redis&gt;</span>
      <input
        class="input term-input"
        v-model="line"
        type="text"
        autocomplete="off"
        spellcheck="false"
        placeholder="输入 Redis 命令"
        aria-label="命令输入"
        @keydown="onKey"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { store, runCommand, pushHistory, historyPrev, historyNext } from '../store'
import { completeCommand } from '../client'

const line = ref('')
const out = ref<HTMLElement | null>(null)

function scrollBottom() {
  nextTick(() => {
    if (out.value) out.value.scrollTop = out.value.scrollHeight
  })
}
watch(() => store.terminalLines.length, scrollBottom)

function submit() {
  const text = line.value
  if (!text.trim()) return
  runCommand(text)
  pushHistory(text)
  line.value = ''
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    e.preventDefault()
    submit()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    const h = historyPrev()
    if (h != null) line.value = h
  } else if (e.key === 'ArrowDown') {
    e.preventDefault()
    const h = historyNext()
    if (h != null) line.value = h
  } else if (e.key === 'Tab') {
    e.preventDefault()
    complete()
  }
}

function complete() {
  const parts = line.value.split(/\s+/)
  const last = parts[parts.length - 1] || ''
  const matches = completeCommand(last)
  if (matches.length === 1) {
    parts[parts.length - 1] = matches[0]
    line.value = parts.join(' ')
  } else if (matches.length > 1) {
    store.terminalLines.push({ kind: 'info', text: matches.join('   ') })
    scrollBottom()
  }
}
</script>
