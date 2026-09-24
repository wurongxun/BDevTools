<template>
  <div v-if="store.selectedKey" class="key-info">
    <span class="kname">{{ store.selectedKey }}</span>
    <span class="ki"><b>类型</b>{{ info?.type || '—' }}</span>
    <span class="ki"><b>内存</b>{{ sizeText }}</span>

    <!-- TTL 可视化：倒计时 + 进度条 -->
    <span class="ki">
      <b>TTL</b>
      <template v-if="info && info.ttl === -1">永久</template>
      <template v-else-if="info && info.ttl === -2">不存在</template>
      <template v-else>
        {{ remaining }}s
        <span class="ttl-bar" :class="{ warn: isWarn }"><i :style="{ width: progressPct }" /></span>
      </template>
    </span>

    <span class="spacer" />

    <template v-if="info && (info.type === 'string')">
      <input class="input" style="width: 90px" v-model="ttlInput" placeholder="过期秒数" />
      <button class="mini-btn" type="button" @click="setTtl">设过期</button>
      <button class="mini-btn" type="button" @click="persist">永久</button>
    </template>
    <button class="mini-btn" type="button" @click="showRename = true">重命名</button>
    <button class="mini-btn" type="button" @click="showCopy = true">复制</button>
    <button class="mini-btn" type="button" @click="openDialog('compare', { keyA: store.selectedKey })">对比</button>
    <button class="mini-btn danger" type="button" @click="delKey">删除</button>

    <!-- 重命名 -->
    <Modal v-if="showRename" title="重命名 Key" @close="showRename = false">
      <div class="field">
        <label>新名称</label>
        <input class="input" v-model="renameTo" :placeholder="store.selectedKey || ''" />
      </div>
      <template #footer>
        <button class="btn" type="button" @click="showRename = false">取消</button>
        <button class="btn primary" type="button" @click="doRename">确定</button>
      </template>
    </Modal>

    <!-- 复制 -->
    <Modal v-if="showCopy" title="复制 Key" @close="showCopy = false">
      <div class="field">
        <label>目标名称（同实例）</label>
        <input class="input" v-model="copyTo" :placeholder="(store.selectedKey || '') + '_copy'" />
      </div>
      <template #footer>
        <button class="btn" type="button" @click="showCopy = false">取消</button>
        <button class="btn primary" type="button" @click="doCopy">复制</button>
      </template>
    </Modal>
  </div>
  <div v-else class="key-info"><span class="kname" style="color: var(--text-2)">未选择 key</span></div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import Modal from './Modal.vue'
import { store, getClient, reloadKeyInfo, refreshKeys, selectKey, toast, openDialog, recordAudit } from '../store'
import { formatBytes } from '../utils'

const info = computed(() => store.keyInfo)
const sizeText = computed(() => (info.value?.size != null ? formatBytes(info.value.size) : '—'))

const remaining = computed(() => {
  if (!info.value) return -2
  const ttl = info.value.ttl
  if (ttl < 0) return ttl
  const elapsed = Math.floor((Date.now() - store.keyLoadedAt) / 1000)
  return Math.max(0, ttl - elapsed)
})
const progressPct = computed(() => {
  if (!info.value || info.value.ttl <= 0) return '0%'
  const p = remaining.value / info.value.ttl
  return Math.max(0, Math.min(1, p)) * 100 + '%'
})
const isWarn = computed(() => remaining.value > 0 && remaining.value < 60)

const ttlInput = ref('')
const showRename = ref(false)
const showCopy = ref(false)
const renameTo = ref('')
const copyTo = ref('')

async function setTtl() {
  const sec = parseInt(ttlInput.value, 10)
  if (isNaN(sec) || sec <= 0) {
    toast('请输入有效秒数', 'err')
    return
  }
  const c = getClient()
  if (!c || !store.selectedKey) return
  const ok = await c.expire(store.selectedKey, sec)
  if (ok) {
    recordAudit('EXPIRE', `EXPIRE ${store.selectedKey} ${sec}`, store.selectedKey)
    toast('已设置过期时间', 'ok')
    reloadKeyInfo()
    refreshKeys()
  }
}
async function persist() {
  const c = getClient()
  if (!c || !store.selectedKey) return
  const ok = await c.persist(store.selectedKey)
  if (ok) {
    recordAudit('PERSIST', `PERSIST ${store.selectedKey}`, store.selectedKey)
    toast('已转为永久', 'ok')
    reloadKeyInfo()
    refreshKeys()
  }
}
async function delKey() {
  const c = getClient()
  if (!c || !store.selectedKey) return
  const k = store.selectedKey
  await c.del(k)
  recordAudit('DEL', `DEL ${k}`, k)
  toast('已删除 ' + k, 'ok')
  store.selectedKey = null
  store.keyInfo = null
  refreshKeys()
}
async function doRename() {
  const c = getClient()
  if (!c || !store.selectedKey) return
  const oldK = store.selectedKey
  const newK = renameTo.value.trim()
  if (!newK || newK === oldK) {
    showRename.value = false
    return
  }
  await c.rename(oldK, newK)
  recordAudit('RENAME', `RENAME ${oldK} ${newK}`, oldK)
  toast('已重命名为 ' + newK, 'ok')
  showRename.value = false
  selectKey(newK)
  refreshKeys()
}
async function doCopy() {
  const c = getClient()
  if (!c || !store.selectedKey) return
  const oldK = store.selectedKey
  const newK = copyTo.value.trim() || oldK + '_copy'
  const ok = await c.copy(oldK, newK, true)
  if (ok) {
    recordAudit('COPY', `COPY ${oldK} ${newK}`, newK)
    toast('已复制到 ' + newK, 'ok')
  } else {
    toast('复制失败（目标已存在且未覆盖）', 'err')
  }
  showCopy.value = false
  refreshKeys()
}
</script>
