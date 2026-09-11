<template>
  <div class="table-data-view">
    <!-- 顶部操作栏：新增/导出/导入 -->
    <div class="toolbar">
      <el-input
        v-model="whereClause"
        placeholder="WHERE 条件 (可选)"
        size="small"
        style="width: 240px"
        clearable
      />
      <el-input
        v-model="orderBy"
        placeholder="ORDER BY (可选)"
        size="small"
        style="width: 180px; margin-left: 8px"
        clearable
      />
      <app-button
        size="small"
        type="primary"
        style="margin-left: 8px"
        @click="loadData"
        :loading="loading"
        >查询</app-button
      >
      <app-button size="small" @click="showAddDialog = true">新增行</app-button>
      <app-button size="small" @click="handleExport">导出</app-button>
      <app-button size="small" @click="handleImport">导入</app-button>
    </div>
    <!-- 表格区 -->
    <div class="table-wrap">
      <el-table
        :data="tableData"
        border
        size="small"
        style="width: 100%"
        v-loading="loading"
        max-height="500"
      >
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
              <el-input
                v-model="editValue"
                size="small"
                @keyup.enter="saveEdit(row)"
                @blur="saveEdit(row)"
              />
            </span>
            <span v-else @dblclick="startEdit(row, idx)">{{ row[String(idx)] }}</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="100" fixed="right">
          <template #default="{ row }">
            <app-button link type="danger" size="small" @click="handleDeleteRow(row)"
              >删除</app-button
            >
          </template>
        </el-table-column>
      </el-table>
    </div>
    <!-- 底部查询/分页区：复用公共结果 footer（上一页/下一页、每页行数输入框、导出、刷新） -->
    <ResultFooter
      :page="page"
      :page-size="pageSize"
      :show-pager="true"
      :has-more="page * pageSize < total"
      :loading="loading"
      :export-sql="exportSql"
      :config-id="configId"
      :total="total"
      @prev="goPrev"
      @next="goNext"
      @page-size-change="handlePageSizeChange"
      @refresh="loadData"
      @totalQuantity="handleTotalQuantity"
    >
      <template #info>
        <el-icon class="info-icon"><DataAnalysis /></el-icon>
        <span class="stat-rows" :title="`共 ${total} 行`">共 {{ total.toLocaleString() }} 行</span>
        <span class="sep">·</span>
        <span>第 {{ page }} 页 / 当前 {{ tableData.length }} 行</span>
      </template>
    </ResultFooter>
    <!-- 新增行对话框（统一使用 GlobalDialog） -->
    <GlobalDialog v-model="showAddDialog" title="新增行" width="600px">
      <el-form label-width="100px">
        <el-form-item v-for="col in columns" :key="col.name" :label="col.name">
          <el-input v-model="addFormData[col.name]" :placeholder="col.data_type" size="small" />
        </el-form-item>
      </el-form>
      <template #footer>
        <app-button size="small" @click="showAddDialog = false">取消</app-button>
        <app-button size="small" type="primary" @click="handleAddRow">确定</app-button>
      </template>
    </GlobalDialog>

    <!-- 删除行确认 -->
    <GlobalDialog v-model="showDeleteConfirm" title="确认删除" width="400px">
      <div>确定要删除此行数据吗？</div>
      <template #footer>
        <app-button size="small" @click="showDeleteConfirm = false">取消</app-button>
        <app-button size="small" type="danger" plain @click="confirmDeleteRow">删除</app-button>
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
import { ref, reactive, onMounted, watch, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { DataAnalysis } from '@element-plus/icons-vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import ResultFooter from './ResultFooter.vue'
import {
  dbQueryTableData,
  dbInsertRow,
  dbUpdateRow,
  dbDeleteRow,
  dbExecuteSql
} from '../api/operations'
import { dbTableColumns } from '../api/metadata'
import type { ColumnInfo } from '../types'
import ImportExportDialog from './ImportExportDialog.vue'
import { useConnectionStore } from '../store'

const props = defineProps<{
  configId: string
  database: string
  table: string
}>()

const connectionStore = useConnectionStore()

/** 确保连接可用（关闭 tab 断开后重新打开时自动重连），返回是否就绪 */
async function ensureConnection(): Promise<boolean> {
  if (connectionStore.isConnected(props.configId)) return true
  const res = await connectionStore.connect(props.configId)
  if (res.code !== 200) {
    ElMessage.error('连接失败：' + (res.msg || '未知错误'))
    return false
  }
  return true
}

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

watch(
  () => [props.database, props.table],
  async () => {
    page.value = 1
    // 顺序加载：先列后数据，避免并发时旧表列名/新表数据错配
    await loadColumns()
    await loadData()
  }
)

async function loadColumns() {
  // 确保连接可用（关闭 tab 断开后重新打开时自动重连）
  if (!(await ensureConnection())) return
  try {
    const res = await dbTableColumns(props.configId, props.database, props.table)
    if (res.code === 200) {
      columns.value = res.data
      console.log(
        '[TableDataView] columns loaded:',
        res.data.length,
        res.data.map((c) => c.name)
      )
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
  // 确保连接可用（分页/查询按钮单独触发时同样生效）
  if (!(await ensureConnection())) return
  loading.value = true
  try {
    console.log(
      '[TableDataView] loadData:',
      props.database,
      props.table,
      page.value,
      pageSize.value,
      whereClause.value,
      orderBy.value
    )
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
      tableData.value = res.data.rows.map((row) => {
        const obj: Record<string, any> = {}
        res.data.columns.forEach((col, idx) => {
          obj[String(idx)] = row[idx]
        })
        return obj
      })
      console.log(
        '[TableDataView] data loaded:',
        tableData.value.length,
        'rows, total:',
        total.value
      )
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

/** 导出 SQL：按当前表构造（全量导出，不含 WHERE/ORDER BY 筛选） */
const exportSql = computed(() => `SELECT * FROM ${props.table}`)
const countSql = computed(() => `SELECT COUNT(*) AS total FROM ${props.table}`)
/** 上一页 */
function goPrev() {
  if (page.value > 1) {
    page.value -= 1
    loadData()
  }
}

/** 下一页 */
function goNext() {
  if (page.value * pageSize.value < total.value) {
    page.value += 1
    loadData()
  }
}

/** 每页行数变化：重置到第一页重新查询 */
function handlePageSizeChange(size: number) {
  pageSize.value = size
  page.value = 1
  loadData()
}
/** 单独查询总数量变化：更新总数量 */
async function handleTotalQuantity() {
  const res = await dbExecuteSql(
    props.configId,
    countSql.value,
    props.database
  )
  if (res.code === 200) {
    console.log(res)
    total.value = Number(res.data.rows[0][0]);
  } else {
    ElMessage.error('查询失败: ' + res.msg)
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

  const pkCol = columns.value.find((c) => c.is_primary_key)
  if (!pkCol) {
    ElMessage.warning('未找到主键列，无法更新')
    editingRow.value = null
    return
  }

  const pkIdx = columns.value.indexOf(pkCol)
  const pkValue = row[String(pkIdx)]

  const data: Record<string, any> = {}
  data[col.name] = editValue.value

  try {
    const res = await dbUpdateRow(
      props.configId,
      props.database,
      props.table,
      pkCol.name,
      pkValue,
      data
    )
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
      Object.keys(addFormData).forEach((k) => {
        addFormData[k] = ''
      })
      await loadData()
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e) {
    ElMessage.error('插入失败')
  }
}

async function handleDeleteRow(row: Record<string, any>) {
  const pkCol = columns.value.find((c) => c.is_primary_key)
  if (!pkCol) {
    ElMessage.warning('未找到主键列，无法删除')
    return
  }

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
    height: 100%;
    overflow: hidden;

}
.toolbar {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  row-gap: 4px;
  padding: 6px 8px;
}
.table-wrap {
  flex: 1;
  min-height: 0;
  overflow: auto;
}
.info-icon {
  font-size: 13px;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}
.stat-rows {
  font-family: var(--font-family-mono);
  white-space: nowrap;
}
.sep {
  color: var(--el-border-color);
  flex-shrink: 0;
}
</style>
