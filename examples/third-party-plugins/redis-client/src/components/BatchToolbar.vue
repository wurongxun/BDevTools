<template>
  <div v-if="store.selectedKeys.length" class="batch-bar">
    <span class="count">已选 {{ store.selectedKeys.length }}</span>
    <button class="mini-btn danger" type="button" @click="showDel = true">批量删除</button>
    <button class="mini-btn" type="button" @click="showExpire = true">批量设过期</button>
    <button class="mini-btn" type="button" @click="batchPersist">批量永久</button>
    <button class="mini-btn" type="button" @click="openExport">导出选中</button>
    <span class="spacer" />
    <button class="mini-btn" type="button" @click="clearSelection">清空选择</button>

    <Modal v-if="showDel" title="确认批量删除" @close="showDel = false">
      <div>确定删除选中的 {{ store.selectedKeys.length }} 个 key？此操作不可恢复。</div>
      <template #footer>
        <button class="btn" type="button" @click="showDel = false">取消</button>
        <button class="btn danger" type="button" @click="batchDelete">删除</button>
      </template>
    </Modal>

    <Modal v-if="showExpire" title="批量设置过期" @close="showExpire = false">
      <div class="field">
        <label>过期秒数</label>
        <input class="input" v-model="expireSec" placeholder="例如 3600" />
      </div>
      <template #footer>
        <button class="btn" type="button" @click="showExpire = false">取消</button>
        <button class="btn primary" type="button" @click="doExpire">确定</button>
      </template>
    </Modal>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import Modal from './Modal.vue'
import { store, clearSelection, getClient, refreshKeys, openDialog, toast, recordAudit } from '../store'

const showDel = ref(false)
const showExpire = ref(false)
const expireSec = ref('')

async function batchDelete() {
  const c = getClient()
  if (!c) return
  const keys = [...store.selectedKeys]
  const n = await c.batchDelete(keys)
  recordAudit('BATCH_DELETE', `批量删除 ${n} 个 key`, keys.join(','))
  toast(`已删除 ${n} 个 key`, 'ok')
  showDel.value = false
  clearSelection()
  refreshKeys()
}
async function doExpire() {
  const sec = parseInt(expireSec.value, 10)
  if (isNaN(sec) || sec <= 0) {
    toast('请输入有效秒数', 'err')
    return
  }
  const c = getClient()
  if (!c) return
  const keys = [...store.selectedKeys]
  const n = await c.batchExpire(keys, sec)
  recordAudit('BATCH_EXPIRE', `批量设置过期 ${sec}s × ${n} 个 key`, keys.join(','))
  toast(`已为 ${n} 个 key 设置过期`, 'ok')
  showExpire.value = false
  clearSelection()
  refreshKeys()
}
async function batchPersist() {
  const c = getClient()
  if (!c) return
  const keys = [...store.selectedKeys]
  const n = await c.batchPersist(keys)
  recordAudit('BATCH_PERSIST', `批量转永久 × ${n} 个 key`, keys.join(','))
  toast(`已为 ${n} 个 key 转为永久`, 'ok')
  clearSelection()
  refreshKeys()
}
function openExport() {
  openDialog('importExport', { mode: 'export', scope: 'selected' })
}
</script>
