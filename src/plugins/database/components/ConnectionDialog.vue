<template>
  <GlobalDialog
    v-model="visible"
    :title="isEdit ? '编辑连接' : '新建连接'"
    width="560px"
    @close="handleClose"
  >
    <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
      <el-form-item label="连接名称" prop="name">
        <el-input v-model="form.name" placeholder="请输入连接名称" />
      </el-form-item>
      <el-form-item label="数据库类型" prop="db_type">
        <el-select v-model="form.db_type" @change="onDbTypeChange">
          <el-option label="MySQL" value="mysql" />
          <el-option label="PostgreSQL" value="postgresql" />
        </el-select>
      </el-form-item>
      <el-form-item label="主机地址" prop="host">
        <el-input v-model="form.host" placeholder="localhost 或 IP 地址" />
      </el-form-item>
      <el-form-item label="端口" prop="port">
        <el-input-number v-model="form.port" :min="1" :max="65535" />
      </el-form-item>
      <el-form-item label="用户名" prop="username">
        <el-input v-model="form.username" placeholder="数据库用户名" />
      </el-form-item>
      <el-form-item label="密码" prop="password">
        <el-input v-model="form.password" type="password" show-password placeholder="数据库密码" />
      </el-form-item>
      <el-form-item label="默认数据库">
        <el-input v-model="form.database_name" placeholder="可选，连接后选择的默认数据库" />
      </el-form-item>

      <el-divider>SSH 隧道（可选）</el-divider>
      <el-form-item label="启用 SSH">
        <el-switch v-model="form.ssh_enabled" />
      </el-form-item>
      <template v-if="form.ssh_enabled">
        <el-form-item label="SSH 主机" prop="ssh_host">
          <el-input v-model="form.ssh_host" placeholder="SSH 服务器地址" />
        </el-form-item>
        <el-form-item label="SSH 端口">
          <el-input-number v-model="form.ssh_port" :min="1" :max="65535" :default-value="22" />
        </el-form-item>
        <el-form-item label="SSH 用户名">
          <el-input v-model="form.ssh_username" placeholder="SSH 用户名" />
        </el-form-item>
        <el-form-item label="SSH 密码">
          <el-input v-model="form.ssh_password" type="password" show-password placeholder="SSH 密码" />
        </el-form-item>
        <el-form-item label="私钥路径">
          <el-input v-model="form.ssh_key_path" placeholder="可选，SSH 私钥文件路径" />
        </el-form-item>
      </template>
    </el-form>
    <template #footer>
      <el-button size="small" sm @click="handleTest" :loading="testing">测试连接</el-button>
      <el-button size="small" @click="handleClose">取消</el-button>
      <el-button size="small" type="primary" @click="handleSubmit" :loading="submitting">确定</el-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import GlobalDialog from '@/components/GlobalDialog.vue'
import type { DbConnectionConfig } from '../types'
import { dbConnectionTest } from '../api/connection'

const props = defineProps<{
  modelValue: boolean
  editConfig?: DbConnectionConfig | null
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'submit': [config: DbConnectionConfig]
}>()

const visible = ref(false)
const isEdit = ref(false)
const testing = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()

const defaultForm = (): DbConnectionConfig => ({
  name: '',
  db_type: 'mysql',
  host: 'localhost',
  port: 3306,
  username: 'root',
  password: '',
  database_name: '',
  ssh_enabled: false,
  ssh_host: '',
  ssh_port: 22,
  ssh_username: '',
  ssh_password: '',
  ssh_key_path: '',
})

const form = reactive<DbConnectionConfig>(defaultForm())

const rules: FormRules = {
  name: [{ required: true, message: '请输入连接名称', trigger: 'blur' }],
  db_type: [{ required: true, message: '请选择数据库类型', trigger: 'change' }],
  host: [{ required: true, message: '请输入主机地址', trigger: 'blur' }],
  port: [{ required: true, message: '请输入端口', trigger: 'blur' }],
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
  password: [{ required: true, message: '请输入密码', trigger: 'blur' }],
}

watch(() => props.modelValue, (val) => {
  visible.value = val
  if (val) {
    if (props.editConfig) {
      isEdit.value = true
      Object.assign(form, props.editConfig)
    } else {
      isEdit.value = false
      Object.assign(form, defaultForm())
    }
  }
})

function onDbTypeChange(type: string) {
  form.port = type === 'mysql' ? 3306 : 5432
}

async function handleTest() {
  try {
    await formRef.value?.validate()
  } catch { return }
  testing.value = true
  try {
    const res = await dbConnectionTest({ ...form })
    if (res.code === 200) {
      ElMessage.success('连接成功')
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e: any) {
    ElMessage.error('连接测试失败: ' + (e.message || e))
  } finally {
    testing.value = false
  }
}

function handleSubmit() {
  formRef.value?.validate((valid) => {
    if (!valid) return
    submitting.value = true
    try {
      emit('submit', { ...form })
    } finally {
      submitting.value = false
    }
  })
}

function handleClose() {
  if (visible.value) visible.value = false
  emit('update:modelValue', false)
}
</script>
