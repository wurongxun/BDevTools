<template>
  <ul class="key-list">
    <template v-if="store.groupBy && grouped">
      <template v-for="g in grouped" :key="g.group">
        <li class="key-group-title">{{ g.group }}（{{ g.items.length }}）</li>
        <li
          v-for="k in g.items"
          :key="k.name"
          class="key-item"
          :class="{ active: k.name === store.selectedKey, soon: isSoon(k) }"
          @click="selectKey(k.name)"
        >
          <input
            type="checkbox"
            :checked="store.selectedKeys.includes(k.name)"
            @click.stop
            @change="toggleSelect(k.name)"
          />
          <span class="kt" :style="{ background: color(k.type) }">{{ k.type }}</span>
          <span class="kn">{{ k.name }}</span>
          <span v-if="k.ttl > 0 && k.ttl < 60" class="expiry">⏱{{ k.ttl }}s</span>
        </li>
      </template>
    </template>
    <template v-else>
      <li
        v-for="k in store.keys"
        :key="k.name"
        class="key-item"
        :class="{ active: k.name === store.selectedKey, soon: isSoon(k) }"
        @click="selectKey(k.name)"
      >
        <input
          type="checkbox"
          :checked="store.selectedKeys.includes(k.name)"
          @click.stop
          @change="toggleSelect(k.name)"
        />
        <span class="kt" :style="{ background: color(k.type) }">{{ k.type }}</span>
        <span class="kn">{{ k.name }}</span>
        <span v-if="k.ttl > 0 && k.ttl < 60" class="expiry">⏱{{ k.ttl }}s</span>
      </li>
    </template>

    <li v-if="!store.keys.length && !store.loadingKeys" class="key-empty">没有匹配的 key</li>
    <li v-if="store.loadingKeys" class="key-empty">加载中…</li>
    <li v-if="store.hasMoreKeys" class="key-empty" style="cursor: pointer" @click="loadMoreKeys">
      加载更多…
    </li>
  </ul>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { store, groupedKeys, selectKey, toggleSelect, loadMoreKeys } from '../store'
import { TYPE_COLORS } from '../types'
import type { KeyMeta } from '../types'

const grouped = computed(() => groupedKeys.value)

function color(t: string): string {
  return (TYPE_COLORS as any)[t] || '#9aa0a6'
}
function isSoon(k: KeyMeta): boolean {
  return k.ttl > 0 && k.ttl < 60
}
</script>
