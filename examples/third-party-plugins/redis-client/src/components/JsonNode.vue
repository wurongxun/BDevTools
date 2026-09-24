<template>
  <div class="jt-node">
    <div class="jt-row">
      <span
        v-if="isContainer"
        class="jt-toggle"
        type="button"
        role="button"
        :aria-expanded="open"
        @click="open = !open"
        >{{ open ? '▾' : '▸' }}</span
      >
      <span v-else class="jt-toggle"></span>
      <span v-if="keyName !== null" class="jt-key">{{ keyName }}</span>
      <template v-if="isContainer">
        <span class="jt-punc">{{ open ? (isArray ? '[' : '{') : collapsedHint }}</span>
      </template>
      <template v-else>
        <span :class="valueClass">{{ displayValue }}</span>
      </template>
    </div>

    <template v-if="isContainer && open">
      <JsonNode
        v-for="(item, k) in childEntries"
        :key="String(k)"
        :key-name="isArray ? null : String(k)"
        :value="item"
        :depth="depth + 1"
      />
      <div class="jt-row jt-close"><span class="jt-toggle"></span><span class="jt-punc">{{ isArray ? ']' : '}' }}</span></div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import JsonNode from './JsonNode.vue'

const props = defineProps<{
  keyName: string | null
  value: any
  depth: number
}>()

const open = ref(props.depth < 2) // 默认展开到第二层
const isContainer = computed(() => props.value !== null && typeof props.value === 'object')
const isArray = computed(() => Array.isArray(props.value))

const childEntries = computed<any[] | Record<string, any>>(() => {
  if (!isContainer.value) return []
  return props.value as any
})

const collapsedHint = computed(() => {
  const n = isArray.value ? (props.value as any[]).length : Object.keys(props.value).length
  return isArray.value ? `[…(${n})]` : `{…(${n})}}`
})

const valueClass = computed(() => {
  const v = props.value
  if (v === null) return 'jt-null'
  switch (typeof v) {
    case 'string':
      return 'jt-str'
    case 'number':
      return 'jt-num'
    case 'boolean':
      return 'jt-bool'
    default:
      return 'jt-null'
  }
})

const displayValue = computed(() => {
  const v = props.value
  if (v === null) return 'null'
  if (typeof v === 'string') return JSON.stringify(v)
  return String(v)
})
</script>
