<template>
  <section class="db-picker">
    <div class="section-label">数据库（key 数）</div>
    <div class="db-grid">
      <div
        v-for="db in 16"
        :key="db"
        class="db-cell"
        :class="{ active: store.currentDb === db - 1 }"
        @click="selectDb(db - 1)"
      >
        <span class="n">{{ db - 1 }}</span>
        <span class="c">{{ store.dbCounts[db - 1] ?? 0 }}</span>
      </div>
    </div>
  </section>

  <section class="key-tools">
    <input class="input" :value="store.search" @input="onSearch" placeholder="搜索 key（支持 * 通配）" />
    <div class="row">
      <select class="select small" :value="store.typeFilter" @change="onType">
        <option value="">全部类型</option>
        <option value="string">string</option>
        <option value="hash">hash</option>
        <option value="list">list</option>
        <option value="set">set</option>
        <option value="zset">zset</option>
        <option value="stream">stream</option>
      </select>
      <label class="chk">
        <input type="checkbox" :checked="store.groupBy" @change="onGroup" /> 前缀分组
      </label>
    </div>
    <div class="key-meta">
      {{ store.keys.length }} 个 key{{ store.hasMoreKeys ? '（还有更多，可加载）' : '' }}
    </div>
    <div class="row">
      <button class="mini-btn" type="button" @click="openDialog('importExport', { mode: 'import' })">导入</button>
      <button class="mini-btn" type="button" @click="openDialog('importExport', { mode: 'export', scope: 'all' })">导出全部</button>
    </div>
  </section>

  <KeyList />
  <BatchToolbar />
</template>

<script setup lang="ts">
import { store, selectDb, setSearch, setTypeFilter, setGroupBy, openDialog } from '../store'
import KeyList from './KeyList.vue'
import BatchToolbar from './BatchToolbar.vue'

let timer: any = null
function onSearch(e: Event) {
  const v = (e.target as HTMLInputElement).value
  clearTimeout(timer)
  timer = setTimeout(() => setSearch(v), 250)
}
function onType(e: Event) {
  setTypeFilter((e.target as HTMLSelectElement).value)
}
function onGroup(e: Event) {
  setGroupBy((e.target as HTMLInputElement).checked)
}
</script>
