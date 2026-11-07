<template>
  <GlobalDialog
    v-model="visible"
    :title="isEdit ? '编辑连接' : '新建连接'"
    width="560px"
    @close="handleClose"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="90px"
      label-position="right"
    >
      <el-form-item label="连接名称" prop="name">
        <el-input v-model="form.name" placeholder="请输入连接名称" />
      </el-form-item>

      <el-form-item label="协议类型" prop="agreement">
        <el-radio-group v-model="form.agreement">
          <el-radio-button size="small" value="ssh">SSH</el-radio-button>
          <el-radio-button size="small" value="sftp">SFTP</el-radio-button>
          <el-radio-button size="small" value="ftp" disabled title="FTP 协议暂未实现">FTP</el-radio-button>
        </el-radio-group>
      </el-form-item>

      <el-form-item label="主机地址" prop="address">
        <el-input v-model="form.address" placeholder="IP 地址或域名" />
      </el-form-item>

      <el-form-item label="端口" prop="port">
        <el-input-number
          v-model="form.port"
          :min="1"
          :max="65535"
          :step="1"
          style="width: 100%"
        />
      </el-form-item>

      <el-form-item label="用户名" prop="username">
        <el-input v-model="form.username" placeholder="登录用户名" />
      </el-form-item>

      <el-form-item label="认证方式" prop="auth_type">
        <el-radio-group v-model="form.auth_type">
          <el-radio-button size="small" value="password">密码</el-radio-button>
          <el-radio-button size="small" value="pubkey">私钥</el-radio-button>
          <el-radio-button size="small" value="agent">Agent</el-radio-button>
        </el-radio-group>
      </el-form-item>

      <template v-if="form.auth_type === 'pubkey'">
        <el-form-item label="私钥文件" prop="private_key_path">
          <div style="display: flex; gap: 8px; width: 100%">
            <el-input
              v-model="form.private_key_path"
              placeholder="选择私钥文件路径"
              readonly
              style="flex: 1"
            />
            <app-button size="default" @click="handleSelectKeyFile">浏览</app-button>
          </div>
        </el-form-item>
        <el-form-item label="私钥口令">
          <el-input
            v-model="form.passphrase"
            type="password"
            placeholder="加密私钥的口令（未加密可留空）"
            show-password
          />
        </el-form-item>
      </template>

      <el-form-item v-if="form.auth_type === 'password'" label="密码" prop="password">
        <el-input
          v-model="form.password"
          type="password"
          placeholder="登录密码"
          show-password
        />
      </el-form-item>

      <el-form-item label="字符编码">
        <el-select v-model="form.encoding" style="width: 100%">
          <el-option
            v-for="opt in ENCODING_OPTIONS"
            :key="opt.value"
            :label="opt.label"
            :value="opt.value"
          />
        </el-select>
      </el-form-item>

      <el-form-item label="终端类型">
        <el-select v-model="form.term_type" style="width: 100%" filterable allow-create>
          <el-option
            v-for="opt in TERM_TYPE_OPTIONS"
            :key="opt.value"
            :label="opt.label"
            :value="opt.value"
          />
        </el-select>
      </el-form-item>

      <el-form-item label="保活间隔">
        <el-input-number
          v-model="form.keepalive_interval"
          :min="0"
          :max="3600"
          :step="5"
          style="width: 100%"
        />
        <span class="form-hint">秒；0 表示不发送 SSH keepalive</span>
      </el-form-item>

      <el-form-item label="备注">
        <el-input v-model="form.description" placeholder="会话说明（可选）" />
      </el-form-item>

      <el-form-item label="颜色标签">
        <el-select v-model="form.color" placeholder="不标色" clearable style="width: 100%">
          <el-option v-for="c in SESSION_COLORS" :key="c.value" :label="c.label" :value="c.value">
            <span class="color-option"><i :style="{ background: c.value || 'transparent' }" />{{ c.label }}</span>
          </el-option>
        </el-select>
        <span class="form-hint">分组可用 <code>/</code> 写成多级路径，如「生产/华东」；树视图会按层级展开</span>
      </el-form-item>

      <el-form-item label="分组" prop="favorites_name">
        <el-select
          v-model="form.favorites_name"
          placeholder="选择分组（可选）"
          clearable
          filterable
          allow-create
          style="width: 100%"
        >
          <el-option
            v-for="g in groupOptions"
            :key="g"
            :label="g"
            :value="g"
          />
        </el-select>
      </el-form-item>

      <el-form-item label="置顶">
        <el-switch v-model="form.affix" />
      </el-form-item>

      <el-collapse class="advanced-collapse">
        <el-collapse-item title="高级：代理 / 跳板机 / 算法 / 二次验证" name="advanced">
          <div class="advanced-section">
            <div class="advanced-title">网络代理</div>
            <el-form-item label="代理类型" label-width="96px">
              <el-select v-model="advanced.proxy.kind" style="width: 100%">
                <el-option v-for="o in PROXY_KIND_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
            <template v-if="advanced.proxy.kind !== 'none'">
              <el-form-item label="代理地址" label-width="96px">
                <div class="inline-fields">
                  <el-input v-model="advanced.proxy.host" placeholder="代理主机" />
                  <el-input-number v-model="advanced.proxy.port" :min="1" :max="65535" controls-position="right" />
                </div>
              </el-form-item>
              <el-form-item label="代理账号" label-width="96px">
                <div class="inline-fields">
                  <el-input v-model="advanced.proxy.username" placeholder="用户名（可选）" />
                  <el-input v-model="advanced.proxy.password" type="password" placeholder="密码（可选）" show-password />
                </div>
              </el-form-item>
            </template>

            <div class="advanced-title">
              跳板机（ProxyJump 链）
              <app-button size="small" @click="addJumpHost">添加一级</app-button>
            </div>
            <p class="form-hint">按顺序逐级跳转；每一跳都会独立校验主机密钥。</p>
            <div v-for="(jump, idx) in advanced.jumpHosts" :key="idx" class="jump-row">
              <div class="jump-row-head">
                <span>第 {{ idx + 1 }} 跳</span>
                <app-button size="small" type="danger" @click="removeJumpHost(idx)">移除</app-button>
              </div>
              <div class="inline-fields">
                <el-input v-model="jump.host" placeholder="主机" />
                <el-input-number v-model="jump.port" :min="1" :max="65535" controls-position="right" />
              </div>
              <div class="inline-fields">
                <el-input v-model="jump.username" placeholder="用户名" />
                <el-select v-model="jump.authType" style="width: 100%">
                  <el-option label="密码" value="password" />
                  <el-option label="私钥" value="pubkey" />
                  <el-option label="Agent" value="agent" />
                </el-select>
              </div>
              <el-input v-if="jump.authType === 'password'" v-model="jump.password" type="password" placeholder="密码" show-password />
              <template v-else-if="jump.authType === 'pubkey'">
                <el-input v-model="jump.privateKeyPath" placeholder="私钥文件绝对路径" />
                <el-input v-model="jump.passphrase" type="password" placeholder="私钥口令（可选）" show-password />
              </template>
            </div>

            <div class="advanced-title">传输与算法</div>
            <el-form-item label="启用压缩" label-width="96px">
              <el-switch v-model="advanced.compress" />
            </el-form-item>
            <el-form-item label="加密算法" label-width="96px">
              <el-input v-model="advanced.ciphers" placeholder="如 aes256-ctr,aes128-ctr（留空用默认）" />
            </el-form-item>
            <el-form-item label="MAC" label-width="96px">
              <el-input v-model="advanced.macs" placeholder="如 hmac-sha2-256（留空用默认）" />
            </el-form-item>
            <el-form-item label="密钥交换" label-width="96px">
              <el-input v-model="advanced.kex" placeholder="如 diffie-hellman-group14-sha256" />
            </el-form-item>
            <el-form-item label="主机密钥" label-width="96px">
              <el-input v-model="advanced.hostKeyAlgo" placeholder="如 ssh-ed25519,rsa-sha2-256" />
            </el-form-item>

            <div class="advanced-title">认证与转发</div>
            <el-form-item label="二次验证" label-width="96px">
              <el-switch v-model="advanced.keyboardInteractive" />
              <span class="form-hint">开启后连接时弹框输入动态验证码（2FA / TOTP）</span>
            </el-form-item>
            <el-form-item label="Agent 转发" label-width="96px">
              <el-switch v-model="advanced.agentForward" />
            </el-form-item>
            <el-form-item label="DISPLAY" label-width="96px">
              <el-input v-model="advanced.x11Display" placeholder="如 localhost:10.0；X11 需另配 -L 6010 规则" />
            </el-form-item>
            <p class="form-hint">ssh2 0.9.6 未封装 X11 通道请求，此处只注入 DISPLAY，实际转发请用「端口转发」页的 -L 规则。</p>
          </div>
        </el-collapse-item>
      </el-collapse>
    </el-form>

    <template #footer>
      <app-button size="small" @click="handleClose">取消</app-button>
      <app-button size="small" type="primary" @click="handleSubmit" :loading="submitting">
        {{ isEdit ? '保存' : '创建' }}
      </app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import { open } from '@/core/runtime'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { useConnectionStore } from '../../store'
import {
  ConnectionType,
  ENCODING_OPTIONS,
  TERM_TYPE_OPTIONS,
  DEFAULT_CONNECTION_OPTIONS,
  PROXY_KIND_OPTIONS,
  SESSION_COLORS,
  defaultAdvancedOptions
} from '../../types'
import type { RemoteConnection, SshAdvancedOptions, SshJumpHost } from '../../types'
import { parseAdvancedOptions, serializeAdvancedOptions } from '../../utils/target'

const connectionStore = useConnectionStore()

const groupOptions = computed(() => connectionStore.connectionGroups)

const props = defineProps<{
  modelValue: boolean
  editData?: RemoteConnection | null
  defaultGroup?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const visible = ref(props.modelValue)
const isEdit = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()

const defaultForm = () => ({
  name: '',
  address: '',
  port: 22,
  username: '',
  password: '',
  agreement: ConnectionType.SSH,
  favorites_name: props.defaultGroup || '',
  affix: false,
  ...DEFAULT_CONNECTION_OPTIONS
})

const form = reactive(defaultForm())
// 高级选项独立成对象，提交时才序列化进 form.advanced_options
const advanced = reactive<SshAdvancedOptions>(defaultAdvancedOptions())

const rules: FormRules = {
  name: [{ required: true, message: '请输入连接名称', trigger: 'blur' }],
  address: [{ required: true, message: '请输入主机地址', trigger: 'blur' }],
  port: [{ required: true, message: '请输入端口号', trigger: 'blur' }],
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
  agreement: [{ required: true, message: '请选择协议类型', trigger: 'change' }],
  password: [{
    validator: (_rule: unknown, value: string, callback: (e?: Error) => void) => {
      if (form.auth_type === 'password' && !value) callback(new Error('请输入登录密码'))
      else callback()
    },
    trigger: 'blur'
  }]
}

/** 老数据 / 导入配置可能缺新字段，逐项兜底，避免表单出现 undefined */
function fillForm(data?: Partial<RemoteConnection> | null) {
  const base = defaultForm()
  Object.assign(form, base, data || {}, {
    passphrase: data?.passphrase ?? base.passphrase,
    encoding: data?.encoding || base.encoding,
    keepalive_interval: data?.keepalive_interval ?? base.keepalive_interval,
    term_type: data?.term_type || base.term_type,
    description: data?.description ?? base.description,
    advanced_options: data?.advanced_options ?? base.advanced_options,
    color: data?.color ?? base.color
  })
  Object.assign(advanced, defaultAdvancedOptions(), parseAdvancedOptions(form.advanced_options))
}

function addJumpHost() {
  const jump: SshJumpHost = {
    host: '', port: 22, username: '', password: '',
    authType: 'password', privateKeyPath: '', passphrase: ''
  }
  advanced.jumpHosts.push(jump)
}

function removeJumpHost(index: number) {
  advanced.jumpHosts.splice(index, 1)
}

watch(() => props.modelValue, (val) => {
  visible.value = val
  if (val && props.editData) {
    isEdit.value = true
    fillForm(props.editData)
  } else if (val) {
    isEdit.value = false
    fillForm()
  }
})

watch(visible, (val) => {
  emit('update:modelValue', val)
})

function handleClose() {
  visible.value = false
  formRef.value?.resetFields()
  fillForm()
  isEdit.value = false
}

async function handleSubmit() {
  if (!formRef.value) return
  // 密码必填与否由 rules 里的条件 validator 决定（公钥 / Agent 认证不需要登录密码）
  const valid = await formRef.value.validate().catch(() => false)
  if (!valid) return

  submitting.value = true
  try {
    const payload = {
      ...form,
      advanced_options: serializeAdvancedOptions(advanced)
    } as RemoteConnection
    if (isEdit.value && props.editData) {
      await connectionStore.updateConnection({ ...payload, id: props.editData.id })
    } else {
      await connectionStore.addConnection(payload)
    }
    handleClose()
  } finally {
    submitting.value = false
  }
}

async function handleSelectKeyFile() {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'Key Files', extensions: ['pem', 'key', 'rsa', 'ed25519', 'pub'] }]
  })
  if (selected) {
    form.private_key_path = selected
  }
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

:deep(.el-radio-group) {
  width: 100%;
  display: flex;
}
:deep(.el-radio-button) {
  flex: 1;
}
.form-hint {
  opacity: 0.7;
  margin: 0 0 var(--space-2) 0;
}
.advanced-collapse {
  margin-top: var(--space-2);
  width: 100%;
}
.advanced-section {
  :deep(.el-form-item) {
    margin-bottom: var(--space-3);
  }
}
.advanced-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  font-weight: 600;
  margin: var(--space-3) 0 var(--space-2) 0;

  @include theme.useTheme {
    color: theme.getVar('textPrimary');
  }
}
.inline-fields {
  display: flex;
  gap: var(--space-2);
  width: 100%;
  > * {
    flex: 1;
    min-width: 0;
  }
}
.jump-row {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-2);
  margin-bottom: var(--space-2);

  @include theme.useTheme {
    border: 1px solid theme.getVar('borderColor');
    border-radius: theme.getVar('borderRadius');
  }
}
.jump-row-head {
  display: flex;
  align-items: center;
  justify-content: space-between;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.color-option {
  display: inline-flex; align-items: center; gap: 8px;
  i {
    display: inline-block; width: 12px; height: 12px; border-radius: 2px;
    border: 1px solid rgba(128, 128, 128, 0.5);
  }
}
</style>
