<template>
  <div class="query-result-view" :aria-label="`查询结果${table ? ' · ' + table : ''}`">
    <!-- ============ 头部：可编辑 SQL（编辑 / 追加条件 / 运行） ============
         语句来源：SQL 查询模式取 props.sql，表浏览模式由表名构造（见 resolvedSql）。
         导出 / 分页 / 总行数统计 / 新增 / 删除 / 导入 / 只读开关都在底部
         ResultFooter，这里刻意不重复入口。 -->
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
      <span v-else-if="canInsert" class="meta-partial" :title="readonlyReason">
        单表查询（<b class="mono">{{ editableTable }}</b>）：可新增行，但无法编辑 / 删除已有行 ——
        {{ readonlyReason }}
      </span>
      <span v-else class="meta-readonly" :title="readonlyReason">
        {{ readonlyReason || '多表或复杂查询：仅支持导出' }}（仍可导出）
      </span>
      <span v-if="columnFilters.length > 0 || columnSort" class="meta-filtered">
        已套用列过滤{{ columnFilters.length > 0 ? ` × ${columnFilters.length}` : '' }}{{ columnSort ? ' + 排序' : '' }}（列头 ▾ 菜单可移除）
      </span>
      <span v-if="bigResult" class="meta-perf" :title="perfHint">
        结果较大（{{ totalRowCount }} 行 × {{ tableColumns.length }} 列）：已按可视区渲染，滚动不会随数据量变慢
      </span>
      <span v-if="selectedRows.length > 0" class="meta-selected">
        已选 {{ selectedRows.length }} 行（导出时默认只导出选中项）
      </span>
      <!-- 无身份表：定位方式必须可见，否则「为什么改不动 / 为什么保存失败」全靠猜 -->
      <span
        v-if="rowKeyHint && rowKeyStrategy.kind !== 'primary'"
        class="meta-partial"
        :title="rowKeyExcludedTitle"
      >
        定位方式：{{ rowKeyHint
        }}<template v-if="rowKeyExcluded.length > 0"
          >（不参与：{{ rowKeyExcluded.map(e => e.col).join('、') }}）</template
        >
      </span>
    </div>

    <!-- ============ 未保存改动提示条 ============ -->
    <PendingChangesBar
      :count="pendingCount"
      :row-count="pendingRowCount"
      :err-count="cellErrors.size"
      :saving="saving"
      @save="startSave"
      @revert="revertAll"
      @preview="startSave"
    />

    <!-- ============ 结果表 ============
         单元格一律只渲染**纯文本**：编辑器是下面那个全局单例（点谁谁被接管）。
         上万个格子各建一个编辑组件是大表卡顿的主因 —— 那些格子 99.9% 根本没人点。
         点击用**事件委托**（容器上一个监听，而不是每格一个），靠 data-row / data-col 定位。 -->
    <div v-if="showTable" class="qr-grid" @click="onGridClick">
      <ResultTable
        ref="tableRef"
        :columns="tableColumns"
        :rows="displayRows"
        :new-row-keys="insertRowKeys"
        :column-metas="resultColumnMetas"
        :total-count="totalRowCount"
        :cell-tooltip="!canEdit"
        row-key="__key"
        :loading="loading"
        :aria-busy="loading"
        :selectable="true"
        :resizable="true"
        :showIndex="true"
        :column-menu="true"
        :row-key-columns="rowKeyColumns"
        :allow-row-key-pick="canPickRowKey"
        @sort-change="handleSortChange"
        @selection-change="handleSelectionChange"
        @column-menu="openColumnMenu"
        @pick-row-key="toggleVirtualKey"
        @clear-row-key="clearVirtualKey"
      >
        <template
          v-for="(col, idx) in cellSlotColumns"
          :key="idx"
          #[`cell-${idx}`]="{ row }"
        >
          <span
            v-if="!cellEditable(row, col)"
            class="qr-cell-text is-readonly"
            :title="`${cellReadonlyReason(row, col) || '只读'}｜${cellText(row, idx)}`"
            >{{ cellText(row, idx) }}</span
          >
          <span
            v-else
            class="qr-cell-text is-editable"
            :class="{
              'is-dirty': isCellDirty(row.__key, col),
              'has-error': !!cellErrors.get(cellErrKey(row.__key, col)),
            }"
            :data-row="row.__key"
            :data-col="idx"
            :title="cellTitle(row, col)"
            >{{ cellText(row, idx) }}</span
          >
        </template>
        <!-- 单表：删除操作列 -->
        <template v-if="canEdit" #actions="{ row }">
          <app-button
            link
            type="danger"
            size="small"
            :disabled="!isRowEditable(row)"
            :title="isRowEditable(row) ? undefined : '该行键值含 NULL，无法唯一定位'"
            @click="handleDeleteRow(row.__rowIdx)"
          >
            删除
          </app-button>
        </template>
      </ResultTable>
    </div>

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
      :total="result.total ?? null"
      :can-insert="canInsert"
      :can-edit="canEdit"
      :show-read-only-switch="hasEditControls"
      :has-selection="selectedRows.length > 0"
      :export-scope="exportScopeText"
      v-model:grid-read-only="gridReadOnly"
      @prev="goPrev"
      @next="goNext"
      @page-change="goToPage"
      @page-size-change="handlePageSizeChange"
      @refresh="handleRefresh"
      @totalQuantity="handleTotalQuantity"
      @fetch-all="requestFetchAll"
      @add-row="focusNewRow"
      @delete-selected="requestDeleteSelected"
      @import="handleImport"
    >
      <template #info>
        <el-icon class="info-icon"><DataAnalysis /></el-icon>
        <template v-if="result.is_query">
<!--          <span class="stat-rows" :title="hasTotal ? `共 ${result.total} 行` : ''">共 {{ totalRowsText }} 行</span>-->
<!--          <span class="sep">·</span>-->
<!--          <span>当前{{ result.rows.length }} 行</span>-->
<!--          <span class="sep">·</span>-->
          <span>{{ result.execution_time_ms }} ms</span>
        </template>
        <template v-else>
          <span class="stat-rows">影响 {{ (result.affected_rows ?? 0).toLocaleString() }} 行</span>
          <span class="sep">·</span>
          <span>{{ result.execution_time_ms }} ms</span>
        </template>
      </template>
    </ResultFooter>

    <!--
      全局唯一的单元格编辑器：谁被点，它就接管谁。
      用 `v-if` + `key` 挂起与销毁 —— 换一个单元格就换一次 key，内部状态天然是干净的。
      刻意放在表格的 v-if / v-else-if / v-else 链**之外**：插进链中间会打断分支匹配。
    -->
    <CellEditor
      v-if="editing"
      :key="editorSeq"
      :anchor="editing.anchor"
      :value="editingValue"
      :column="editingColumn"
      :kind="editingKind"
      :column-title="editing.col"
      @commit="onEditorCommit"
      @cancel="onEditorCancel"
    />

    <!-- 屏幕阅读器状态播报（WCAG 4.1.3 状态消息） -->
    <span class="sr-only" role="status" aria-live="polite">{{ loading ? '正在加载数据，请稍候' : '' }}</span>

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

    <!-- ============ 读取所有行：DBeaver 同款确认（图标 + 说明 + 不再询问 + Y/N） ============ -->
    <GlobalDialog v-model="showFetchAllConfirm" title="读取所有行" width="420px">
      <div class="fetch-all-body">
        <el-icon class="fetch-all-warn"><WarningFilled /></el-icon>
        <div class="fetch-all-text">
          <p>读取结果集中的所有行会花费大量时间和内存。</p>
          <p>确定要获取所有行吗？</p>
        </div>
      </div>
      <el-checkbox v-model="fetchAllDontAsk">不再询问</el-checkbox>
      <template #footer>
        <app-button size="small" @click="confirmFetchAll">是(Y)</app-button>
        <app-button size="small" type="primary" @click="denyFetchAll">否(N)</app-button>
      </template>
    </GlobalDialog>

    <!-- ============ 写库前的 SQL 预览 ============ -->
    <SqlPreviewDialog
      v-model="showPreview"
      :statements="previewStatements"
      :row-count="pendingOps.length"
      :saving="saving"
      @confirm="confirmSave"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, onUnmounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { DataAnalysis, WarningFilled } from '@element-plus/icons-vue'
import { showContextMenu } from '@/components/context-menu/useContextMenu'
import type { MenuItem } from '@/components/context-menu/ContextMenu.vue'
import ResultTable from './ResultTable.vue'
import ResultFooter from './ResultFooter.vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import ImportExportDialog from './ImportExportDialog.vue'
import { useConnectionStore } from '../store'
import { dbExecuteSql, dbApplyEdits, dbPreviewEdits } from '../api/operations'
import { dbTableColumns, dbTableIndexes } from '../api/metadata'
import type { ColumnInfo, IndexInfo, SqlResult, EditOp, EditOutcome, EditStatement } from '../types'
import SqlEditorHeader from '@/components/sql-editor-mini/SqlEditorHeader.vue'
import CellEditor from './CellEditor.vue'
import PendingChangesBar from './PendingChangesBar.vue'
import SqlPreviewDialog from './SqlPreviewDialog.vue'
import {
  cellDisplayText,
  isSqlNull,
  makeRowKey,
  mapResultColumns,
  resolveRowKeyStrategy,
  needsExactOneMatch,
  describeRowKey,
  sameValue,
  toWireValue,
  inferValueKind,
  normalizeCellInput,
  pickCellValue,
  committedNewRowKeys,
  filledColumns,
  hasMeaningfulValue,
  isNewRowKey,
  isRequiredWithoutDefault,
  missingRequiredColumns,
  nextNewRowKey,
  type RowKeyStrategy,
  type ExcludedColumn,
  type CellInput,
  type ValueKind,
} from '../utils/editing'
import { describeRowLimitGap, fetchAllSize } from '../utils/paging'

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
  /**
   * 调用方**已经执行好**的结果：有值时首屏直接展示、不再查一遍。
   * 多语句执行是逐条提交给后端的，每条结果都由调用方转交过来 —— 若本组件再按 SQL
   * 重跑一次，就是 N 次纯冗余往返。只在挂载那次生效，之后语句变化照常重新执行。
   */
  initialResult?: SqlResult | null
  /**
   * 调用方**强制**只读（视图数据、无法定位行的结果集等）。
   *
   * 与内部 `gridReadOnly` 开关的区别：那是用户自己的防误改习惯，随时能关；
   * 这里是结构性只读 —— 界面上不提供解除入口（底部只读开关会隐藏），
   * 否则会变成「界面允许改、执行必失败」的假象：视图大多本来就 UPDATE 不了。
   */
  readOnly?: boolean
}>()

const connectionStore = useConnectionStore()

const DEFAULT_PAGE_SIZE = 200
const loading = ref(false)
const result = ref<SqlResult | null>(null)

/** 网格只读开关的存储键（防误改；每个本机一次设定，长期有效） */
const GRID_READONLY_KEY = 'bdevtools:result-table:grid-readonly'

function readGridReadOnly(): boolean {
  try {
    return localStorage.getItem(GRID_READONLY_KEY) === 'true'
  } catch {
    return false
  }
}

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
/** 表的索引信息（无主键时用来找单列唯一索引作行定位） */
const tableIndexes = ref<IndexInfo[]>([])

/** 主键列们（单表才有；**复合主键要全部算上**，只取第一列会匹配到多行） */
const pkCols = computed(() => tableCols.value.filter(c => c.is_primary_key))

/** 主键在结果列中的下标们（-1 表示某个主键列不在结果里） */
const pkResultIndex = computed(() => {
  const r = result.value
  if (!r || pkCols.value.length === 0) return -1
  const i = r.columns.findIndex(c => c === pkCols.value[0]?.name)
  if (i >= 0) return i
  return r.columns.findIndex(c => String(c).toLowerCase() === (pkCols.value[0]?.name ?? '').toLowerCase())
})

/** 结果列名 → 表元数据（找不到 = 表达式列 / 跨表列） */
const columnMeta = computed(() => mapResultColumns(tableColumns.value, tableCols.value))

/**
 * 同上，但**按结果列顺序**展开成数组 —— 表格按下标渲染，
 * 而 `columnMeta` 是按列名索引的 Map。
 *
 * 单表浏览时有真实的 `ColumnInfo`（类型 / 可空 / 主键齐全）；
 * 纯 SQL 查询时整份为空，表格会退化成「按采样值推断并在表头标明是推断」。
 */
const resultColumnMetas = computed(() => tableColumns.value.map(name => columnMeta.value.get(name)))

/**
 * 行定位策略：主键 → 单列唯一索引 → 无。
 * 没有可靠的行定位手段时**一律不可编辑** —— 一条匹配到多行的 UPDATE 是事故，不是 bug。
 */
const rowKeyStrategy = computed<RowKeyStrategy>(() => {
  if (!editableTable.value || !result.value?.is_query) return { kind: 'none', reason: '' }
  return resolveRowKeyStrategy(tableColumns.value, tableCols.value, tableIndexes.value, {
    virtualKey: virtualKeyCols.value,
    allowFullMatch: true,
  })
})

/**
 * 这条 UPDATE / DELETE 是否必须「恰好命中一行」（命中数不对 ⇒ 整批回滚）。
 *
 * 只有**数据库担保唯一**的定位（主键 / 唯一索引）才敢放宽：那种情况下
 * affected = 0 只是「这行已被别处删掉」，是可接受的结局；而拼出来的定位条件
 * 一旦命中 0 行或多行，说明它压根没定位住那一行 —— 继续提交就是在赌。
 */
const expectOneMatch = computed(() => needsExactOneMatch(rowKeyStrategy.value))

/** 当前靠什么定位一行（无身份表必须让用户看得见，否则「为什么改不动」全靠猜） */
const rowKeyHint = computed(() => describeRowKey(rowKeyStrategy.value))
/** 整行匹配时被排除的列：要解释「哪些列没参与定位、为什么」 */
const rowKeyExcluded = computed<ExcludedColumn[]>(() => {
  const s = rowKeyStrategy.value
  return s.kind === 'full' ? s.excluded : []
})

/**
 * 当前用来定位一行的列（传给表格，用于在列头做标记）。
 * 整行匹配时**刻意不给**：那种情况本来就是「每一列都参与」，逐列打标记反而看不出重点
 * （提示条里已经整句说明了）。
 */
const rowKeyColumns = computed<string[]>(() => {
  const s = rowKeyStrategy.value
  return s.kind === 'primary' || s.kind === 'unique' || s.kind === 'virtual' ? s.cols : []
})

/** 被排除的列的完整说明（悬停可见：为什么这些列没参与定位） */
const rowKeyExcludedTitle = computed(() =>
  rowKeyExcluded.value.map(e => `${e.col}：${e.reason}`).join('；')
)

/** 键列在结果中的下标（用于从原始行里取值拼 WHERE） */
const keyColumnIndexes = computed<number[]>(() => {
  if (rowKeyStrategy.value.kind === 'none') return []
  const cols = tableColumns.value
  return rowKeyStrategy.value.cols.map(name => {
    const exact = cols.indexOf(name)
    if (exact >= 0) return exact
    return cols.findIndex(c => c.toLowerCase() === name.toLowerCase())
  })
})

/** 结果集中出现同名列：更新时无法区分打到哪一列，直接禁编辑 */
const hasDuplicateColumns = computed(() => {
  const seen = new Set<string>()
  for (const c of tableColumns.value) {
    const key = String(c).toLowerCase()
    if (seen.has(key)) return true
    seen.add(key)
  }
  return false
})

/** 每列的可编辑性 + 编辑器类型 */
const columnEditInfo = computed(() => {
  const map = new Map<string, { editable: boolean; reason?: string; kind: any }>()
  for (const name of tableColumns.value) {
    if (!canEdit.value) {
      map.set(name, { editable: false, reason: readonlyReason.value, kind: 'text' })
      continue
    }
    const meta = columnMeta.value.get(name)
    const info = computeEditability(meta)
    map.set(name, hasDuplicateColumns.value
      ? { ...info, editable: false, reason: '结果中存在同名列，无法确定写哪一列' }
      : info)
  }
  return map
})

function computeEditability(meta: ColumnInfo | undefined) {
  if (!meta) return { editable: false, reason: '结果列不属于该表（表达式或跨表列）', kind: 'text' as const }
  if (meta.is_generated) return { editable: false, reason: '计算列，由数据库生成', kind: 'text' as const }
  const kind = inferValueKind(meta)
  if (kind === 'binary') return { editable: false, reason: '二进制列暂不支持编辑', kind }
  return { editable: true, kind }
}

/**
 * 网格只读开关（防止误改；持久化到 localStorage，跟随本机习惯）。
 *
 * 初值把宿主的 `readOnly` **或**进来：结构性只读的场景里，即使本机习惯是「可编辑」，
 * 也不能一进来就摆出一副能改的样子（用户改半天点保存才发现失败）。
 */
const gridReadOnly = ref(!!props.readOnly || readGridReadOnly())
watch(
  () => props.readOnly,
  v => {
    if (v) gridReadOnly.value = true
  }
)
watch(gridReadOnly, v => {
  try {
    localStorage.setItem(GRID_READONLY_KEY, String(v))
  } catch {
    /* 存不下就只在本次会话生效 */
  }
})

// ==================== 虚拟行标识（伪主键） ====================
/**
 * 没有主键、也没有唯一索引的表，靠什么定位一行**只能由用户说了算**
 * （MySQL Workbench 的 pseudo primary key、DBeaver 的 virtual key 都是这个思路）。
 *
 * 作用域是「连接 + 库 + 表」而不是「表」：唯一性只在这一点上成立，
 * 换张表（甚至换个库里的同名表）都不能沿用。
 *
 * 声明归声明，**执行时仍然要校验**（见 `expectOneMatch`）——
 * 用户挑错了列最多是保存失败，不该是误改一片。
 */
const VIRTUAL_KEY_PREFIX = 'bdevtools:result-table:rowkey:'
const virtualKeyCols = ref<string[]>([])

const virtualKeyScope = computed(() => {
  if (!editableTable.value) return ''
  return `${props.configId ?? ''}.${props.database ?? ''}.${editableTable.value}`
})

function loadVirtualKey(): string[] {
  const scope = virtualKeyScope.value
  if (!scope) return []
  try {
    const raw = localStorage.getItem(VIRTUAL_KEY_PREFIX + scope)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    return Array.isArray(parsed) ? parsed.filter((c): c is string => typeof c === 'string') : []
  } catch {
    return []
  }
}

watch(
  virtualKeyScope,
  () => {
    virtualKeyCols.value = loadVirtualKey()
  },
  { immediate: true }
)

function persistVirtualKey(cols: string[]) {
  const scope = virtualKeyScope.value
  if (!scope) return
  try {
    if (cols.length === 0) localStorage.removeItem(VIRTUAL_KEY_PREFIX + scope)
    else localStorage.setItem(VIRTUAL_KEY_PREFIX + scope, JSON.stringify(cols))
  } catch {
    /* 存不下就只在本次会话生效 */
  }
}

/** 列头右键「设为行标识」：已经选中的列再点一次 = 取消，否则并入当前选择（可多选） */
function toggleVirtualKey(col: string) {
  const lower = col.toLowerCase()
  const has = virtualKeyCols.value.some(c => c.toLowerCase() === lower)
  const next = has ? virtualKeyCols.value.filter(c => c.toLowerCase() !== lower) : [...virtualKeyCols.value, col]
  virtualKeyCols.value = next
  persistVirtualKey(next)
  ElMessage.success(has ? `已取消行标识：${col}` : `已设为行标识：${next.join('、')}`)
}

function clearVirtualKey() {
  if (virtualKeyCols.value.length === 0) return
  virtualKeyCols.value = []
  persistVirtualKey([])
  ElMessage.info('已清除手动指定的行标识')
}

/** 允许在列头指定行标识：这张表本来就没有「数据库担保唯一」的定位手段时才有意义 */
const canPickRowKey = computed(() => {
  const k = rowKeyStrategy.value.kind
  return hasEditControls.value && (k === 'none' || k === 'virtual' || k === 'full')
})

/**
 * 结果集属于「某张可写的单表」—— 编辑类控件（新增 / 删除 / 导入 / 只读开关）
 * 在底部 Footer 的显示前提。**刻意不含只读开关自己的状态**：
 * 开关一旦打开 canInsert / canEdit 都会变 false，若拿它们当显示条件，
 * 开关会在点开的瞬间把自己藏起来，用户再也关不掉。
 */
const hasEditControls = computed(
  () =>
    !!editableTable.value &&
    !!result.value?.is_query &&
    !hasDuplicateColumns.value &&
    !props.readOnly
)

/**
 * 能不能**新增**行。
 *
 * 刻意**不要求**行定位手段：INSERT 只需要知道往哪张表插，不需要「唯一定位某一行」——
 * 那是 UPDATE / DELETE 才有的需求。以前这两件事绑在同一个 `canEdit` 上，
 * 结果是没主键的表连新增都被一起禁掉，而那本来是完全安全的操作。
 * INSERT 也不需要表里已有数据 —— 空结果集同样可以插第一行。
 */
const canInsert = computed(() => hasEditControls.value && !gridReadOnly.value)

/** 能不能改**已有**行：定位不到行就不许改（一条匹配多行的 UPDATE 不是小失误，是事故） */
const canEdit = computed(() => canInsert.value && rowKeyStrategy.value.kind !== 'none')

/** 不可编辑的原因（展示给用户，比一句「只读」有用得多） */
const readonlyReason = computed(() => {
  if (props.readOnly) return '视图数据不支持写操作（绝大多数视图不可更新）'
  if (!result.value?.is_query) return '该语句不是查询'
  if (!editableTable.value) return '多表 / 复杂查询：无法定位到唯一表'
  if (hasDuplicateColumns.value) return '结果存在同名列'
  if (rowKeyStrategy.value.kind === 'none') return rowKeyStrategy.value.reason
  if (gridReadOnly.value) return '已开启只读（底部开关可关闭）'
  return ''
})

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

/**
 * 查询成功后判定该结果是否可编辑，并加载行定位所需的元数据。
 *
 * **语句基底必须取「未套 Used 列过滤前的原句」**：列头过滤会把语句包成
 * `SELECT * FROM (原句) AS _flt …`，而 `detectSingleTable` 不认这种包装 ——
 * 早先直接用 `sqlText` 判定，导致「一用列过滤，编辑能力就消失」。
 */
async function resolveEditable() {
  editableTable.value = null
  tableCols.value = []
  tableIndexes.value = []
  clearPending({ silent: true })
  const r = result.value
  if (!r || !r.is_query || !props.configId || !props.database) return
  const base = preFilterSql.value.trim() || sqlText.value
  const t = detectSingleTable(base)
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
    // 索引只用于「没有主键时找单列唯一索引」，失败不影响编辑能力本身
    const idxRes = await dbTableIndexes(props.configId, props.database, t).catch(() => null)
    if (idxRes && idxRes.code === 200) tableIndexes.value = idxRes.data
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

/** 执行当前 SQL（从头查询第一页）。有未保存改动时先问一句，别默默丢掉 */
async function runQuery() {
  const sql = sqlText.value.trim()
  if (!sql || !props.configId || loading.value) return
  if (!(await guardPending())) return
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

/** 翻页：用当前 SQL 重新查询指定页。同样先拦一下未保存改动 */
async function fetchPage(
  p: number,
  size: number,
  opts?: { skipGuard?: boolean; skipGapCheck?: boolean }
) {
  const sql = sqlText.value.trim()
  if (!sql || !props.configId || loading.value) return
  if (!opts?.skipGuard && !(await guardPending())) return
  if (!(await ensureConnection())) return
  loading.value = true
  try {
    const res = await dbExecuteSql(props.configId, sql, props.database, size, (p - 1) * size)
    if (res.code === 200) {
      result.value = { ...res.data, total: result.value?.total ?? res.data.total }
      page.value = p
      pageSize.value = size
      await resolveEditable()
      // 请求 N 行却只回来 M 行：分页契约里这是不可能的，说明中途有人把行数改小了。
      // 一定要说出来 —— 否则表现出来就是「填了页大小，界面毫无变化」。
      const gap = opts?.skipGapCheck
        ? null
        : describeRowLimitGap(size, res.data.rows.length, !!res.data.has_more)
      if (gap) ElMessage.warning(gap)
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

/** 页码输入框跳转。与 goPrev / goNext 走同一条 fetchPage（含未保存改动拦截） */
function goToPage(p: number) {
  if (p !== page.value) fetchPage(p, pageSize.value)
}

// ==================== 读取所有行 ====================
/**
 * DBeaver 同款确认：「读取所有行」可能拉回几百万行，时间和内存都是实打实的代价，
 * 必须先问一句；「不再询问」记下来，下次直接取。
 */
const FETCH_ALL_SKIP_KEY = 'bdevtools:database:fetch-all-skip-confirm'
const showFetchAllConfirm = ref(false)
const fetchAllDontAsk = ref(false)

function requestFetchAll() {
  if (!result.value?.has_more || loading.value) return
  let skip = false
  try {
    skip = localStorage.getItem(FETCH_ALL_SKIP_KEY) === '1'
  } catch {
    /* 存不下就每次都问 */
  }
  if (skip) {
    doFetchAll()
    return
  }
  fetchAllDontAsk.value = false
  showFetchAllConfirm.value = true
}

/** 是(Y)：勾了「不再询问」就记住；否则下次还问 */
function confirmFetchAll() {
  if (fetchAllDontAsk.value) {
    try {
      localStorage.setItem(FETCH_ALL_SKIP_KEY, '1')
    } catch {
      /* 忽略：只是不再记住 */
    }
  }
  showFetchAllConfirm.value = false
  doFetchAll()
}

function denyFetchAll() {
  showFetchAllConfirm.value = false
}

/** 真正去取：已知总行数按总数要；未知走兜底上限（并跳过「行数降级」报警 —— 那是主动限流） */
function doFetchAll() {
  const size = fetchAllSize(result.value?.total ?? null)
  fetchPage(1, size, { skipGapCheck: size === result.value?.total ? false : true })
}

// Y / N 快捷键（对话框打开期间生效；DBeaver 同款）
function onFetchAllKeydown(e: KeyboardEvent) {
  const target = e.target as HTMLElement | null
  if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) {
    return
  }
  const k = e.key.toLowerCase()
  if (k === 'y') confirmFetchAll()
  else if (k === 'n') denyFetchAll()
}

watch(showFetchAllConfirm, v => {
  if (v) window.addEventListener('keydown', onFetchAllKeydown, true)
  else window.removeEventListener('keydown', onFetchAllKeydown, true)
})
onUnmounted(() => window.removeEventListener('keydown', onFetchAllKeydown, true))

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
// 列头菜单：DBeaver 式字段排序 / 过滤
// ===================================================================
// 思路：**不解析原语句**，把当前语句包成子查询后在外层追加 WHERE / ORDER BY：
//   SELECT * FROM (原语句) AS _flt WHERE cond1 AND cond2 ORDER BY `col` DESC
// 任何复杂语句（JOIN / UNION / 聚合）都安全。
//
// 条件的**替换规则**（关键：同一字段重复设置不能叠加成 `id = 1 AND id = 2`，
// 那会永远查出空集）：
//   · 不同字段        → 各自保留，AND 连接（多字段过滤是本功能的主要用途）
//   · 同字段 同操作符 → 替换（改条件值：id = 1 再设 id = 2 ⇒ id = 2）
//   · 同字段 区间操作符（> >= < <=）→ 同方向替换、反方向共存：
//     `id > 1` 再设 `id >= 5` ⇒ 只剩 `id >= 5`；`id > 1` 再设 `id < 10` ⇒ 两条都留
//   · 同字段 非区间操作符（= <> LIKE IS [NOT] NULL）→ 该字段的旧条件（含区间）全部
//     作废。它们两两矛盾，`id = 1 AND id > 5` 只会查出空集。
// 「移除所有」时还原到进入过滤前的语句（preFilterSql）。

type FilterOp = '=' | '<>' | '>' | '>=' | '<' | '<=' | 'LIKE' | 'IS NULL' | 'IS NOT NULL'

/** 自定义过滤允许的操作符（IS NULL 两个不需要输入值，排除） */
type CustomOp = Extract<FilterOp, '=' | '<>' | '>' | '>=' | '<' | '<=' | 'LIKE'>

/**
 * 区间操作符的方向：`>` / `>=` 都是下界，`<` / `<=` 都是上界。
 * 同一字段**同方向**只留最新一条（`id > 1` 再设 `id >= 5` ⇒ 只剩 `id >= 5`，
 * 两条叠一起纯属废话），**反方向**才共存（一个下界 + 一个上界 = 区间）。
 */
/**
 * LIKE 的转义符。刻意**不用反斜杠**：本项目要跑 mysql / postgresql / sqlserver / oracle，
 * 而反斜杠在它们的字符串字面量里语义不一致 —— MySQL 想表达「一个反斜杠」得写 '\\'
 * （只写 '\' 会把后面的引号转义掉，直接语法错误），PostgreSQL 在
 * standard_conforming_strings 下 '\\' 又是两个字符，ESCAPE 会报 invalid escape string。
 * '/' 在四者的字符串字面量里都不特殊，`ESCAPE '/'` 到处都能跑。
 */
const LIKE_ESC = '/'

const RANGE_DIR: Partial<Record<FilterOp, 'gt' | 'lt'>> = {
  '>': 'gt',
  '>=': 'gt',
  '<': 'lt',
  '<=': 'lt',
}

/** 一条已生效的列过滤条件 */
interface ColumnFilter {
  /** 原始列名（不带反引号）—— 判断「是不是同一个字段」靠它 */
  field: string
  /** 操作符 —— 决定它与同字段其它条件是替换还是共存 */
  op: FilterOp
  /** 拼好的 SQL 片段，形如：`id` = 2 */
  cond: string
}

const columnFilters = ref<ColumnFilter[]>([])
const columnSort = ref<{ col: string; dir: 'ASC' | 'DESC' } | null>(null)
/** 进入过滤前的原始语句（移除所有过滤/排序时还原到它） */
const preFilterSql = ref('')
/** 最近一次由本功能生成的语句：与 sqlText 不一致 ⇒ 用户手动改过 ⇒ 重新锚定基底 */
const lastGeneratedSql = ref('')

function quoteCol(name: string): string {
  return `\`${String(name).replace(/`/g, '``')}\``
}

/** 用户手动编辑过语句就不再叠加旧条件——以当前语句为新基底、清空已套条件 */
function ensureFilterBase() {
  if (sqlText.value !== lastGeneratedSql.value) {
    preFilterSql.value = sqlText.value
    columnFilters.value = []
    columnSort.value = null
  }
}

function regenerate() {
  const base = preFilterSql.value.trim().replace(/;\s*$/, '')
  let sql = `SELECT * FROM (${base}) AS _flt`
  if (columnFilters.value.length) {
    sql += `\n  WHERE ${columnFilters.value.map(f => f.cond).join('\n  AND ')}`
  }
  if (columnSort.value) sql += `\n  ORDER BY ${quoteCol(columnSort.value.col)} ${columnSort.value.dir}`
  lastGeneratedSql.value = sql
  applyAndRun(sql)
}

/**
 * 设置某字段的条件：按上面的替换规则先剔除会被它取代的旧条件，再放入。
 * 之所以必须记 field 和 op，是因为「叠加」判断只有拿到这两者才做得了。
 */
function addColumnCondition(field: string, op: FilterOp, cond: string) {
  ensureFilterBase()
  const dir = RANGE_DIR[op]
  const next = columnFilters.value.filter(f => {
    if (f.field !== field) return true // 别的字段：一律保留（多字段 AND 是本功能主用途）
    if (!dir) return false // 非区间操作符：该字段的旧条件（含区间）全部作废
    const old = RANGE_DIR[f.op]
    // 区间操作符：同方向替换，反方向保留（下界 + 上界 = 区间）
    return !!old && old !== dir
  })
  next.push({ field, op, cond })
  columnFilters.value = next
  regenerate()
}

/** 只清某个字段的条件（其余字段保留）—— 菜单里与「移除所有」并存 */
function clearFieldFilters(field: string) {
  ensureFilterBase()
  if (!columnFilters.value.some(f => f.field === field)) return
  columnFilters.value = columnFilters.value.filter(f => f.field !== field)
  if (!columnFilters.value.length && !columnSort.value) {
    // 最后一个条件也被清掉了 ⇒ 直接还原原始语句，别留一个空 WHERE 的包装
    lastGeneratedSql.value = preFilterSql.value
    applyAndRun(preFilterSql.value)
    return
  }
  regenerate()
}

function setColumnSort(col: string, dir: 'ASC' | 'DESC') {
  ensureFilterBase()
  columnSort.value = { col, dir }
  // 服务端 ORDER BY 生效后，表头点击的客户端排序必须让位：两套排序同时作用时，
  // 用户看到的行序和 SQL 里的 ORDER BY 不一致，会以为排序没生效
  sortState.value = { prop: '', order: null }
  regenerate()
}

function clearColumnFilters() {
  if (!columnFilters.value.length && !columnSort.value) return
  columnFilters.value = []
  columnSort.value = null
  lastGeneratedSql.value = preFilterSql.value
  applyAndRun(preFilterSql.value)
}

/** 自定义过滤：弹输入框要一个值。纯数字形态的输入按数字字面量处理（避免 int 列比较报类型错） */
async function customColumnCondition(field: string, op: CustomOp) {
  try {
    const { value } = await ElMessageBox.prompt(
      // 自定义过滤让用户自己用通配符；要查字面量就用 / 转义（与自动过滤同一套约定）
      op === 'LIKE' ? `匹配值（% 与 _ 是通配符；查字面 % 请写 ${LIKE_ESC}%）` : '比较值',
      `${field} ${op} …`,
      { confirmButtonText: '执行', cancelButtonText: '取消', inputValue: '' }
    )
    const v = (value ?? '').trim()
    if (!v) return
    const lit = /^-?\d+(\.\d+)?$/.test(v) ? Number(v) : v
    const cond =
      op === 'LIKE'
        ? `${quoteCol(field)} LIKE ${sqlLiteral(lit)} ESCAPE '${LIKE_ESC}'`
        : `${quoteCol(field)} ${op} ${sqlLiteral(lit)}`
    addColumnCondition(field, op, cond)
  } catch {
    /* 用户取消 */
  }
}

/** 菜单条件文案太长时截断（完整条件在 SQL 里可见） */
function truncateLabel(s: string, n = 28): string {
  return s.length > n ? `${s.slice(0, n)}…` : s
}

/** 弹出列头菜单（DBeaver 风格：排序 / 按当前值过滤 / 自定义 / NULL / 移除全部） */
function openColumnMenu(colIdx: number, event: MouseEvent | KeyboardEvent) {
  const field = tableColumns.value[colIdx]
  if (!field) return
  // 「当前值」：优先取选中行，否则取当前页第一行
  const row = selectedRows.value[0] ?? displayRows.value[0]
  const raw = row?.[String(colIdx)]
  // 只有标量能安全进 SQL 字面量：对象 / 数组（JSON 列、BLOB）拿去比较没有意义，
  // 与其拼出一个必然查不到东西的条件，不如不给这个入口
  const isScalar =
    raw !== null &&
    raw !== undefined &&
    raw !== '' &&
    (typeof raw === 'string' || typeof raw === 'number' || typeof raw === 'boolean')
  const col = quoteCol(field)
  // 该字段已生效的条件：显示出来 + 给菜单项打 ✓，让「重设会替换」这件事看得见
  const current = columnFilters.value.filter(f => f.field === field)
  const isActive = (op: FilterOp) => current.some(f => f.op === op)
  const mark = (op: FilterOp) => (isActive(op) ? '✓ ' : '')
  const sortMark = (dir: 'ASC' | 'DESC') =>
    columnSort.value?.col === field && columnSort.value?.dir === dir ? '✓ ' : ''
  const items: MenuItem[] = []
  // 结果里出现同名列（如 SELECT a.id, b.id）：外层包装后 WHERE `id` = 1 只能命中
  // 第一个同名列。与其让用户以为「过滤没生效」，不如提前说明
  if (tableColumns.value.filter(c => c === field).length > 1) {
    items.push(
      { label: '⚠ 结果中存在同名列，条件只会作用于第一个', disabled: true },
      { divider: true }
    )
  }
  if (current.length) {
    items.push(
      {
        label: truncateLabel(`当前：${current.map(f => f.cond).join(' 且 ')}`, 40),
        icon: '⦿',
        disabled: true,
      },
      { label: `清除 ${field} 的条件`, icon: '✕', onClick: () => clearFieldFilters(field) },
      { divider: true }
    )
  }
  items.push(
    { label: sortMark('ASC') + `${field} 升序`, icon: '↑', onClick: () => setColumnSort(field, 'ASC') },
    { label: sortMark('DESC') + `${field} 降序`, icon: '↓', onClick: () => setColumnSort(field, 'DESC') },
    { divider: true }
  )
  if (isScalar) {
    const eq = `${col} = ${sqlLiteral(raw)}`
    const ne = `${col} <> ${sqlLiteral(raw)}`
    const gt = `${col} > ${sqlLiteral(raw)}`
    const gte = `${col} >= ${sqlLiteral(raw)}`
    const lt = `${col} < ${sqlLiteral(raw)}`
    const lte = `${col} <= ${sqlLiteral(raw)}`
    // 「按当前值」的 LIKE 必须转义值里的 % / _ —— 它们是 LIKE 通配符，
    // 单元格里写着 "100%" 时不转义就变成「匹配任意前后缀」了。
    // 连转义符本身（/）也要转，否则值以 / 结尾时会把我们补的后缀 % 给转义掉
    const likePat = sqlLiteral(`%${String(raw).replace(/([%_/])/g, LIKE_ESC + '$1')}%`)
    const like = `${col} LIKE ${likePat} ESCAPE '${LIKE_ESC}'`
    items.push(
      { label: mark('=') + truncateLabel(eq), onClick: () => addColumnCondition(field, '=', eq) },
      { label: mark('<>') + truncateLabel(ne), onClick: () => addColumnCondition(field, '<>', ne) },
      { label: mark('>') + truncateLabel(gt), onClick: () => addColumnCondition(field, '>', gt) },
      { label: mark('>=') + truncateLabel(gte), onClick: () => addColumnCondition(field, '>=', gte) },
      { label: mark('<') + truncateLabel(lt), onClick: () => addColumnCondition(field, '<', lt) },
      { label: mark('<=') + truncateLabel(lte), onClick: () => addColumnCondition(field, '<=', lte) },
      { label: mark('LIKE') + truncateLabel(like), onClick: () => addColumnCondition(field, 'LIKE', like) }
    )
  } else {
    items.push({
      label:
        row === undefined || row === null
          ? '（结果为空，可用自定义过滤）'
          : '（当前单元格没有可用作过滤的标量值，请用自定义过滤）',
      disabled: true,
    })
  }
  items.push(
    { divider: true },
    {
      label: '自定义过滤…',
      icon: '✎',
      children: (['=', '<>', '>', '>=', '<', '<=', 'LIKE'] as const).map(op => ({
        label: `${field} ${op} …`,
        onClick: () => void customColumnCondition(field, op),
      })),
    },
    { divider: true },
    {
      label: mark('IS NULL') + `${field} IS NULL`,
      onClick: () => addColumnCondition(field, 'IS NULL', `${col} IS NULL`),
    },
    {
      label: mark('IS NOT NULL') + `${field} IS NOT NULL`,
      onClick: () => addColumnCondition(field, 'IS NOT NULL', `${col} IS NOT NULL`),
    }
  )
  if (columnFilters.value.length || columnSort.value) {
    items.push(
      { divider: true },
      { label: '移除所有过滤 / 排序', icon: '✕', danger: true, onClick: () => clearColumnFilters() }
    )
  }
  // 鼠标事件有坐标；键盘（Enter/Space）触发时无 clientX/Y，退回用触发元素定位到按钮旁
  if (event instanceof MouseEvent) {
    showContextMenu({ x: event.clientX, y: event.clientY, items })
  } else {
    const rect = (event.target as HTMLElement | null)?.getBoundingClientRect()
    showContextMenu({ x: rect ? rect.left : 0, y: rect ? rect.bottom : 0, items })
  }
}
// 排序 / 选中
// ===================================================================
const sortState = ref<{ prop: string; order: 'ascending' | 'descending' | null }>({ prop: '', order: null })
const selectedRows = ref<any[]>([])
/** 表格实例：用于「新增行」按钮滚动到尾部的空行 */
const tableRef = ref<InstanceType<typeof ResultTable> | null>(null)

/**
 * 新增行：不是弹窗，而是把表格尾部的那条空行（幽灵行）滚到眼前。
 * 空结果集也有这条空行 —— 那时的它就是这张表的第一行。
 */
function focusNewRow() {
  const last = insertRows.value[insertRows.value.length - 1]
  if (!last) {
    ElMessage.warning(readonlyReason.value || '当前结果不支持新增行')
    return
  }
  tableRef.value?.scrollToRow(last.__key)
}

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

/**
 * 表格行。**每一格的值 = 原始值 + 未保存改动**（脏值是覆盖上去的，不改 `result.rows`），
 * 这样「还没写库」与「已经写库」在界面上永远分得清。
 *
 * `__key` 是该行在数据层面的身份（由键列原值算出），`__rowIdx` 是它在 `result.rows`
 * 里的下标。**改动一律按 `__key` 记**，按下标记会在翻页 / 重查 / 排序后挂到别的行上。
 */
interface GridRow extends Record<string, any> {
  __key: string
  __rowIdx: number
  /** 主键 / 唯一键的**原值**（WHERE 永远用原值，哪怕用户改了键列本身） */
  __keyVals: Record<string, unknown>
}

const dataRows = computed<GridRow[]>(() => {
  const r = result.value
  if (!r || !r.is_query) return []
  const strategy = rowKeyStrategy.value
  const keyIdxs = keyColumnIndexes.value
  const rows: GridRow[] = r.rows.map((row, i) => {
    const keyVals: Record<string, unknown> = {}
    if (strategy.kind !== 'none') {
      strategy.cols.forEach((name, j) => {
        keyVals[name] = row[keyIdxs[j]]
      })
    }
    const key =
      strategy.kind === 'none'
        ? `__row:${i}`
        : makeRowKey(strategy.cols.map(n => keyVals[n]), strategy) || `__row:${i}`
    // 刻意**不**在这里叠加 dirtyCells：那会让每一行都依赖脏集合，
    // 改一格 ⇒ 整份 rows 换引用 ⇒ 表格全量重算（连虚拟滚动都救不回来）。
    // 脏值的呈现走下面的 cellValue() —— 它只在单元格插槽里被读，
    // Vue 能精确追踪到「哪一个 span 依赖了哪一格」，做到改哪格刷哪格。
    const obj: GridRow = { __key: key, __rowIdx: i, __keyVals: keyVals }
    r.columns.forEach((_c, idx) => {
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

/**
 * 表格尾部的新增行：**若干条已填过的 + 末尾一条空的**。
 *
 * 空的最后一条就是所谓的「幽灵行」（DBeaver / DataGrip / Navicat 的共同做法）：
 * 在它上面敲下第一个值，它就转正，`nextNewRowKey` 随即给出下一个序号，
 * 于是尾部又自动长出一条新的空行 —— 可以连续加多行，不需要任何额外状态。
 * 反过来，把某行的值全清掉，它从脏集合里消失，尾部也随之回落。
 *
 * 唯一真源是**脏集合本身**（键为 `__new:N`），没有「这行准备好了没有」的第二份状态。
 */
const insertRows = computed<GridRow[]>(() => {
  void dirtyVersion.value
  if (!canInsert.value) return []
  const dirty = dirtyCells.value
  return [...committedNewRowKeys(dirty), nextNewRowKey(dirty)].map(makeInsertRow)
})

/** 传给表格的数据：新增行永远在最后，**不参与排序与头部过滤**（它们还没进数据库） */
const displayRows = computed<GridRow[]>(() => [...dataRows.value, ...insertRows.value])

/** 新增行的骨架：所有格子的「原值」都是 null —— 改一格记一处脏值即可 */
function makeInsertRow(key: string): GridRow {
  const obj: GridRow = { __key: key, __rowIdx: -1, __keyVals: {} }
  tableColumns.value.forEach((_c, i) => {
    obj[String(i)] = null
  })
  return obj
}

/** 传给 ResultTable 的「哪些行是新行」—— 表格不需要懂业务规则，只需要拿到名单 */
const insertRowKeys = computed(() => insertRows.value.map(r => r.__key))
const newRowCount = computed(() => {
  void dirtyVersion.value
  return committedNewRowKeys(dirtyCells.value).length
})

/** 某行在数据中的原始值数组（判断是否真改动、构造 WHERE 都要用） */
function rowOriginal(row: GridRow): unknown[] {
  return result.value?.rows[row.__rowIdx] ?? []
}

/**
 * 单元格**当前应该显示的值**：没改过就是原值，改过则是待写的值。
 *
 * 这一层是「不要 watch 整份数据」能成立的关键：
 * 只有调用它的那个 span 会订阅 dirtyCells，编辑一格不会带动表格其它部分。
 */
function cellValue(row: GridRow, colIdx: number): CellInput {
  void dirtyVersion.value
  return pickCellValue(row, row.__key, colIdx, tableColumns.value[colIdx], dirtyCells.value)
}

/** 展示文本；脏值里的 NULL 标记要还原成 `[NULL]`（它在库里的确是 NULL） */
function cellText(row: GridRow, colIdx: number): string {
  const v = cellValue(row, colIdx)
  return isSqlNull(v) ? '[NULL]' : cellDisplayText(v)
}

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

  // 单表 + 有可靠行定位：按「所有键列都相等」过滤（复合主键要全部参与，
  // 只用第一列会把同首列值的其它行一起导出）
  const strategy = rowKeyStrategy.value
  if (editableTable.value && strategy.kind !== 'none' && keyColumnIndexes.value.every(i => i >= 0)) {
    const conditions = sel.map(row => {
      const parts = strategy.cols.map((name, j) =>
        `\`${name.replace(/`/g, '``')}\` = ${sqlLiteral(row[String(keyColumnIndexes.value[j])])}`
      )
      return `(${parts.join(' AND ')})`
    })
    return `SELECT * FROM \`${editableTable.value}\` WHERE ${conditions.join(' OR ')}`
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

/**
 * 导出弹窗里那句「导出范围」。必须跟 `exportSql` 用**同一个判定**，
 * 否则会出「界面说只导出 5 行、实际导出全表」这种话术与行为打架的事。
 */
const exportScopeText = computed(() => {
  const n = selectedRows.value.length
  if (n === 0) return '当前查询的全部结果'
  return buildSelectedExportSql()
    ? `仅勾选的 ${n} 行`
    : '当前查询的全部结果（勾选行无法精确定位，已忽略勾选）'
})

// ===================================================================
// 单表编辑：脏数据收集 → 预览 → 事务批量写 → 本地回写
// ===================================================================
/**
 * 未保存改动：`行 key → (列名 → 待写入的值)`。
 * 值是 `CellInput` —— 可能是 SQL_NULL 标记，代表「写 NULL」，
 * 这样与「值是空字符串」「值本来就是 null 且没动」三者互不混淆。
 */
const dirtyCells = ref<Map<string, Map<string, CellInput>>>(new Map())
/** Map 的内部变更不会被 Vue 侦测到，改动后 bump 一下让它重新算 */
const dirtyVersion = ref(0)
/** 上一次保存失败的位置：`行 key::列名` → 原因 */
const cellErrors = ref<Map<string, string>>(new Map())
const saving = ref(false)


const showDeleteConfirm = ref(false)
const deletingRowPk = ref<{ cols: string[]; values: unknown[] } | null>(null)

const showImportExport = ref(false)

const showPreview = ref(false)
const previewStatements = ref<EditStatement[]>([])
/** 待提交的原子 Zuordnung：每台 op 对应哪些 (行, 列)，用于把失败结果落回单元格 */
let pendingTargets: { key: string; cols: string[] }[] = []
let pendingOps: EditOp[] = []

const pendingCount = computed(() => {
  void dirtyVersion.value
  let n = 0
  for (const m of dirtyCells.value.values()) n += m.size
  return n
})
const pendingRowCount = computed(() => dirtyCells.value.size)

function cellErrKey(key: string, col: string): string {
  return `${key}::${col}`
}

function isCellDirty(key: string, col: string): boolean {
  void dirtyVersion.value
  return !!dirtyCells.value.get(key)?.has(col)
}

/**
 * 该行能不能改。
 *
 * 键列值里有 NULL 时 `makeRowKey` 算不出稳定身份，会退化成「行下标」——
 * 那时改动的归属会随翻页 / 排序整体平移，写库就是挂到别的行上，所以整行只读。
 */
function isRowEditable(row: GridRow): boolean {
  // 新增行还没进库，谈不上「唯一定位它」—— 只要这张本身可插入就能填
  if (isNewRowKey(row.__key)) return canInsert.value && !gridReadOnly.value
  return canEdit.value && !!row.__key && !row.__key.startsWith('__row:')
}

/** 单元格可编辑 = 整行可定位 + 该列可写 + 未开全局只读 */
function cellEditable(row: GridRow, col: string): boolean {
  return isRowEditable(row) && !!columnEditInfo.value.get(col)?.editable && !gridReadOnly.value
}

function cellReadonlyReason(row: GridRow, col: string): string | undefined {
  if (!isRowEditable(row)) {
    return isNewRowKey(row.__key)
      ? columnEditInfo.value.get(col)?.reason ?? '该列不可编辑'
      : '该行缺少可用的键值（键列含 NULL），无法唯一定位'
  }
  return columnEditInfo.value.get(col)?.reason
}

/** 单元格的鼠标提示：把「当前值 / 有没有改过 / 上次保存为什么失败」一次说清 */
function cellTitle(row: GridRow, col: string): string {
  const text = cellText(row, tableColumns.value.indexOf(col))
  const err = cellErrors.value.get(cellErrKey(row.__key, col))
  if (err) return `保存失败：${err}｜${text}`
  if (isCellDirty(row.__key, col)) return `${text}（已修改，未保存）`
  return `${text}（单击编辑）`
}

const showTable = computed(
  () => !!result.value?.is_query && (result.value?.columns?.length ?? 0) > 0
)

// ===================================================================
// 单例编辑器：谁被点就接管谁
// ===================================================================
/** 同一时刻最多一个被编辑的格子 —— 这是「不建上万个组件实例」能成立的前提 */
interface EditingCell {
  rowKey: string
  colIdx: number
  col: string
  /** 被点的单元格节点：编辑器靠它定位，它滚出可视区就放弃编辑 */
  anchor: HTMLElement
}
const editing = ref<EditingCell | null>(null)
/** 每次打开递增：换 key 让编辑器重建，省得在内部维护「开 / 关」两套分支 */
const editorSeq = ref(0)

const editingRow = computed(() =>
  editing.value ? displayRows.value.find(r => r.__key === editing.value!.rowKey) ?? null : null
)
const editingValue = computed<CellInput>(() => {
  const e = editing.value
  const row = editingRow.value
  if (!e || !row) return null
  return cellValue(row, e.colIdx)
})
const editingColumn = computed(() => (editing.value ? columnMeta.value.get(editing.value.col) : undefined))
const editingKind = computed<ValueKind>(
  () => (editing.value ? columnEditInfo.value.get(editing.value.col)?.kind ?? 'text' : 'text')
)

/** 点击用事件委托：一个监听顶一万个（每格各挂一个 click 也是实打实的开销） */
function onGridClick(ev: MouseEvent) {
  if (gridReadOnly.value) return
  const target = ev.target as HTMLElement | null
  const cell = target?.closest?.('[data-col]') as HTMLElement | null
  if (!cell) return
  const colIdx = Number(cell.dataset.col)
  const rowKey = cell.dataset.row ?? ''
  const col = tableColumns.value[colIdx]
  if (col === undefined || !rowKey) return
  const row = displayRows.value.find(r => r.__key === rowKey)
  if (!row || !cellEditable(row, col)) return
  editing.value = { rowKey, colIdx, col, anchor: cell }
  editorSeq.value++
}

function onEditorCommit(value: CellInput) {
  const e = editing.value
  const row = editingRow.value
  editing.value = null
  if (!e || !row) return
  onCellCommit(row, e.col, value)
}

function onEditorCancel() {
  editing.value = null
}

/** 单元格提交：记进脏集合（**不写库**） */
function onCellCommit(row: GridRow, col: string, value: CellInput) {
  if (!isRowEditable(row)) return
  const original = rowOriginal(row)[tableColumns.value.indexOf(col)]
  // 数值列按数值判断是否「真改过」（7 与 007 落到库里是一个值），字符串列按字符比
  const kind = columnEditInfo.value.get(col)?.kind ?? 'text'
  const target = dirtyCells.value.get(row.__key) ?? new Map<string, CellInput>()
  if (sameValue(value, original, kind)) {
    target.delete(col)
    if (target.size === 0) dirtyCells.value.delete(row.__key)
    else dirtyCells.value.set(row.__key, target)
  } else {
    target.set(col, value)
    dirtyCells.value.set(row.__key, target)
  }
  // 这一格改了，之前同格的失败提示就算过去了
  cellErrors.value.delete(cellErrKey(row.__key, col))
  // 新增行额外多一道：NOT NULL 且没有默认值的列没填，当场说清楚。
  // 留给数据库的话，要等到点了保存、事务已经开始之后才得到一句 `cannot be null`。
  if (isNewRowKey(row.__key) && isRequiredWithoutDefault(columnMeta.value.get(col))) {
    if (!hasMeaningfulValue(target.get(col))) {
      cellErrors.value.set(cellErrKey(row.__key, col), '必填：该列不允许 NULL 且没有默认值')
    }
  }
  dirtyVersion.value++
}

function clearPending(opts?: { silent?: boolean }) {
  editing.value = null
  if (dirtyCells.value.size === 0 && cellErrors.value.size === 0) return
  dirtyCells.value.clear()
  cellErrors.value.clear()
  dirtyVersion.value++
  if (!opts?.silent) ElMessage.info('已放弃未保存的改动')
}

function revertAll() {
  clearPending()
}

/** 翻页 / 重跑 / 换语句前，先确认不会因为未保存改动被静默丢掉 */
async function guardPending(): Promise<boolean> {
  if (pendingCount.value === 0) return true
  try {
    const extra = newRowCount.value > 0 ? `（含 ${newRowCount.value} 条新增行）` : ''
    await ElMessageBox.confirm(
      `有 ${pendingCount.value} 处改动${extra}还没保存，继续会丢掉它们。要现在保存吗？`,
      '未保存的改动',
      { distinguishCancelAndClose: true, confirmButtonText: '先保存', cancelButtonText: '丢弃并继续' }
    )
    // 点「先保存」：保存完由调用方自行判断是否继续
    await startSave()
    return false
  } catch (action) {
    // cancel / close ⇒ 用户选择丢弃
    dirtyCells.value.clear()
    cellErrors.value.clear()
    dirtyVersion.value++
    return true
  }
}

/**
 * 待写值 → 传给后端的值。
 *
 * 先按列类型归一化：**数字 / 时间 / JSON 列的空输入是 NULL 而不是空串** ——
 * MySQL 严格模式下 `SET int_col = ''` 会报 `Incorrect integer value: ''`，
 * 与其把这句话原样丢回给用户，不如在出库前就翻译好。
 * 之后数字列仍由字符串转 Number，让后端拼出数字字面量（`= 2` 而非 `= '2'`）。
 */
function coerceForWire(v: CellInput, col: string): unknown {
  const meta = columnMeta.value.get(col)
  const { value } = normalizeCellInput(v, meta)
  const raw = toWireValue(value)
  if (raw === null) return null
  const kind = inferValueKind(meta)
  if (kind === 'number' && typeof raw === 'string' && raw.trim() !== '') {
    const n = Number(raw.trim())
    if (!Number.isNaN(n)) return n
  }
  if (kind === 'bool' && typeof raw === 'boolean') return raw
  return raw
}

/**
 * 出库前的最后一道校验：编辑器里已经拦过一轮，这里对**所有待写值**再走一遍，
 * 覆盖那些不是从编辑器进来的改动（新增行表单等）。
 */
function firstInvalidCell(): string | null {
  for (const cells of dirtyCells.value.values()) {
    for (const [column, v] of cells) {
      const { error } = normalizeCellInput(v, columnMeta.value.get(column))
      if (error) return `${column}：${error}`
    }
  }
  return null
}

/**
 * 把脏集合翻译成一组 EditOp（一条 op = 一行）。
 *
 * 顺序：**先 INSERT 再 UPDATE** —— INSERT 排在前面，预览出来的语句顺序才稳定，
 * 同一个操作预览两次得到完全一样的结果（否则每次点预览看到的顺序都在变）。
 */
function buildOps(): { ops: EditOp[]; targets: { key: string; cols: string[] }[] } {
  const table = editableTable.value
  const strategy = rowKeyStrategy.value
  if (!table) return { ops: [], targets: [] }
  const ops: EditOp[] = []
  const targets: { key: string; cols: string[] }[] = []

  // 新增行：只写「填了东西」的列，其余让它落回列默认值 / NULL
  void dirtyVersion.value
  for (const key of committedNewRowKeys(dirtyCells.value)) {
    const pending = dirtyCells.value.get(key)
    if (!pending) continue
    const cols = filledColumns(pending, tableColumns.value)
    if (cols.length === 0) continue // 全空的行不算待插入
    ops.push({
      op: 'insert',
      table,
      data: cols.map(column => ({ column, value: coerceForWire(pending.get(column)!, column) })),
    })
    targets.push({ key, cols })
  }

  if (strategy.kind === 'none') return { ops, targets }
  for (const row of dataRows.value) {
    const pending = dirtyCells.value.get(row.__key)
    if (!pending || pending.size === 0) continue
    const data = [...pending.entries()].map(([column, v]) => ({
      column,
      value: coerceForWire(v, column),
    }))
    const keys = strategy.cols.map(name => ({ column: name, value: row.__keyVals[name] ?? null }))
    ops.push({ op: 'update', table, keys, data, expect_one: expectOneMatch.value })
    targets.push({ key: row.__key, cols: data.map(d => d.column) })
  }
  return { ops, targets }
}

/**
 * 新增行里缺了哪些必填列 —— 一次说清，而不是让数据库逐条吐 `cannot be null`。
 * @returns `[新增行序号(从 1 开始), 缺失的列名[]]`
 */
function pendingInsertGaps(): [number, string[]][] {
  void dirtyVersion.value
  const keys = committedNewRowKeys(dirtyCells.value)
  const gaps: [number, string[]][] = []
  keys.forEach((key, i) => {
    const missing = missingRequiredColumns(
      tableColumns.value,
      dirtyCells.value.get(key),
      c => columnMeta.value.get(c)
    )
    if (missing.length > 0) gaps.push([i + 1, missing])
  })
  return gaps
}

/** 预览 + 弹出确认框（语句由后端生成，与实际执行的完全一致） */
async function startSave() {
  if (!props.configId || !props.database) {
    ElMessage.error('缺少连接信息')
    return
  }
  const invalid = firstInvalidCell()
  if (invalid) {
    ElMessage.error(`有单元格的值写不进数据库：${invalid}`)
    return
  }
  const gaps = pendingInsertGaps()
  if (gaps.length > 0) {
    const detail = gaps.map(([n, cols]) => `第 ${n} 条：${cols.join('、')}`).join('；')
    ElMessage.error(`新增行还有必填列没填（这些列不允许 NULL 且没有默认值）—— ${detail}`)
    return
  }
  const { ops, targets } = buildOps()
  if (ops.length === 0) return
  await previewOps(ops, targets)
}

/**
 * 生成语句 → 弹出预览。写库的**唯一通道**：
 * 单元格改动、新增行、删除行最后都汇聚到这里，共享同一个事务与同一份预览。
 */
async function previewOps(ops: EditOp[], targets: { key: string; cols: string[] }[]) {
  if (!props.configId || !props.database) {
    ElMessage.error('缺少连接信息')
    return
  }
  saving.value = true
  try {
    const res = await dbPreviewEdits(props.configId, props.database, ops)
    if (res.code !== 200) {
      ElMessage.error('生成失败: ' + (res.msg || '未知错误'))
      return
    }
    pendingOps = ops
    pendingTargets = targets
    previewStatements.value = res.data.statements
    showPreview.value = true
  } catch (e: any) {
    ElMessage.error('生成失败: ' + (e?.message || e))
  } finally {
    saving.value = false
  }
}

/** 真执行：成功后把值本地写回 `result.rows`，不动整页（避免丢上下文，也省一次全量查询） */
async function confirmSave() {
  if (!props.configId || !props.database || pendingOps.length === 0) return
  saving.value = true
  try {
    const res = await dbApplyEdits(props.configId, props.database, pendingOps)
    const data = res.data
    if (!data) {
      ElMessage.error('提交失败: ' + (res.msg || '未知错误'))
      return
    }
    let okCount = 0
    let failCount = 0
    data.outcomes.forEach((outcome: EditOutcome) => {
      const target = pendingTargets[outcome.index]
      if (!target) return
      if (outcome.ok && outcome.affected > 0) {
        okCount++
        applyToLocalRow(target)
      } else if (outcome.ok) {
        // 受影响 0 行：行已被别处删掉，值没有写进去 —— 必须明确报错，不能当成功
        failCount++
        markError(target, '未更新任何行：该行可能已被其它会话删除')
      } else {
        failCount++
        markError(target, outcome.error || '执行失败')
      }
    })
    if (failCount === 0) {
      ElMessage.success(`已保存 ${okCount} 行`)
      showPreview.value = false
      const hadInsert = pendingOps.some(o => o.op === 'insert')
      const hadDelete = pendingOps.some(o => o.op === 'delete')
      dirtyCells.value.clear()
      dirtyVersion.value++
      // 新增行的真实身份要等库里生成（自增主键会被填上），而且它可能落在当前页 /
      // 当前排序之外；删除之后当前页也会缺一行 —— 这两种情况都只能重查一次才知道。
      if (hadInsert || hadDelete) await fetchPage(page.value, pageSize.value, { skipGuard: true })
    } else {
      ElMessage.error(
        data.transactional
          ? `全部回滚：${failCount} 行失败（${okCount} 行成功但未提交）`
          : `${failCount} 行失败，${okCount} 行已提交`
      )
    }
  } catch (e: any) {
    ElMessage.error('提交失败: ' + (e?.message || e))
  } finally {
    saving.value = false
  }
}

/** 成功写库后回写本地数据（保持待写的原始形态，NULL 标记落到 null） */
function applyToLocalRow(target: { key: string; cols: string[] }) {
  const pending = dirtyCells.value.get(target.key)
  const rows = result.value?.rows
  if (!pending || !rows) return
  const row = displayRows.value.find(r => r.__key === target.key)
  if (!row) return
  const original = rows[row.__rowIdx]
  if (!original) return
  for (const col of target.cols) {
    const idx = tableColumns.value.indexOf(col)
    if (idx < 0) continue
    const v = pending.get(col)
    original[idx] = isSqlNull(v) ? null : v
    pending.delete(col)
    cellErrors.value.delete(cellErrKey(target.key, col))
  }
  if (pending.size === 0) dirtyCells.value.delete(target.key)
  dirtyVersion.value++
}

function markError(target: { key: string; cols: string[] }, msg: string) {
  for (const col of target.cols) {
    cellErrors.value.set(cellErrKey(target.key, col), msg)
  }
  dirtyVersion.value++
}

/**
 * 删除选中的行（底部 footer 的删除按钮）。
 *
 * 刻意**不自己扛事务**：和单元格改动、新增行一样生成 op 后交给 `previewOps`，
 * 用户能在预览里看到 DELETE 的完整 WHERE 再决定要不要执行。
 */
async function requestDeleteSelected() {
  const strategy = rowKeyStrategy.value
  const table = editableTable.value
  if (!table || strategy.kind === 'none') {
    ElMessage.warning(readonlyReason.value || '该结果不支持删除')
    return
  }
  const rows = selectedRows.value.filter((r: any) => !isNewRowKey(r.__key))
  if (rows.length === 0) {
    ElMessage.warning(
      selectedRows.value.length > 0 ? '选中的是尚未入库的新增行，不需要删除' : '请先勾选要删除的行'
    )
    return
  }
  // 拼出来的定位条件里出现 NULL 是**允许的**（写成 `IS NULL` 参与匹配），
  // 能不能唯一定位交给 expect_one 在运行时判定；只有主键 / 唯一索引那种
  // 「列值不该为 NULL」的定位才需要当场拒绝。
  const blocked = rows.find((r: any) =>
    strategy.cols.some((c: string) => r.__keyVals?.[c] === null || r.__keyVals?.[c] === undefined)
  )
  if (blocked && !expectOneMatch.value) {
    ElMessage.warning('选中行里有键值为 NULL 的（无法唯一定位），已取消删除')
    return
  }
  if (!(await ensureConnection())) return
  const ops: EditOp[] = rows.map((r: any) => ({
    op: 'delete',
    table,
    keys: strategy.cols.map((column: string) => ({
      column,
      value: r.__keyVals[column] ?? null,
    })),
    expect_one: expectOneMatch.value,
  }))
  await previewOps(
    ops,
    rows.map((r: any) => ({ key: r.__key, cols: [] }))
  )
}

/**
 * 删除行：同样走批量通道。
 * WHERE 用**所有键列**（复合主键只拼第一列会误删其它行）。
 */
function handleDeleteRow(rowIdx: number) {
  editing.value = null
  const strategy = rowKeyStrategy.value
  if (!editableTable.value || strategy.kind === 'none') {
    ElMessage.warning(readonlyReason.value || '该结果不可编辑')
    return
  }
  const row = result.value?.rows[rowIdx]
  if (!row) return
  const values = keyColumnIndexes.value.map(i => row[i])
  // 主键 / 唯一索引的键值为 NULL 时 WHERE 会退化成 `IS NULL`，可能一次删掉多行 —— 直接拒绝。
  // 拼出来的定位条件（虚拟键 / 整行匹配）不在此列：NULL 正常参与匹配，命不中由 expect_one 拦。
  if (!expectOneMatch.value && values.some(v => v === null || v === undefined)) {
    ElMessage.warning('该行的键值含 NULL，无法唯一定位，拒绝删除')
    return
  }
  deletingRowPk.value = { cols: strategy.cols, values }
  showDeleteConfirm.value = true
}

async function confirmDeleteRow() {
  showDeleteConfirm.value = false
  const target = deletingRowPk.value
  deletingRowPk.value = null
  if (!target || !props.configId || !props.database || !editableTable.value) return
  saving.value = true
  try {
    const ops: EditOp[] = [
      {
        op: 'delete',
        table: editableTable.value,
        keys: target.cols.map((c, i) => ({ column: c, value: target.values[i] ?? null })),
        expect_one: expectOneMatch.value,
      },
    ]
    const res = await dbApplyEdits(props.configId, props.database, ops)
    const outcome = res.data?.outcomes?.[0]
    if (res.code === 200 && outcome?.ok && outcome.affected > 0) {
      ElMessage.success('删除成功')
      await fetchPage(page.value, pageSize.value, { skipGuard: true })
    } else if (res.code === 200 && outcome?.ok) {
      ElMessage.error('未删除任何行：该行可能已被删除')
    } else {
      ElMessage.error('删除失败: ' + (outcome?.error || res.msg || '未知错误'))
    }
  } catch (e: any) {
    ElMessage.error('删除失败: ' + (e?.message || e))
  } finally {
    saving.value = false
  }
}

function handleImport() {
  showImportExport.value = true
}

// ===================================================================
// 大结果集
// ===================================================================
/**
 * 行 / 列再多也不做「分批渲染」了：表格自身带虚拟滚动（见 ResultTable），
 * 只渲染可视区那几十行 × 那几十列，DOM 节点数与总数据量脱钩。
 * 真正做到的是「编辑不重建数据」—— 见 displayRows 上方的注释。
 */
/** 结果集行数。表格只渲染可视区那几十行，但这个总数要给提示文案与导出用 */
const totalRowCount = computed(() => dataRows.value.length)
/** 网格要算的单元格数量级大到一个屏幕装不下时才提示（虚拟滚动已接管渲染） */
const bigResult = computed(() => totalRowCount.value * tableColumns.value.length > 20000)
const perfHint = computed(
  () =>
    `约 ${totalRowCount.value * tableColumns.value.length} 个单元格。表格按可视区渲染，` +
    '开启「只读」可去掉每格的可编辑标记，进一步减少样式计算。'
)

// 语句变化（切换子结果 / 切换表 / 表浏览模式）时同步编辑器并重新执行
/** 尚未消费的「外部已执行结果」，只用一次（挂载时），之后置空 */
let pendingInitialResult = props.initialResult ?? null

watch(
  resolvedSql,
  (s) => {
    // 换语句 = 换上下文：旧的撤销栈与过滤词对新语句没有意义，一并重置
    sqlText.value = s
    filterKeyword.value = ''
    headerRef.value?.setFilter('')
    history.value = [s]
    historyCursor.value = 0
    // 首屏：调用方已带结果就直接铺上去（多语句执行时省掉 N 次冗余往返），还能保留其分页/总行数
    if (pendingInitialResult) {
      const preset = pendingInitialResult
      pendingInitialResult = null
      result.value = preset
      void resolveEditable()
      return
    }
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

/* 表格容器：点击在这里做事件委托（见 onGridClick） */
.qr-grid {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

/* 单元格一律是纯文本 —— 编辑由全局单例浮层接管。
   脏值 / 保存失败这些状态原本画在编辑器里，现在搬到这里。 */
.qr-cell-text {
  display: block;
  width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}
.qr-cell-text.is-readonly {
  color: var(--el-text-color-secondary);
}
.qr-cell-text.is-editable {
  cursor: text;
}
.qr-cell-text.is-dirty {
  color: var(--stateWarningFg);
  font-weight: 600;
}
.qr-cell-text.is-dirty::after {
  content: '•';
  margin-left: 3px;
}
.qr-cell-text.has-error {
  color: var(--stateErrorFg);
  text-decoration: underline wavy var(--el-color-danger);
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
/* 能新增但改不了已有行 */
.meta-partial {
  color: var(--stateWarningFg);
}
.meta-partial b {
  color: var(--el-color-primary);
}
.meta-selected {
  color: var(--el-color-primary);
  font-weight: 600;
}
.meta-perf {
  color: var(--stateWarningFg);
}
.meta-filtered {
  color: var(--stateWarningFg);
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
/* 读取所有行确认框：警告图标 + 两行说明（对齐 DBeaver 的布局） */
.fetch-all-body {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}
.fetch-all-warn {
  flex-shrink: 0;
  font-size: 28px;
  color: var(--stateWarningFg);
  margin-top: 2px;
}
.fetch-all-text {
  flex: 1;
  min-width: 0;
}
.fetch-all-text p {
  margin: 0 0 6px;
  line-height: 1.6;
  color: var(--el-text-color-primary);
}
</style>
