<template>
  <el-dialog
    v-model="visible"
    :title="isEdit ? '编辑连接' : '新建连接'"
    width="520px"
    :close-on-click-modal="false"
    @close="handleClose"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="80px"
      label-position="right"
    >
      <el-form-item label="连接名称" prop="name">
        <el-input v-model="form.name" placeholder="请输入连接名称" />
      </el-form-item>

      <el-form-item label="协议类型" prop="agreement">
        <el-radio-group v-model="form.agreement">
          <el-radio-button value="ssh">SSH</el-radio-button>
          <el-radio-button value="sftp">SFTP</el-radio-button>
          <el-radio-button value="ftp">FTP</el-radio-button>
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
          <el-radio-button value="password">密码</el-radio-button>
          <el-radio-button value="pubkey">私钥</el-radio-button>
        </el-radio-group>
      </el-form-item>

      <el-form-item v-if="form.auth_type === 'pubkey'" label="私钥文件" prop="private_key_path">
        <div style="display: flex; gap: 8px; width: 100%">
          <el-input
            v-model="form.private_key_path"
            placeholder="选择私钥文件路径"
            readonly
            style="flex: 1"
          />
          <el-button size="default" @click="handleSelectKeyFile">浏览</el-button>
        </div>
      </el-form-item>

      <el-form-item :label="form.auth_type === 'pubkey' ? '密码(可选)' : '密码'" prop="password">
        <el-input
          v-model="form.password"
          type="password"
          :placeholder="form.auth_type === 'pubkey' ? '私钥密码（可选）' : '登录密码'"
          show-password
        />
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
    </el-form>

    <template #footer>
      <el-button @click="handleClose">取消</el-button>
      <el-button type="primary" @click="handleSubmit" :loading="submitting">
        {{ isEdit ? '保存' : '创建' }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import { open } from '@tauri-apps/plugin-dialog'
import { useConnectionStore } from '../../store'
import { ConnectionType } from '../../types'
import type { RemoteConnection } from '../../types'

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
  auth_type: 'password' as 'password' | 'pubkey',
  private_key_path: ''
})

const form = reactive(defaultForm())

const rules: FormRules = {
  name: [{ required: true, message: '请输入连接名称', trigger: 'blur' }],
  address: [{ required: true, message: '请输入主机地址', trigger: 'blur' }],
  port: [{ required: true, message: '请输入端口号', trigger: 'blur' }],
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
  agreement: [{ required: true, message: '请选择协议类型', trigger: 'change' }]
}

watch(() => props.modelValue, (val) => {
  visible.value = val
  if (val && props.editData) {
    isEdit.value = true
    Object.assign(form, props.editData)
  } else if (val) {
    isEdit.value = false
    Object.assign(form, defaultForm())
  }
})

watch(visible, (val) => {
  emit('update:modelValue', val)
})

function handleClose() {
  visible.value = false
  formRef.value?.resetFields()
  Object.assign(form, defaultForm())
  isEdit.value = false
}

async function handleSubmit() {
  if (!formRef.value) return
  const valid = await formRef.value.validate().catch(() => false)
  if (!valid) return

  submitting.value = true
  try {
    if (isEdit.value && props.editData) {
      await connectionStore.updateConnection({
        ...form,
        id: props.editData.id
      } as RemoteConnection)
    } else {
      await connectionStore.addConnection(form as RemoteConnection)
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
:deep(.el-dialog) {
  border-radius: 8px;
}
:deep(.el-radio-group) {
  width: 100%;
  display: flex;
}
:deep(.el-radio-button) {
  flex: 1;
}
</style>
