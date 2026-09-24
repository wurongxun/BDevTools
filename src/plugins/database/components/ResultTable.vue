<template>
  <div class="result-table" :aria-busy="ariaBusy || loading">
    <el-table
      ref="tableRef"
      :data="rows"
      :border="border"
      :stripe="stripe"
      :size="size"
      :height="height"
      :max-height="maxHeight"
      :show-header="showHeader"
      :highlight-current-row="highlightCurrentRow"
      :row-key="rowKey"
      v-loading="loading"
      element-loading-text="加载中…"
      style="width: 100%"
      @select="handleSelect"
      @select-all="handleSelectAll"
      @row-click="handleRowClick"
      @sort-change="handleSortChange"
      @header-dragend="handleHeaderDragend"
    >
      <!-- 选择列 -->
      <el-table-column
        v-if="selectable"
        type="selection"
        width="48"
        align="center"
        :selectable="selectableFn"
      />

      <!-- 序号列 -->
      <el-table-column
        v-if="showIndex"
        type="index"
        label="#"
        width="60"
        align="center"
        fixed="left"
      />

      <!-- 数据列 -->
      <el-table-column
        v-for="(col, idx) in columns"
        :key="idx"
        :prop="String(idx)"
        :label="col"
        :min-width="colMinWidth"
        :width="getColumnWidth(idx)"
        show-overflow-tooltip
        :sortable="sortable ? 'custom' : false"
        :resizable="resizable"
        align="left"
      >
        <template #default="{ row }">
          <slot :name="`cell-${idx}`" :row="row" :column="col" :index="idx">
            <span class="cell-content">{{ formatCell(row[String(idx)]) }}</span>
          </slot>
        </template>
      </el-table-column>

      <!-- 操作列（预留） -->
      <el-table-column
        v-if="$slots.actions"
        label="操作"
        width="120"
        align="center"
        fixed="right"
      >
        <template #default="{ row, $index }">
          <slot name="actions" :row="row" :index="$index" />
        </template>
      </el-table-column>

      <!-- 空状态 -->
      <template #empty>
        <el-empty :description="emptyText" :image-size="80">
          <template #image>
            <svg-icon v-if="emptyIcon" :icon-class="emptyIcon" />
          </template>
        </el-empty>
      </template>
    </el-table>

    <!-- 底部状态栏 -->
    <div v-if="showFooter" class="table-footer">
      <span class="footer-info">
        共 <b>{{ rows.length }}</b> 条记录
        <span v-if="selection.length > 0">，已选 <b>{{ selection.length }}</b> 条</span>
      </span>
      <slot name="footer" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'

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
  /** 表格高度 */
  height?: string | number
  /** 最大高度 */
  maxHeight?: string | number
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
  /** 行 key */
  rowKey?: string | ((row: any) => string)
  /** 空数据文本 */
  emptyText?: string
  /** 空数据图标（svg-icon 的 iconClass） */
  emptyIcon?: string
  /** 是否显示底部状态栏 */
  showFooter?: boolean
  /** 列宽映射（列索引 => 宽度） */
  columnWidths?: Record<number, number>
  /** 可选行判断函数 */
  selectableFn?: (row: any, index: number) => boolean
  /** 无障碍：区域是否处于忙碌状态（外部显式声明，与 loading 取并集） */
  ariaBusy?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  border: true,
  stripe: true,
  size: 'small',
  colMinWidth: 100,
  showHeader: true,
  selectable: false,
  showIndex: false,
  sortable: false,
  resizable: true,
  highlightCurrentRow: false,
  emptyText: '暂无数据',
  emptyIcon: '',
  showFooter: false,
  columnWidths: () => ({}),
  ariaBusy: false,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  selectableFn: (_row: any, _index: number) => true,
})

const emit = defineEmits<{
  /** 行点击 */
  (e: 'row-click', row: any, index: number): void
  /** 选中变化 */
  (e: 'selection-change', selection: any[]): void
  /** 排序变化 */
  (e: 'sort-change', column: string, order: string | null): void
  /** 列宽拖拽结束 */
  (e: 'column-resize', columnIndex: number, width: number): void
}>()

const tableRef = ref()
const selection = ref<any[]>([])

// 获取列宽
const getColumnWidth = (idx: number) => {
  return props.columnWidths[idx] || undefined
}

// 格式化单元格内容（null / undefined / 空串 显示为 -，对象序列化为 JSON）
const formatCell = (value: any) => {
  if (value === null || value === undefined || value === '') {
    return '-'
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

// 行点击
const handleRowClick = (row: any, _column: any, index: number) => {
  emit('row-click', row, index)
}

// 单选
const handleSelect = (newSelection: any[]) => {
  selection.value = newSelection
  emit('selection-change', newSelection)
}

// 全选
const handleSelectAll = (newSelection: any[]) => {
  selection.value = newSelection
  emit('selection-change', newSelection)
}

// 排序变化
const handleSortChange = ({ prop, order }: { prop: string; order: string | null }) => {
  emit('sort-change', prop, order)
}

// 列宽拖拽结束
const handleHeaderDragend = (newWidth: number, _oldWidth: number, column: any) => {
  const idx = Number(column.prop)
  if (!isNaN(idx)) {
    emit('column-resize', idx, newWidth)
  }
}

// 暴露方法
defineExpose({
  tableRef,
  clearSelection: () => tableRef.value?.clearSelection(),
  toggleRowSelection: (row: any, selected?: boolean) =>
    tableRef.value?.toggleRowSelection(row, selected),
  sort: (prop: string, order: string) => tableRef.value?.sort(prop, order),
})

// 监听 rows 变化，清空选中
watch(
  () => props.rows,
  () => {
    if (props.selectable) {
      selection.value = []
    }
  }
)
</script>

<style scoped>
.result-table {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  min-height: 0;
}

.result-table :deep(.el-table) {
  flex: 1;
  min-height: 0;
}

.result-table :deep(.el-table__body-wrapper) {
  overflow: auto;
}

.cell-content {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  line-height: 1.5;
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
</style>
