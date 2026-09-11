<template>
  <GlobalDialog v-model="visible" title="远程日志（SSH / SFTP）" width="520px">
    <!-- 已保存的连接：点击整行载入表单，右侧提供编辑 / 删除 -->
    <SavedConnList
      :items="savedItems"
      title="已保存的连接"
      :active-id="editingId"
      @pick="fillFrom"
      @edit="startEdit"
      @remove="askDelete"
    />

    <el-form label-width="84px" label-position="left">
      <el-form-item label="名称">
        <el-input v-model="name" placeholder="用于识别该连接（可留空）" />
      </el-form-item>
      <el-form-item label="主机">
        <el-input v-model="host" placeholder="例如 192.168.1.10" />
      </el-form-item>
      <el-form-item label="端口">
        <el-input v-model.number="port" type="number" />
      </el-form-item>
      <el-form-item label="用户名">
        <el-input v-model="username" placeholder="root" />
      </el-form-item>
      <el-form-item label="认证方式">
        <el-select v-model="authType" style="width: 100%">
          <el-option label="密码" value="password" />
          <el-option label="私钥" value="pubkey" />
        </el-select>
      </el-form-item>
      <el-form-item :label="authType === 'pubkey' ? '私钥路径' : '密码'">
        <el-input
          v-if="authType === 'pubkey'"
          v-model="privateKeyPath"
          placeholder="id_rsa 本地路径"
        />
        <el-input v-else v-model="password" type="password" show-password placeholder="登录密码" />
      </el-form-item>
      <el-form-item label="远程路径">
        <el-input v-model="remotePath" placeholder="例如 /var/log/app/error.log" />
      </el-form-item>
    </el-form>

    <template #footer>
      <app-button size="small" :disabled="pulling" @click="visible = false">取消</app-button>
      <app-button
        v-if="editingId"
        size="small"
        type="danger"
        plain
        @click="cancelEdit"
      >
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
      <app-button
        size="small"
        type="primary"
        :loading="pulling"
        :disabled="!canPull"
        @click="pull"
      >
        拉取并打开
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
import { computed, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import GlobalDialog from '@/components/GlobalDialog.vue'
import SavedConnList, { type SavedConnItem } from './SavedConnList.vue'
import { useLogStore } from '../store/logs'
import {
  loadRemoteConns,
  saveRemoteConns,
  type RemoteConnRecord,
} from '../core/storage'

const props = defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [boolean] }>()

const store = useLogStore()

/** 透传外部 v-model：GlobalDialog 的开关直接映射为父级的 modelValue */
const visible = computed({
  get: () => props.modelValue,
  set: (v: boolean) => emit('update:modelValue', v),
})

const name = ref('')
const host = ref('')
const port = ref(22)
const username = ref('')
const password = ref('')
const authType = ref<'password' | 'pubkey'>('password')
const privateKeyPath = ref('')
const remotePath = ref('')
const pulling = ref(false)

/** 已保存的连接列表 */
const saved = ref<RemoteConnRecord[]>([])
/** 正在编辑的连接 id（null = 新建模式） */
const editingId = ref<string | null>(null)
const saving = ref(false)
/** 删除确认弹窗 */
const confirmVisible = ref(false)
const pendingDelete = ref<RemoteConnRecord | null>(null)

const canPull = computed(
  () => !!host.value && !!username.value && !!remotePath.value && !pulling.value,
)
/** 保存连接的最低要求（拉取还需要密码等，保存可以只存配置骨架） */
const canRemember = computed(
  () => !!host.value && !!username.value && !!remotePath.value,
)

/** 列表展示数据：名称 + user@host:port · 远程路径 */
const savedItems = computed<SavedConnItem[]>(() =>
  saved.value.map(c => ({
    id: c.id,
    name: c.name,
    subtitle: `${c.username}@${c.host}:${c.port} · ${c.remotePath}`,
  })),
)

/** 打开弹窗时载入已保存连接；列表为空时以空 array 兜底（KV 无该键） */
watch(
  () => props.modelValue,
  async (open) => {
    if (!open) return
    editingId.value = null
    pendingDelete.value = null
    saved.value = await loadRemoteConns()
  },
  { immediate: true },
)

/** 由远程路径推导默认显示名（取文件名；无路径时退回 user@host） */
function defaultName(): string {
  const base = remotePath.value.trim().split(/[\\/]/).pop() || ''
  if (base) return base
  const who = [username.value.trim(), host.value.trim()].filter(Boolean).join('@')
  return who || '远程连接'
}

/** 点列表正文：把该记录载入表单，不进入编辑态（避免误覆盖原记录） */
function fillFrom(item: SavedConnItem) {
  const c = saved.value.find(x => x.id === item.id)
  if (!c) return
  name.value = c.name
  host.value = c.host
  port.value = c.port
  username.value = c.username
  password.value = c.password
  authType.value = c.authType
  privateKeyPath.value = c.privateKeyPath
  remotePath.value = c.remotePath
}

/** 编辑：载入表单并进入编辑态（后续点「更新连接」写回同一条记录） */
function startEdit(item: SavedConnItem) {
  fillFrom(item)
  editingId.value = item.id
}

/** 退出编辑态（表单内容保留，回到「新建」模式，避免误覆盖原记录） */
function cancelEdit() {
  editingId.value = null
}

function collect(): Omit<RemoteConnRecord, 'id' | 'savedAt'> {
  return {
    name: name.value.trim() || defaultName(),
    host: host.value.trim(),
    port: Number(port.value) || 22,
    username: username.value.trim(),
    password: password.value,
    authType: authType.value,
    privateKeyPath: privateKeyPath.value.trim(),
    remotePath: remotePath.value.trim(),
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
        id: `rc_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`,
        savedAt: Date.now(),
      })
    }
    await saveRemoteConns(list)
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
    await saveRemoteConns(list)
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

async function pull() {
  if (!canPull.value) return
  pulling.value = true
  try {
    const src = await store.openRemoteFile({
      host: host.value,
      port: Number(port.value) || 22,
      username: username.value,
      password: password.value,
      authType: authType.value,
      privateKeyPath: privateKeyPath.value || undefined,
      remotePath: remotePath.value,
    })
    if (!src) {
      ElMessage.error(store.errorMsg || '远程日志拉取失败')
      return
    }
    ElMessage.success(`已打开远程日志：${src.name}`)
    // 表单保留内容，便于连接失败时改参数重试；编辑态一并退出
    editingId.value = null
    visible.value = false
  } finally {
    pulling.value = false
  }
}
</script>

<style scoped>
.confirm-message {
  font-size: var(--font-size-base);
  line-height: 1.6;
}
</style>
