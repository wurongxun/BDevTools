<template>
  <GlobalDialog v-model="visible" title="添加实时日志流" width="440px" @close="onClosed">
    <!-- 已保存的连接：点击整行载入表单，右侧提供编辑 / 删除 -->
    <SavedConnList
      :items="savedItems"
      title="已保存的流"
      :active-id="editingId"
      @pick="fillFrom"
      @edit="startEdit"
      @remove="askDelete"
    />

    <el-form label-width="84px" @submit.prevent  label-position="left">
      <el-form-item label="协议">
        <el-radio-group v-model="form.protocol">
          <el-radio value="ws">WebSocket</el-radio>
          <el-radio value="tcp">TCP</el-radio>
        </el-radio-group>
      </el-form-item>

      <template v-if="form.protocol === 'ws'">
        <el-form-item label="WebSocket URL">
          <el-input
            v-model="form.url"
            placeholder="ws://127.0.0.1:8080/logs 或 wss://..."
            @keyup.enter="connect"
          />
        </el-form-item>
      </template>
      <template v-else>
        <el-form-item label="主机">
          <el-input v-model="form.host" placeholder="127.0.0.1" @keyup.enter="connect" />
        </el-form-item>
        <el-form-item label="端口">
          <el-input v-model="form.port" placeholder="8080" @keyup.enter="connect" />
        </el-form-item>
      </template>

      <el-form-item label="名称">
        <el-input v-model="form.name" placeholder="可选，留空自动生成" />
      </el-form-item>
    </el-form>

    <template #footer>
      <app-button size="small" @click="visible = false">取消</app-button>
      <app-button v-if="editingId" size="small" type="danger" plain @click="cancelEdit">
        取消编辑
      </app-button>
      <app-button
        size="small"
        :disabled="!canRemember || saving"
        :loading="saving"
        @click="rememberConn"
      >
        {{ editingId ? '更新连接' : '保存连接' }}
      </app-button>
      <app-button size="small" type="primary" :loading="connecting" :disabled="!canConnect" @click="connect">
        连接
      </app-button>
    </template>
  </GlobalDialog>

  <!-- 删除确认（Tauri WebView2 不支持原生 confirm，统一走 GlobalDialog） -->
  <GlobalDialog v-model="confirmVisible" title="删除连接" width="400px">
    <div class="confirm-message">
      确定删除连接「{{ pendingDelete?.name }}」吗？此操作不可撤销。
    </div>
    <template #footer>
      <app-button size="small" @click="confirmVisible = false">取消</app-button>
      <app-button size="small" type="danger" @click="confirmDelete">删除</app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import GlobalDialog from '@/components/GlobalDialog.vue'
import SavedConnList, { type SavedConnItem } from './SavedConnList.vue'
import { useLogStore } from '../store/logs'
import {
  loadStreamConns,
  saveStreamConns,
  type StreamConnRecord,
} from '../core/storage'

const props = defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [boolean] }>()

const store = useLogStore()
const connecting = ref(false)

/** 透传外部 v-model：GlobalDialog 的开关直接映射为父级的 modelValue */
const visible = computed({
  get: () => props.modelValue,
  set: (v: boolean) => emit('update:modelValue', v),
})

const form = reactive({
  protocol: 'ws' as 'ws' | 'tcp',
  url: '',
  host: '',
  port: '',
  name: '',
})

/** 已保存的流连接 */
const saved = ref<StreamConnRecord[]>([])
/** 正在编辑的连接 id（null = 新建模式） */
const editingId = ref<string | null>(null)
const saving = ref(false)
/** 删除确认弹窗 */
const confirmVisible = ref(false)
const pendingDelete = ref<StreamConnRecord | null>(null)

/** 协议 + 地址是否满足「连接」的最低要求 */
const canConnect = computed(() =>
  form.protocol === 'ws' ? !!form.url.trim() : !!form.host.trim() && !!form.port.trim(),
)
/** 保存连接的最低要求（与连接要求一致） */
const canRemember = computed(() => canConnect.value)

/** 列表展示数据：名称 + 协议地址摘要 */
const savedItems = computed<SavedConnItem[]>(() =>
  saved.value.map(c => ({
    id: c.id,
    name: recordName(c),
    subtitle: c.protocol === 'ws' ? `ws · ${c.url}` : `tcp · ${c.host}:${c.port}`,
  })),
)

/** 打开弹窗时载入已保存连接（KV 无该键时返回空数组） */
watch(
  () => props.modelValue,
  async (open) => {
    if (!open) return
    editingId.value = null
    pendingDelete.value = null
    saved.value = await loadStreamConns()
  },
  { immediate: true },
)

/** 列表展示用的默认名（表单提交时用当前输入推导，故单独处理） */
function defaultName(protocol: 'ws' | 'tcp'): string {
  return protocol === 'ws' ? form.url.trim() || 'WebSocket 流' : `${form.host.trim()}:${form.port.trim()}`
}

/** 记录 → 列表展示名（独立于表单，用于渲染已保存条目） */
function recordName(c: StreamConnRecord): string {
  if (c.name.trim()) return c.name.trim()
  return c.protocol === 'ws' ? c.url || 'WebSocket 流' : `${c.host}:${c.port}`
}

/** 关闭后清空表单并退出编辑态，避免下次打开残留上次输入 */
function onClosed() {
  form.url = ''
  form.host = ''
  form.port = ''
  form.name = ''
  editingId.value = null
}

/** 点列表正文：仅载入表单，不进入编辑态（避免误覆盖原记录） */
function fillFrom(item: SavedConnItem) {
  const c = saved.value.find(x => x.id === item.id)
  if (!c) return
  form.protocol = c.protocol
  form.url = c.url
  form.host = c.host
  form.port = c.port
  form.name = c.name
}

/** 编辑：载入表单并进入编辑态（后续点「更新连接」写回同一条记录） */
function startEdit(item: SavedConnItem) {
  fillFrom(item)
  editingId.value = item.id
}

/** 退出编辑态（表单内容保留，回到「新建」模式） */
function cancelEdit() {
  editingId.value = null
}

/** 收集当前表单为一条记录 */
function collect(): Omit<StreamConnRecord, 'id' | 'savedAt'> {
  const isWs = form.protocol === 'ws'
  return {
    name: form.name.trim() || defaultName(form.protocol),
    protocol: form.protocol,
    url: isWs ? form.url.trim() : '',
    host: isWs ? '' : form.host.trim(),
    port: isWs ? '' : form.port.trim(),
  }
}

/** 保存 / 更新连接：写回插件 KV */
async function rememberConn() {
  if (!canRemember.value || saving.value) return
  saving.value = true
  try {
    const data = collect()
    const list = [...saved.value]
    const idx = list.findIndex(c => c.id === editingId.value)
    if (idx >= 0) {
      list[idx] = { ...list[idx], ...data, savedAt: Date.now() }
    } else {
      list.push({
        ...data,
        id: `sc_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`,
        savedAt: Date.now(),
      })
    }
    await saveStreamConns(list)
    saved.value = list
    editingId.value = null
    ElMessage.success(idx >= 0 ? '连接已更新' : '连接已保存')
  } catch {
    ElMessage.error('保存连接失败')
  } finally {
    saving.value = false
  }
}

function askDelete(item: SavedConnItem) {
  const c = saved.value.find(x => x.id === item.id)
  if (!c) return
  pendingDelete.value = c
  confirmVisible.value = true
}

async function confirmDelete() {
  const target = pendingDelete.value
  if (!target) return
  try {
    const list = saved.value.filter(c => c.id !== target.id)
    await saveStreamConns(list)
    saved.value = list
    if (editingId.value === target.id) editingId.value = null
    ElMessage.success('连接已删除')
  } catch {
    ElMessage.error('删除连接失败')
  } finally {
    confirmVisible.value = false
    pendingDelete.value = null
  }
}

async function connect() {
  if (form.protocol === 'ws') {
    if (!form.url.trim()) {
      ElMessage.warning('请输入 WebSocket URL')
      return
    }
  } else {
    if (!form.host.trim() || !form.port.trim()) {
      ElMessage.warning('请输入主机与端口')
      return
    }
  }
  connecting.value = true
  try {
    const src = await store.openStreamSource({
      name: form.name.trim() || undefined,
      protocol: form.protocol,
      url: form.protocol === 'ws' ? form.url.trim() : undefined,
      host: form.protocol === 'tcp' ? form.host.trim() : undefined,
      port: form.protocol === 'tcp' ? Number(form.port) : undefined,
    })
    if (src) {
      ElMessage.success(`已连接实时流：${src.name}`)
      // 表单保留内容，便于连接失败时改参数重试；编辑态一并退出
      editingId.value = null
      visible.value = false
    } else {
      ElMessage.error('实时流连接失败')
    }
  } catch (e) {
    ElMessage.error((e as Error).message || '实时流连接异常')
  } finally {
    connecting.value = false
  }
}
</script>

<style scoped>
.confirm-message {
  font-size: var(--font-size-base);
  line-height: 1.6;
}
</style>
