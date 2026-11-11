<template>
  <div class="compare">
    <!-- 少于两个 dump：无从对比，直接引导 -->
    <div v-if="others.length === 0" class="cmp-empty">
      <el-icon class="cmp-empty-ic"><DataAnalysis /></el-icon>
      <div>
        <b>需要至少两个堆转储才能对比</b>
        <span>当前只有「{{ dump.name }}」。从左侧再打开一个 .hprof，然后回到这个子标签选择对比对象。</span>
      </div>
    </div>

    <template v-else>
      <div class="cmp-toolbar">
        <span class="cmp-side">A · <b class="mono" :title="dump.path">{{ dump.name }}</b></span>
        <span class="cmp-vs">对比</span>
        <el-select v-model="bPath" size="small" class="cmp-pick" placeholder="选择 B 侧堆转储" @change="reload">
          <el-option v-for="d in others" :key="d.id" :value="d.path" :label="d.name" />
        </el-select>
        <el-radio-group v-model="groupBy" size="small" @change="reload">
          <el-radio-button value="class">按类</el-radio-button>
          <el-radio-button value="package">按包</el-radio-button>
        </el-radio-group>
        <el-input
          v-model="query"
          size="small"
          clearable
          class="cmp-search"
          placeholder="筛选名称（支持 * ? 通配）"
          aria-label="筛选类名或包名"
          @keyup.enter="reload"
          @clear="reload"
        >
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <el-checkbox v-model="onlyChanged" size="small" @change="reload">只看变化</el-checkbox>
        <el-select v-model="sort" size="small" class="cmp-sort" aria-label="排序方式" @change="reload">
          <el-option value="delta_shallow" label="按 Δ浅堆" />
          <el-option value="delta_count" label="按 Δ对象数" />
          <el-option value="shallow_b" label="按 B 浅堆" />
          <el-option value="name" label="按名称" />
        </el-select>
        <app-button size="small" text aria-label="重新对比" @click="reload">
          <el-icon><Refresh /></el-icon>
          <span>刷新</span>
        </app-button>
        <ExportButton
          :rows="state.data?.rows ?? []"
          :columns="exportColumns"
          :filename="`${dump.name}-vs-${bName}.csv`"
          label="CSV"
        />
      </div>

      <div v-if="state.error" class="cmp-error">
        <el-icon><WarningFilled /></el-icon>
        <span>{{ state.error }}</span>
        <app-button size="small" text @click="reload">
          <el-icon><Refresh /></el-icon>
          <span>重试</span>
        </app-button>
      </div>

      <template v-else-if="state.data">
        <!-- 两侧合计 + Δ -->
        <div class="cmp-stats">
          <div class="cmp-stat">
            <span class="v">{{ formatInt(state.data.objects_a) }}</span>
            <span class="k">A 对象数</span>
          </div>
          <div class="cmp-stat">
            <span class="v">{{ formatInt(state.data.objects_b) }}</span>
            <span class="k">B 对象数</span>
          </div>
          <div class="cmp-stat" :class="toneClass(state.data.delta_objects)">
            <span class="v">{{ signedInt(state.data.delta_objects) }}</span>
            <span class="k">Δ对象数</span>
          </div>
          <div class="cmp-stat">
            <span class="v">{{ formatBytes(state.data.shallow_a) }}</span>
            <span class="k">A 浅堆</span>
          </div>
          <div class="cmp-stat">
            <span class="v">{{ formatBytes(state.data.shallow_b) }}</span>
            <span class="k">B 浅堆</span>
          </div>
          <div class="cmp-stat" :class="toneClass(state.data.delta_shallow)">
            <span class="v">{{ signedBytes(state.data.delta_shallow) }}</span>
            <span class="k">Δ浅堆</span>
          </div>
          <div class="cmp-stat">
            <span class="v">{{ formatInt(state.data.added) }} / {{ formatInt(state.data.removed) }} / {{ formatInt(state.data.changed) }}</span>
            <span class="k">新增 / 消失 / 变化</span>
          </div>
        </div>

        <div class="cmp-note">{{ state.data.note }}</div>

        <div class="cmp-table" v-loading="state.loading">
          <el-table :data="state.data.rows" height="100%" size="small" border stripe>
            <el-table-column type="index" label="#" width="56" align="right" />
            <el-table-column :label="groupBy === 'package' ? 'Package' : 'Class Name'" min-width="280" show-overflow-tooltip>
              <template #default="{ row }">
                <span class="mono">{{ row.name }}</span>
                <span class="cmp-tag" :class="row.status">{{ statusLabel(row.status) }}</span>
              </template>
            </el-table-column>
            <el-table-column label="A 对象" width="94" align="right">
              <template #default="{ row }"><span class="num">{{ formatInt(row.count_a) }}</span></template>
            </el-table-column>
            <el-table-column label="B 对象" width="94" align="right">
              <template #default="{ row }"><span class="num">{{ formatInt(row.count_b) }}</span></template>
            </el-table-column>
            <el-table-column label="Δ对象" width="100" align="right" sortable prop="delta_count">
              <template #default="{ row }">
                <span class="num" :class="toneClass(row.delta_count)">{{ signedInt(row.delta_count) }}</span>
              </template>
            </el-table-column>
            <el-table-column label="A 浅堆" width="108" align="right">
              <template #default="{ row }"><span class="num">{{ formatBytes(row.shallow_a) }}</span></template>
            </el-table-column>
            <el-table-column label="B 浅堆" width="108" align="right">
              <template #default="{ row }"><span class="num">{{ formatBytes(row.shallow_b) }}</span></template>
            </el-table-column>
            <el-table-column label="Δ浅堆" width="124" align="right" sortable prop="delta_shallow">
              <template #default="{ row }">
                <span class="num strong" :class="toneClass(row.delta_shallow)">{{ signedBytes(row.delta_shallow) }}</span>
              </template>
            </el-table-column>
            <template #empty>
              <div class="cmp-empty-row">
                {{ onlyChanged ? '两侧没有差异（该过滤下全部相同）' : '没有匹配的分组' }}
              </div>
            </template>
          </el-table>
        </div>

        <div class="cmp-footer">
          <app-button size="small" text :disabled="page === 0" @click="goPage(-1)">上一页</app-button>
          <span class="cmp-page">第 {{ page + 1 }} / {{ totalPages }} 页 · 每页 {{ PAGE }} 行 · 共 {{ formatInt(state.data.total) }} 组</span>
          <app-button size="small" text :disabled="page + 1 >= totalPages" @click="goPage(1)">下一页</app-button>
          <span class="cmp-elapsed">{{ state.data.elapsed_ms }} ms</span>
        </div>
      </template>

      <div v-else-if="!state.loading" class="cmp-empty">
        <div>
          <b>选择 B 侧堆转储开始对比</b>
          <span>对比只比类/包维度的对象数与浅堆；Δretained 请分别看两个 dump 的「支配树 → 按类」。</span>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { DataAnalysis, Refresh, Search, WarningFilled } from '@element-plus/icons-vue'
import { useMatStore } from '../../store'
import { fetchCompareHistograms, formatBytes, formatInt } from '../../api/matApi'
import type { CompareResult, CompareSort } from '../../api/matApi'
import type { MatDump } from '../../types'
import type { ExportColumn } from '../../utils/export'
import ExportButton from '../ExportButton.vue'

/**
 * 多 dump 对比（histogram diff）：A 侧固定为当前 dump tab，B 侧从其它已打开的 dump 里选。
 *
 * 只比 **类/包维度**的 count 与 shallow —— 两侧都走同一个后端聚合函数，所以「单个 dump 的
 * 直方图」与「参与对比时的数字」必然一致。Δretained 需要两份支配树（各一次 LT，分钟级，
 * 且会同时占满会话 LRU 的两个槽位），刻意不做，界面与返回的 note 都指向「各自看支配树」。
 *
 * Δ 的配色：**变多为警示色、变少为成功色**（内存增长才是要查的方向）。
 */
const props = defineProps<{ dump: MatDump }>()
const store = useMatStore()

const PAGE = 200
const groupBy = ref<'class' | 'package'>('class')
const query = ref('')
const onlyChanged = ref(true)
const sort = ref<CompareSort>('delta_shallow')
const page = ref(0)
const bPath = ref('')
const state = ref<{ loading: boolean; error: string; data: CompareResult | null }>({
  loading: false, error: '', data: null,
})

/** 可选的 B 侧：排除自己（后端也会拒同一文件，这里提前挡掉） */
const others = computed(() => store.dumps.filter((d) => d.path !== props.dump.path))
const bName = computed(() => others.value.find((d) => d.path === bPath.value)?.name || 'B')

const totalPages = computed(() => Math.max(1, Math.ceil((state.value.data?.total ?? 0) / PAGE)))

const exportColumns: ExportColumn[] = [
  { key: 'name', label: 'Name' },
  { key: 'kind', label: 'Kind' },
  { key: 'count_a', label: 'Count A' },
  { key: 'count_b', label: 'Count B' },
  { key: 'delta_count', label: 'Delta Count' },
  { key: 'shallow_a', label: 'Shallow A (bytes)' },
  { key: 'shallow_b', label: 'Shallow B (bytes)' },
  { key: 'delta_shallow', label: 'Delta Shallow (bytes)' },
  { key: 'status', label: 'Status' },
]

function signedInt(v: number): string {
  return v > 0 ? `+${formatInt(v)}` : formatInt(v)
}

function signedBytes(v: number): string {
  if (!v) return '0 B'
  return (v > 0 ? '+' : '-') + formatBytes(Math.abs(v))
}

/** 增长用警示色、减少用成功色；0 保持中性（不要给「没变」上色） */
function toneClass(v: number): string {
  if (v > 0) return 'up'
  if (v < 0) return 'down'
  return ''
}

const STATUS_LABELS: Record<string, string> = {
  added: '仅 B',
  removed: '仅 A',
  changed: '变了',
  same: '相同',
}
function statusLabel(s: string): string {
  return STATUS_LABELS[s] || s
}

async function load(): Promise<void> {
  if (!bPath.value) return
  state.value = { loading: true, error: '', data: state.value.data }
  try {
    const data = await fetchCompareHistograms(props.dump.path, bPath.value, {
      groupBy: groupBy.value,
      query: query.value,
      onlyChanged: onlyChanged.value,
      sort: sort.value,
      offset: page.value * PAGE,
      limit: PAGE,
    })
    state.value = { loading: false, error: '', data }
  } catch (e: any) {
    state.value = { loading: false, error: e?.message || String(e), data: null }
  }
}

function reload(): void {
  page.value = 0
  void load()
}

function goPage(delta: number): void {
  const next = page.value + delta
  if (next < 0 || next >= totalPages.value) return
  page.value = next
  void load()
}

/** 首次进入 / dump 切换：默认选中第一个可用的 B 侧 */
function ensureB(): void {
  if (bPath.value && others.value.some((d) => d.path === bPath.value)) return
  bPath.value = others.value[0]?.path || ''
}

onMounted(() => {
  ensureB()
  void load()
})

watch(
  () => props.dump.id,
  () => {
    state.value = { loading: false, error: '', data: null }
    page.value = 0
    query.value = ''
    ensureB()
    void load()
  },
)

// 用户新开 / 关闭 dump 时，若原先的 B 侧被关掉则自动换一个
watch(
  () => others.value.map((d) => d.path).join('|'),
  () => {
    const before = bPath.value
    ensureB()
    if (before !== bPath.value) void load()
  },
)
</script>

<style scoped lang="scss">
.compare {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.cmp-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
  flex-wrap: wrap;
}

.cmp-side {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cmp-vs {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
}

.cmp-pick { width: 200px; }
.cmp-search { width: 200px; }
.cmp-sort { width: 116px; }

.cmp-error {
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

.cmp-stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(112px, 1fr));
  gap: 1px;
  background: var(--el-border-color-lighter);
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}

.cmp-stat {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 7px 12px;
  background: var(--el-fill-color-lighter);
  min-width: 0;

  .v {
    font-size: var(--font-size-lg);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--el-text-color-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .k {
    font-size: var(--font-size-xs);
    color: var(--el-text-color-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &.up .v { color: var(--stateWarningFg); }
  &.down .v { color: var(--stateSuccessFg); }
}

.cmp-note {
  padding: 6px 12px;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}

.cmp-table { flex: 1; min-height: 0; }

.mono { font-family: var(--font-family-mono); }

.num { font-variant-numeric: tabular-nums; }
.num.strong { font-weight: 600; }
.up { color: var(--stateWarningFg); }
.down { color: var(--stateSuccessFg); }

.cmp-tag {
  margin-left: 6px;
  padding: 0 4px;
  font-size: var(--font-size-xs);
  border-radius: var(--radius-xs);
  border: 1px solid var(--el-border-color-lighter);
  color: var(--el-text-color-secondary);

  &.added { color: var(--stateWarningFg); border-color: var(--el-color-warning-light-5); }
  &.removed { color: var(--stateSuccessFg); border-color: var(--el-color-success-light-5); }
}

.cmp-empty-row {
  padding: 16px;
  text-align: center;
  color: var(--el-text-color-secondary);
  font-size: var(--font-size-sm);
}

.cmp-footer {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 12px;
  border-top: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}

.cmp-page,
.cmp-elapsed {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.cmp-elapsed { margin-left: auto; }

.cmp-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 24px;
  color: var(--el-text-color-secondary);
  font-size: var(--font-size-sm);

  div {
    display: flex;
    flex-direction: column;
    gap: 3px;
    max-width: 520px;
  }

  b { color: var(--el-text-color-primary); }

  span {
    font-size: var(--font-size-xs);
    line-height: 1.7;
  }
}

.cmp-empty-ic {
  font-size: 30px;
  opacity: 0.5;
}
</style>
