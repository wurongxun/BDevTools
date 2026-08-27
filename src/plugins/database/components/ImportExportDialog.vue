<template>
  <GlobalDialog
    v-model="visible"
    :title="mode === 'export' ? '导出数据' : '导入数据'"
    width="480px"
    @close="handleClose"
  >
    <el-form label-width="100px">
      <el-form-item label="目标表">
        <span>{{ table }}</span>
      </el-form-item>

      <template v-if="mode === 'export'">
        <el-form-item label="导出格式">
          <el-radio-group v-model="exportFormat">
            <el-radio value="csv">CSV</el-radio>
            <el-radio value="excel">Excel</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="保存路径">
          <el-input v-model="exportPath" placeholder="选择文件保存路径" />
          <el-button size="small" @click="chooseExportPath" style="margin-left: 8px">浏览</el-button>
        </el-form-item>
      </template>

      <template v-else>
        <el-form-item label="文件格式">
          <el-radio-group v-model="importFormat">
            <el-radio value="csv">CSV</el-radio>
            <el-radio value="excel">Excel</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="选择文件">
          <el-input v-model="importPath" placeholder="选择要导入的文件" />
          <el-button size="small" @click="chooseImportFile" style="margin-left: 8px">浏览</el-button>
        </el-form-item>
        <el-form-item v-if="importFormat === 'csv'" label="包含表头">
          <el-switch v-model="hasHeader" />
        </el-form-item>
      </template>
    </el-form>

    <div v-if="importResult" class="import-result">
      <el-alert :title="`导入完成: 成功 ${importResult.success_count} 行, 失败 ${importResult.error_count} 行`" type="info" />
      <div v-if="importResult.errors.length > 0" class="error-list">
        <div v-for="(err, idx) in importResult.errors" :key="idx" class="error-item">{{ err }}</div>
      </div>
    </div>

    <template #footer>
      <el-button size="small" @click="handleClose">关闭</el-button>
      <el-button size="small" type="primary" @click="handleExecute" :loading="executing" :disabled="mode === 'import' && !importPath">
        {{ mode === 'export' ? '导出' : '导入' }}
      </el-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { save } from '@/core/runtime'
import { open } from '@/core/runtime'
import { dbExportCsv, dbExportExcel, dbImportCsv, dbImportExcel } from '../api/importExport'
import type { ImportResult } from '../types'

const props = defineProps<{
  modelValue: boolean
  configId: string
  database: string
  table: string
  mode: 'import' | 'export'
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const visible = ref(false)
const executing = ref(false)
const exportFormat = ref<'csv' | 'excel'>('csv')
const exportPath = ref('')
const importFormat = ref<'csv' | 'excel'>('csv')
const importPath = ref('')
const hasHeader = ref(true)
const importResult = ref<ImportResult | null>(null)

watch(() => props.modelValue, (val) => { visible.value = val })

async function chooseExportPath() {
  const ext = exportFormat.value === 'csv' ? 'csv' : 'xlsx'
  const path = await save({
    filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
    defaultPath: `${props.table}.${ext}`,
  })
  if (path) exportPath.value = path
}

async function chooseImportFile() {
  const ext = importFormat.value === 'csv' ? 'csv' : 'xlsx'
  const path = await open({
    filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
  })
  if (path) importPath.value = path
}

async function handleExecute() {
  executing.value = true
  try {
    if (props.mode === 'export') {
      if (!exportPath.value) { ElMessage.warning('请选择保存路径'); return }
      const fn = exportFormat.value === 'csv' ? dbExportCsv : dbExportExcel
      const res = await fn(props.configId, props.database, props.table, exportPath.value)
      if (res.code === 200) {
        ElMessage.success(`导出成功，共 ${res.data.row_count} 行`)
        handleClose()
      } else {
        ElMessage.error(res.msg)
      }
    } else {
      if (!importPath.value) { ElMessage.warning('请选择文件'); return }
      let res
      if (importFormat.value === 'csv') {
        res = await dbImportCsv(props.configId, props.database, props.table, importPath.value, hasHeader.value)
      } else {
        res = await dbImportExcel(props.configId, props.database, props.table, importPath.value)
      }
      if (res.code === 200) {
        importResult.value = res.data
        ElMessage.success(`导入完成，成功 ${res.data.success_count} 行`)
      } else {
        ElMessage.error(res.msg)
      }
    }
  } catch (e: any) {
    ElMessage.error('操作失败: ' + (e.message || e))
  } finally {
    executing.value = false
  }
}

function handleClose() {
  visible.value = false
  importResult.value = null
  emit('update:modelValue', false)
}
</script>

<style scoped>
.import-result {
  margin-top: 12px;
}
.error-list {
  margin-top: 8px;
  max-height: 150px;
  overflow: auto;
  background: #fef0f0;
  padding: 8px;
  border-radius: var(--radius-md);
}
.error-item {
  font-size: var(--font-size-base);
  color: #f56c6c;
  line-height: 1.6;
}
</style>
