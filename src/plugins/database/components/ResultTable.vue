<template>
  <div ref="wrapRef" class="result-table" :aria-busy="ariaBusy || loading">
    <VxeTable
      ref="tableRef"
      :data="rows"
      :height="tableHeight"
      :border="border ? true : false"
      :stripe="stripe"
      :size="vxeSize"
      :loading="loading"
      :show-header="showHeader"
      :row-config="rowConfig"
      :column-config="columnConfig"
      :sort-config="sortConfig"
      :checkbox-config="CHECKBOX_CONFIG"
      :row-class-name="rowClassNameFn"
      :virtual-y-config="virtualYConfig"
      :virtual-x-config="VIRTUAL_X_CONFIG"
      :empty-text="emptyText"
      class="result-vxe"
      @checkbox-change="emitSelection"
      @checkbox-all="emitSelection"
      @cell-click="handleCellClick"
      @sort-change="handleSortChange"
      @column-resizable-change="handleResizableChange"
    >
      <!--
        多选列。表头由表格自己渲染全选框 —— 刻意**不**给这一列写 header 插槽，
        一旦自定义就会把全选框盖掉。
        行的身份走 `row-config.keyField`（业务 ID）⇒ 滚动回收复用 DOM 时不会把勾选错位。
      -->
      <VxeColumn v-if="selectable" type="checkbox" :width="36" fixed="left" align="center" />

      <!--
        序号列。刻意**不用** vxe 的 `type="seq"`：它的序号渲染走内部实现，
        没法让「尚未入库的新增行」显示成 `*` 而不是下一个数字
        （这个标记很重要 —— 一眼要能看出哪几行是新加的）。
      -->
      <VxeColumn v-if="showIndex" title="#" :width="44" fixed="left" align="center">
        <template #default="{ row, rowIndex }">
          <span :class="{ 'row-star': isNewRow(row) }">{{
            isNewRow(row) ? '*' : rowIndex + 1
          }}</span>
        </template>
      </VxeColumn>

      <!-- 数据列 -->
      <VxeColumn
        v-for="(col, idx) in columns"
        :key="idx"
        :field="String(idx)"
        :title="col"
        :width="getColumnWidth(idx)"
        :min-width="colMinWidth"
        :sortable="sortable"
        :show-header-overflow="true"
        :show-overflow="cellTooltip"
        :align="columnAligns[idx] || 'left'"
        header-align="left"
      >
        <!-- 自定义表头：字段名 + 可选的「排序/过滤」入口按钮。
             按钮必须 stop 冒泡 —— th 的 click 会触发排序、mousedown 会进列宽拖拽。
             字段名区域不拦截，点它仍然走原有的点表头排序。 -->
        <!--
          表头刻意排成两行：第一行「列名 + 角标」，第二行「类型标注」。
          挤在同一行的话，类型要和列名抢横向空间，短值列照样被表头顶宽 ——
          而表头宽度正是决定「一列能有多窄」的主力。换行的代价是一屏少半行，
          换来的是横向多出好几列，这笔买卖划算。
        -->
        <template #header>
          <span class="col-header" @contextmenu.stop="handleHeaderContextMenu(idx, $event)">
            <span class="col-header-line">
              <span class="col-header-label">{{ col }}</span>
              <span v-if="isRowKeyCol(col)" class="col-header-key" title="这一列用于定位一行">键</span>
              <span
                v-if="columnMenu"
                class="col-header-btn"
                title="排序 / 过滤"
                role="button"
                tabindex="0"
                aria-label="排序 / 过滤"
                @click.stop.prevent="emit('column-menu', idx, $event)"
                @keydown.enter.stop.prevent="emit('column-menu', idx, $event)"
                @keydown.space.stop.prevent="emit('column-menu', idx, $event)"
                @mousedown.stop
              >▾</span>
            </span>
            <span
              v-if="typeTexts[idx]"
              class="col-header-type"
              :class="{ 'is-inferred': typeInferred[idx] }"
              :title="typeTitles[idx]"
              >{{ typeTexts[idx] }}</span
            >
          </span>
        </template>
        <template #default="{ row }">
          <slot :name="`cell-${idx}`" :row="row" :column="col" :index="idx">
            <!-- NULL 与空串是两回事：NULL 给一个显式标记，空串保持空白。
                 否则「这格到底是没值还是值为空」只能靠猜。可用 showNullMark 关掉。 -->
            <span
              v-if="showNullMark && isNullish(row[String(idx)])"
              class="cell-content cell-null"
              title="NULL"
              >NULL</span
            >
            <!-- 自带 title：外部关掉 cellTooltip 时，hover 仍然能看到完整值 -->
            <span v-else class="cell-content" :title="String(row[String(idx)] ?? '')">{{
              formatCell(row[String(idx)])
            }}</span>
          </slot>
        </template>
      </VxeColumn>

      <!-- 操作列（预留） -->
      <VxeColumn
        v-if="$slots.actions"
        title="操作"
        :width="120"
        fixed="right"
        align="center"
      >
        <template #default="{ row, rowIndex }">
          <slot name="actions" :row="row" :index="rowIndex" />
        </template>
      </VxeColumn>

      <!-- 空状态 -->
      <template #empty>
        <el-empty :description="emptyText" :image-size="80">
          <template #image>
            <svg-icon v-if="emptyIcon" :icon-class="emptyIcon" />
          </template>
        </el-empty>
      </template>
    </VxeTable>

    <!-- 底部状态栏 -->
    <div v-if="showFooter" class="table-footer">
      <span class="footer-info">
        共 <b>{{ totalCount ?? rows.length }}</b> 条记录
        <span v-if="selection.length > 0">，已选 <b>{{ selection.length }}</b> 条</span>
      </span>
      <slot name="footer" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, onUpdated, ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { showContextMenu } from '@/components/context-menu/useContextMenu'
import type { MenuItem } from '@/components/context-menu/ContextMenu.vue'
import { VxeColumn, VxeTable } from 'vxe-table'
import 'vxe-table/es/style.css'
import type { ColumnInfo } from '../types'
import {
  computeColumnWidths,
  inferColumnKinds,
  inferredKindLabel,
  typeLabelFromInfo,
} from '../utils/columnWidth'

interface Props {
  /** 列名 */
  columns: string[]
  /** 行数据（键为 String(列索引)） */
  rows: Record<string, any>[]
  /** 加载中 */
  loading?: boolean
  /** 是否显示边框 */
  border?: boolean
  /** 是否斑马纹 */
  stripe?: boolean
  /** 表格尺寸 */
  size?: 'large' | 'default' | 'small'
  /**
   * 表格高度（像素）。虚拟滚动**必须**知道可视区有多高才能算出要渲染多少行，
   * 所以没给时会去量容器（见 tableHeight）。
   *
   * 注意：这里刻意没有 `maxHeight` —— 和「固定像素高度」是互斥的两种布局，
   * 留一个永远不生效的参数只会误导后来的人。
   */
  height?: string | number
  /** 列最小宽度 */
  colMinWidth?: number
  /** 是否显示表头 */
  showHeader?: boolean
  /** 是否可选择 */
  selectable?: boolean
  /** 是否显示序号 */
  showIndex?: boolean
  /** 是否可排序 */
  sortable?: boolean
  /** 列宽是否可拖拽 */
  resizable?: boolean
  /** 是否高亮当前行 */
  highlightCurrentRow?: boolean
  /**
   * 行身份字段名（**必须是行上稳定唯一的业务 ID**）。
   *
   * 虚拟滚动会回收滚出视口的行 DOM 再复用给新数据 —— 这时候行必须有一个不随滚动、
   * 排序、翻页变化的身份，否则勾选、编辑态、脏标记都会错位。
   * 因此这里**只接受字段名**，不接受「数组下标」，也不接受函数：调用方要保证该字段存在且唯一。
   */
  rowKey?: string
  /** 空数据文本 */
  emptyText?: string
  /** 空数据图标（svg-icon 的 iconClass） */
  emptyIcon?: string
  /** 是否显示底部状态栏 */
  showFooter?: boolean
  /** 列宽映射（列索引 => 宽度） */
  columnWidths?: Record<number, number>
  /**
   * 各列的真实表元数据，**按结果列的顺序**（查不到的一列为 undefined）。
   *
   * 只有「单表浏览」这类场景才有（`TableDataResult` 返回完整的 `ColumnInfo`）；
   * SQL 查询结果压根没有类型信息，此时整份为空，表头退化成显示推断出来的类型。
   */
  columnMetas?: (ColumnInfo | undefined)[]
  /** 表头是否显示第二行「类型标注」 */
  showColumnTypes?: boolean
  /**
   * 是否按「首屏采样值 + 列名」估算列宽。
   *
   * 关掉就退回「统一退回 min-width」的老行为 —— 万一某个数据源的宽度估算出了问题，
   * 这是可以立刻止损的开关，不用回滚代码。
   */
  autoColumnWidth?: boolean
  /** 可选行判断函数（注：vxe 的行级禁用与表头全选另有实现，本表当前无使用方，保留以待扩展） */
  selectableFn?: (row: any, index: number) => boolean
  /** 无障碍：区域是否处于忙碌状态（外部显式声明，与 loading 取并集） */
  ariaBusy?: boolean
  /** 列头是否显示「排序 / 过滤」入口按钮（点击后 emit column-menu，菜单内容由外部组装） */
  columnMenu?: boolean
  /**
   * 单元格溢出时是否用表格自带的 tooltip。
   *
   * 它会对**每个单元格**挂 mouseenter / mouseleave 监听 —— 几千个单元格时是实打实的负担。
   * 自定义单元格插槽自己带 `title` 时应当关掉它（外部传 false）。
   */
  cellTooltip?: boolean
  /**
   * 是否把 NULL 值渲染成显式的 `NULL` 标记（灰色斜体）。
   *
   * 数据库结果集里 NULL 和空字符串是两种不同的值，全都显示成空白会让用户分不清。
   * 关掉后退化成原来的行为（NULL / 空串都显示为空白）。
   */
  showNullMark?: boolean
  /** 底部「共 N 条」用的真实总数 */
  totalCount?: number
  /**
   * 哪些行是「尚未入库的新增行」（传 `rowKey` 字段的值列表）。
   *
   * 表格本身不懂业务规则，只按这份名单做三件事：序号显示为 `*`、不许勾选、
   * 加上待插入的样式。勾选尤其重要 —— 勾选后可以「只导出选中项」，
   * 而拿这些还没进库的行去构造 WHERE 必然查不到东西。
   */
  newRowKeys?: string[]
  /**
   * 当前用来定位一行的列（列头打标记用）。
   * 表格不知道「主键 / 唯一索引 / 手动指定」的区别，只知道哪几列是行标识。
   */
  rowKeyColumns?: string[]
  /**
   * 是否允许在列头右键指定行标识。
   * 只有「没有主键也没有唯一索引」的表才需要 —— 有数据库担保唯一时给这个入口纯属噪音。
   */
  allowRowKeyPick?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  border: true,
  stripe: true,
  size: 'small',
  // 采样宽度能生效的前提：min-width 是 vxe 的硬下限，定在 100 会让「短值列变窄」完全不起作用
  colMinWidth: 56,
  showHeader: true,
  selectable: false,
  showIndex: false,
  sortable: false,
  resizable: true,
  highlightCurrentRow: false,
  rowKey: '__key',
  emptyText: '暂无数据',
  emptyIcon: '',
  showFooter: false,
  columnWidths: () => ({}),
  columnMetas: () => [],
  showColumnTypes: true,
  autoColumnWidth: true,
  ariaBusy: false,
  columnMenu: false,
  cellTooltip: true,
  showNullMark: true,
  totalCount: undefined,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  selectableFn: (_row: any, _index: number) => true,
  newRowKeys: () => [],
  rowKeyColumns: () => [],
  allowRowKeyPick: false,
})

/** 新增行的身份集合（O(1) 判断，避免在每行渲染里做数组 includes） */
const newRowKeySet = computed(() => new Set(props.newRowKeys ?? []))
/** 行标识列（大小写不敏感：不同库返回的列名大小写政策不一样） */
const rowKeyColSet = computed(
  () => new Set((props.rowKeyColumns ?? []).map(c => String(c).toLowerCase()))
)
function isRowKeyCol(col: unknown): boolean {
  return rowKeyColSet.value.has(String(col).toLowerCase())
}
function isNewRow(row: any): boolean {
  return newRowKeySet.value.has(String(row?.[props.rowKey]))
}
/** vxe 的行样式回调：只读块 property name，不是 Vue 的 class 对象 */
function rowClassNameFn({ row }: { row: any }): string {
  return isNewRow(row) ? 'is-new-row' : ''
}
function checkboxCheckMethod({ row }: { row: any }): boolean {
  return !isNewRow(row)
}

const emit = defineEmits<{
  /** 行点击 */
  (e: 'row-click', row: any, index: number): void
  /** 选中变化 */
  (e: 'selection-change', selection: any[]): void
  /** 排序变化 */
  (e: 'sort-change', column: string, order: string | null): void
  /** 列宽拖拽结束 */
  (e: 'column-resize', columnIndex: number, width: number): void
  /** 列头菜单按钮点击（参数：列索引、鼠标/键盘事件——用于把菜单定位到按钮旁） */
  (e: 'column-menu', columnIndex: number, event: MouseEvent | KeyboardEvent): void
  /** 列头右键「设为行标识 / 取消行标识」：怎么定位一行由调用方决定并存下来 */
  (e: 'pick-row-key', column: string): void
  /** 清除手动指定的行标识 */
  (e: 'clear-row-key'): void
}>()

const tableRef = ref()
const selection = ref<any[]>([])

// ===================================================================
// 虚拟滚动配置
// ===================================================================
/**
 * 纵向：`mode='scroll'` 走原生滚动并按**行的实际高度**累计偏移量 —— 这是支持动态行高的那一档
 * （默认的 `'wheel'` 会把滚动事件劫持成「按固定行高换算」，性能略好但要求行高一致）。
 * `gt` = 超过多少行才启用；行数很少时虚拟反而多一层换算开销，没必要。
 */
const virtualYConfig = { enabled: true, gt: 30, oSize: 2, mode: 'scroll' as const }
/** 横向：列很多时同样只渲染可视区的列。`gt` 语义同上（超过多少列才启用） */
const VIRTUAL_X_CONFIG = { enabled: true, gt: 24, oSize: 1 }

/**
 * 行身份。**必须指向业务 ID**：虚拟滚动按 key 缓存勾选与行状态，
 * 用下标的话滚动几屏后 key 会与别的行撞上（症状：勾选自己跳、编辑值落到别的行）。
 */
const rowConfig = computed(() => ({
  keyField: props.rowKey,
  useKey: true,
  // 悬停高亮：Element 版本一直开着，这里保持同样的观感
  isHover: true,
  isCurrent: props.highlightCurrentRow,
}))

const columnConfig = computed(() => ({ resizable: props.resizable, useKey: true }))
/** 排序只 emit、不让表格自己排：结果集的分页与过滤都在外部（服务端 SQL / 上层 computed）里做 */
const sortConfig = { remote: true, trigger: 'cell' as const }
const CHECKBOX_CONFIG = { highlight: true, checkMethod: checkboxCheckMethod }

// ===================================================================
// 高度：虚拟滚动必须知道可视高度
// ===================================================================
const wrapRef = ref<HTMLElement | null>(null)
const measuredHeight = ref(0)
let resizeObserver: ResizeObserver | null = null

/**
 * 表格高度优先取 props.height；没给就量容器。
 * 量出来的值传回去后容器高度不会再变（父级是 flex + min-height:0），所以不存在自激循环；
 * 仍然加了 1px 阈值，防止浮点抖动引发无意义的重排。
 */
const tableHeight = computed(() => {
  const h = props.height
  if (h !== undefined && h !== null && String(h) !== '') return h
  return measuredHeight.value > 0 ? measuredHeight.value : undefined
})

const vxeSize = computed(() => (props.size === 'large' ? 'medium' : props.size))

onMounted(() => {
  const el = wrapRef.value
  if (!el || typeof ResizeObserver === 'undefined') return
  resizeObserver = new ResizeObserver(() => {
    const h = el.clientHeight
    if (h > 0 && Math.abs(h - measuredHeight.value) > 1) measuredHeight.value = h
  })
  resizeObserver.observe(el)
  measuredHeight.value = el.clientHeight
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  resizeObserver = null
})

// ===================================================================
// 选中：只在数据整批换掉时清理（引用比较，O(1)）
// ===================================================================
/**
 * 刻意**不 watch rows**：监听整份数据会被 Vue 逐行做依赖收集，几百上千行时每次赋值都是一轮遍历。
 * 替代做法是「组件更新后比对引用」—— 换了数据集就清勾选，同一份数据的改动什么都不做。
 */
let lastRowsRef: Record<string, any>[] | null = null
onUpdated(() => {
  if (props.rows === lastRowsRef) return
  const isFirst = lastRowsRef === null
  lastRowsRef = props.rows
  if (isFirst) return
  clearSelection()
})

function clearSelection() {
  if (!props.selectable) return
  selection.value = []
  try {
    tableRef.value?.clearCheckboxRow()
  } catch {
    // 表格还没 mounted（首帧空数据）：没有可清的
  }
  emit('selection-change', [])
}

function emitSelection() {
  if (!props.selectable) return
  const records = (tableRef.value?.getCheckboxRecords?.() as any[]) ?? []
  selection.value = records
  emit('selection-change', records)
}

/**
 * 把指定 `rowKey` 的行滚进视野（「定位到新增行」用）。
 * 虚拟滚动下目标行可能还没渲染 —— vxe 会先按数据集算出它的位置再滚动过去。
 */
function scrollToRow(key: string) {
  const t = tableRef.value as any
  if (!t?.scrollToRow) return
  const row = props.rows.find(r => String(r?.[props.rowKey]) === key)
  if (row) t.scrollToRow(row)
}

// 获取列宽
// ===================================================================
// 表头类型标注 + 列宽采样
//
// 以前每列都退回统一的 min-width（100px）：自增 ID 列、布尔列照样占满一百个像素，
// 横向一半以上的空间是空的。改成按「首屏采样值 + 列名」估算贴合宽度之后，
// 省下来的宽度直接换成「一屏多看几列」。
// ===================================================================

/** 参与类型推断与宽度估算的采样行数 */
const METRIC_SAMPLE_ROWS = 50

/**
 * 按采样值推断每列的语义归类。
 *
 * 与上面的 `columnAligns` 是同一套哲学（绝大多数值都同意才认），但两者刻意各走各的：
 * 对齐推错只影响观感，类型是要写进表头给用户读的，召回条件不该共用。
 */
const inferredKinds = computed(() =>
  inferColumnKinds(props.columns, props.rows, METRIC_SAMPLE_ROWS)
)

/**
 * 表头第二行的内容。三档优先级，最终不一定每列都有东西可显示：
 * 1. 有真实表元数据（单表浏览）→ 显示它，最可信；
 * 2. 没有 → 落到采样推断，并打上 `inferred` 让 UI 用斜体区分「这是猜的」；
 * 3. 推断下来只是普通文本列 → 什么都不显示（「这列是文本」属于零信息）。
 */
const headerTypes = computed(() =>
  props.columns.map((_name, idx) => {
    if (!props.showColumnTypes) return null
    const info = props.columnMetas?.[idx]
    if (info) {
      return {
        text: typeLabelFromInfo(info),
        inferred: false,
        title: info.is_nullable ? info.data_type : `${info.data_type} · NOT NULL`,
      }
    }
    const guessed = inferredKindLabel(inferredKinds.value[idx])
    if (!guessed) return null
    return { text: guessed, inferred: true, title: `按前 ${METRIC_SAMPLE_ROWS} 行数据推断` }
  })
)

/** 拆成三个平行数组：模板里按下标取值，省得写一串可能为 null 的类型断言 */
const typeTexts = computed(() => headerTypes.value.map(t => t?.text ?? ''))
const typeInferred = computed(() => headerTypes.value.map(t => !!t?.inferred))
const typeTitles = computed(() => headerTypes.value.map(t => t?.title ?? ''))

/**
 * 估算列宽：内容是主力，表头是下限，语义是上限。
 * `kinds` 复用上面的推断结果，避免只为取宽度上限再遍历一遍采样值。
 */
const autoWidths = computed(() =>
  computeColumnWidths(props.columns, props.rows, {
    minWidth: props.colMinWidth,
    sampleRows: METRIC_SAMPLE_ROWS,
    headerSubLabels: typeTexts.value,
    kinds: inferredKinds.value,
  })
)

/** 优先级：用户手动拖过 > 采样估算 > 没有（交回 vxe 自己分配剩余宽度） */
const getColumnWidth = (idx: number) => {
  const manual = props.columnWidths[idx]
  if (manual) return manual
  return props.autoColumnWidth ? autoWidths.value[idx] : undefined
}

/** 只有 null / undefined 算「无值」；空串是有值的（要显示成空白而不是 NULL 标记） */
const isNullish = (v: any): boolean => v === null || v === undefined

/**
 * 判断一个值是否「长得像数字」——用于列对齐推断。
 * 排除前导零的编码（如 "007"、"0123"）：这类多是 ID / 编号，右对齐反而错位。
 */
const isNumericLike = (v: any): boolean => {
  if (typeof v === 'number') return Number.isFinite(v)
  if (typeof v !== 'string') return false
  const s = v.trim()
  if (s === '') return false
  if (/^-?0\d/.test(s)) return false
  return !Number.isNaN(Number(s))
}

/**
 * 按列采样推断对齐方式：数值列右对齐（比对位数 / 量级更快），其余左对齐。
 *
 * 结果集不带列类型元信息，只能按值猜。刻意**只采样前若干行** —— 大数据集逐格扫描
 * 太贵；用 0.8 阈值要求「绝大多数非空值都是数值」才右对齐，避免混入文本的列被误判。
 * computed 按 rows 引用缓存，换数据集才重算。
 */
const ALIGN_SAMPLE_ROWS = 50
const ALIGN_NUMERIC_RATIO = 0.8
const columnAligns = computed<('left' | 'right')[]>(() => {
  const colCount = props.columns.length
  const aligns: ('left' | 'right')[] = new Array(colCount).fill('left')
  const sample = props.rows.slice(0, ALIGN_SAMPLE_ROWS)
  if (!sample.length) return aligns
  for (let c = 0; c < colCount; c++) {
    const key = String(c)
    let numeric = 0
    let seen = 0
    for (const row of sample) {
      const v = row[key]
      if (isNullish(v) || v === '') continue
      seen++
      if (isNumericLike(v)) numeric++
    }
    aligns[c] = seen > 0 && numeric / seen >= ALIGN_NUMERIC_RATIO ? 'right' : 'left'
  }
  return aligns
})

// 格式化单元格内容（null / undefined / 空串 显示为 -，对象序列化为 JSON）
const formatCell = (value: any) => {
  if (value === null || value === undefined || value === '') {
    return ''
  }
  if (typeof value === 'object') {
    try {
      return JSON.stringify(value)
    } catch {
      return String(value)
    }
  }
  return value
}

// 单元格点击 → 行点击（虚拟滚动下 rowIndex 是数据集里的真实下标）
const handleCellClick = ({ row, rowIndex }: { row: any; rowIndex: number }) => {
  emit('row-click', row, rowIndex ?? 0)
}

/**
 * 排序：vxe 的 order 是 `asc` / `desc` / `''`（空＝取消），对外沿用 Element 的
 * `ascending` / `descending` / null —— 上层 `sortState` 一直在用后者。
 *
 * 参数类型刻意写得比 vxe 的 `SortChangeEventParams` 宽（`order: string`）：
 * 那个类型没有从 `vxe-table` 导出（只在内部依赖 `vxe-pc-ui` 里），
 * 而 `SortOrder` 里的 `''` 又会跟精确到字面量的签名打架。
 */
const ORDER_TO_EL: Record<string, string> = { asc: 'ascending', desc: 'descending' }
const ORDER_TO_VXE: Record<string, string> = { ascending: 'asc', descending: 'desc' }

function handleSortChange({ field, order }: { field: string; order: string | null }) {
  emit('sort-change', field, order ? ORDER_TO_EL[order] ?? null : null)
}

/** vxe 的列对象：数据列的 field 是列索引字符串，其余列没有 field */
function columnIndexOf(column: any): number | null {
  const raw = column?.field as string | undefined
  if (!raw) return null
  const idx = Number(raw)
  return Number.isInteger(idx) && idx >= 0 && idx < props.columns.length ? idx : null
}

function handleResizableChange({ column, resizeWidth }: { column: any; resizeWidth: number }) {
  const idx = columnIndexOf(column)
  if (idx !== null && resizeWidth) {
    emit('column-resize', idx, resizeWidth)
  }
}

// ===================================================================
// 全选列表头：复制全部字段名（分隔符可配，默认逗号）
// ===================================================================

/** 分隔符预设。默认逗号 —— 「复制字段名」最常见的用途就是拼 SQL 的字段列表 */
const SEP_PRESETS: { label: string; value: string }[] = [
  { label: '逗号', value: ',' },
  { label: '分号', value: ';' },
  { label: '空格', value: ' ' },
  { label: '竖线', value: '|' },
  { label: '换行', value: '\n' },
  { label: '制表符', value: '\t' },
]

const SEP_STORAGE_KEY = 'bdevtools:result-table:column-name-separator'

/** 分隔符 → 可显示、可编辑的文本：换行/制表符否则既看不见也输不进来 */
const sepToVisible = (v: string) =>
  v.replace(/\\/g, '\\\\').replace(/\n/g, '\\n').replace(/\t/g, '\\t').replace(/\r/g, '\\r')

/**
 * 反过来：把 `\\n` / `\\t` / `\\r` / `\\\\` 还原成真实字符。
 * 必须一次扫描完成 —— 分两步替换的话 `\\\\n`（字面反斜杠 + n）会被误判成换行。
 */
const parseSepInput = (raw: string) =>
  raw.replace(
    /\\(n|t|r|\\)/g,
    (_m, c: string) => (c === 'n' ? '\n' : c === 't' ? '\t' : c === 'r' ? '\r' : '\\')
  )

const readStoredSep = () => {
  try {
    return localStorage.getItem(SEP_STORAGE_KEY) ?? ','
  } catch {
    return ',' // 隐私模式 / 禁用存储：退化成默认值，复制功能不受影响
  }
}

const separator = ref<string>(readStoredSep())

const setSeparator = (v: string) => {
  separator.value = v
  try {
    localStorage.setItem(SEP_STORAGE_KEY, v)
  } catch {
    // 存不下就只在本次会话生效
  }
}

const copyAllColumnNames = () => {
  const text = props.columns.join(separator.value)
  if (!text) {
    ElMessage.warning('没有可复制的字段')
    return
  }
  navigator.clipboard
    .writeText(text)
    .then(() => ElMessage.success(`已复制 ${props.columns.length} 个字段名`))
    .catch(() => ElMessage.error('复制失败'))
}

const promptCustomSeparator = () => {
  ElMessageBox.prompt(
    '字段名之间的分隔符，支持转义：\\n 换行、\\t 制表符、\\\\ 反斜杠。留空则直接拼接。',
    '自定义分隔符',
    {
      inputValue: sepToVisible(separator.value),
      inputPlaceholder: '例如：,  或  \\n',
      confirmButtonText: '确定并复制',
      cancelButtonText: '取消',
    }
  )
    .then(({ value }) => {
      setSeparator(parseSepInput(String(value ?? '')))
      copyAllColumnNames()
    })
    .catch(() => {
      // 用户取消
    })
}

/**
 * 全选列表头右键 ⇒ 复制全部字段名。
 *
 * 挂在全选列是因为它的语义天然作用于「所有列」；数据列头右键仍然只复制那一列。
 * 选一个预设分隔符＝设为默认 + 立刻按它复制（省一次点击）。
 * 注意：`selectable=false` 时没有这一列，也就没有这个入口。
 */
const openColumnNamesMenu = (event: MouseEvent) => {
  if (!props.columns.length) return
  event.preventDefault()
  const n = props.columns.length
  showContextMenu({
    x: event.clientX,
    y: event.clientY,
    items: [
      {
        label: `复制全部字段名（${n} 个）`,
        icon: '📋',
        onClick: copyAllColumnNames,
      },
      { divider: true },
      {
        label: `分隔符（当前：${sepToVisible(separator.value)}）`,
        icon: '⚙',
        children: [
          ...SEP_PRESETS.map((p) => ({
            label: p.label,
            icon: p.value === separator.value ? '✓' : '',
            shortcut: sepToVisible(p.value),
            onClick: () => {
              setSeparator(p.value)
              copyAllColumnNames()
            },
          })),
          { divider: true },
          { label: '自定义…', icon: '✏️', onClick: promptCustomSeparator },
        ],
      },
    ],
  })
}

/**
 * 表头右键。`idx === null` 代表全选列 ⇒ 复制**全部**字段名，否则复制该列字段名。
 *
 * 没有走表格自带的 header-cell-menu：那个事件要额外开启 menu 模块才会派发，
 * 而我们需要的是「任何情况下右键都能用」，直接在 header 插槽的根元素上监听最省事。
 */
function handleHeaderContextMenu(idx: number | null, event: MouseEvent) {
  if (idx === null) {
    if (props.selectable) openColumnNamesMenu(event)
    return
  }
  if (idx < 0 || idx >= props.columns.length) return

  event.preventDefault()
  const name = props.columns[idx]
  const items: MenuItem[] = [
    {
      label: '复制字段名',
      icon: '📋',
      onClick: () => {
        navigator.clipboard
          .writeText(name)
          .then(() => {
            ElMessage.success('字段名已复制')
          })
          .catch(() => {
            ElMessage.error('复制失败')
          })
      },
    },
  ]
  // 无主键 / 无唯一索引的表：让用户自己指定「拿哪一列当行标识」。
  // 这一列只是**声明**，执行时仍会校验命中数（所以挑错了也不会误伤一片）。
  if (props.allowRowKeyPick) {
    const picked = isRowKeyCol(name)
    items.push({
      label: picked ? `取消行标识：${name}` : `设为行标识：${name}`,
      icon: '🔑',
      onClick: () => emit('pick-row-key', name),
    })
    if (rowKeyColSet.value.size > 0) {
      items.push({ label: '清除行标识', icon: '🧹', onClick: () => emit('clear-row-key') })
    }
  }
  showContextMenu({ x: event.clientX, y: event.clientY, items })
}

// 暴露方法
defineExpose({
  tableRef,
  clearSelection,
  toggleRowSelection: (row: any, selected?: boolean) =>
    tableRef.value?.setCheckboxRow([row], selected ?? true),
  sort: (prop: string, order: string) =>
    tableRef.value?.sort({ field: prop, order: ORDER_TO_VXE[order] ?? order }),
  getSelection: () => selection.value,
  scrollToRow,
})
</script>

<style scoped>
.result-table {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  min-height: 0;
}

/* vxe 的高度必须由父级给（虚拟滚动的视窗高度就靠它），这里只让它填满可用空间 */
.result-table > :deep(.vxe-table) {
  flex: 1;
  min-height: 0;
}

/* 自定义表头：字段名 + 菜单按钮。
   slot 内容在 ResultTable 模板里编译，带本组件的 data-v 属性 ⇒ scoped 样式直接生效。 */
/* 表头两行结构：第一行「列名 + 角标」，第二行「类型标注」。
   scoped 对 slot 内容是生效的（它在本组件模板里编译，带本组件的 data-v）。 */
.col-header {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 1px;
  min-width: 0;
  max-width: 100%;
}

.col-header-line {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  max-width: 100%;
}

.col-header-label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 类型标注：次要色 + 小字号，不与列名争视觉权重 */
.col-header-type {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-size-xs);
  line-height: 1.35;
  color: var(--el-text-color-secondary);
}

/* 推断出来的类型没有数据库背书：斜体 + 更淡的颜色，明确区分于真类型 */
.col-header-type.is-inferred {
  font-style: italic;
  opacity: 0.7;
}

/* 「这一列用于定位一行」的角标（主键 / 唯一索引 / 手动指定的行标识） */
.col-header-key {
  flex-shrink: 0;
  padding: 0 3px;
  border-radius: 3px;
  font-size: 10px;
  line-height: 14px;
  color: var(--stateWarningFg);
  border: 1px solid var(--el-color-warning-light-5);
  background: var(--el-color-warning-light-9);
}

.col-header-btn {
  flex-shrink: 0;
  cursor: pointer;
  opacity: 0.4;
  font-size: 10px;
  line-height: 1;
  padding: 1px 3px;
  border-radius: 3px;
  color: var(--el-text-color-secondary);
  user-select: none;
}

.col-header-btn:hover {
  opacity: 1;
  color: var(--el-color-primary);
  background: var(--el-fill-color);
}

.vt-head-plain {
  display: inline-block;
  width: 100%;
  text-align: center;
  color: var(--el-text-color-secondary);
  font-size: 10px;
  user-select: none;
}

.table-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-2) var(--space-3);
  border-top: 1px solid var(--el-border-color-lighter);
  background: var(--el-bg-color);
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}

.table-footer .footer-info b {
  color: var(--el-text-color-primary);
  font-weight: 600;
}

/* NULL 标记：与真实值拉开区分度，用次要色 + 斜体，一眼能扫出哪些格是空的 */
.cell-null {
  color: var(--el-text-color-placeholder);
  font-style: italic;
}
</style>

<style>
/* ===================================================================
   vxe-table 主题对齐
   包自带 `--vxe-ui-*` 变量，全部重新指到项目的 Element 令牌上，
   于是深色 / 浅色自动跟随，不需要为两套主题各写一遍。
   =================================================================== */
.result-vxe {
  --vxe-ui-font-color: var(--el-text-color-regular);
  --vxe-ui-font-primary-color: var(--el-color-primary);
  --vxe-ui-font-success-color: var(--stateSuccessFg);
  --vxe-ui-font-info-color: var(--stateInfoFg);
  --vxe-ui-font-warning-color: var(--stateWarningFg);
  --vxe-ui-font-danger-color: var(--stateErrorFg);
  --vxe-ui-layout-background-color: var(--el-bg-color);
  --vxe-ui-base-popup-border-color: var(--el-border-color);
  --vxe-ui-input-border-color: var(--el-border-color);

  --vxe-ui-table-background-color: var(--el-bg-color);
  --vxe-ui-table-border-color: var(--el-border-color-lighter);
  --vxe-ui-table-header-background-color: var(--el-fill-color-light);
  --vxe-ui-table-header-font-color: var(--el-text-color-primary);
  --vxe-ui-table-footer-background-color: var(--el-bg-color);
  --vxe-ui-table-footer-font-color: var(--el-text-color-regular);
  --vxe-ui-table-row-hover-background-color: var(--el-fill-color);
  --vxe-ui-table-row-striped-background-color: var(--el-fill-color-lighter);
  --vxe-ui-table-row-current-background-color: var(--el-fill-color);
  --vxe-ui-table-row-checkbox-checked-background-color: var(--el-fill-color);
  --vxe-ui-table-column-hover-background-color: var(--el-fill-color);
  --vxe-ui-table-column-current-background-color: var(--el-fill-color);
  --vxe-ui-table-resizable-line-color: var(--el-border-color);
  --vxe-ui-table-resizable-drag-line-color: var(--el-color-primary);
  --vxe-ui-table-fixed-scrolling-box-shadow-color: rgba(0, 0, 0, 0.12);

  /* 结果集是「一屏尽量多给几行」的场景，官方 small 档偏高 */
  --vxe-ui-table-row-height-small: 30px;
  --vxe-ui-table-row-line-height: 20px;
  --vxe-ui-table-cell-padding-small: 0 6px;
  --vxe-ui-font-size-small: var(--font-size-sm, 12px);
  font-family: var(--font-family-base, inherit);
}

.result-vxe .vxe-cell--label,
.result-vxe .vxe-cell {
  font-size: var(--font-size-sm, 12px);
}

/* 等宽数字：让每个数字占等宽，列内小数点 / 位数纵向对齐，
   配合数值列右对齐可显著提升扫描效率，且零空间成本。 */
.result-vxe .vxe-cell {
  font-variant-numeric: tabular-nums;
}

/* 待插入的新增行（幽灵行）：给一个稳定的视觉标记，
   不然「这几行到底是查出来的还是我刚加的」只能靠记性。 */
.result-vxe .vxe-body--row.is-new-row td.vxe-body--column {
  background-color: color-mix(in srgb, var(--el-color-success) 12%, transparent);
}
.result-vxe .vxe-body--row.is-new-row:hover td.vxe-body--column {
  background-color: color-mix(in srgb, var(--el-color-success) 20%, transparent);
}
.result-vxe .row-star {
  color: var(--stateSuccessFg);
  font-weight: 700;
}

/* 表格容器自身不要撑出滚动条：滚动由 vxe 内部虚拟容器负责 */
.result-vxe .vxe-table--body-wrapper {
  overscroll-behavior: contain;
}
</style>
