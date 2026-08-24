<template>
  <div class="table-data-view">
    <div class="toolbar">
      <el-input v-model="whereClause" placeholder="WHERE 条件 (可选)" size="small" style="width: 240px" clearable />
      <el-input v-model="orderBy" placeholder="ORDER BY (可选)" size="small" style="width: 180px; margin-left: 8px" clearable />
      <el-button size="small" type="primary" @click="loadData" :loading="loading">查询</el-button>
      <el-button size="small" @click="showAddDialog = true">新增行</el-button>
      <el-button size="small" @click="handleExport">导出</el-button>
      <el-button size="small" @click="handleImport">导入</el-button>
    </div>
    <el-table :data="tableData" border size="small" style="width: 100%" v-loading="loading" max-height="500">
      <el-table-column
        v-for="(col, idx) in columns"
        :key="idx"
        :prop="String(idx)"
        :label="col.name"
        min-width="120"
        show-overflow-tooltip
      >
        <template #default="{ row }">
          <span v-if="editingRow !== null && editingCol === idx">
            <el-input v-model="editValue" size="small" @keyup.enter="saveEdit(row)" @blur="saveEdit(row)" />
          </span>
          <span v-else @dblclick="startEdit(row, idx)">{{ row[String(idx)] }}</span>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="100" fixed="right">
        <template #default="{ row }">
          <el-button link type="danger" size="small" @click="handleDeleteRow(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>
    <div class="pagination">
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[50, 100, 200, 500, 1000]"
        layout="total, sizes, prev, pager, next"
        @size-change="loadData"
        @current-change="loadData"
      />
    </div>

    <!-- 新增行对话框（统一使用 GlobalDialog） -->
    <GlobalDialog v-model="showAddDialog" title="新增行" width="500px">
      <el-form label-width="100px">
        <el-form-item v-for="col in columns" :key="col.name" :label="col.name">
          <el-input v-model="addFormData[col.name]" :placeholder="col.data_type" size="small" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button size="small" @click="showAddDialog = false">取消</el-button>
        <el-button size="small" type="primary" @click="handleAddRow">确定</el-button>
      </template>
    </GlobalDialog>

    <!-- 删除行确认 -->
    <GlobalDialog v-model="showDeleteConfirm" title="确认删除" width="400px">
      <div>确定要删除此行数据吗？</div>
      <template #footer>
        <el-button size="small" @click="showDeleteConfirm = false">取消</el-button>
        <el-button size="small" type="danger" plain @click="confirmDeleteRow">删除</el-button>
      </template>
    </GlobalDialog>

    <!-- 导入导出对话框 -->
    <ImportExportDialog
      v-model="showImportExport"
      :config-id="configId"
      :database="database"
      :table="table"
      :mode="importExportMode"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, watch } from 'vue'
import { ElMessage } from 'element-plus'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { dbQueryTableData, dbInsertRow, dbUpdateRow, dbDeleteRow } from '../api/operations'
import { dbTableColumns } from '../api/metadata'
import type { ColumnInfo } from '../types'
import ImportExportDialog from './ImportExportDialog.vue'

const props = defineProps<{
  configId: string
  database: string
  table: string
}>()

const columns = ref<ColumnInfo[]>([])
const tableData = ref<Record<string, any>[]>([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(200)
const whereClause = ref('')
const orderBy = ref('')
const loading = ref(false)

// 编辑状态
const editingRow = ref<number | null>(null)
const editingCol = ref<number | null>(null)
const editValue = ref('')

// 新增行
const showAddDialog = ref(false)
const addFormData = reactive<Record<string, string>>({})

// 导入导出
const showImportExport = ref(false)
const importExportMode = ref<'import' | 'export'>('export')

onMounted(async () => {
  await loadColumns()
  await loadData()
})

watch(() => [props.database, props.table], () => {
  page.value = 1
  loadColumns()
  loadData()
})

async function loadColumns() {
  try {
    const res = await dbTableColumns(props.configId, props.database, props.table)
    if (res.code === 200) {
      columns.value = res.data
      console.log('[TableDataView] columns loaded:', res.data.length, res.data.map(c => c.name))
    } else {
      console.error('[TableDataView] loadColumns failed:', res.msg)
      ElMessage.error('加载字段失败: ' + res.msg)
    }
  } catch (e) {
    console.error('[TableDataView] loadColumns error:', e)
    ElMessage.error('加载字段失败')
  }
}

async function loadData() {
  loading.value = true
  try {
    console.log('[TableDataView] loadData:', props.database, props.table, page.value, pageSize.value, whereClause.value, orderBy.value)
    const res = await dbQueryTableData(
      props.configId,
      props.database,
      props.table,
      page.value,
      pageSize.value,
      whereClause.value || undefined,
      orderBy.value || undefined
    )
    if (res.code === 200) {
      total.value = res.data.total
      // 如果 columns 为空，从查询结果中填充列信息
      if (columns.value.length === 0 && res.data.columns.length > 0) {
        columns.value = res.data.columns
      }
      tableData.value = res.data.rows.map(row => {
        const obj: Record<string, any> = {}
        res.data.columns.forEach((col, idx) => {
          obj[String(idx)] = row[idx]
        })
        return obj
      })
      console.log('[TableDataView] data loaded:', tableData.value.length, 'rows, total:', total.value)
      console.log('[TableDataView] data res:', res)
    } else {
      console.error('[TableDataView] loadData failed:', res.msg)
      ElMessage.error('查询失败: ' + res.msg)
    }
  } catch (e) {
    console.error('[TableDataView] loadData error:', e)
    ElMessage.error('查询失败')
  } finally {
    loading.value = false
  }
}

function startEdit(row: Record<string, any>, colIdx: number) {
  editingRow.value = Number(row[String(0)]) // 使用第一列作为行标识
  editingCol.value = colIdx
  editValue.value = row[String(colIdx)] || ''
}

async function saveEdit(row: Record<string, any>) {
  if (editingRow.value === null || editingCol.value === null) return
  const col = columns.value[editingCol.value]
  if (!col) return

  const pkCol = columns.value.find(c => c.is_primary_key)
  if (!pkCol) { ElMessage.warning('未找到主键列，无法更新'); editingRow.value = null; return }

  const pkIdx = columns.value.indexOf(pkCol)
  const pkValue = row[String(pkIdx)]

  const data: Record<string, any> = {}
  data[col.name] = editValue.value

  try {
    const res = await dbUpdateRow(props.configId, props.database, props.table, pkCol.name, pkValue, data)
    if (res.code === 200) {
      ElMessage.success('更新成功')
      await loadData()
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e) {
    ElMessage.error('更新失败')
  }
  editingRow.value = null
  editingCol.value = null
}

async function handleAddRow() {
  try {
    const res = await dbInsertRow(props.configId, props.database, props.table, { ...addFormData })
    if (res.code === 200) {
      ElMessage.success('插入成功')
      showAddDialog.value = false
      Object.keys(addFormData).forEach(k => { addFormData[k] = '' })
      await loadData()
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e) {
    ElMessage.error('插入失败')
  }
}

async function handleDeleteRow(row: Record<string, any>) {
  const pkCol = columns.value.find(c => c.is_primary_key)
  if (!pkCol) { ElMessage.warning('未找到主键列，无法删除'); return }

  const pkIdx = columns.value.indexOf(pkCol)
  deletingRowPk.value = { col: pkCol.name, value: row[String(pkIdx)] }
  showDeleteConfirm.value = true
}

// 删除确认状态（统一使用 GlobalDialog）
const showDeleteConfirm = ref(false)
const deletingRowPk = ref<{ col: string; value: any } | null>(null)

async function confirmDeleteRow() {
  showDeleteConfirm.value = false
  const pk = deletingRowPk.value
  deletingRowPk.value = null
  if (!pk) return
  try {
    const res = await dbDeleteRow(props.configId, props.database, props.table, pk.col, pk.value)
    if (res.code === 200) {
      ElMessage.success('删除成功')
      await loadData()
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e) {
    ElMessage.error('删除失败')
  }
}

function handleExport() {
  importExportMode.value = 'export'
  showImportExport.value = true
}

function handleImport() {
  importExportMode.value = 'import'
  showImportExport.value = true
}
</script>

<style scoped>
.table-data-view {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px;
}
.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
}
.pagination {
  display: flex;
  justify-content: flex-end;
  padding: 8px 0;
}
</style>
