<template>
  <div class="histogram">
    <!-- 加载失败 -->
    <div v-if="state.error" class="hg-error">
      <el-icon><WarningFilled /></el-icon>
      <span>{{ state.error }}</span>
      <app-button size="small" text @click="load">
        <el-icon><Refresh /></el-icon>
      </app-button>
    </div>

    <!-- 加载中 -->
    <div v-else-if="state.loading" v-loading="true" class="hg-loading"></div>

    <template v-else-if="state.data">
      <!-- 工具条：搜索 + 统计 -->
      <div class="hg-toolbar">
        <el-input
          v-model="keyword"
          size="small"
          clearable
          class="hg-search"
          placeholder="筛选类名（支持正则）"
          aria-label="筛选类名"
        >
          <template #prefix>
            <el-icon><Search /></el-icon>
          </template>
        </el-input>
        <span class="hg-meta">
          {{ filtered.length }} / {{ state.data.total_classes }} 类 ·
          {{ formatInt(state.data.total_objects) }} 对象 ·
          浅堆 {{ formatBytes(state.data.total_shallow_bytes) }}
        </span>
        <span v-if="state.data.truncated" class="hg-truncated">仅显示前 {{ state.data.rows.length }} 行</span>
        <span class="hg-elapsed">解析耗时 {{ state.data.elapsed_ms }} ms</span>
      </div>

      <!-- 直方图表格：el-table-v2 虚拟滚动（行数多时只渲染可视窗口，避免整表 DOM 卡顿） -->
      <div ref="tableWrap" class="hg-table">
        <el-table-v2
          v-if="view.w > 0 && view.h > 0"
          :columns="columns"
          :data="filtered"
          :width="view.w"
          :height="view.h"
          :row-height="ROW_H"
          :header-height="HEADER_H"
          row-key="class_name"
          :sort-by="sortBy"
          class="hg-vtable"
          @column-sort="onColumnSort"
        >
          <template #empty>
            <div class="hg-empty">无匹配的类</div>
          </template>
        </el-table-v2>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, h, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Refresh, Search, WarningFilled } from '@element-plus/icons-vue'
import type { Column } from 'element-plus'
import { useMatStore } from '../../store'
import { formatBytes, formatInt } from '../../api/matApi'
import type { MatDump } from '../../types'

/**
 * 直方图：按类统计对象数量与浅堆（Shallow Heap）。
 * 结果按 dump 缓存于 store（切子 tab / 切 dump 返回不重复解析）。
 * 搜索关键字兼容正则（EP table 自带过滤用不到，这里做服务端行内过滤）。
 *
 * 大表性能：后端不限制行数（返回全部类，真实大 dump 可达数万行），
 * 表格用 el-table-v2 虚拟列表（仅渲染可视窗口 + 缓冲行），
 * 行高固定（ROW_H），列定义见下方 columns computed。
 */
const props = defineProps<{
  dump: MatDump
}>()

const ROW_H = 28
const HEADER_H = 32

const store = useMatStore()
const keyword = ref('')

type SortKey = 'class_name' | 'count' | 'shallow_bytes'
type SortOrderV2 = 'asc' | 'desc'
/** 当前排序列（受控给 el-table-v2，驱动表头排序高亮） */
const sortBy = ref<{ key: SortKey; order: SortOrderV2 }>({ key: 'shallow_bytes', order: 'desc' })

/** 表格可视尺寸（容器 flex:1，由 ResizeObserver 实时测量后喂给虚拟表） */
const tableWrap = ref<HTMLElement | null>(null)
const view = ref({ w: 0, h: 0 })
let ro: ResizeObserver | null = null

const state = computed(() => store.histCache[props.dump.id] ?? { loading: false, error: '', data: null })

/** 加载直方图（缓存命中不重复请求） */
async function load() {
  await store.ensureHistogram(props.dump.id)
}

const filtered = computed(() => {
  const rows = state.value.data?.rows ?? []
  if (!keyword.value) return rows
  let re: RegExp | null = null
  try {
    re = new RegExp(keyword.value, 'i')
  } catch {
    re = null
  }
  return rows.filter(r => {
    if (re) return re.test(r.class_name)
    return r.class_name.toLowerCase().includes(keyword.value.toLowerCase())
  })
})

/** 表格列定义：index 固定列 + 类名弹性列 + 数值/占比列（全部 cellRenderer 渲染，规避虚拟行模板开销） */
const columns = computed<Column<any>[]>(() => {
  const total = state.value.data?.total_shallow_bytes ?? 0
  return [
    {
      key: 'idx',
      title: '#',
      width: 56,
      fixed: true,
      align: 'right',
      cellRenderer: ({ rowIndex }) => h('span', { class: 'hg-idx' }, String(rowIndex + 1)),
    },
    {
      key: 'class_name',
      dataKey: 'class_name',
      title: 'Class Name',
      width: 240,
      minWidth: 200,
      flexGrow: 1,
      flexShrink: 0,
      sortable: true,
      cellRenderer: ({ cellData }) => h('span', { class: 'hg-mono hg-cls', title: cellData }, cellData),
    },
    {
      key: 'count',
      dataKey: 'count',
      title: '对象数量',
      width: 120,
      align: 'right',
      flexShrink: 0,
      sortable: true,
      cellRenderer: ({ cellData }) => h('span', { class: 'hg-num' }, formatInt(cellData)),
    },
    {
      key: 'shallow_bytes',
      dataKey: 'shallow_bytes',
      title: '浅堆大小 (Shallow)',
      width: 150,
      align: 'right',
      flexShrink: 0,
      sortable: true,
      cellRenderer: ({ cellData }) => h('span', { class: 'hg-num hg-strong' }, formatBytes(cellData)),
    },
    {
      key: 'pct',
      dataKey: 'shallow_bytes',
      title: '占比',
      width: 150,
      align: 'right',
      flexShrink: 0,
      cellRenderer: ({ cellData }) => {
        const p = total ? (cellData / total) * 100 : 0
        return h('div', { class: 'hg-pct-cell' }, [
          h('div', { class: 'hg-pct-bar', style: { width: `${p}%` } }),
          h('span', { class: 'hg-pct-text' }, `${p.toFixed(2)}%`),
        ])
      },
    },
    {
      key: 'act',
      title: '操作',
      width: 88,
      fixed: true,
      align: 'center',
      flexShrink: 0,
      cellRenderer: ({ rowData }) =>
        h(
          'button',
          {
            class: 'hg-jump',
            title: `引用追踪：${rowData.class_name} 的实例`,
            onClick: (e: MouseEvent) => {
              e.stopPropagation()
              store.openRefs(props.dump.id, { kind: 'class', query: rowData.class_name })
            },
          },
          '追踪'
        ),
    },
  ]
})

/** el-table-v2 表头点击排序（仅 sortable 列触发）：更新受控 sortBy + 就地排序（行数少，内存排） */
function onColumnSort({ key, order }: { key: string | number | symbol; order: SortOrderV2 }) {
  const k = key as SortKey
  if (k !== 'class_name' && k !== 'count' && k !== 'shallow_bytes') return
  sortBy.value = { key: k, order }
  const rows = state.value.data!.rows
  const dir = order === 'asc' ? 1 : -1
  rows.sort((a, b) => {
    if (k === 'class_name') return a.class_name.localeCompare(b.class_name) * dir
    const va = k === 'count' ? a.count : a.shallow_bytes
    const vb = k === 'count' ? b.count : b.shallow_bytes
    return (va - vb) * dir
  })
}

function measure() {
  const el = tableWrap.value
  if (!el) return
  const r = el.getBoundingClientRect()
  view.value = { w: Math.floor(r.width), h: Math.floor(r.height) }
}

/** 数据就绪（表格容器挂载）后再测量/挂观察：加载态容器不在 DOM，onMounted 时可能量不到 */
watch(
  () => state.value.data,
  () => {
    void nextTick(() => {
      measure()
      if (tableWrap.value && ro) ro.observe(tableWrap.value)
    })
  },
  { immediate: true }
)

onMounted(() => {
  void load()
  measure()
  ro = new ResizeObserver(measure)
  if (tableWrap.value) ro.observe(tableWrap.value)
})

onBeforeUnmount(() => {
  ro?.disconnect()
  ro = null
})
</script>

<style scoped lang="scss">
.histogram {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.hg-error {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  font-size: var(--font-size-sm);
  color: var(--el-color-danger);
  background: var(--el-color-danger-light-9);
  border-bottom: 1px solid var(--el-color-danger-light-7);

  span {
    flex: 1;
    min-width: 0;
  }
}

.hg-loading {
  flex: 1;
}

.hg-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
  flex-wrap: wrap;
}

.hg-search {
  width: 260px;
}

.hg-meta {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.hg-truncated {
  font-size: var(--font-size-xs);
  color: var(--el-color-warning);
}

.hg-elapsed {
  margin-left: auto;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
}

.hg-table {
  flex: 1;
  min-height: 0;
  position: relative;
}
</style>

<!--
  以下样式作用于 el-table-v2 虚拟列表内部：
  cellRenderer 用 h() 创建的节点不带 scoped data-v 属性，scoped 样式匹配不到，
  因此表格内部样式放非 scoped 块（类名加 hg- 前缀防全局泄漏）。
-->
<style lang="scss">
.hg-vtable {
  font-size: var(--font-size-base); // 12px，覆盖 EP 默认 14px，与原 el-table small 观感一致
}

.hg-vtable .el-table-v2__header-cell {
  font-size: var(--font-size-base);
  font-weight: 600;
}

/* 非当前排序列 hover 也显示排序箭头（EP 默认仅当前列常显） */
.hg-vtable .el-table-v2__header-cell.is-sortable:hover .el-table-v2__sort-icon {
  display: flex;
  opacity: 0.6;
}

.hg-mono {
  font-family: var(--font-family-mono);
}

.hg-jump {
  border: 1px solid var(--el-border-color);
  border-radius: var(--radius-xs);
  background: transparent;
  color: var(--el-color-primary);
  font-size: 12px;
  line-height: 1.6;
  padding: 1px 8px;
  cursor: pointer;
}

.hg-jump:hover {
  background: var(--el-color-primary-light-9);
  border-color: var(--el-color-primary-light-5);
}

/* 类名超长省略（虚拟行宽固定，靠 cell overflow 裁剪不换行） */
.hg-cls {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.hg-num {
  font-variant-numeric: tabular-nums;

  &.hg-strong {
    font-weight: 600;
  }
}

.hg-idx {
  font-variant-numeric: tabular-nums;
  color: var(--el-text-color-secondary);
}

.hg-pct-cell {
  position: relative;
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: flex-end;
}

.hg-pct-bar {
  position: absolute;
  right: 0;
  top: 50%;
  height: 12px;
  transform: translateY(-50%);
  background: var(--el-color-primary-light-7);
  opacity: 0.35;
  border-radius: var(--radius-xs);
}

.hg-pct-text {
  position: relative;
  font-size: var(--font-size-xs);
  font-variant-numeric: tabular-nums;
  color: var(--el-text-color-secondary);
}

.hg-empty {
  padding: 16px;
  text-align: center;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}
</style>
