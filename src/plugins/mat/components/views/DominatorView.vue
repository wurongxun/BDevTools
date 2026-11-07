<template>
  <div class="dominator">
    <div v-if="state.error" class="dm-error">
      <el-icon><WarningFilled /></el-icon>
      <span>{{ state.error }}</span>
      <app-button size="small" text @click="reload">
        <el-icon><Refresh /></el-icon>
        <span>重试</span>
      </app-button>
    </div>

    <div v-else-if="state.loading" v-loading="true" class="dm-loading">
      <WorkerProgress
        v-if="worker && !worker.finished"
        :dump-id="dump.id"
        hint="分析跑在独立子进程里，主窗口可以正常操作；完成后本页自动显示结果。"
        counters
      />
      <span v-else>首次计算支配树可能需要数十秒（与泄漏检测共用结果，之后为毫秒级）</span>
    </div>

    <template v-else-if="state.data">
      <div class="dm-toolbar">
        <el-radio-group v-model="groupBy" size="small" @change="setGroup">
          <el-radio-button value="object">对象</el-radio-button>
          <el-radio-button value="class">按类</el-radio-button>
          <el-radio-button value="package">按包</el-radio-button>
        </el-radio-group>
        <el-input
          v-model="query"
          size="small"
          clearable
          class="dm-search"
          placeholder="筛选类名（回车应用）"
          aria-label="筛选类名"
          @keyup.enter="applyQuery"
          @clear="applyQuery"
        >
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <span class="dm-meta">
          可达 {{ formatInt(state.data.reach_nodes) }} 对象 ·
          浅堆 {{ formatBytes(state.data.reach_shallow) }} ·
          共 {{ formatInt(state.data.total) }} 行
        </span>
        <span v-if="state.data.cached" class="dm-cached">缓存命中</span>
        <app-button size="small" text :aria-label="'刷新支配树'" @click="reload">
          <el-icon><Refresh /></el-icon>
          <span>刷新</span>
        </app-button>
        <ExportButton
          :rows="state.data.rows"
          :columns="exportColumns"
          :filename="`${dump.name}-dominator-${groupBy}.csv`"
          label="CSV"
        />
      </div>

      <div v-if="crumbs.length" class="dm-crumbs">
        <app-button size="small" text :aria-label="'返回顶层'" @click="backTop">顶层</app-button>
        <template v-for="(c, i) in crumbs" :key="c.objId">
          <span class="dm-sep">/</span>
          <app-button size="small" text :aria-label="`回到 ${c.className}`" @click="goCrumb(i)">
            {{ c.className }}
          </app-button>
        </template>
      </div>

      <div v-if="state.data.truncated" class="dm-note warn">
        {{ state.data.note || '支配树不可用' }}
        <app-button size="small" text @click="store.openSettings()">调整分析预算</app-button>
      </div>
      <div v-else-if="state.data.note" class="dm-note">{{ state.data.note }}</div>

      <div ref="tableWrap" class="dm-table">
        <el-table
          v-if="tableH > 0"
          :data="state.data.rows"
          :height="tableH"
          size="small"
          row-key="obj_id"
          border
          stripe
        >
          <el-table-column type="index" label="#" width="56" align="right" />
          <el-table-column prop="class_name" label="Class Name" min-width="240" show-overflow-tooltip>
            <template #default="{ row }">
              <span class="dm-mono">{{ row.class_name }}</span>
              <span v-if="row.is_root" class="dm-root-tag">GC Root</span>
            </template>
          </el-table-column>
          <el-table-column label="Object ID" width="160" show-overflow-tooltip>
            <template #default="{ row }"><span class="dm-mono">{{ row.obj_id || '—' }}</span></template>
          </el-table-column>
          <el-table-column label="Shallow" width="110" align="right">
            <template #default="{ row }"><span class="dm-num">{{ formatBytes(row.shallow_bytes) }}</span></template>
          </el-table-column>
          <el-table-column label="Retained" width="120" align="right" sortable prop="retained_bytes">
            <template #default="{ row }"><span class="dm-num strong">{{ formatBytes(row.retained_bytes) }}</span></template>
          </el-table-column>
          <el-table-column label="占比" width="150" align="right">
            <template #default="{ row }">
              <div class="dm-pct">
                <div class="dm-pct-bar" :style="{ width: `${Math.min(100, row.pct)}%` }"></div>
                <span>{{ row.pct.toFixed(2) }}%</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column label="支配对象" width="100" align="right" prop="retained_objects" />
          <el-table-column label="孩子" width="76" align="right" prop="children" />
          <el-table-column label="操作" width="210" fixed="right" align="center">
            <template #default="{ row }">
              <app-button
                v-if="groupBy === 'object' && row.obj_id && row.children > 0"
                size="small"
                text
                :aria-label="`展开 ${row.class_name} 的支配子树`"
                @click="expand(row)"
              >展开</app-button>
              <app-button
                v-if="groupBy === 'object' && row.obj_id"
                size="small"
                text
                :aria-label="`查看 ${row.class_name} 的保留集`"
                @click="openRetained(row)"
              >保留集</app-button>
              <app-button
                v-if="groupBy === 'object' && row.obj_id"
                size="small"
                text
                :aria-label="`在引用追踪中查看 ${row.class_name}`"
                @click="track(row)"
              >追踪</app-button>
            </template>
          </el-table-column>
          <template #empty><div class="dm-empty">无匹配对象</div></template>
        </el-table>
      </div>

      <div class="dm-footer">
        <app-button size="small" text :disabled="page === 0" @click="goPage(-1)">上一页</app-button>
        <span class="dm-page">第 {{ page + 1 }} / {{ totalPages }} 页 · 每页 {{ PAGE }} 行</span>
        <app-button size="small" text :disabled="page + 1 >= totalPages" @click="goPage(1)">下一页</app-button>
        <span class="dm-elapsed">{{ state.data.elapsed_ms }} ms</span>
      </div>

      <div v-if="retainedLoading" class="dm-retained loading">正在计算保留集…</div>
      <div v-else-if="retained" class="dm-retained">
        <div class="dm-retained-head">
          <b>保留集</b>
          <span class="dm-mono">{{ retained.obj_id }}</span>
          <span>{{ formatInt(retained.total) }} 对象 · {{ formatBytes(retained.bytes) }}</span>
          <span v-if="retained.note" class="dm-note-inline">{{ retained.note }}</span>
          <app-button size="small" text aria-label="关闭保留集" @click="retained = null">
            <el-icon><Close /></el-icon>
          </app-button>
        </div>
        <div v-if="retained.rows.length" class="dm-retained-list">
          <span v-for="r in retained.rows.slice(0, 12)" :key="r.obj_id" class="dm-chip" :title="`${r.class_name} · ${formatBytes(r.retained_bytes)}`">
            {{ r.class_name }} · {{ formatBytes(r.retained_bytes) }}
          </span>
          <span v-if="retained.rows.length > 12" class="dm-meta">… 共 {{ retained.rows.length }} 行</span>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Close, Refresh, Search, WarningFilled } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { useMatStore } from '../../store'
import WorkerProgress from '../WorkerProgress.vue'
import { fetchDomTree, fetchRetainedSet, formatBytes, formatInt } from '../../api/matApi'
import type { DomRow, DomTreeResult, RetainedSetResult } from '../../api/matApi'
import type { MatDump } from '../../types'
import type { ExportColumn } from '../../utils/export'
import ExportButton from '../ExportButton.vue'

/**
 * Dominator Tree：对象级 Retained 排序 / 按类 / 按包聚合 / 子树展开 / 保留集。
 * 后端把支配树缓存在会话里（与泄漏检测共用一次 LT），这里全部走服务端分页，
 * 每页最多 PAGE 行，不做前端全量物化。
 */
const props = defineProps<{ dump: MatDump }>()
const store = useMatStore()
/** 后台分析作业（有就显示进度，别让用户对着一个转圈猜） */
const worker = computed(() => store.workerJobOf(props.dump.id))

const PAGE = 200
type GroupBy = 'object' | 'class' | 'package'
const groupBy = ref<GroupBy>('object')
const query = ref('')
const page = ref(0)
const crumbs = ref<{ objId: string; className: string }[]>([])
const state = ref<{ loading: boolean; error: string; data: DomTreeResult | null }>({
  loading: false, error: '', data: null,
})
const retained = ref<RetainedSetResult | null>(null)
const retainedLoading = ref(false)

const totalPages = computed(() => Math.max(1, Math.ceil((state.value.data?.total ?? 0) / PAGE)))
const exportColumns: ExportColumn[] = [
  { key: 'obj_id', label: 'Object ID' },
  { key: 'class_name', label: 'Class Name' },
  { key: 'kind', label: 'Kind' },
  { key: 'shallow_bytes', label: 'Shallow (bytes)' },
  { key: 'retained_bytes', label: 'Retained (bytes)' },
  { key: 'retained_objects', label: 'Retained Objects' },
  { key: 'pct', label: 'Retained %' },
  { key: 'children', label: 'Dominator Children' },
]

async function load(): Promise<void> {
  state.value = { loading: true, error: '', data: state.value.data }
  try {
    // 支配树是最贵的一步：大 dump 先让后台进程算完再查，避免主进程冻结
    if ((await store.ensureAnalysisReady(props.dump.id, true)) === 'canceled') {
      throw new Error('后台分析已取消')
    }
    const data = await fetchDomTree(props.dump.path, {
      groupBy: groupBy.value,
      parentObjId: crumbs.value.length ? crumbs.value[crumbs.value.length - 1].objId : '',
      query: query.value,
      offset: page.value * PAGE,
      limit: PAGE,
    })
    state.value = { loading: false, error: '', data }
  } catch (e: any) {
    state.value = { loading: false, error: e?.message || String(e), data: null }
  }
}

function reload(): void { void load() }

function setGroup(): void {
  crumbs.value = []
  page.value = 0
  retained.value = null
  void load()
}

function applyQuery(): void {
  page.value = 0
  void load()
}

function goPage(delta: number): void {
  const next = page.value + delta
  if (next < 0 || next >= totalPages.value) return
  page.value = next
  retained.value = null
  void load()
}

function expand(row: DomRow): void {
  crumbs.value.push({ objId: row.obj_id, className: row.class_name })
  page.value = 0
  retained.value = null
  void load()
}

function goCrumb(i: number): void {
  crumbs.value = crumbs.value.slice(0, i + 1)
  page.value = 0
  retained.value = null
  void load()
}

function backTop(): void {
  crumbs.value = []
  page.value = 0
  retained.value = null
  void load()
}

function track(row: DomRow): void {
  store.openRefs(props.dump.id, { kind: 'object', objId: row.obj_id, className: row.class_name })
}

async function openRetained(row: DomRow): Promise<void> {
  retained.value = null
  retainedLoading.value = true
  try {
    retained.value = await fetchRetainedSet(props.dump.path, row.obj_id, 200)
  } catch (e: any) {
    ElMessage.error('保留集查询失败：' + (e?.message || String(e)))
  } finally {
    retainedLoading.value = false
  }
}

/** el-table 的像素 height 是硬前提：容器 flex:1，尺寸由 ResizeObserver 回填 */
const tableWrap = ref<HTMLElement | null>(null)
const tableH = ref(0)
let ro: ResizeObserver | null = null
function measure(): void {
  const el = tableWrap.value
  if (!el) return
  tableH.value = Math.max(120, Math.floor(el.getBoundingClientRect().height))
}

onMounted(() => {
  void load()
  measure()
  ro = new ResizeObserver(measure)
  if (tableWrap.value) ro.observe(tableWrap.value)
})
onBeforeUnmount(() => { ro?.disconnect(); ro = null })

// KeepAlive 下同一组件实例会被复用给不同 dump（顶层 tab 切换）
watch(() => props.dump.id, () => {
  crumbs.value = []
  page.value = 0
  retained.value = null
  query.value = ''
  void load()
})
</script>

<style scoped lang="scss">
.dominator {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.dm-error {
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

.dm-loading {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}

.dm-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
  flex-wrap: wrap;
}

.dm-search { width: 220px; }

.dm-meta, .dm-cached, .dm-page, .dm-elapsed {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.dm-cached { color: var(--stateSuccessFg); }
.dm-elapsed { margin-left: auto; }

.dm-crumbs {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 4px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
  flex-wrap: wrap;
}

.dm-sep { color: var(--el-text-color-placeholder); }

.dm-note {
  padding: 6px 12px;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;

  &.warn { color: var(--stateWarningFg); }
}

.dm-table { flex: 1; min-height: 0; position: relative; }

.dm-mono { font-family: var(--font-family-mono); }

.dm-root-tag {
  margin-left: 6px;
  padding: 0 4px;
  font-size: var(--font-size-xs);
  color: var(--stateInfoFg);
  border: 1px solid var(--el-color-info-light-5);
  border-radius: var(--radius-xs);
}

.dm-num { font-variant-numeric: tabular-nums; }
.dm-num.strong { font-weight: 600; }

.dm-pct {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  font-size: var(--font-size-xs);
  font-variant-numeric: tabular-nums;
}

.dm-pct-bar {
  position: absolute;
  right: 0;
  top: 50%;
  height: 12px;
  transform: translateY(-50%);
  background: var(--el-color-primary-light-7);
  opacity: 0.35;
  border-radius: var(--radius-xs);
}

.dm-pct span { position: relative; }

.dm-empty {
  padding: 16px;
  text-align: center;
  color: var(--el-text-color-secondary);
  font-size: var(--font-size-sm);
}

.dm-footer {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 12px;
  border-top: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}

.dm-retained {
  max-height: 132px;
  overflow: auto;
  padding: 6px 12px;
  border-top: 1px solid var(--el-border-color-lighter);
  background: var(--el-fill-color-light);
  flex-shrink: 0;
  font-size: var(--font-size-xs);

  &.loading { color: var(--el-text-color-secondary); }
}

.dm-retained-head {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.dm-note-inline { color: var(--stateWarningFg); }

.dm-retained-list {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  margin-top: 4px;
}

.dm-chip {
  padding: 1px 6px;
  border: 1px solid var(--el-border-color);
  border-radius: var(--radius-xs);
  font-family: var(--font-family-mono);
  max-width: 280px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
