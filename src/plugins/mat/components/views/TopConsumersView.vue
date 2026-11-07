<template>
  <div class="top-consumers">
    <div v-if="state.error" class="tc-error">
      <el-icon><WarningFilled /></el-icon>
      <span>{{ state.error }}</span>
      <app-button size="small" text @click="reload">
        <el-icon><Refresh /></el-icon>
        <span>重试</span>
      </app-button>
    </div>

    <div v-else-if="state.loading" v-loading="true" class="tc-loading">
      <WorkerProgress
        v-if="worker && !worker.finished"
        :dump-id="dump.id"
        hint="分析跑在独立子进程里，主窗口可以正常操作；完成后本页自动显示结果。"
        counters
      />
      <span v-else>正在计算支配树（Top Consumers 复用支配树结果，首次可能需要数十秒）</span>
    </div>

    <template v-else-if="state.data">
      <div class="tc-toolbar">
        <el-radio-group v-model="groupBy" size="small" @change="setGroup">
          <el-radio-button value="class">按类</el-radio-button>
          <el-radio-button value="package">按包</el-radio-button>
        </el-radio-group>
        <el-select v-model="limit" size="small" class="tc-limit" @change="reload">
          <el-option :value="20" label="前 20" />
          <el-option :value="50" label="前 50" />
          <el-option :value="100" label="前 100" />
          <el-option :value="200" label="前 200" />
        </el-select>
        <span class="tc-meta">
          可达 {{ formatInt(state.data.reach_nodes) }} 对象 ·
          浅堆 {{ formatBytes(state.data.reach_shallow) }} ·
          共 {{ formatInt(state.data.total) }} 组
        </span>
        <span v-if="state.data.cached" class="tc-cached">缓存命中</span>
        <app-button size="small" text aria-label="刷新 Top Consumers" @click="reload">
          <el-icon><Refresh /></el-icon>
          <span>刷新</span>
        </app-button>
        <ExportButton
          :rows="state.data.rows"
          :columns="exportColumns"
          :filename="`${dump.name}-top-consumers-${groupBy}.csv`"
          label="CSV"
        />
      </div>

      <div v-if="rows.length" class="tc-preset">
        <div class="tc-preset-head">
          <b>占比前 {{ Math.min(10, rows.length) }} 名</b>
          <span class="tc-meta">口径：支配树顶层对象的 retained，彼此不相交，求和≈可达堆</span>
        </div>
        <div class="tc-bars">
          <div v-for="r in rows.slice(0, 10)" :key="r.name" class="tc-bar-row" :title="`${r.name} · ${formatBytes(r.retained_bytes)} · ${r.pct.toFixed(2)}%`">
            <span class="tc-bar-name">{{ r.name }}</span>
            <span class="tc-bar-track">
              <span class="tc-bar-fill" :style="{ width: `${Math.min(100, r.pct)}%` }"></span>
            </span>
            <span class="tc-bar-pct">{{ r.pct.toFixed(2) }}%</span>
          </div>
        </div>
      </div>

      <div v-if="state.data.note" class="tc-note">{{ state.data.note }}</div>
      <div v-else-if="rows.length < state.data.total" class="tc-note">
        仅显示 retained 前 {{ rows.length }} 组；完整排行请导出 CSV 或在支配树中按类聚合。
      </div>

      <div class="tc-table">
        <el-table :data="rows" height="100%" size="small" border stripe>
          <el-table-column type="index" label="#" width="56" align="right" />
          <el-table-column
            :prop="'name'"
            :label="groupBy === 'package' ? 'Package' : 'Class Name'"
            min-width="240"
            show-overflow-tooltip
          >
            <template #default="{ row }"><span class="tc-mono">{{ row.name }}</span></template>
          </el-table-column>
          <el-table-column label="顶层对象" width="100" align="right" prop="objects" />
          <el-table-column label="Shallow" width="110" align="right">
            <template #default="{ row }">{{ formatBytes(row.shallow_bytes) }}</template>
          </el-table-column>
          <el-table-column label="Retained" width="120" align="right" sortable prop="retained_bytes">
            <template #default="{ row }"><span class="tc-strong">{{ formatBytes(row.retained_bytes) }}</span></template>
          </el-table-column>
          <el-table-column label="占比" width="150" align="right">
            <template #default="{ row }">
              <div class="tc-pct">
                <div class="tc-pct-bar" :style="{ width: `${Math.min(100, row.pct)}%` }"></div>
                <span>{{ row.pct.toFixed(2) }}%</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column label="最大来源类" min-width="220" show-overflow-tooltip>
            <template #default="{ row }"><span class="tc-mono">{{ row.top_class || '—' }}</span></template>
          </el-table-column>
          <el-table-column label="操作" width="110" fixed="right" align="center">
            <template #default="{ row }">
              <app-button
                size="small"
                text
                :aria-label="`在引用追踪中查看 ${row.name}`"
                @click="track(row)"
              >追踪</app-button>
            </template>
          </el-table-column>
          <template #empty><div class="tc-empty">没有可聚合的支配对象</div></template>
        </el-table>
      </div>

      <div class="tc-footer">
        <span class="tc-meta">共 {{ formatInt(state.data.total) }} 组，显示 {{ rows.length }} 组</span>
        <span class="tc-elapsed">{{ state.data.elapsed_ms }} ms</span>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { Refresh, WarningFilled } from '@element-plus/icons-vue'
import { useMatStore } from '../../store'
import WorkerProgress from '../WorkerProgress.vue'
import { fetchTopConsumers, formatBytes, formatInt } from '../../api/matApi'
import type { ConsumerRow, TopConsumersResult } from '../../api/matApi'
import type { MatDump } from '../../types'
import type { ExportColumn } from '../../utils/export'
import ExportButton from '../ExportButton.vue'

/**
 * Top Consumers：按类 / 包聚合 retained 排行。
 *
 * **口径要害**：后端只聚合支配树**顶层**（超根的直接孩子）。顶层支配对象的 retained
 * 集合互不相交，因此求和≈可达堆、占比可加；这与「支配树」子视图里 groupBy 的聚合
 * （子树有重叠、占比不可加）是两回事，界面注释里已写明，避免两个数字对不上被当成 bug。
 *
 * 图表用纯 CSS 条形（不引 echarts）：只在 10 行、两列宽度上表达占比，
 * 引入图表库要额外处理深色主题时序（useDark 先改 ref 后加类），收益不抵成本。
 */
const props = defineProps<{ dump: MatDump }>()
const store = useMatStore()
/** 后台分析作业（有就显示进度，别让用户对着一个转圈猜） */
const worker = computed(() => store.workerJobOf(props.dump.id))

type GroupBy = 'class' | 'package'
const groupBy = ref<GroupBy>('class')
const limit = ref(50)
const state = ref<{ loading: boolean; error: string; data: TopConsumersResult | null }>({
  loading: false, error: '', data: null,
})

const rows = computed<ConsumerRow[]>(() => state.value.data?.rows ?? [])

const exportColumns: ExportColumn[] = [
  { key: 'name', label: 'Name' },
  { key: 'kind', label: 'Kind' },
  { key: 'objects', label: 'Top Objects' },
  { key: 'shallow_bytes', label: 'Shallow (bytes)' },
  { key: 'retained_bytes', label: 'Retained (bytes)' },
  { key: 'pct', label: 'Retained %' },
  { key: 'top_class', label: 'Top Class' },
]

async function load(): Promise<void> {
  state.value = { loading: true, error: '', data: state.value.data }
  try {
    // 排行的 retained 来自支配树：大 dump 先让后台进程算完
    if ((await store.ensureAnalysisReady(props.dump.id, true)) === 'canceled') {
      throw new Error('后台分析已取消')
    }
    const data = await fetchTopConsumers(props.dump.path, groupBy.value, limit.value)
    state.value = { loading: false, error: '', data }
  } catch (e: any) {
    state.value = { loading: false, error: e?.message || String(e), data: null }
  }
}

function reload(): void { void load() }

function setGroup(): void { void load() }

/** 按类聚合时可以直接带类名跳引用追踪；按包聚合没有单一目标，交给「最大来源类」 */
function track(row: ConsumerRow): void {
  const target = row.kind === 'package' ? row.top_class : row.name
  if (!target) return
  store.openRefs(props.dump.id, { kind: 'class', query: target })
}

onMounted(() => { void load() })

// KeepAlive 复用实例：换 dump 时丢弃旧结果重查（不同文件的支配树完全不同）
watch(() => props.dump.id, () => {
  state.value = { loading: false, error: '', data: null }
  void load()
})
</script>

<style scoped lang="scss">
.top-consumers {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.tc-error {
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

.tc-loading {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}

.tc-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
  flex-wrap: wrap;
}

.tc-limit { width: 96px; }

.tc-meta,
.tc-cached,
.tc-elapsed {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.tc-cached { color: var(--stateSuccessFg); }

.tc-preset {
  padding: 8px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  background: var(--el-fill-color-lighter);
  flex-shrink: 0;
}

.tc-preset-head {
  display: flex;
  align-items: baseline;
  gap: 10px;
  margin-bottom: 6px;
  flex-wrap: wrap;
  font-size: var(--font-size-xs);
}

.tc-bars {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.tc-bar-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--font-size-xs);
}

.tc-bar-name {
  width: 240px;
  flex-shrink: 0;
  font-family: var(--font-family-mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--el-text-color-regular);
}

.tc-bar-track {
  flex: 1;
  height: 10px;
  min-width: 40px;
  background: var(--el-fill-color);
  border-radius: var(--radius-xs);
  overflow: hidden;
}

.tc-bar-fill {
  display: block;
  height: 100%;
  background: var(--el-color-primary-light-5);
  border-radius: var(--radius-xs);
}

.tc-bar-pct {
  width: 62px;
  text-align: right;
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
  color: var(--el-text-color-secondary);
}

.tc-note {
  padding: 6px 12px;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}

.tc-table { flex: 1; min-height: 0; }

.tc-mono { font-family: var(--font-family-mono); }
.tc-strong { font-weight: 600; font-variant-numeric: tabular-nums; }

.tc-pct {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  font-size: var(--font-size-xs);
  font-variant-numeric: tabular-nums;
}

.tc-pct-bar {
  position: absolute;
  right: 0;
  top: 50%;
  height: 12px;
  transform: translateY(-50%);
  background: var(--el-color-primary-light-7);
  opacity: 0.35;
  border-radius: var(--radius-xs);
}

.tc-pct span { position: relative; }

.tc-empty {
  padding: 16px;
  text-align: center;
  color: var(--el-text-color-secondary);
  font-size: var(--font-size-sm);
}

.tc-footer {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 12px;
  border-top: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}

.tc-elapsed { margin-left: auto; }
</style>
