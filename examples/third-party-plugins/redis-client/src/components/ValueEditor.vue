<template>
  <div class="ve" v-if="key">
    <!-- 字符串：多格式视图 + 编辑 -->
    <div v-if="type === 'string'" class="ve-str">
      <div class="str-toolbar">
        <div class="fmt-tabs">
          <button
            v-for="f in formats"
            :key="f.v"
            type="button"
            :class="{ active: fmt === f.v }"
            @click="setFmt(f.v)"
          >
            {{ f.label }}
          </button>
        </div>

        <template v-if="fmt === 'json'">
          <button class="mini-btn" type="button" @click="jsonTree = !jsonTree">
            {{ jsonTree ? '文本视图' : 'JSON 树' }}
          </button>
        </template>

        <span class="spacer" />
        <span class="meta">{{ linesLabel }}</span>
        <button class="btn sm primary" type="button" @click="saveString" :disabled="saving">保存</button>
      </div>

      <!-- 错误提示 -->
      <div v-if="jsonError" class="json-err">⚠ {{ jsonError }}</div>

      <!-- 十六进制视图（只读） -->
      <div v-if="fmt === 'hex'" class="hex-view">
        <div v-for="(row, i) in hexRows" :key="i" class="hex-row">
          <span class="hex-off">{{ row.offset }}</span>
          <span class="hex-bytes">{{ row.bytes }}</span>
          <span class="hex-ascii">{{ row.ascii }}</span>
        </div>
        <div class="pager" v-if="hasMoreHex">
          <span>已显示 {{ shownHex }} / {{ hexAll.length }} 字节</span>
          <span class="spacer" />
          <button class="mini-btn" type="button" @click="shownHex += hexPage">加载更多</button>
        </div>
      </div>

      <!-- JSON 树 -->
      <div v-else-if="fmt === 'json' && jsonTree && !jsonError" class="jt">
        <JsonNode :key-name="null" :value="parsedJson" :depth="0" />
      </div>

      <!-- 纯文本 / XML / YAML / JSON 文本 -->
      <template v-else>
        <textarea class="str-area" v-model="editText" spellcheck="false"></textarea>
        <div class="pager" v-if="hasMoreLines">
          <span>已显示 {{ shownLines }} / {{ totalLines }} 行</span>
          <span class="spacer" />
          <button class="mini-btn" type="button" @click="shownLines += linePage">加载更多</button>
        </div>
      </template>
    </div>

    <!-- Hash 编辑器 -->
    <div v-else-if="type === 'hash'" class="ve-table">
      <div class="editor-toolbar">
        <span class="meta">{{ hashRows.length }} 个字段</span>
        <span class="spacer" />
        <button class="btn sm" type="button" @click="reload">刷新</button>
      </div>
      <table class="vtable">
        <thead>
          <tr><th>字段</th><th>值</th><th style="width:120px">操作</th></tr>
        </thead>
        <tbody>
          <tr v-for="r in hashRows" :key="r.field">
            <td class="mono">{{ r.field }}</td>
            <td><input class="cell-input" v-model="r.draftValue" @blur="saveHash(r)" /></td>
            <td>
              <div class="row-actions">
                <button class="mini-btn" type="button" @click="saveHash(r)">存</button>
                <button class="mini-btn danger" type="button" @click="delHash(r.field)">删</button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
      <div class="add-row">
        <input class="input" v-model="newHashField" placeholder="字段名" />
        <input class="input" v-model="newHashValue" placeholder="值" @keyup.enter="addHash" />
        <button class="btn" type="button" @click="addHash">添加</button>
      </div>
    </div>

    <!-- List 编辑器 -->
    <div v-else-if="type === 'list'" class="ve-table">
      <div class="editor-toolbar">
        <span class="meta">{{ listItems.length }} 个元素</span>
        <span class="spacer" />
        <button class="btn sm" type="button" @click="reload">刷新</button>
      </div>
      <table class="vtable">
        <thead>
          <tr><th style="width:60px">#</th><th>值</th><th style="width:120px">操作</th></tr>
        </thead>
        <tbody>
          <tr v-for="(it, i) in listItems" :key="i">
            <td class="idx">{{ i }}</td>
            <td><input class="cell-input" v-model="it.draft" @blur="saveList(i)" /></td>
            <td>
              <div class="row-actions">
                <button class="mini-btn" type="button" @click="saveList(i)">存</button>
                <button class="mini-btn danger" type="button" @click="delList(it.value)">删</button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
      <div class="add-row">
        <input class="input" v-model="newListItem" placeholder="新元素值" @keyup.enter="addList" />
        <button class="btn" type="button" @click="addList">右侧推入</button>
      </div>
    </div>

    <!-- Set 编辑器 -->
    <div v-else-if="type === 'set'" class="ve-table">
      <div class="editor-toolbar">
        <span class="meta">{{ setMembers.length }} 个成员</span>
        <span class="spacer" />
        <button class="btn sm" type="button" @click="reload">刷新</button>
      </div>
      <table class="vtable">
        <thead>
          <tr><th>成员</th><th style="width:120px">操作</th></tr>
        </thead>
        <tbody>
          <tr v-for="m in setMembers" :key="m.value">
            <td>
              <input class="cell-input" v-model="m.draft" @blur="saveSet(m)" />
            </td>
            <td>
              <div class="row-actions">
                <button class="mini-btn" type="button" @click="saveSet(m)">存</button>
                <button class="mini-btn danger" type="button" @click="delSet(m.value)">删</button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
      <div class="add-row">
        <input class="input" v-model="newSetMember" placeholder="新成员" @keyup.enter="addSet" />
        <button class="btn" type="button" @click="addSet">添加</button>
      </div>
    </div>

    <!-- ZSet 编辑器 -->
    <div v-else-if="type === 'zset'" class="ve-table">
      <div class="editor-toolbar">
        <span class="meta">{{ zsetMembers.length }} 个成员</span>
        <span class="spacer" />
        <button class="btn sm" type="button" @click="reload">刷新</button>
      </div>
      <table class="vtable">
        <thead>
          <tr><th>成员</th><th style="width:120px">分值</th><th style="width:120px">操作</th></tr>
        </thead>
        <tbody>
          <tr v-for="m in zsetMembers" :key="m.member">
            <td><input class="cell-input" v-model="m.draftMember" @blur="saveZSet(m)" /></td>
            <td><input class="cell-input" v-model="m.draftScore" @blur="saveZSet(m)" /></td>
            <td>
              <div class="row-actions">
                <button class="mini-btn" type="button" @click="saveZSet(m)">存</button>
                <button class="mini-btn danger" type="button" @click="delZSet(m.member)">删</button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
      <div class="add-row">
        <input class="input" style="flex:.4" v-model="newZMember" placeholder="成员" />
        <input class="input" style="flex:.3" v-model="newZScore" placeholder="分值" @keyup.enter="addZSet" />
        <button class="btn" type="button" @click="addZSet">添加</button>
      </div>
    </div>

    <!-- Stream 编辑器 -->
    <div v-else-if="type === 'stream'" class="ve-table">
      <div class="editor-toolbar">
        <span class="meta">{{ streamEntries.length }} 条条目</span>
        <span class="spacer" />
        <button class="btn sm" type="button" @click="reload">刷新</button>
      </div>
      <table class="vtable">
        <thead>
          <tr><th style="width:200px">ID</th><th>字段</th><th style="width:90px">操作</th></tr>
        </thead>
        <tbody>
          <tr v-for="e in streamEntries" :key="e.id">
            <td class="mono">{{ e.id }}</td>
            <td>
              <span v-for="(v, f) in e.fields" :key="f" class="kv-pill">
                <b>{{ f }}</b>={{ v }}
              </span>
            </td>
            <td><button class="mini-btn danger" type="button" @click="delStream(e.id)">删</button></td>
          </tr>
        </tbody>
      </table>
      <div class="add-row">
        <input class="input" style="flex:.4" v-model="newStreamField" placeholder="字段名" />
        <input class="input" style="flex:.4" v-model="newStreamValue" placeholder="字段值" />
        <button class="btn" type="button" @click="addStream">追加条目</button>
      </div>
    </div>

    <div v-else class="placeholder">未知类型：{{ type }}</div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import JsonNode from './JsonNode.vue'
import { store, getClient, reloadKeyInfo, refreshKeys, toast } from '../store'
import { toUtf8Bytes, formatBytes } from '../utils'
import type { ValueFormat, RedisType, StreamEntry } from '../types'

const key = computed(() => store.selectedKey)
const type = computed<RedisType>(() => (store.keyInfo?.type as RedisType) || 'none')

const formats: { v: ValueFormat; label: string }[] = [
  { v: 'text', label: 'Text' },
  { v: 'json', label: 'JSON' },
  { v: 'xml', label: 'XML' },
  { v: 'yaml', label: 'YAML' },
  { v: 'hex', label: 'Hex' }
]
const fmt = ref<ValueFormat>('text')
const jsonTree = ref(false)
const saving = ref(false)

const editText = ref('')
const jsonError = ref('')
const parsedJson = ref<any>(null)
const linePage = 200
const shownLines = ref(linePage)
const totalLines = computed(() => (editText.value ? editText.value.split('\n').length : 0))
const hasMoreLines = computed(() => shownLines.value < totalLines.value)
const linesLabel = computed(() => {
  const byteCount = toUtf8Bytes(editText.value).length
  return `${totalLines.value} 行 · ${formatBytes(byteCount)}`
})

const setFmt = (f: ValueFormat) => {
  fmt.value = f
  if (f === 'json') validateJson()
  if (f !== 'hex') shownLines.value = linePage
}

function validateJson() {
  jsonError.value = ''
  parsedJson.value = null
  if (fmt.value !== 'json') return
  const t = editText.value.trim()
  if (!t) return
  try {
    parsedJson.value = JSON.parse(t)
  } catch (e: any) {
    jsonError.value = e.message
  }
}

// ---- 十六进制视图 ----
const hexAll = computed(() => toUtf8Bytes(editText.value))
const hexPage = 1024
const shownHex = ref(hexPage)
const hasMoreHex = computed(() => shownHex.value < hexAll.value.length)
function toPrintable(b: number): string {
  return b >= 0x20 && b < 0x7f ? String.fromCharCode(b) : '.'
}
const hexRows = computed(() => {
  const bytes = hexAll.value.slice(0, shownHex.value)
  const rows: { offset: string; bytes: string; ascii: string }[] = []
  for (let i = 0; i < bytes.length; i += 16) {
    const slice = bytes.slice(i, i + 16)
    const hex = Array.from(slice)
      .map((b) => b.toString(16).padStart(2, '0'))
      .join(' ')
    const ascii = Array.from(slice)
      .map((b) => toPrintable(b))
      .join('')
    rows.push({ offset: i.toString(16).padStart(8, '0'), bytes: hex, ascii })
  }
  return rows
})

async function saveString() {
  const c = getClient()
  if (!c || !key.value) return
  if (fmt.value === 'json') {
    validateJson()
    if (jsonError.value) {
      toast('JSON 格式错误，未保存', 'err')
      return
    }
  }
  if (fmt.value === 'hex') {
    toast('Hex 视图为只读，请切回文本/JSON 编辑', 'err')
    return
  }
  saving.value = true
  try {
    await c.set(key.value, editText.value)
    toast('已保存', 'ok')
    reloadKeyInfo()
    refreshKeys()
  } finally {
    saving.value = false
  }
}

// ---- Hash ----
interface HashRow { field: string; value: string; draftValue: string }
const hashRows = ref<HashRow[]>([])
const newHashField = ref('')
const newHashValue = ref('')
async function saveHash(r: HashRow) {
  const c = getClient()
  if (!c || !key.value) return
  await c.hset(key.value, r.field, r.draftValue)
  r.value = r.draftValue
  toast('已更新字段', 'ok')
  reloadKeyInfo()
}
async function delHash(field: string) {
  const c = getClient()
  if (!c || !key.value) return
  await c.hdel(key.value, field)
  hashRows.value = hashRows.value.filter((r) => r.field !== field)
  toast('已删除字段', 'ok')
  reloadKeyInfo()
  refreshKeys()
}
async function addHash() {
  const c = getClient()
  if (!c || !key.value || !newHashField.value) return
  await c.hset(key.value, newHashField.value, newHashValue.value)
  hashRows.value.push({ field: newHashField.value, value: newHashValue.value, draftValue: newHashValue.value })
  newHashField.value = ''
  newHashValue.value = ''
  reloadKeyInfo()
}

// ---- List ----
interface ListItem { value: string; draft: string }
const listItems = ref<ListItem[]>([])
const newListItem = ref('')
async function saveList(i: number) {
  const c = getClient()
  if (!c || !key.value) return
  await c.lset(key.value, i, listItems.value[i].draft)
  listItems.value[i].value = listItems.value[i].draft
  toast('已更新元素', 'ok')
  reloadKeyInfo()
}
async function delList(value: string) {
  const c = getClient()
  if (!c || !key.value) return
  await c.lrem(key.value, value)
  listItems.value = listItems.value.filter((it) => it.value !== value)
  toast('已删除元素', 'ok')
  reloadKeyInfo()
  refreshKeys()
}
async function addList() {
  const c = getClient()
  if (!c || !key.value || !newListItem.value) return
  await c.rpush(key.value, newListItem.value)
  listItems.value.push({ value: newListItem.value, draft: newListItem.value })
  newListItem.value = ''
  reloadKeyInfo()
}

// ---- Set ----
interface SetMember { value: string; draft: string }
const setMembers = ref<SetMember[]>([])
const newSetMember = ref('')
async function saveSet(m: SetMember) {
  const c = getClient()
  if (!c || !key.value) return
  if (m.draft === m.value) return
  await c.srem(key.value, m.value)
  await c.sadd(key.value, m.draft)
  m.value = m.draft
  toast('已更新成员', 'ok')
  reloadKeyInfo()
}
async function delSet(value: string) {
  const c = getClient()
  if (!c || !key.value) return
  await c.srem(key.value, value)
  setMembers.value = setMembers.value.filter((m) => m.value !== value)
  toast('已删除成员', 'ok')
  reloadKeyInfo()
  refreshKeys()
}
async function addSet() {
  const c = getClient()
  if (!c || !key.value || !newSetMember.value) return
  await c.sadd(key.value, newSetMember.value)
  setMembers.value.push({ value: newSetMember.value, draft: newSetMember.value })
  newSetMember.value = ''
  reloadKeyInfo()
}

// ---- ZSet ----
interface ZMember { member: string; score: number; draftMember: string; draftScore: string }
const zsetMembers = ref<ZMember[]>([])
const newZMember = ref('')
const newZScore = ref('')
async function saveZSet(m: ZMember) {
  const c = getClient()
  if (!c || !key.value) return
  const score = parseFloat(m.draftScore)
  if (isNaN(score)) {
    toast('分值必须是数字', 'err')
    return
  }
  if (m.draftMember !== m.member) await c.zrem(key.value, m.member)
  await c.zadd(key.value, m.draftMember, score)
  m.member = m.draftMember
  m.score = score
  toast('已更新成员', 'ok')
  reloadKeyInfo()
}
async function delZSet(member: string) {
  const c = getClient()
  if (!c || !key.value) return
  await c.zrem(key.value, member)
  zsetMembers.value = zsetMembers.value.filter((m) => m.member !== member)
  toast('已删除成员', 'ok')
  reloadKeyInfo()
  refreshKeys()
}
async function addZSet() {
  const c = getClient()
  if (!c || !key.value || !newZMember.value) return
  const score = parseFloat(newZScore.value)
  if (isNaN(score)) {
    toast('分值必须是数字', 'err')
    return
  }
  await c.zadd(key.value, newZMember.value, score)
  zsetMembers.value.push({ member: newZMember.value, score, draftMember: newZMember.value, draftScore: String(score) })
  newZMember.value = ''
  newZScore.value = ''
  reloadKeyInfo()
}

// ---- Stream ----
const streamEntries = ref<StreamEntry[]>([])
const newStreamField = ref('')
const newStreamValue = ref('')
async function delStream(id: string) {
  const c = getClient()
  if (!c || !key.value) return
  await c.xdel(key.value, id)
  streamEntries.value = streamEntries.value.filter((e) => e.id !== id)
  toast('已删除条目', 'ok')
  reloadKeyInfo()
  refreshKeys()
}
async function addStream() {
  const c = getClient()
  if (!c || !key.value || !newStreamField.value) return
  await c.xadd(key.value, { [newStreamField.value]: newStreamValue.value })
  newStreamField.value = ''
  newStreamValue.value = ''
  await reloadActual()
  reloadKeyInfo()
}

async function reloadActual() {
  const c = getClient()
  if (!c || !key.value) return
  const t = type.value
  if (t === 'string') {
    const raw = await c.get(key.value)
    editText.value = raw == null ? '' : raw
    shownLines.value = linePage
    validateJson()
  } else if (t === 'hash') {
    const o = await c.hgetall(key.value)
    hashRows.value = Object.keys(o).map((f) => ({ field: f, value: o[f], draftValue: o[f] }))
  } else if (t === 'list') {
    const arr = await c.lrange(key.value, 0, -1)
    listItems.value = arr.map((v) => ({ value: v, draft: v }))
  } else if (t === 'set') {
    const arr = await c.smembers(key.value)
    setMembers.value = arr.map((v) => ({ value: v, draft: v }))
  } else if (t === 'zset') {
    const arr = await c.zrangeAll(key.value)
    zsetMembers.value = arr.map((m) => ({ member: m.member, score: m.score, draftMember: m.member, draftScore: String(m.score) }))
  } else if (t === 'stream') {
    streamEntries.value = await c.xrange(key.value, '-', '+')
  }
}

function reload() {
  reloadActual()
}

watch(
  () => [store.selectedKey, store.keyInfo?.type],
  () => {
    fmt.value = 'text'
    jsonTree.value = false
    if (key.value) reloadActual()
  },
  { immediate: true }
)
</script>

<style scoped>
.meta { font-size: 11px; color: var(--text-2); }
.ve { display: flex; flex-direction: column; gap: 10px; }
.kv-pill { display: inline-block; margin: 2px 6px 2px 0; padding: 1px 8px; border-radius: 10px; background: var(--accent-soft); font-size: 12px; font-family: var(--mono); }
.kv-pill b { color: var(--accent); }
.mono { font-family: var(--mono); font-size: 12px; }
.idx { font-family: var(--mono); color: var(--text-2); text-align: right; }
</style>
