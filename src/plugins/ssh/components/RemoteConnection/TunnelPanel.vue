<template>
  <div class="tunnel-panel">
    <div class="tunnel-toolbar">
      <app-button size="small" type="primary" @click="openRuleDialog()">新建转发规则</app-button>
      <app-button size="small" @click="refreshStatus">刷新状态</app-button>
      <app-button size="small" @click="stopAll">停止全部</app-button>
      <span class="tunnel-hint">
        {{ connectionLabel }} · 规则保存在本机；任务只在应用运行期间有效，重启后需重新启动。
      </span>
    </div>

    <div class="tunnel-table-wrap">
      <el-table :data="rules" size="small" height="100%" empty-text="暂无转发规则">
        <el-table-column prop="label" label="名称" min-width="120">
          <template #default="{ row }">
            {{ row.label || kindLabel(row.kind) }}
          </template>
        </el-table-column>
        <el-table-column label="类型" width="110">
          <template #default="{ row }">{{ kindLabel(row.kind) }}</template>
        </el-table-column>
        <el-table-column label="监听" min-width="150">
          <template #default="{ row }">{{ row.bindHost || '127.0.0.1' }}:{{ row.bindPort }}</template>
        </el-table-column>
        <el-table-column label="目标" min-width="160">
          <template #default="{ row }">
            {{ row.kind === 'dynamic' ? '由 SOCKS5 请求决定' : `${row.targetHost}:${row.targetPort}` }}
          </template>
        </el-table-column>
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <span class="tunnel-status" :class="`is-${statusOf(row.id)?.status || 'stopped'}`">
              {{ statusText(row.id) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="连接 / 流量" min-width="140">
          <template #default="{ row }">
            <template v-if="statusOf(row.id)">
              {{ statusOf(row.id)!.connections }} / {{ formatBytes(statusOf(row.id)!.bytesUp + statusOf(row.id)!.bytesDown) }}
            </template>
            <span v-else>—</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="230" fixed="right">
          <template #default="{ row }">
            <div class="tunnel-actions">
              <app-button
                v-if="statusOf(row.id)?.status !== 'running'"
                size="small"
                type="primary"
                @click="startRule(row)"
              >启动</app-button>
              <app-button v-else size="small" @click="stopRule(row)">停止</app-button>
              <app-button size="small" @click="openRuleDialog(row)">编辑</app-button>
              <app-button size="small" type="danger" @click="deleteRule(row)">删除</app-button>
            </div>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <div class="exec-section">
      <div class="exec-title">远程命令（ssh_exec）</div>
      <div class="exec-input-row">
        <el-input
          v-model="execCommand"
          placeholder="输入单条命令，如 uname -a"
          @keyup.enter="runExec()"
        />
        <app-button size="small" type="primary" :loading="execRunning" @click="runExec()">执行</app-button>
      </div>
      <pre v-if="execOutput" class="exec-output">{{ execOutput }}</pre>
      <p class="tunnel-hint">
        stdout / stderr 分流并返回退出码；不分配 PTY，适合脚本与健康检查。
      </p>
    </div>

    <!-- 转发规则编辑 -->
    <GlobalDialog
      v-model="showRuleDialog"
      :title="editingRuleId ? '编辑转发规则' : '新建转发规则'"
      width="480px"
    >
      <el-form :model="ruleForm" label-width="96px">
        <el-form-item label="类型">
          <el-select v-model="ruleForm.kind" style="width: 100%">
            <el-option
              v-for="o in FORWARD_KIND_OPTIONS"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="监听地址">
          <el-input v-model="ruleForm.bindHost" placeholder="127.0.0.1" />
        </el-form-item>
        <el-form-item label="监听端口">
          <el-input-number v-model="ruleForm.bindPort" :min="1" :max="65535" style="width: 100%" />
        </el-form-item>
        <el-form-item v-if="ruleForm.kind !== 'dynamic'" label="目标地址">
          <el-input
            v-model="ruleForm.targetHost"
            :placeholder="ruleForm.kind === 'remote' ? '本机可达的地址，如 127.0.0.1' : '远端可达的地址，如 127.0.0.1'"
          />
        </el-form-item>
        <el-form-item v-if="ruleForm.kind !== 'dynamic'" label="目标端口">
          <el-input-number v-model="ruleForm.targetPort" :min="1" :max="65535" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="ruleForm.label" placeholder="如：内网数据库 / 本地 SOCKS 代理" />
        </el-form-item>
      </el-form>
      <p class="tunnel-hint">
        -L 本地监听并转发到远端；-R 在远端监听并转发回本机；-D 是本地 SOCKS5 代理，目标由客户端请求决定。
      </p>
      <template #footer>
        <app-button size="small" @click="showRuleDialog = false">取消</app-button>
        <app-button size="small" type="primary" @click="saveRule">保存</app-button>
      </template>
    </GlobalDialog>

    <!-- 二次验证：exec / 转发都是独立 SSH 会话，各自要输入一次动态码 -->
    <GlobalDialog v-model="showMfaDialog" title="二次验证" width="420px">
      <p class="tunnel-hint">{{ mfaMessage }}</p>
      <el-input
        v-model="mfaCode"
        placeholder="请输入动态验证码"
        autocomplete="off"
        @keyup.enter="confirmMfa"
      />
      <template #footer>
        <app-button size="small" @click="showMfaDialog = false">取消</app-button>
        <app-button size="small" type="primary" @click="confirmMfa">验证并重试</app-button>
      </template>
    </GlobalDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@/api/rpc'
import GlobalDialog from '@/components/GlobalDialog.vue'
import NotificationUtil from '@/utils/NotificationUtil'
import { FORWARD_KIND_OPTIONS } from '../../types'
import type { ForwardRule, ForwardInfo, ForwardKind, ConnectionTab, RemoteConnection } from '../../types'
import { buildSshTarget } from '../../utils/target'
import { readAllForwardRules, writeMineForConnection } from '../../utils/forwardRules'

const props = defineProps<{
  tab: ConnectionTab
  connection: RemoteConnection
}>()

const rules = ref<ForwardRule[]>([])
const activeTasks = ref<Record<string, ForwardInfo>>({})
let pollTimer: ReturnType<typeof setInterval> | null = null

const execCommand = ref('')
const execOutput = ref('')
const execRunning = ref(false)

const showRuleDialog = ref(false)
const editingRuleId = ref<string | null>(null)
const ruleForm = reactive({
  kind: 'local' as ForwardKind,
  bindHost: '127.0.0.1',
  bindPort: 8080,
  targetHost: '127.0.0.1',
  targetPort: 80,
  label: ''
})

const showMfaDialog = ref(false)
const mfaMessage = ref('')
const mfaCode = ref('')
let mfaRetry: ((code: string) => void) | null = null

function loadRules() {
  rules.value = readAllForwardRules().filter(r => r.connectionId === props.connection.id)
}

function persistAll(mine: ForwardRule[]) {
  writeMineForConnection(props.connection.id, mine)
}

function kindLabel(kind: string) {
  return kind === 'remote' ? '远程 -R' : kind === 'dynamic' ? '动态 -D' : '本地 -L'
}

function formatBytes(n: number) {
  if (n >= 1024 * 1024 * 1024) return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`
  if (n >= 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`
  if (n >= 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${n} B`
}

function statusOf(id: string): ForwardInfo | undefined {
  return activeTasks.value[id]
}

function statusText(id: string) {
  const s = statusOf(id)
  if (!s) return '未启动'
  if (s.status === 'running') return '运行中'
  if (s.status === 'error') return '异常'
  return '已停止'
}

async function refreshStatus() {
  try {
    const res = await invoke<any>('ssh_tunnel_list')
    if (res.code === 200 && Array.isArray(res.data)) {
      const map: Record<string, ForwardInfo> = {}
      for (const item of res.data) map[item.id] = item
      activeTasks.value = map
    }
  } catch { /* 后端未就绪时保持上一轮状态 */ }
}

function openRuleDialog(rule?: ForwardRule) {
  if (rule) {
    editingRuleId.value = rule.id
    Object.assign(ruleForm, {
      kind: rule.kind,
      bindHost: rule.bindHost,
      bindPort: rule.bindPort,
      targetHost: rule.targetHost,
      targetPort: rule.targetPort,
      label: rule.label
    })
  } else {
    editingRuleId.value = null
    Object.assign(ruleForm, {
      kind: 'local', bindHost: '127.0.0.1', bindPort: 8080,
      targetHost: '127.0.0.1', targetPort: 80, label: ''
    })
  }
  showRuleDialog.value = true
}

function saveRule() {
  if (!ruleForm.bindPort || (ruleForm.kind !== 'dynamic' && !ruleForm.targetPort)) {
    NotificationUtil.warn('请填写有效的监听端口与目标端口')
    return
  }
  const id = editingRuleId.value || `fwd-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
  const next: ForwardRule = {
    id,
    connectionId: props.connection.id,
    kind: ruleForm.kind,
    bindHost: ruleForm.bindHost.trim() || '127.0.0.1',
    bindPort: Number(ruleForm.bindPort),
    targetHost: ruleForm.targetHost.trim(),
    targetPort: Number(ruleForm.targetPort),
    label: ruleForm.label.trim()
  }
  const idx = rules.value.findIndex(r => r.id === id)
  if (idx >= 0) rules.value[idx] = next
  else rules.value.push(next)
  persistAll(rules.value)
  showRuleDialog.value = false
  NotificationUtil.success('规则已保存')
}

async function deleteRule(rule: ForwardRule) {
  if (statusOf(rule.id)) await stopRule(rule, true)
  rules.value = rules.value.filter(r => r.id !== rule.id)
  persistAll(rules.value)
}

/** 409 / 411 / 其它错误统一转成可执行的提示，避免静默失败 */
function reportFailure(code: number, msg: string, retry: (mfaCode: string) => void) {
  if (code === 409) {
    NotificationUtil.warn('该主机密钥尚未保存，请先打开此连接的终端并完成指纹确认')
  } else if (code === 411) {
    mfaMessage.value = msg || '服务器要求二次验证'
    mfaCode.value = ''
    mfaRetry = retry
    showMfaDialog.value = true
  } else {
    NotificationUtil.error(msg || '操作失败')
  }
}

function confirmMfa() {
  const code = mfaCode.value.trim()
  if (!code) return
  showMfaDialog.value = false
  const fn = mfaRetry
  mfaRetry = null
  fn?.(code)
}

async function startRule(rule: ForwardRule, mfaCode = '') {
  const target = buildSshTarget(props.connection, { mfaCode })
  try {
    const res = await invoke<any>('ssh_tunnel_start', {
      target,
      rule: JSON.stringify(rule)
    })
    if (res.code === 200) {
      NotificationUtil.success(`转发已启动：${rule.bindHost}:${res.data?.bindPort ?? rule.bindPort}`)
    } else {
      reportFailure(res.code, res.msg, (code) => void startRule(rule, code))
    }
  } catch (e) {
    NotificationUtil.error(`转发启动异常：${e}`)
  }
  await refreshStatus()
}

async function stopRule(rule: ForwardRule, silent = false) {
  try {
    await invoke('ssh_tunnel_stop', { id: rule.id })
    if (!silent) NotificationUtil.success('转发已停止')
  } catch (e) {
    if (!silent) NotificationUtil.error(`停止失败：${e}`)
  }
  await refreshStatus()
}

async function stopAll() {
  try {
    await invoke('ssh_tunnel_stop_all')
    NotificationUtil.success('已停止全部转发')
  } catch { /* ignore */ }
  await refreshStatus()
}

async function runExec(mfaCode = '') {
  const command = execCommand.value.trim()
  if (!command) return
  execRunning.value = true
  try {
    const res = await invoke<any>('ssh_exec', {
      target: buildSshTarget(props.connection, { mfaCode }),
      command,
      timeoutMs: 30000
    })
    if (res.code === 200) {
      const d = res.data || {}
      execOutput.value = [
        d.stdout ? `[stdout]\n${d.stdout}` : '',
        d.stderr ? `[stderr]\n${d.stderr}` : '',
        `[exit code] ${d.exitCode ?? -1}`
      ].filter(Boolean).join('\n')
    } else {
      reportFailure(res.code, res.msg, (code) => void runExec(code))
    }
  } catch (e) {
    NotificationUtil.error(`执行异常：${e}`)
  } finally {
    execRunning.value = false
  }
}

const connectionLabel = computed(() => `${props.connection.username}@${props.connection.address}`)

onMounted(() => {
  loadRules()
  void refreshStatus()
  pollTimer = setInterval(() => void refreshStatus(), 2000)
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
  pollTimer = null
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.tunnel-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  padding: var(--space-3);
  gap: var(--space-3);
  overflow: hidden;

  @include theme.useTheme {
    background-color: theme.getVar('appMainBgColor');
    color: theme.getVar('textPrimary');
  }
}
.tunnel-toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}
.tunnel-hint {
  margin: 0;
  opacity: 0.7;
  line-height: 1.6;
}
.tunnel-table-wrap {
  flex: 1;
  min-height: 160px;
  overflow: hidden;
}
.tunnel-actions {
  display: flex;
  gap: var(--space-1);
  flex-wrap: wrap;
}
.tunnel-status {
  &.is-running { color: var(--stateSuccessFg); }
  &.is-error { color: var(--stateErrorFg); }
  &.is-stopped { opacity: 0.7; }
}
.exec-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  flex-shrink: 0;
  padding-top: var(--space-3);

  @include theme.useTheme {
    border-top: 1px solid theme.getVar('borderColor');
  }
}
.exec-title {
  font-weight: 600;
}
.exec-input-row {
  display: flex;
  gap: var(--space-2);
}
.exec-output {
  max-height: 180px;
  overflow: auto;
  margin: 0;
  padding: var(--space-2);
  white-space: pre-wrap;
  word-break: break-all;
  font-family: var(--font-family-mono);

  @include theme.useTheme {
    background-color: theme.getVar('editorBg');
    border: 1px solid theme.getVar('borderColor');
    border-radius: theme.getVar('borderRadius');
  }
}
</style>
