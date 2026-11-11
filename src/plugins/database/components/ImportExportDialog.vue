<template>
  <GlobalDialog
    v-model="visible"
    :title="mode === 'export' ? '导出数据' : '导入数据'"
    width="480px"
    @close="handleClose"
  >
    <!--
      label-width 与底部「导出查询结果」弹窗**刻意一致**（100px）：
      两个弹窗长得像一套，才不会让人以为是两个软件拼起来的。
    -->
    <el-form label-width="100px">
      <el-form-item label="目标表">
        <span class="form-static mono">{{ database ? `${database}.${table}` : table }}</span>
      </el-form-item>

      <template v-if="mode === 'export'">
        <el-form-item label="导出格式">
          <el-radio-group v-model="exportFormat">
            <el-radio value="csv">CSV</el-radio>
            <el-radio value="excel">Excel</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="保存路径">
          <PathField
            v-model="exportPath"
            placeholder="选择文件保存路径"
            browse-label="浏览保存路径"
            @browse="chooseExportPath"
          />
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
          <PathField
            v-model="importPath"
            placeholder="选择要导入的文件"
            browse-label="浏览导入文件"
            @browse="chooseImportFile"
          />
        </el-form-item>
        <el-form-item v-if="importFormat === 'csv'" label="包含表头">
          <el-switch v-model="hasHeader" />
        </el-form-item>
      </template>
    </el-form>

    <div class="dialog-note">
      {{
        mode === 'export'
          ? '导出由后端整表读取后写文件，不受网格分页影响。'
          : '导入逐行 INSERT，某一行失败不会回滚已成功的行；错误最多列前 10 条。'
      }}
    </div>

    <div v-if="importResult" class="import-result">
      <div class="result-title">导入结果</div>
      <el-alert
        :title="`成功 ${importResult.success_count} 行，失败 ${importResult.error_count} 行`"
        :type="importResult.error_count > 0 ? 'warning' : 'success'"
        :closable="false"
      />
      <div v-if="importResult.errors.length > 0" class="error-list">
        <div v-for="(err, idx) in importResult.errors" :key="idx" class="error-item">{{ err }}</div>
      </div>
    </div>

    <template #footer>
      <app-button size="small" @click="handleClose">取消</app-button>
      <app-button
        size="small"
        type="primary"
        :loading="executing"
        :disabled="!canExecute"
        @click="handleExecute"
      >
        {{ mode === 'export' ? '导出' : '导入' }}
      </app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { ElMessage } from 'element-plus'
import GlobalDialog from '@/components/GlobalDialog.vue'
import PathField from './PathField.vue'
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

/** 主动作能不能点：路径没选就禁用，比「点了弹一句请选择」体面 */
const canExecute = computed(() =>
  props.mode === 'export' ? !!exportPath.value : !!importPath.value
)

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
/* 说明性文字：与表单区隔开，别跟输入项抢注意力 */
.form-static {
  font-size: var(--font-size-md);
  color: var(--el-text-color-primary);
}
.mono {
  font-family: var(--font-family-mono);
}
.dialog-note {
  margin-top: 12px;
  font-size: var(--font-size-sm);
  line-height: 1.6;
  color: var(--el-text-color-secondary);
}
.import-result {
  margin-top: 12px;
}
.result-title {
  font-size: var(--font-size-md);
  font-weight: 600;
  margin-bottom: 6px;
  color: var(--el-text-color-primary);
}
.error-list {
  margin-top: 8px;
  max-height: 150px;
  overflow: auto;
  /* 用 EP danger-light-9 软红底（light=#fef0f0 / dark=#4d1214 已注入），跟随主题 */
  background: var(--el-color-danger-light-9);
  padding: 8px;
  border-radius: var(--radius-md);
}
.error-item {
  font-size: var(--font-size-base);
  /* 错误文字用 EP danger（light=#f56c6c / dark 已注入），跟随主题 */
  color: var(--stateErrorFg);
  line-height: 1.6;
}
</style>
