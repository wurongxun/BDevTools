<template>
  <div style="display: flex; align-items: center; gap: 6px">
    <select class="select" style="width: auto; max-width: 170px" :value="store.activeConnId" @change="onSelect">
      <option v-for="c in store.connections" :key="c.id" :value="c.id">{{ c.name }} ({{ c.mode }})</option>
    </select>
    <button class="icon-btn" type="button" title="新增连接" @click="openNew">＋</button>
    <button class="icon-btn" type="button" title="编辑连接" @click="openEdit">✎</button>
    <button class="icon-btn" type="button" title="删除连接" @click="onDelete">🗑</button>
    <button class="icon-btn" type="button" title="测试连接" @click="onTest">⇆</button>
  </div>
</template>

<script setup lang="ts">
import { store, connectById, deleteConnection, openDialog, toast } from '../store'

function onSelect(e: Event) {
  connectById((e.target as HTMLSelectElement).value)
}
function openNew() {
  openDialog('conn', {})
}
function openEdit() {
  const c = store.connections.find((c) => c.id === store.activeConnId)
  if (c) openDialog('conn', { conn: c })
}
function onDelete() {
  if (!store.activeConnId) return
  if (store.connections.length <= 1) {
    toast('至少保留一个连接', 'err')
    return
  }
  deleteConnection(store.activeConnId)
  toast('已删除连接')
}
async function onTest() {
  if (!store.activeConnId) return
  await connectById(store.activeConnId)
  toast(store.connected ? '连接成功' : '连接失败', store.connected ? 'ok' : 'err')
}
</script>
