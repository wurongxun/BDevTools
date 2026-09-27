<template>
  <div class="query-result-view" :aria-label="`查询结果${table ? ' · ' + table : ''}`">
    <!-- ============ 头部：可编辑 SQL（编辑 / 追加条件 / 运行） ============
         语句来源：SQL 查询模式取 props.sql，表浏览模式由表名构造（见 resolvedSql）。
         导出 / 分页 / 总行数统计都在底部 ResultFooter，这里刻意不重复入口。 -->
    <SqlEditorHeader
      ref="headerRef"
      :model-value="sqlText"
      :loading="loading"
      :table-name="headerTableName"
      :rows="5"
      :is-single-table="!!editableTable"
      :can-undo="historyCursor > 0"
      :can-redo="historyCursor < history.length - 1"
      @update:model-value="onSqlInput"
      @run="() => runQuery()"
      @run-command="onRunCommand"
      @filter="onFilter"
      @undo="onUndo"
      @redo="onRedo"
      @format="onFormat"
      @template="onTemplate"
      @cancel="onCancel"
      @expand-change="onExpandChange"
    />

    <!-- ============ 模式提示 ============ -->
    <div v-if="result && result.is_query" class="qr-meta">
      <span v-if="canEdit" class="meta-editable">
        单表查询（<b class="mono">{{ editableTable }}</b>）：可编辑 / 新增 / 删除 / 导入 / 导出
      </span>
      <span v-else class="meta-readonly">多表或复杂查询：仅支持导出</span>
      <span v-if="selectedRows.length > 0" class="meta-selected">
        已选 {{ selectedRows.length }} 行（导出时默认只导出选中项）
      </span>
      <!-- 单表编辑入口：新增 / 导入属于「结果操作」，与 SQL 编辑区分开，
           故不放头部编辑器；导出与分页在底部 ResultFooter -->
      <span v-if="canEdit" class="meta-actions">
        <app-button size="small" @click="openAddDialog">新增行</app-button>
        <app-button size="small" @click="handleImport">导入</app-button>
      </span>
    </div>

    <!-- ============ 结果表 ============ -->
    <ResultTable
      v-if="result && result.is_query && result.columns.length > 0"
      :columns="tableColumns"
      :rows="displayRows"
      :loading="loading"
      :aria-busy="loading"
      :selectable="true"
      :sortable="true"
      :resizable="true"
      @sort-change="handleSortChange"
      @selection-change="handleSelectionChange"
    >
      <!-- 单表：单元格双击内联编辑（多表 cellSlotColumns 为空，不渲染） -->
      <template
        v-for="(col, idx) in cellSlotColumns"
        :key="idx"
        #[`cell-${idx}`]="{ row }"
      >
        <el-input
          v-if="isEditing(row.__rowIdx, idx)"
          v-model="editValue"
          size="small"
          @keyup.enter="saveEdit(row.__rowIdx)"
          @blur="saveEdit(row.__rowIdx)"
        />
        <span v-else class="editable-cell" @dblclick="startEdit(row.__rowIdx, idx)">{{ formatCell(row[String(idx)]) }}</span>
      </template>
      <!-- 单表：删除操作列 -->
      <template v-if="canEdit" #actions="{ row }">
        <app-button link type="danger" size="small" @click="handleDeleteRow(row.__rowIdx)">删除</app-button>
      </template>
    </ResultTable>

    <!-- 非查询语句（DML / DDL）：仅提示影响行数 -->
    <div v-else-if="result && !result.is_query" class="no-result">
      <el-empty description="该语句不是查询，无结果集返回" :image-size="50" />
    </div>

    <!-- 尚未执行的空状态 -->
    <div v-else class="no-result">
      <el-empty description="执行 SQL 查看结果" :image-size="60" />
    </div>

    <!-- ============ 底部状态 + 分页 + 导出 ============ -->
    <ResultFooter
      v-if="result"
      :page="page"
      :page-size="pageSize"
      :show-pager="!!result.is_query"
      :has-more="!!result.has_more"
      :loading="loading"
      :export-sql="exportSql"
      :config-id="configId"
      :total="result.total || 0"
      @prev="goPrev"
      @next="goNext"
      @page-size-change="handlePageSizeChange"
      @refresh="handleRefresh"
      @totalQuantity="handleTotalQuantity"
    >
      <template #info>
        <el-icon class="info-icon"><DataAnalysis /></el-icon>
        <template v-if="result.is_query">
          <span class="stat-rows" :title="hasTotal ? `共 ${result.total} 行` : ''">共 {{ totalRowsText }} 行</span>
          <span class="sep">·</span>
          <span>第 {{ page }} 页 / 当前 {{ result.rows.length }} 行</span>
          <span class="sep">·</span>
          <span>耗时 {{ result.execution_time_ms }} ms</span>
        </template>
        <template v-else>
          <span class="stat-rows">影响 {{ (result.affected_rows ?? 0).toLocaleString() }} 行</span>
          <span class="sep">·</span>
          <span>耗时 {{ result.execution_time_ms }} ms</span>
        </template>
      </template>
    </ResultFooter>

    <!-- 屏幕阅读器状态播报（WCAG 4.1.3 状态消息） -->
    <span class="sr-only" role="status" aria-live="polite">{{ loading ? '正在加载数据，请稍候' : '' }}</span>

    <!-- ============ 单表：新增行对话框 ============ -->
    <GlobalDialog v-model="showAddDialog" title="新增行" width="600px">
      <el-form label-width="100px">
        <el-form-item v-for="col in tableCols" :key="col.name" :label="col.name">
          <el-input v-model="addFormData[col.name]" :placeholder="col.data_type" size="small" />
        </el-form-item>
      </el-form>
      <template #footer>
        <app-button size="small" @click="showAddDialog = false">取消</app-button>
        <app-button size="small" type="primary" @click="handleAddRow">确定</app-button>
      </template>
    </GlobalDialog>

    <!-- ============ 单表：删除行确认 ============ -->
    <GlobalDialog v-model="showDeleteConfirm" title="确认删除" width="400px">
      <div>确定要删除此行数据吗？</div>
      <template #footer>
        <app-button size="small" @click="showDeleteConfirm = false">取消</app-button>
        <app-button size="small" type="danger" plain @click="confirmDeleteRow">删除</app-button>
      </template>
    </GlobalDialog>

    <!-- ============ 单表：导入 ============ -->
    <ImportExportDialog
      v-model="showImportExport"
      :config-id="configId || ''"
      :database="database || ''"
      :table="editableTable || ''"
      mode="import"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { DataAnalysis } from '@element-plus/icons-vue'
import ResultTable from './ResultTable.vue'
import ResultFooter from './ResultFooter.vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import ImportExportDialog from './ImportExportDialog.vue'
import { useConnectionStore } from '../store'
import { dbExecuteSql, dbInsertRow, dbUpdateRow, dbDeleteRow } from '../api/operations'
import { dbTableColumns } from '../api/metadata'
import type { ColumnInfo, SqlResult } from '../types'
import SqlEditorHeader from '@/components/sql-editor-mini/SqlEditorHeader.vue'

/**
 * 统一查询结果视图
 * - 语句来源（两种模式，语句都在本组件内）：
 *   · SQL 查询模式：调用方传 `sql`（如 SqlTab 传某个子结果的语句）；
 *   · 表数据浏览模式：调用方传 `table`，本组件自行构造 `SELECT * FROM \`table\``。
 * - 头部可编辑 SQL 并重新运行；结果支持分页、当前页排序、行选中。
 * - **单表查询**（能解析出唯一表且结果列都属于该表）→ 可编辑 / 新增 / 删除 / 导入 / 导出。
 * - **多表或复杂查询**（JOIN、子查询、UNION、多表逗号连接等）→ 仅支持导出。
 * - 导出：底部 ResultFooter 的导出按钮。若已选中行，默认只导出选中项（按主键构造过滤 SQL）。
 */
const props = defineProps<{
  /** 连接 ID（必填，未连接时无法执行） */
  configId?: string
  /** 数据库名 */
  database?: string
  /** SQL 查询模式的语句（与 table 二选一） */
  sql?: string
  /** 表数据浏览模式的表名（本组件据此构造 SELECT 语句） */
  table?: string
}>()

const connectionStore = useConnectionStore()

const DEFAULT_PAGE_SIZE = 200
const loading = ref(false)
const result = ref<SqlResult | null>(null)

/** 两种模式统一解析出的初始语句：优先 sql，其次由表名构造 */
const resolvedSql = computed(() => {
  const s = props.sql?.trim()
  if (s) return s
  return props.table ? `SELECT * FROM \`${props.table}\`` : ''
})

const sqlText = ref(resolvedSql.value)

// ===================================================================
// 头部编辑器：SQL 编辑历史（撤销 / 重做）
// ===================================================================
const headerRef = ref<InstanceType<typeof SqlEditorHeader> | null>(null)
const headerExpanded = ref(false)

/** 头部标签栏展示的表名：优先表浏览模式的表，其次解析出的可编辑表 */
const headerTableName = computed(() => props.table || editableTable.value || 'SQL')

const MAX_HISTORY = 50
const history = ref<string[]>([resolvedSql.value])
const historyCursor = ref(0)

/**
 * 写入一条历史。**撤销 / 重做自身不入栈**（直接改 sqlText），否则会自我污染：
 * 撤销后 redo 分支必须丢弃（截断到游标），与常见编辑器的语义一致。
 */
function pushHistory(sql: string) {
  if (history.value[historyCursor.value] === sql) return
  history.value = history.value.slice(0, historyCursor.value + 1)
  history.value.push(sql)
  if (history.value.length > MAX_HISTORY) history.value.shift()
  historyCursor.value = history.value.length - 1
}

/** 用户手动编辑（唯一入栈入口） */
function onSqlInput(v: string) {
  sqlText.value = v
  pushHistory(v)
}

function onUndo() {
  if (historyCursor.value <= 0) return
  historyCursor.value -= 1
  sqlText.value = history.value[historyCursor.value]
}

function onRedo() {
  if (historyCursor.value >= history.value.length - 1) return
  historyCursor.value += 1
  sqlText.value = history.value[historyCursor.value]
}

// ===================================================================
// 头部编辑器：运行 / 模板 / 格式化 / 过滤 / 取消
// ===================================================================
/** 运行下拉的四个动作；explain / count 会**改写编辑器内容**（便于再编辑与复查），入栈后可撤销 */
async function onRunCommand(cmd: string) {
  const sql = sqlText.value.trim()
  if (!sql) return
  switch (cmd) {
    case 'run':
      await runQuery()
      break
    case 'run-all':
      await runAllStatements(sql)
      break
    case 'explain':
      applyAndRun(`EXPLAIN ${sql}`)
      break
    case 'count':
      applyAndRun(`SELECT COUNT(*) AS total FROM (${sql.replace(/;\s*$/, '')}) AS _count`)
      break
  }
}

/** 改写编辑器内容并执行（入栈，可撤销回原语句） */
function applyAndRun(sql: string) {
  sqlText.value = sql
  pushHistory(sql)
  void runQuery()
}

/**
 * 执行所有语句：按 `;` 分割逐条提交（不解析字符串字面量内的分号，够用即可）。
 * 非查询语句累加影响行数，最后一条查询语句的结果用于展示。
 */
async function runAllStatements(sql: string) {
  if (!props.configId || loading.value) return
  const stmts = sql
    .split(';')
    .map(s => s.trim())
    .filter(Boolean)
  if (stmts.length <= 1) {
    await runQuery()
    return
  }
  if (!(await ensureConnection())) return
  loading.value = true
  try {
    let affected = 0
    let lastQuery: SqlResult | null = null
    for (const st of stmts) {
      const res = await dbExecuteSql(props.configId, st, props.database, pageSize.value, 0)
      if (res.code !== 200) {
        ElMessage.error(`执行失败: ${res.msg}`)
        return
      }
      if (res.data.is_query) lastQuery = res.data
      else affected += res.data.affected_rows ?? 0
    }
    if (lastQuery) {
      result.value = lastQuery
      page.value = 1
      await resolveEditable()
    }
    ElMessage.success(
      lastQuery
        ? `已执行 ${stmts.length} 条语句`
        : `已执行 ${stmts.length} 条语句，影响 ${affected} 行`
    )
  } catch (e: any) {
    ElMessage.error('执行失败: ' + (e.message || e))
  } finally {
    loading.value = false
  }
}

/** SQL 模板片段：追加到编辑器末尾（已识别到单表时自动带上表名） */
const SQL_TEMPLATES: Record<string, (table: string) => string> = {
  select: t => `SELECT * FROM ${quoteIdent(t)}`,
  insert: t => `INSERT INTO ${quoteIdent(t)} (col1, col2) VALUES ('v1', 'v2')`,
  update: t => `UPDATE ${quoteIdent(t)} SET col1 = 'v1' WHERE id = 1`,
  delete: t => `DELETE FROM ${quoteIdent(t)} WHERE id = 1`,
  'count-rows': t => `SELECT COUNT(*) FROM ${quoteIdent(t)}`,
}

function quoteIdent(t: string): string {
  return t ? `\`${t.replace(/`/g, '``')}\`` : '表名'
}

function onTemplate(cmd: string) {
  const gen = SQL_TEMPLATES[cmd]
  if (!gen) return
  const snippet = gen(editableTable.value || props.table || '')
  const next = sqlText.value.trim()
    ? `${sqlText.value.replace(/\s+$/, '')}\n${snippet}`
    : snippet
  sqlText.value = next
  pushHistory(next)
  headerRef.value?.expand()
}

/**
 * 极简 SQL 格式化：主要子句前换行 + 关键字大写。
 * 只做「看得清」级别的排版，不追求方言完整正确 —— 复杂语句可能不如专用格式化器，
 * 因此会入栈，不满意可 Ctrl+Z 撤销。字符串 / 标识符内部一律跳过。
 */
function formatSql(sql: string): string {
  const re =
    /\b(select|from|where|and|or|group\s+by|order\s+by|having|limit|offset|insert\s+into|values|update|set|delete\s+from|left\s+join|right\s+join|inner\s+join|join|on|union\s+all|union)\b/gi
  // 标记引号内的下标：这些位置的关键字不参与换行
  const quoted = new Array<boolean>(sql.length).fill(false)
  let quote: string | null = null
  for (let i = 0; i < sql.length; i++) {
    const c = sql[i]
    if (quote) {
      quoted[i] = true
      if (c === quote) quote = null
      continue
    }
    if (c === "'" || c === '"' || c === '`') {
      quote = c
      quoted[i] = true
    }
  }
  let out = ''
  let last = 0
  let m: RegExpExecArray | null
  re.lastIndex = 0
  while ((m = re.exec(sql))) {
    if (quoted[m.index]) continue
    out += sql.slice(last, m.index).replace(/\s+$/, '')
    if (out && !out.endsWith('\n')) out += '\n'
    out += m[0].toUpperCase() + ' '
    last = m.index + m[0].length
  }
  out += sql.slice(last)
  return out.replace(/[ \t]+\n/g, '\n').replace(/\n{2,}/g, '\n').trim()
}

function onFormat() {
  const next = formatSql(sqlText.value)
  if (next === sqlText.value) return
  sqlText.value = next
  pushHistory(next)
}

/** 过滤：只作用于当前页已加载的行（客户端过滤），不改写 SQL、不重新查询 */
const filterKeyword = ref('')

function onFilter(keyword: string) {
  filterKeyword.value = (keyword ?? '').trim()
}

/** 取消：放弃编辑，回到 tab 传来的语句（或表浏览模式构造的语句）并收起编辑器 */
function onCancel() {
  const s = resolvedSql.value
  if (s !== sqlText.value) {
    sqlText.value = s
    pushHistory(s)
  }
  headerRef.value?.collapse()
}

function onExpandChange(expanded: boolean) {
  headerExpanded.value = expanded
}

const page = ref(1)
const pageSize = ref(DEFAULT_PAGE_SIZE)

// 后端是否统计到总行数（仅纯 SELECT 首页返回）
const hasTotal = computed(() => result.value != null && result.value.total != null)
const totalRowsText = computed(() => {
  const r = result.value
  if (!r) return '0'
  const t = r.total ?? null
  return (t != null ? t : r.rows.length).toLocaleString()
})

// ===================================================================
// 单表 / 多表判定 + 编辑能力
// ===================================================================
const editableTable = ref<string | null>(null)
const tableCols = ref<ColumnInfo[]>([])

/** 主键列（单表才有） */
const pkCol = computed(() => tableCols.value.find(c => c.is_primary_key) ?? null)

/** 主键在当前结果列中的索引（-1 表示结果未包含主键列） */
const pkResultIndex = computed(() => {
  const r = result.value
  const pk = pkCol.value
  if (!r || !pk) return -1
  const i = r.columns.indexOf(pk.name)
  if (i >= 0) return i
  return r.columns.findIndex(c => String(c).toLowerCase() === pk.name.toLowerCase())
})

/** 是否可编辑（单表 + 有主键 + 结果含主键列 + 是查询结果） */
const canEdit = computed(
  () => !!editableTable.value && !!pkCol.value && pkResultIndex.value >= 0 && !!result.value?.is_query
)

/**
 * 从简单 SELECT 中解析唯一表名；多表 / JOIN / 子查询 / UNION 一律返回 null。
 * 仅支持 `SELECT ... FROM tbl [alias] [WHERE ...]` 这类单表语句。
 */
function detectSingleTable(sql: string): string | null {
  // 去掉行注释与块注释
  const s = sql
    .replace(/--[^\n]*/g, ' ')
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .trim()
    .replace(/;\s*$/, '')
  if (!/^select\b/i.test(s)) return null
  const fromIdx = s.search(/\bfrom\b/i)
  if (fromIdx < 0) return null
  let rest = s.slice(fromIdx + 4).trim()
  // 截断到下一个子句关键字
  const stop = rest.search(/\b(where|group|order|limit|having|union)\b/i)
  let fromPart = stop >= 0 ? rest.slice(0, stop) : rest
  if (/\bjoin\b/i.test(fromPart)) return null
  // 逗号连接的多表
  const items = fromPart
    .split(',')
    .map(x => x.trim())
    .filter(Boolean)
  if (items.length !== 1) return null
  // 仅接受「标识符 [别名]」，排除 FROM (子查询) 等
  const m = /^([`\w.]+)(?:\s+(?:as\s+)?\w+)?$/i.exec(items[0])
  if (!m) return null
  const t = m[1].split('.').pop()?.replace(/`/g, '')
  return t || null
}

/** 查询成功后判定该结果是否可编辑（单表且结果列均属于该表） */
async function resolveEditable() {
  editableTable.value = null
  tableCols.value = []
  const r = result.value
  if (!r || !r.is_query || !props.configId || !props.database) return
  const t = detectSingleTable(sqlText.value)
  if (!t) return
  try {
    const res = await dbTableColumns(props.configId, props.database, t)
    if (res.code !== 200) return
    const cols = res.data
    const names = new Set(cols.map(c => c.name.toLowerCase()))
    // 结果列必须全部属于该表（排除表达式列/跨表列）
    const allFromTable = r.columns.every(c => names.has(String(c).toLowerCase()))
    if (!allFromTable) return
    editableTable.value = t
    tableCols.value = cols
  } catch {
    /* 解析失败按不可编辑处理 */
  }
}

// ===================================================================
// 执行 / 翻页
// ===================================================================
/** 确保连接可用（关闭 tab 断开后重新打开时自动重连），返回是否就绪 */
async function ensureConnection(): Promise<boolean> {
  if (!props.configId) return false
  if (connectionStore.isConnected(props.configId)) return true
  const res = await connectionStore.connect(props.configId)
  if (res.code !== 200) {
    ElMessage.error('连接失败：' + (res.msg || '未知错误'))
    return false
  }
  return true
}

/** 执行当前 SQL（从头查询第一页） */
async function runQuery() {
  const sql = sqlText.value.trim()
  if (!sql || !props.configId || loading.value) return
  if (!(await ensureConnection())) return
  loading.value = true
  try {
    const res = await dbExecuteSql(props.configId, sql, props.database, pageSize.value, 0)
    if (res.code === 200) {
      result.value = res.data
      page.value = 1
      await resolveEditable()
    } else {
      ElMessage.error('查询失败: ' + res.msg)
    }
  } catch (e: any) {
    ElMessage.error('查询失败: ' + (e.message || e))
  } finally {
    loading.value = false
  }
}

/** 翻页：用当前 SQL 重新查询指定页 */
async function fetchPage(p: number, size: number) {
  const sql = sqlText.value.trim()
  if (!sql || !props.configId || loading.value) return
  if (!(await ensureConnection())) return
  loading.value = true
  try {
    const res = await dbExecuteSql(props.configId, sql, props.database, size, (p - 1) * size)
    if (res.code === 200) {
      result.value = { ...res.data, total: result.value?.total ?? res.data.total }
      page.value = p
      pageSize.value = size
      await resolveEditable()
    } else {
      ElMessage.error('查询失败: ' + res.msg)
    }
  } catch (e: any) {
    ElMessage.error('查询失败: ' + (e.message || e))
  } finally {
    loading.value = false
  }
}

function handlePageSizeChange(size: number) {
  fetchPage(1, size)
}

function goPrev() {
  if (page.value > 1) fetchPage(page.value - 1, pageSize.value)
}

function goNext() {
  if (result.value?.has_more) fetchPage(page.value + 1, pageSize.value)
}

/** 刷新：重跑第一页（重新统计 total） */
function handleRefresh() {
  fetchPage(1, pageSize.value)
}

/** 获取总行数：用当前 SQL 包装 COUNT 重新查询 */
async function handleTotalQuantity() {
  const sql = sqlText.value.trim()
  if (!sql || !props.configId || !result.value) return
  const countSql = `SELECT COUNT(*) AS total FROM (${sql}) AS _count`
  const res = await dbExecuteSql(props.configId, countSql, props.database)
  if (res.code === 200) {
    result.value = { ...result.value, total: Number(res.data.rows[0][0]) }
  } else {
    ElMessage.error('查询失败: ' + res.msg)
  }
}

// ===================================================================
// 排序 / 选中
// ===================================================================
const sortState = ref<{ prop: string; order: 'ascending' | 'descending' | null }>({ prop: '', order: null })
const selectedRows = ref<any[]>([])

function handleSortChange(prop: string, order: string | null) {
  sortState.value = { prop, order: order as 'ascending' | 'descending' | null }
}

function handleSelectionChange(selection: any[]) {
  selectedRows.value = selection
}

// 单元格比较：null/undefined 永远排末尾；数字按数值；其余按本地化字符串
function compareCell(a: any, b: any): number {
  if (a === null || a === undefined) return b === null || b === undefined ? 0 : 1
  if (b === null || b === undefined) return -1
  if (typeof a === 'number' && typeof b === 'number') return a - b
  return String(a).localeCompare(String(b), 'zh-Hans-CN')
}

/** 单元格格式化（null / undefined / 空串 显示为 -，对象序列化为 JSON） */
function formatCell(value: any) {
  if (value === null || value === undefined || value === '') return '-'
  if (typeof value === 'object') {
    try {
      return JSON.stringify(value)
    } catch {
      return String(value)
    }
  }
  return value
}

const tableColumns = computed<string[]>(() => result.value?.columns ?? [])

/** 仅单表可编辑时挂单元格编辑插槽；多表返回空数组（避免为每行渲染可编辑单元格） */
const cellSlotColumns = computed<string[]>(() => (canEdit.value ? tableColumns.value : []))

const displayRows = computed<Record<string, any>[]>(() => {
  const r = result.value
  if (!r || !r.is_query) return []
  const rows = r.rows.map((row, i) => {
    // __rowIdx 保留该行在 result.rows 中的原始下标（排序后仍可定位，编辑/删除用）
    const obj: Record<string, any> = { __rowIdx: i }
    r.columns.forEach((col, idx) => {
      obj[String(idx)] = row[idx]
    })
    return obj
  })
  // 头部过滤框：只过滤当前页已加载的行（客户端过滤，不改写 SQL）
  const kw = filterKeyword.value.toLowerCase()
  let list = rows
  if (kw) {
    list = rows.filter(o =>
      r.columns.some((_c, i) => String(o[String(i)] ?? '').toLowerCase().includes(kw))
    )
  }
  const { prop, order } = sortState.value
  if (prop && order) {
    const dir = order === 'ascending' ? 1 : -1
    list.sort((x, y) => dir * compareCell(x[prop], y[prop]))
  }
  return list
})

// ===================================================================
// 导出：选中行时默认只导出选中项
// ===================================================================
/** 值转 SQL 字面量（字符串加引号并转义单引号） */
function sqlLiteral(v: any): string {
  if (v === null || v === undefined) return 'NULL'
  if (typeof v === 'number' || typeof v === 'bigint') return String(v)
  if (typeof v === 'boolean') return v ? '1' : '0'
  const s = typeof v === 'object' ? JSON.stringify(v) : String(v)
  return `'${s.replace(/\\/g, '\\\\').replace(/'/g, "''")}'`
}

/** 按选中行构造「只导出选中项」的 SQL；无法安全构造时返回 null */
function buildSelectedExportSql(): string | null {
  const r = result.value
  const sel = selectedRows.value
  if (!r || !r.is_query || sel.length === 0) return null

  // 单表 + 结果含主键：按主键 IN 过滤（最稳妥）
  if (editableTable.value && pkCol.value && pkResultIndex.value >= 0) {
    const values = sel.map(row => sqlLiteral(row[String(pkResultIndex.value)]))
    return `SELECT * FROM \`${editableTable.value}\` WHERE \`${pkCol.value.name}\` IN (${values.join(', ')})`
  }

  // 多表：退化用行值 IN 过滤；存在 NULL 时行值比较必然不匹配，放弃精确导出
  const hasNull = sel.some(row => r.columns.some((_c, i) => row[String(i)] === null || row[String(i)] === undefined))
  if (hasNull) return null
  const cols = r.columns.map(c => `\`${String(c).replace(/`/g, '``')}\``).join(', ')
  const tuples = sel
    .map(row => `(${r.columns.map((_c, i) => sqlLiteral(row[String(i)])).join(', ')})`)
    .join(', ')
  return `SELECT * FROM (${sqlText.value.trim().replace(/;\s*$/, '')}) AS _sel WHERE (${cols}) IN (${tuples})`
}

/** 传给底部 ResultFooter 的导出语句：有选中行时只导出选中项 */
const exportSql = computed(() => {
  const r = result.value
  if (!r || !r.is_query) return ''
  return buildSelectedExportSql() ?? sqlText.value.trim()
})

// ===================================================================
// 单表编辑：内联修改 / 新增 / 删除
// ===================================================================
const editingRow = ref<number | null>(null)
const editingCol = ref<number | null>(null)
const editValue = ref('')

const showAddDialog = ref(false)
const addFormData = reactive<Record<string, string>>({})

const showDeleteConfirm = ref(false)
const deletingRowPk = ref<{ col: string; value: any } | null>(null)

const showImportExport = ref(false)

function isEditing(rowIdx: number, colIdx: number): boolean {
  return editingRow.value === rowIdx && editingCol.value === colIdx
}

/** 取原始值（用于编辑初值，null 显示为空串） */
function toEditableString(value: any): string {
  if (value === null || value === undefined) return ''
  return typeof value === 'object' ? JSON.stringify(value) : String(value)
}

function startEdit(rowIdx: number, colIdx: number) {
  if (!canEdit.value) return
  editingRow.value = rowIdx
  editingCol.value = colIdx
  editValue.value = toEditableString(result.value?.rows[rowIdx]?.[colIdx])
}

async function saveEdit(rowIdx: number) {
  if (editingRow.value === null || editingCol.value === null) return
  const colName = result.value?.columns[editingCol.value]
  const pk = pkCol.value
  const pkIdx = pkResultIndex.value
  editingRow.value = null
  editingCol.value = null
  if (!colName || !pk || pkIdx < 0 || !props.configId || !props.database || !editableTable.value) return

  const pkValue = result.value?.rows[rowIdx]?.[pkIdx]
  const data: Record<string, any> = { [colName]: editValue.value }
  try {
    const res = await dbUpdateRow(props.configId, props.database, editableTable.value, pk.name, pkValue, data)
    if (res.code === 200) {
      ElMessage.success('更新成功')
      await fetchPage(page.value, pageSize.value)
    } else {
      ElMessage.error(res.msg)
    }
  } catch {
    ElMessage.error('更新失败')
  }
}

function openAddDialog() {
  Object.keys(addFormData).forEach(k => {
    addFormData[k] = ''
  })
  showAddDialog.value = true
}

async function handleAddRow() {
  if (!props.configId || !props.database || !editableTable.value) return
  try {
    const res = await dbInsertRow(props.configId, props.database, editableTable.value, { ...addFormData })
    if (res.code === 200) {
      ElMessage.success('插入成功')
      showAddDialog.value = false
      await fetchPage(page.value, pageSize.value)
    } else {
      ElMessage.error(res.msg)
    }
  } catch {
    ElMessage.error('插入失败')
  }
}

function handleDeleteRow(rowIdx: number) {
  const pk = pkCol.value
  const pkIdx = pkResultIndex.value
  if (!pk || pkIdx < 0) {
    ElMessage.warning('未找到主键列，无法删除')
    return
  }
  deletingRowPk.value = { col: pk.name, value: result.value?.rows[rowIdx]?.[pkIdx] }
  showDeleteConfirm.value = true
}

async function confirmDeleteRow() {
  showDeleteConfirm.value = false
  const pk = deletingRowPk.value
  deletingRowPk.value = null
  if (!pk || !props.configId || !props.database || !editableTable.value) return
  try {
    const res = await dbDeleteRow(props.configId, props.database, editableTable.value, pk.col, pk.value)
    if (res.code === 200) {
      ElMessage.success('删除成功')
      await fetchPage(page.value, pageSize.value)
    } else {
      ElMessage.error(res.msg)
    }
  } catch {
    ElMessage.error('删除失败')
  }
}

function handleImport() {
  showImportExport.value = true
}

// 语句变化（切换子结果 / 切换表 / 表浏览模式）时同步编辑器并重新执行
watch(
  resolvedSql,
  (s) => {
    // 换语句 = 换上下文：旧的撤销栈与过滤词对新语句没有意义，一并重置
    sqlText.value = s
    filterKeyword.value = ''
    headerRef.value?.setFilter('')
    history.value = [s]
    historyCursor.value = 0
    runQuery()
  },
  { immediate: true }
)
</script>

<style scoped>
.query-result-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
/* 头部编辑器：高度由内容决定，不参与压缩（展开态不能被结果区挤扁） */
.sql-header {
  flex-shrink: 0;
}

/* 模式提示条 */
.qr-meta {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  padding: 4px var(--space-3);
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}
.meta-editable b {
  color: var(--el-color-primary);
}
.meta-selected {
  color: var(--el-color-primary);
  font-weight: 600;
}
.meta-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: var(--space-1);
  flex-shrink: 0;
}
.mono {
  font-family: var(--font-family-mono);
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
.no-result {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
.editable-cell {
  cursor: text;
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
  border: 0;
}
</style>
