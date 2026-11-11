<template>
  <GlobalDialog
    v-model="visible"
    title="SSH 设置中心"
    width="min(1080px, calc(100vw - 64px))"
    body-max-height="78vh"
    :before-close="confirmClose"
  >
    <div class="ssh-settings-root">
      <el-tabs v-model="activeTab" class="settings-tabs">
        <el-tab-pane label="高亮集" name="highlight"><HighlightPane /></el-tab-pane>
        <el-tab-pane label="配色方案" name="color"><ColorPane /></el-tab-pane>
        <el-tab-pane label="键盘映射" name="keyboard"><KeyBindingPane /></el-tab-pane>
        <el-tab-pane label="宏 / 登录脚本" name="macro"><MacroPane /></el-tab-pane>
        <el-tab-pane label="自动登录" name="expect"><ExpectPane /></el-tab-pane>

        <el-tab-pane label="会话日志" name="log">
          <div class="pane">
            <div class="pane-toolbar">
              <label class="inline-check"><input v-model="prefs.logSettings.enabled" type="checkbox" />启用录制</label>
              <label class="inline-check"><input v-model="prefs.logSettings.autoRecord" type="checkbox" />连接后自动录制</label>
              <label class="inline-check"><input v-model="prefs.logSettings.timestamp" type="checkbox" />行首时间戳</label>
              <label class="inline-check"><input v-model="prefs.logSettings.logInput" type="checkbox" />记录输入</label>
              <label class="inline-check"><input v-model="prefs.logSettings.maskSensitive" type="checkbox" />掩码口令</label>
            </div>
            <div class="pane-hint">
              每个终端工具栏的 ● 按钮单独控制该会话的录制起停；日志按行写入，自动去掉 ANSI 控制序列。
              目录：<code>{{ logDir || '（尚未创建）' }}</code>
            </div>
            <div class="pane-toolbar">
              <app-button size="small" @click="refreshLogs">刷新列表</app-button>
              <span class="pane-hint">共 {{ logFiles.length }} 个日志文件</span>
            </div>
            <div class="log-list">
              <div v-for="f in logFiles" :key="f.name" class="log-row">
                <span class="log-name" :title="f.name">{{ f.name }}</span>
                <span class="log-meta">{{ formatSize(f.size) }} · {{ f.modified }}</span>
                <app-button size="small" link @click="viewLog(f.name)">查看</app-button>
                <app-button size="small" link @click="copyLog(f.name)">复制全文</app-button>
                <app-button size="small" link @click="removeLog(f.name)">删除</app-button>
              </div>
              <div v-if="!logFiles.length" class="pane-empty">暂无录制日志</div>
            </div>
            <el-input
              v-if="logPreview"
              :model-value="logPreview"
              type="textarea"
              :rows="8"
              readonly
              spellcheck="false"
              class="log-preview"
            />
          </div>
        </el-tab-pane>

        <el-tab-pane label="会话保险库" name="vault">
          <div class="pane">
            <div class="vault-status" :class="prefs.vault.unlocked ? 'vault-ok' : 'vault-locked'">
              状态：{{ !prefs.vault.enabled ? '未创建（密码以明文保存）' : prefs.vault.unlocked ? '已解锁' : '已锁定' }}
            </div>
            <div class="pane-hint">
              主密码派生密钥（PBKDF2 12 万轮）后用 AES-256-GCM 加密会话密码 / 私钥口令 / 代理与跳板机口令。
              密钥只存在进程内存，应用重启后需重新解锁；锁定期间无法编辑会话，连接也拿不到明文密码。
              元数据：<code>{{ prefs.vault.path || '—' }}</code>
            </div>

            <template v-if="!prefs.vault.enabled">
              <div class="pane-row">
                <span class="pane-label">主密码</span>
                <el-input v-model="vaultPassword" type="password" size="small" show-password placeholder="至少 6 位" />
              </div>
              <div class="pane-row">
                <span class="pane-label">确认密码</span>
                <el-input v-model="vaultPassword2" type="password" size="small" show-password />
              </div>
              <label class="inline-check"><input v-model="vaultMigrate" type="checkbox" />同时加密现有明文凭据（推荐）</label>
              <div class="pane-toolbar">
                <app-button size="small" type="primary" @click="setupVault">创建保险库</app-button>
              </div>
            </template>

            <template v-else-if="!prefs.vault.unlocked">
              <div class="pane-row">
                <span class="pane-label">主密码</span>
                <el-input v-model="vaultPassword" type="password" size="small" show-password @keyup.enter="unlockVault" />
              </div>
              <div class="pane-toolbar">
                <app-button size="small" type="primary" @click="unlockVault">解锁</app-button>
              </div>
            </template>

            <template v-else>
              <div class="pane-toolbar">
                <app-button size="small" @click="lockVault">立即锁定</app-button>
                <app-button size="small" @click="encryptExisting">加密残余明文凭据</app-button>
              </div>
              <div class="pane-row">
                <span class="pane-label">取消保险库</span>
                <el-input v-model="vaultPassword" type="password" size="small" show-password placeholder="输入主密码确认" />
                <app-button size="small" type="danger" plain @click="disableVault">解密并移除</app-button>
              </div>
              <div class="pane-hint">取消时先把库里的密文还原为明文，再删除元数据；顺序不会颠倒，避免会话变成永久解不开的孤儿密文。</div>
            </template>
            <div v-if="vaultNotice" class="pane-notice">{{ vaultNotice }}</div>
          </div>
        </el-tab-pane>

        <el-tab-pane label="导入导出" name="transfer">
          <div class="pane">
            <div class="pane-toolbar">
              <app-button size="small" type="primary" plain @click="buildExport">生成完整配置</app-button>
              <app-button size="small" @click="copyTransfer">复制文本</app-button>
              <app-button size="small" @click="doImport">从文本导入</app-button>
              <label class="file-button">
                导入 .xsh 文件
                <input type="file" accept=".xsh,.txt" multiple @change="onXshFiles" />
              </label>
            </div>
            <div class="pane-hint">
              完整配置包含会话、分组、转发规则、高亮集、配色、宏、自动登录规则与键位；<strong>含明文密码</strong>，请妥善保管。
              导入时转发规则按会话名重新绑定新 id。Xshell 的密码是专有加密格式，无法还原，导入后需手动补填。
            </div>
            <el-input
              v-model="transferText"
              type="textarea"
              :rows="10"
              spellcheck="false"
              placeholder="点击「生成完整配置」查看导出内容，或粘贴此前导出的 JSON"
            />
            <div v-if="transferNotice" class="pane-notice">{{ transferNotice }}</div>
          </div>
        </el-tab-pane>
      </el-tabs>
    </div>
    <template #footer>
      <div class="ssh-settings-footer">
        <button
          type="button"
          class="save-state"
          :class="`save-${prefs.saveState}`"
          :title="prefs.saveError || '改动会自动保存到本地，可随时关闭窗口'"
          @click="manualSave"
        >
          <span class="save-dot" />
          <span>{{ saveStatusText }}</span>
        </button>
        <div class="footer-actions">
          <app-button size="small" @click="manualSave">立即保存</app-button>
          <app-button size="small" type="primary" @click="finish">完成</app-button>
        </div>
      </div>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { ElMessageBox } from 'element-plus'
import { invoke } from '@/api/rpc'
import GlobalDialog from '@/components/GlobalDialog.vue'
import NotificationUtil from '@/utils/NotificationUtil'
import { useConnectionStore, useSshPreferencesStore } from '../../../store'
import { buildSessionBundle, importSessionBundle } from '../../../utils/sessionBundle'
import { decodeXshBuffer, parseXsh } from '../../../utils/xsh'
import HighlightPane from './HighlightPane.vue'
import ColorPane from './ColorPane.vue'
import KeyBindingPane from './KeyBindingPane.vue'
import MacroPane from './MacroPane.vue'
import ExpectPane from './ExpectPane.vue'

const prefs = useSshPreferencesStore()
const connectionStore = useConnectionStore()
const visible = ref(false)
const activeTab = ref('highlight')

// ==================== 自动保存反馈 ====================
// 偏好是 deep watch + 200ms debounce 写到 localStorage。以前全静默，用户不知道脚本有没有存下来，
// 不敢关窗口；现在把状态常驻在 footer，并在任何关闭路径上强制 flush。
const saveStatusText = computed(() => {
  if (prefs.saveState === 'saving') return '正在保存…'
  if (prefs.saveState === 'error') return `保存失败：${prefs.saveError || '本地存储不可用'}`
  const t = prefs.lastSavedAt
    ? new Date(prefs.lastSavedAt).toLocaleTimeString('zh-CN', { hour12: false })
    : ''
  return t ? `已自动保存 · ${t}` : '已自动保存（改动实时写入本地）'
})

function manualSave() {
  if (prefs.saveNow()) NotificationUtil.success('偏好已保存到本地')
  else NotificationUtil.error(`保存失败：${prefs.saveError}`)
}

/** 成功才允许关闭；失败时明确告知「会丢失」，不让人在不知情的情况下丢掉脚本 */
async function confirmClose(): Promise<boolean> {
  if (prefs.saveNow()) return true
  try {
    await ElMessageBox.confirm(
      '偏好未能写入本地存储（可能配额已满或被浏览器禁用）。关闭后本次改动会丢失，是否仍要关闭？',
      '保存失败',
      { type: 'warning', confirmButtonText: '仍要关闭', cancelButtonText: '返回编辑' }
    )
    return true
  } catch {
    return false
  }
}

async function finish() {
  if (await confirmClose()) visible.value = false
}

/** TerminalView 只发意图，设置中心是全局单例，避免每个 tab 各挂一份对话框 */
function onOpenSettings(event: Event) {
  const tab = (event as CustomEvent).detail?.tab
  if (typeof tab === 'string' && tab) activeTab.value = tab
  visible.value = true
}

onMounted(() => window.addEventListener('ssh-open-settings', onOpenSettings))
onUnmounted(() => window.removeEventListener('ssh-open-settings', onOpenSettings))

watch(activeTab, tab => {
  if (tab === 'log') void refreshLogs()
  if (tab === 'vault') void prefs.refreshVaultStatus()
})

// ==================== 会话日志 ====================
const logDir = ref('')
const logFiles = ref<any[]>([])
const logPreview = ref('')

function formatSize(size: number): string {
  if (size >= 1024 * 1024) return `${(size / 1024 / 1024).toFixed(1)} MB`
  if (size >= 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${size} B`
}

async function refreshLogs() {
  try {
    const dir = await invoke<any>('ssh_log_dir')
    if (dir?.code === 200) logDir.value = dir.data
    const list = await invoke<any>('ssh_log_list')
    if (list?.code === 200) logFiles.value = Array.isArray(list.data) ? list.data : []
  } catch { /* 插件裁剪时静默 */ }
}

async function viewLog(name: string) {
  const res = await invoke<any>('ssh_log_read', { name, maxBytes: 131072 })
  logPreview.value = res?.code === 200 ? String(res.data || '') : `读取失败：${res?.msg}`
}

async function copyLog(name: string) {
  const res = await invoke<any>('ssh_log_read', { name, maxBytes: 4 * 1024 * 1024 })
  if (res?.code !== 200) { NotificationUtil.warn(`读取失败：${res?.msg}`); return }
  try {
    await navigator.clipboard.writeText(String(res.data || ''))
    NotificationUtil.success('日志内容已复制到剪贴板（最多 4MB）')
  } catch {
    logPreview.value = String(res.data || '')
    NotificationUtil.warn('剪贴板不可用，已在下方预览框展示')
  }
}

async function removeLog(name: string) {
  const res = await invoke<any>('ssh_log_delete', { name })
  if (res?.code === 200) {
    NotificationUtil.success('已删除')
    if (logPreview.value) logPreview.value = ''
    await refreshLogs()
  } else {
    NotificationUtil.warn(`删除失败：${res?.msg}`)
  }
}

// ==================== 保险库 ====================
const vaultPassword = ref('')
const vaultPassword2 = ref('')
const vaultMigrate = ref(true)
const vaultNotice = ref('')

/**
 * 解锁 / 锁定后必须强制重读会话表：
 * 前端 store 里缓存的是上一次 `connection_list` 解密后的明文密码，
 * 不重载的话「已解锁」却仍拿着空密码连接，或者「已锁定」却还能用缓存的明文登录。
 */
async function reloadConnections() {
  if (typeof connectionStore.loadConnections === 'function') {
    await connectionStore.loadConnections(true)
  }
}

async function setupVault() {
  if (vaultPassword.value !== vaultPassword2.value) {
    vaultNotice.value = '两次输入的主密码不一致'
    return
  }
  const res = await prefs.setupVault(vaultPassword.value, vaultMigrate.value)
  vaultNotice.value = `${res.ok ? '成功' : '失败'}：${res.message}${res.ok ? `（已处理 ${res.count ?? 0} 条会话）` : ''}`
  vaultPassword.value = ''
  vaultPassword2.value = ''
  if (res.ok) await reloadConnections()
}

async function unlockVault() {
  const res = await prefs.unlockVault(vaultPassword.value)
  vaultNotice.value = `${res.ok ? '已解锁' : '解锁失败'}：${res.message}`
  vaultPassword.value = ''
  if (res.ok) await reloadConnections()
}

async function lockVault() {
  const res = await prefs.lockVault()
  vaultNotice.value = res.ok ? '已锁定，连接配置中的密码暂不可用' : `锁定失败：${res.message}`
  if (res.ok) await reloadConnections()
}

async function encryptExisting() {
  const res = await prefs.encryptExisting()
  vaultNotice.value = `${res.ok ? '完成' : '失败'}：${res.message}（${res.count ?? 0} 条会话）`
  if (res.ok) await reloadConnections()
}

async function disableVault() {
  const res = await prefs.disableVault(vaultPassword.value)
  vaultNotice.value = `${res.ok ? '保险库已移除' : '失败'}：${res.message}`
  vaultPassword.value = ''
  if (res.ok) await reloadConnections()
}

// ==================== 导入导出 ====================
const transferText = ref('')
const transferNotice = ref('')

function buildExport() {
  const bundle = buildSessionBundle(connectionStore, prefs)
  transferText.value = JSON.stringify({ exportedAt: new Date().toISOString(), ...bundle }, null, 2)
  transferNotice.value = `已生成 ${bundle.connections.length} 条会话、${bundle.forwardRules?.length || 0} 条转发规则的完整配置。`
}

async function copyTransfer() {
  if (!transferText.value) buildExport()
  try {
    await navigator.clipboard.writeText(transferText.value)
    NotificationUtil.success('已复制到剪贴板')
  } catch {
    NotificationUtil.warn('剪贴板不可用，请手动全选复制')
  }
}

async function doImport() {
  if (!transferText.value.trim()) { transferNotice.value = '请先粘贴配置文本'; return }
  const res = await importSessionBundle(transferText.value, connectionStore, prefs)
  transferNotice.value = res.error
    ? res.error
    : `导入完成：${res.count} 条会话、${res.rules} 条转发规则；偏好已合并（重名项会追加而非覆盖）。`
}

async function onXshFiles(event: Event) {
  const input = event.target as HTMLInputElement
  const files = Array.from(input.files || [])
  const drafts: any[] = []
  const warnings: string[] = []
  for (const file of files) {
    try {
      const text = decodeXshBuffer(await file.arrayBuffer())
      const result = parseXsh(text, file.name)
      drafts.push(result.connection)
      warnings.push(...result.warnings)
    } catch { warnings.push(`${file.name} 解析失败`) }
  }
  input.value = ''
  if (!drafts.length) { transferNotice.value = '没有可导入的 .xsh 文件'; return }
  const count = await connectionStore.importConnections(JSON.stringify({ connections: drafts }))
  transferNotice.value = `已导入 ${count} 条 .xsh 会话。${warnings.length ? '注意事项：' + [...new Set(warnings)].join(' ') : ''}`
}
</script>

<style lang="scss">
/* 非 scoped：pane 是子组件，公共表格样式统一在这里提供，全部限定在设置中心根节点内 */
.ssh-settings-root {
  .settings-tabs .el-tabs__content { max-height: 68vh; overflow: auto; padding-right: 6px; }

  .pane { display: flex; flex-direction: column; gap: 10px; font-size: 13px; }
  .pane-toolbar { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .pane-toolbar-stack { flex-direction: column; align-items: stretch; }
  .pane-hint { color: #888; font-size: 12px; line-height: 1.6; }
  .pane-notice { padding: 6px 10px; border-radius: 4px; background: rgba(64, 158, 255, 0.1); font-size: 12px; }
  .pane-empty { color: #999; text-align: center; padding: 18px; font-size: 12px; }
  .pane-row { display: flex; align-items: center; gap: 8px; }
  .pane-label { min-width: 74px; color: #888; font-size: 12px; }
  .pane-icon-btn { border: none; background: transparent; cursor: pointer; font-size: 15px; color: #999; }
  .pane-icon-btn:hover { color: #f56c6c; }
  .inline-check { display: inline-flex; align-items: center; gap: 4px; font-size: 12px; white-space: nowrap; }
  .file-button {
    display: inline-flex; align-items: center; padding: 4px 10px; border: 1px solid #dcdfe6;
    border-radius: 4px; cursor: pointer; font-size: 12px;
    input[type='file'] { display: none; }
  }

  .hl-layout, .macro-layout { display: flex; gap: 12px; min-height: 320px; }
  .hl-sets, .macro-list { width: 200px; flex-shrink: 0; border-right: 1px solid rgba(128, 128, 128, 0.2); padding-right: 8px; }
  .hl-rules, .macro-editor { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 8px; }
  .hl-set, .macro-item {
    display: flex; align-items: center; gap: 6px; padding: 6px; border-radius: 4px; cursor: pointer;
    &.active { background: rgba(64, 158, 255, 0.12); }
  }
  .hl-set-name, .macro-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hl-count, .macro-auto { font-size: 11px; color: #888; }
  .hl-rule-head, .hl-rule, .kb-head, .kb-row, .ex-head, .ex-row {
    display: grid; align-items: center; gap: 6px; font-size: 12px;
  }
  .hl-rule-head, .hl-rule { grid-template-columns: minmax(120px, 2fr) 62px 62px minmax(180px, 1.6fr) 22px; }
  .kb-head, .kb-row { grid-template-columns: 1fr 90px 140px 120px 1.2fr 36px 22px; }
  .ex-head, .ex-row { grid-template-columns: 1fr 1.4fr 1.4fr 44px 44px 44px 22px; }
  .hl-rule-head, .kb-head, .ex-head { color: #888; font-weight: 500; }
  .color-cell { display: flex; align-items: center; gap: 2px; input[type='color'] { width: 28px; height: 24px; padding: 0; border: none; } }
  .hl-options, .kb-mods { display: flex; gap: 6px; flex-wrap: wrap; label { white-space: nowrap; } }
  // 宏步骤：卡片式，内容区全宽多行，不再把所有控件挤在一行里
  .macro-save-strip {
    display: flex; align-items: center; gap: 8px; padding: 6px 10px;
    border-radius: 4px; font-size: 12px; background: rgba(64, 158, 255, 0.08);
    .pane-hint { flex: 1; }
  }
  .macro-items { display: flex; flex-direction: column; gap: 2px; }
  .macro-count { font-size: 11px; color: #888; flex-shrink: 0; }
  .macro-steps { display: flex; flex-direction: column; gap: 8px; }
  .macro-step-card {
    display: flex; flex-direction: column; gap: 6px; padding: 8px 10px;
    border: 1px solid rgba(128, 128, 128, 0.24); border-radius: 6px;
  }
  .step-head { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .step-spacer { flex: 1; }
  .step-index { width: 18px; color: #888; }
  .step-kind { width: 120px; flex-shrink: 0; }
  .step-arg { width: 132px; flex-shrink: 0; }
  .step-text { width: 100%; font-family: var(--font-family-mono); font-size: 12px; }
  .step-hint { color: #888; font-size: 12px; padding: 2px 0 4px; }
  .macro-name-input { max-width: 320px; }

  .scheme-list { display: flex; flex-direction: column; gap: 4px; }
  .scheme-item {
    display: flex; align-items: center; gap: 10px; padding: 7px 10px; border-radius: 4px; cursor: pointer;
    &.active { background: rgba(64, 158, 255, 0.12); }
  }
  .scheme-name { flex: 1; }
  .scheme-source, .log-meta { font-size: 11px; color: #888; }
  .scheme-swatches { display: flex; gap: 2px; i { width: 14px; height: 14px; border-radius: 2px; display: block; } }

  .log-list { max-height: 180px; overflow: auto; display: flex; flex-direction: column; gap: 2px; }
  .log-row { display: flex; align-items: center; gap: 8px; padding: 3px 4px; border-bottom: 1px solid rgba(128,128,128,0.12); }
  .log-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: monospace; }
  .log-preview { margin-top: 8px; font-family: monospace; }

  .vault-status { padding: 8px 12px; border-radius: 4px; font-weight: 500; }
  .vault-ok { background: rgba(34, 197, 94, 0.14); color: #16a34a; }
  .vault-locked { background: rgba(234, 179, 8, 0.14); color: #b45309; }
}


/* 保存状态：footer 插槽位于 .ssh-settings-root 之外，且宏面板内也复用同一套状态色 */
.ssh-settings-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
  padding: 0 8px;
  font-size: 12px;

  .footer-actions { display: flex; gap: 8px; flex-shrink: 0; }

  .save-state {
    display: inline-flex; align-items: center; gap: 6px;
    min-width: 0; max-width: 62%; padding: 2px 4px;
    border: none; background: transparent; cursor: pointer;
    font-size: 12px; color: #888; white-space: nowrap;
    overflow: hidden; text-overflow: ellipsis;
  }
}

// 状态色限定在设置中心的 footer 与宏面板内，避免 .save-error 这类通用类名污染全局
.ssh-settings-footer,
.macro-save-strip {
  .save-dot {
    display: inline-block; width: 7px; height: 7px; border-radius: 50%;
    flex-shrink: 0; background: #22c55e;
  }
  .save-saving .save-dot,
  .save-dot.save-saving { background: #eab308; animation: ssh-save-pulse 1s ease-in-out infinite; }
  .save-error .save-dot,
  .save-dot.save-error { background: #ef4444; }
  .save-error { color: #ef4444; }
}
@keyframes ssh-save-pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.35; } }
</style>
