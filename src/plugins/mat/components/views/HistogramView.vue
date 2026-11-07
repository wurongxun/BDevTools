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
      <!-- 工具条：搜索 + 分组 + 深堆列 + 统计 + 导出 -->
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
        <el-select v-model="groupBy" size="small" class="hg-group" aria-label="分组方式">
          <el-option value="class" label="按类" />
          <el-option value="package" label="按包" />
        </el-select>
        <el-checkbox v-model="showRetained" size="small" class="hg-switch">深堆列</el-checkbox>
        <span class="hg-meta">
          {{ filtered.length }} / {{ state.data.total_classes }} 类 ·
          {{ formatInt(state.data.total_objects) }} 对象 ·
          浅堆 {{ formatBytes(state.data.total_shallow_bytes) }}
        </span>
        <span v-if="retainedLoading" class="hg-truncated">深堆计算中…</span>
        <span v-if="state.data.truncated" class="hg-truncated">仅显示前 {{ state.data.rows.length }} 行</span>
        <span class="hg-elapsed">解析耗时 {{ state.data.elapsed_ms }} ms</span>
        <ExportButton
          :rows="exportRows"
          :columns="exportColumns"
          :filename="`${dump.name}-histogram-${groupBy}.csv`"
          label="CSV"
        />
      </div>

      <!-- 直方图表格：el-table-v2 虚拟滚动（行数多时只渲染可视窗口） -->
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
import { ElMessage } from 'element-plus'
import type { Column } from 'element-plus'
import { useMatStore } from '../../store'
import { fetchDomTree, formatBytes, formatInt } from '../../api/matApi'
import type { MatDump } from '../../types'
import type { ExportColumn } from '../../utils/export'
import ExportButton from '../ExportButton.vue'

/**
 * 直方图：按类 / 按包统计对象数量、浅堆与深堆（Retained Heap）。
 * 结果按 dump 缓存于 store；深堆列**按需开启**——开启才触发支配树计算（与泄漏检测、
 * Dominator Tree 共用会话缓存，同一 dump 只算一次 LT），避免每次进入本视图都重算。
 *
 * 大表性能：后端不限制行数（真实大 dump 可达数万行），表格用 el-table-v2 虚拟列表；
 * 深堆值不写回行对象，cellRenderer 直接查 retainedMap ⇒ 不重建整份行数据。
 */
const props = defineProps<{
  dump: MatDump
}>()

const ROW_H = 28
const HEADER_H = 32

const store = useMatStore()
const keyword = ref('')
const groupBy = ref<'class' | 'package'>('class')
const showRetained = ref(false)
const retainedLoading = ref(false)
const retainedMap = ref<Map<string, number>>(new Map())

type SortKey = 'class_name' | 'count' | 'shallow_bytes' | 'retained_bytes'
type SortOrderV2 = 'asc' | 'desc'
const sortBy = ref<{ key: SortKey; order: SortOrderV2 }>({ key: 'shallow_bytes', order: 'desc' })

/** 表格可视尺寸（容器 flex:1，由 ResizeObserver 实时测量后喂给虚拟表） */
const tableWrap = ref<HTMLElement | null>(null)
const view = ref({ w: 0, h: 0 })
let ro: ResizeObserver | null = null

const state = computed(() => store.histCache[props.dump.id] ?? { loading: false, error: '', data: null })

async function load() {
  await store.ensureHistogram(props.dump.id)
}

/** HPROF 类名可能是 "java/util/HashMap" 或 "java.util.HashMap"，统一取包名 */
function pkgOf(name: string): string {
  const norm = name.replace(/\./g, '/')
  const i = norm.lastIndexOf('/')
  return i >= 0 ? norm.slice(0, i) : '(default)'
}

/** 深堆列：按当前分组方式分页拉全部分组的 retained（dom 缓存命中后很快） */
async function loadRetained(): Promise<void> {
  if (!showRetained.value) return
  retainedLoading.value = true
  retainedMap.value = new Map()
  try {
    // 深堆列要支配树：大 dump 先让后台进程算完，别在主进程里现跑 Lengauer-Tarjan
    await store.ensureAnalysisReady(props.dump.id, true)
    const map = new Map<string, number>()
    const PAGE = 2000
    let offset = 0
    for (let i = 0; i < 30; i++) {
      const res = await fetchDomTree(props.dump.path, {
        groupBy: groupBy.value,
        query: '',
        offset,
        limit: PAGE,
      })
      if (res.truncated) {
        ElMessage.warning(res.note || '支配树不可用，深堆列无法显示')
        showRetained.value = false
        break
      }
      for (const r of res.rows) {
        if (r.class_name) map.set(r.class_name, r.retained_bytes)
      }
      offset += res.rows.length
      if (res.rows.length === 0 || offset >= res.total) break
    }
    retainedMap.value = map
    if (offset >= 60_000) ElMessage.warning('分组过多，深堆列仅加载前 60000 组')
  } catch (e: any) {
    ElMessage.error('深堆列加载失败：' + (e?.message || String(e)))
    showRetained.value = false
  } finally {
    retainedLoading.value = false
  }
}

watch([showRetained, groupBy], () => {
  if (showRetained.value) void loadRetained()
})

/** 分组 → 关键词过滤 → 排序（排序在派生数组上做，不污染 store 缓存） */
const filtered = computed(() => {
  const rows = state.value.data?.rows ?? []
  let base: { class_name: string; count: number; shallow_bytes: number }[] = rows
  if (groupBy.value === 'package') {
    const m = new Map<string, { class_name: string; count: number; shallow_bytes: number }>()
    for (const r of rows) {
      const pkg = pkgOf(r.class_name)
      const e = m.get(pkg) ?? { class_name: pkg, count: 0, shallow_bytes: 0 }
      e.count += r.count
      e.shallow_bytes += r.shallow_bytes
      m.set(pkg, e)
    }
    base = [...m.values()]
  }
  let out = base
  if (keyword.value) {
    let re: RegExp | null = null
    try { re = new RegExp(keyword.value, 'i') } catch { re = null }
    out = base.filter(r => re ? re.test(r.class_name) : r.class_name.toLowerCase().includes(keyword.value.toLowerCase()))
  }
  const k = sortBy.value.key
  const dir = sortBy.value.order === 'asc' ? 1 : -1
  return [...out].sort((a, b) => {
    if (k === 'class_name') return a.class_name.localeCompare(b.class_name) * dir
    if (k === 'retained_bytes') {
      const va = retainedMap.value.get(a.class_name) ?? -1
      const vb = retainedMap.value.get(b.class_name) ?? -1
      return (va - vb) * dir
    }
    const va = k === 'count' ? a.count : a.shallow_bytes
    const vb = k === 'count' ? b.count : b.shallow_bytes
    return (va - vb) * dir
  })
})

const columns = computed<Column<any>[]>(() => {
  const total = state.value.data?.total_shallow_bytes ?? 0
  const cols: Column<any>[] = [
    {
      key: 'idx', title: '#', width: 56, fixed: true, align: 'right',
      cellRenderer: ({ rowIndex }) => h('span', { class: 'hg-idx' }, String(rowIndex + 1)),
    },
    {
      key: 'class_name', dataKey: 'class_name', title: groupBy.value === 'package' ? 'Package' : 'Class Name',
      width: 240, minWidth: 200, flexGrow: 1, flexShrink: 0, sortable: true,
      cellRenderer: ({ cellData }) => h('span', { class: 'hg-mono hg-cls', title: cellData }, cellData),
    },
    {
      key: 'count', dataKey: 'count', title: '对象数量', width: 120, align: 'right', flexShrink: 0, sortable: true,
      cellRenderer: ({ cellData }) => h('span', { class: 'hg-num' }, formatInt(cellData)),
    },
    {
      key: 'shallow_bytes', dataKey: 'shallow_bytes', title: '浅堆大小 (Shallow)', width: 150, align: 'right', flexShrink: 0, sortable: true,
      cellRenderer: ({ cellData }) => h('span', { class: 'hg-num hg-strong' }, formatBytes(cellData)),
    },
  ]
  if (showRetained.value) {
    cols.push({
      key: 'retained_bytes', title: '深堆 (Retained)', width: 160, align: 'right', flexShrink: 0, sortable: true,
      cellRenderer: ({ rowData }) => {
        const v = retainedMap.value.get(rowData.class_name)
        return h('span', { class: 'hg-num hg-strong' }, v == null ? '—' : formatBytes(v))
      },
    })
  }
  cols.push(
    {
      key: 'pct', dataKey: 'shallow_bytes', title: '浅堆占比', width: 150, align: 'right', flexShrink: 0,
      cellRenderer: ({ cellData }) => {
        const p = total ? (cellData / total) * 100 : 0
        return h('div', { class: 'hg-pct-cell' }, [
          h('div', { class: 'hg-pct-bar', style: { width: `${p}%` } }),
          h('span', { class: 'hg-pct-text' }, `${p.toFixed(2)}%`),
        ])
      },
    },
    {
      key: 'act', title: '操作', width: 88, fixed: true, align: 'center', flexShrink: 0,
      cellRenderer: ({ rowData }) =>
        h('button', {
          class: 'hg-jump',
          title: `引用追踪：${rowData.class_name}`,
          onClick: (e: MouseEvent) => {
            e.stopPropagation()
            store.openRefs(props.dump.id, { kind: 'class', query: rowData.class_name })
          },
        }, '追踪'),
    }
  )
  return cols
})

/** 表头排序只更新受控状态，实际排序在 filtered computed 里 */
function onColumnSort({ key, order }: { key: string | number | symbol; order: SortOrderV2 }) {
  const k = key as SortKey
  if (!['class_name', 'count', 'shallow_bytes', 'retained_bytes'].includes(k)) return
  sortBy.value = { key: k, order }
}

const exportColumns: ExportColumn[] = [
  { key: 'class_name', label: 'Class / Package' },
  { key: 'count', label: 'Objects' },
  { key: 'shallow_bytes', label: 'Shallow (bytes)' },
  { key: 'retained_bytes', label: 'Retained (bytes)' },
]
const exportRows = computed(() =>
  filtered.value.map(r => ({
    ...r,
    retained_bytes: retainedMap.value.get(r.class_name) ?? '',
  }))
)

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
  color: var(--stateErrorFg);
  background: var(--el-color-danger-light-9);
  border-bottom: 1px solid var(--el-color-danger-light-7);

  span { flex: 1; min-width: 0; }
}

.hg-loading { flex: 1; }

.hg-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
  flex-wrap: wrap;
}

.hg-search { width: 240px; }
.hg-group { width: 104px; }
.hg-switch { margin-right: 0; }

.hg-meta {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.hg-truncated {
  font-size: var(--font-size-xs);
  color: var(--stateWarningFg);
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
  cellRenderer 用 h() 创建的节点不带 scoped data-v 属性，scoped 样式匹配不到，
  因此表格内部样式放非 scoped 块（类名加 hg- 前缀防全局泄漏）。
-->
<style lang="scss">
.hg-vtable {
  font-size: var(--font-size-base);
}

.hg-vtable .el-table-v2__header-cell {
  font-size: var(--font-size-base);
  font-weight: 600;
}

.hg-vtable .el-table-v2__header-cell.is-sortable:hover .el-table-v2__sort-icon {
  display: flex;
  opacity: 0.6;
}

.hg-mono { font-family: var(--font-family-mono); }

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

.hg-cls {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.hg-num {
  font-variant-numeric: tabular-nums;

  &.hg-strong { font-weight: 600; }
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
