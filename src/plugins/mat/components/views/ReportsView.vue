<template>
  <div class="reports">
    <el-tabs v-model="active" class="rp-tabs" @tab-change="onTabChange">
      <!-- ==================== 线程 ==================== -->
      <el-tab-pane name="threads" label="线程">
        <div class="rp-pane">
          <div class="rp-toolbar">
            <span class="rp-meta">
              共 <b class="mono">{{ formatInt(th.data?.total ?? 0) }}</b> 个线程对象 ·
              线程根 <b class="mono">{{ formatInt(th.data?.thread_roots ?? 0) }}</b>
              <template v-if="th.data">
                · 已列出 {{ th.data.rows.length }}
              </template>
            </span>
            <el-input
              v-model="thQuery"
              size="small"
              clearable
              class="rp-search"
              placeholder="筛选线程名 / 类名"
              aria-label="筛选线程名或类名"
            >
              <template #prefix><el-icon><Search /></el-icon></template>
            </el-input>
            <app-button size="small" text aria-label="刷新线程报告" @click="loadThreads(true)">
              <el-icon><Refresh /></el-icon>
              <span>刷新</span>
            </app-button>
            <ExportButton
              :rows="thRows"
              :columns="threadColumns"
              :filename="`${dump.name}-threads.csv`"
              label="CSV"
            />
          </div>
          <div v-if="th.error" class="rp-error"><el-icon><WarningFilled /></el-icon><span>{{ th.error }}</span></div>
          <div v-else-if="th.data?.note" class="rp-note">{{ th.data.note }}</div>
          <div class="rp-table">
            <el-table :data="thRows" height="100%" size="small" border stripe v-loading="th.loading">
              <el-table-column type="index" label="#" width="56" align="right" />
              <el-table-column label="线程名" min-width="240" show-overflow-tooltip>
                <template #default="{ row }">
                  <span>{{ row.name }}</span>
                  <span v-if="row.is_root" class="rp-tag">GC Root</span>
                </template>
              </el-table-column>
              <el-table-column label="类" min-width="220" show-overflow-tooltip>
                <template #default="{ row }"><span class="mono">{{ row.class_name }}</span></template>
              </el-table-column>
              <el-table-column label="Object ID" width="150" show-overflow-tooltip>
                <template #default="{ row }"><span class="mono">{{ row.obj_id }}</span></template>
              </el-table-column>
              <el-table-column label="Shallow" width="100" align="right">
                <template #default="{ row }">{{ formatBytes(row.shallow_bytes) }}</template>
              </el-table-column>
              <el-table-column label="状态位" width="90" align="right" prop="thread_status">
                <template #default="{ row }">
                  <span class="mono" title="JVM 内部 threadStatus 字段原值，未映射为 Thread.State（避免给出错误语义）">
                    {{ row.thread_status }}
                  </span>
                </template>
              </el-table-column>
              <el-table-column label="操作" width="90" fixed="right" align="center">
                <template #default="{ row }">
                  <app-button size="small" text aria-label="在引用追踪中查看该线程" @click="track(row.obj_id, row.class_name)">追踪</app-button>
                </template>
              </el-table-column>
              <template #empty><div class="rp-empty">没有线程对象（该 dump 可能不含完整堆数据）</div></template>
            </el-table>
          </div>
        </div>
      </el-tab-pane>

      <!-- ==================== 类加载器 ==================== -->
      <el-tab-pane name="loaders" label="类加载器">
        <div class="rp-pane">
          <div class="rp-toolbar">
            <span class="rp-meta">
              共 <b class="mono">{{ formatInt(cl.data?.total_loaders ?? 0) }}</b> 个加载器 ·
              已加载 <b class="mono">{{ formatInt(cl.data?.total_classes ?? 0) }}</b> 个类
              <template v-if="cl.data?.duplicates?.length">
                · <span class="warn">同名类 {{ cl.data!.duplicates.length }} 组</span>
              </template>
            </span>
            <el-input
              v-model="clQuery"
              size="small"
              clearable
              class="rp-search"
              placeholder="筛选加载器类名"
              aria-label="筛选加载器类名"
            >
              <template #prefix><el-icon><Search /></el-icon></template>
            </el-input>
            <app-button size="small" text aria-label="刷新类加载器报告" @click="loadLoaders(true)">
              <el-icon><Refresh /></el-icon>
              <span>刷新</span>
            </app-button>
            <ExportButton
              :rows="clRows"
              :columns="loaderColumns"
              :filename="`${dump.name}-classloaders.csv`"
              label="CSV"
            />
          </div>
          <div v-if="cl.error" class="rp-error"><el-icon><WarningFilled /></el-icon><span>{{ cl.error }}</span></div>
          <div v-else-if="cl.data?.note" class="rp-note">{{ cl.data.note }}</div>
          <div class="rp-table">
            <el-table :data="clRows" height="100%" size="small" border stripe v-loading="cl.loading">
              <el-table-column type="index" label="#" width="56" align="right" />
              <el-table-column label="加载器类" min-width="240" show-overflow-tooltip>
                <template #default="{ row }">
                  <span class="mono">{{ row.loader_class || '(bootstrap / 未归属)' }}</span>
                </template>
              </el-table-column>
              <el-table-column label="加载器对象" width="150" show-overflow-tooltip>
                <template #default="{ row }">
                  <span class="mono">{{ row.loader_obj_id || '—' }}</span>
                  <span v-if="row.loader_obj_id" class="rp-link" title="在引用追踪中查看" @click="track(row.loader_obj_id, row.loader_class)">追踪</span>
                </template>
              </el-table-column>
              <el-table-column label="类数" width="90" align="right" prop="classes" sortable />
              <el-table-column label="同名重复" width="100" align="right" prop="duplicate_classes" />
              <el-table-column label="示例类" min-width="300" show-overflow-tooltip>
                <template #default="{ row }">{{ row.sample.join(', ') || '—' }}</template>
              </el-table-column>
              <template #empty><div class="rp-empty">没有类加载器数据</div></template>
            </el-table>
          </div>
          <div v-if="cl.data?.duplicates?.length" class="rp-dup">
            <div class="rp-dup-head">
              <b>同名类重复加载</b>
              <span class="rp-meta">同名不同 loader ⇒ 常见于热部署 / 多版本依赖冲突</span>
            </div>
            <el-table :data="cl.data.duplicates" height="140" size="small" border>
              <el-table-column prop="simple_name" label="类名" min-width="240" show-overflow-tooltip />
              <el-table-column prop="loaders" label="加载器数" width="100" align="right" />
              <el-table-column label="出现位置" min-width="320" show-overflow-tooltip>
                <template #default="{ row }"><span class="mono">{{ row.occurrences.join(' · ') }}</span></template>
              </el-table-column>
            </el-table>
          </div>
        </div>
      </el-tab-pane>

      <!-- ==================== 内存浪费 ==================== -->
      <el-tab-pane name="waste" label="内存浪费">
        <div class="rp-pane">
          <div class="rp-toolbar">
            <span class="rp-meta">
              重复字符串浪费 <b class="mono warn">{{ formatBytes(wa.data?.duplicate_wasted_bytes ?? 0) }}</b>
              · 重复组 <b class="mono">{{ formatInt(wa.data?.duplicate_groups ?? 0) }}</b>
              · 扫描字符串 <b class="mono">{{ formatInt(wa.data?.strings_scanned ?? 0) }}</b>
              · 空集合 <b class="mono">{{ formatInt(wa.data?.empty_collections ?? 0) }}</b>
              <span v-if="wa.data?.sampled" class="rp-tag sampled">采样值</span>
            </span>
            <app-button size="small" text aria-label="刷新内存浪费报告" @click="loadWaste(true)">
              <el-icon><Refresh /></el-icon>
              <span>刷新</span>
            </app-button>
            <ExportButton
              :rows="wa.data?.duplicate_strings ?? []"
              :columns="dupStringColumns"
              :filename="`${dump.name}-waste-strings.csv`"
              label="重复字符串"
            />
            <ExportButton
              :rows="wa.data?.collections ?? []"
              :columns="collectionColumns"
              :filename="`${dump.name}-waste-collections.csv`"
              label="集合填充"
            />
          </div>
          <div v-if="wa.error" class="rp-error"><el-icon><WarningFilled /></el-icon><span>{{ wa.error }}</span></div>
          <div v-else-if="wa.data?.note" class="rp-note">{{ wa.data.note }}</div>
          <div class="rp-split" v-loading="wa.loading">
            <div class="rp-section-title">重复字符串（等值但不同对象，可通过 interning 回收）</div>
            <div class="rp-table">
              <el-table :data="wa.data?.duplicate_strings ?? []" height="100%" size="small" border stripe>
                <el-table-column type="index" label="#" width="56" align="right" />
                <el-table-column label="值" min-width="320" show-overflow-tooltip>
                  <template #default="{ row }"><span class="mono">{{ row.value }}</span></template>
                </el-table-column>
                <el-table-column label="份数" width="80" align="right" prop="count" />
                <el-table-column label="单份浅堆" width="110" align="right">
                  <template #default="{ row }">{{ formatBytes(row.shallow_bytes) }}</template>
                </el-table-column>
                <el-table-column label="浪费" width="110" align="right" prop="wasted_bytes" sortable>
                  <template #default="{ row }"><span class="warn">{{ formatBytes(row.wasted_bytes) }}</span></template>
                </el-table-column>
                <template #empty><div class="rp-empty">没有重复字符串</div></template>
              </el-table>
            </div>
          </div>
          <div class="rp-split">
            <div class="rp-section-title">集合填充率（平均元素数偏低 ⇒ 容量预估过大或大量空集合）</div>
            <div class="rp-table">
              <el-table :data="wa.data?.collections ?? []" height="100%" size="small" border stripe>
                <el-table-column label="集合类" min-width="240" show-overflow-tooltip>
                  <template #default="{ row }"><span class="mono">{{ row.class_name }}</span></template>
                </el-table-column>
                <el-table-column label="实例数" width="90" align="right" prop="instances" />
                <el-table-column label="空实例" width="90" align="right" prop="empty_instances" />
                <el-table-column label="平均元素" width="100" align="right">
                  <template #default="{ row }">{{ row.avg_fill.toFixed(2) }}</template>
                </el-table-column>
                <el-table-column label="最大元素" width="100" align="right" prop="max_size" />
                <el-table-column label="浅堆合计" width="110" align="right">
                  <template #default="{ row }">{{ formatBytes(row.shallow_bytes) }}</template>
                </el-table-column>
                <template #empty><div class="rp-empty">没有集合实例</div></template>
              </el-table>
            </div>
          </div>
        </div>
      </el-tab-pane>

      <!-- ==================== 不可达对象 ==================== -->
      <el-tab-pane name="unreachable" label="不可达对象">
        <div class="rp-pane">
          <div class="rp-toolbar">
            <span class="rp-meta">
              不可达 <b class="mono warn">{{ formatInt(un.data?.total_objects ?? 0) }}</b> 个对象 ·
              浅堆 <b class="mono warn">{{ formatBytes(un.data?.total_shallow ?? 0) }}</b> ·
              可达 <b class="mono">{{ formatInt(un.data?.reachable_nodes ?? 0) }}</b>
            </span>
            <app-button size="small" text aria-label="刷新不可达对象报告" @click="loadUnreachable(true)">
              <el-icon><Refresh /></el-icon>
              <span>刷新</span>
            </app-button>
            <ExportButton
              :rows="un.data?.rows ?? []"
              :columns="unreachableColumns"
              :filename="`${dump.name}-unreachable.csv`"
              label="CSV"
            />
          </div>
          <div v-if="un.error" class="rp-error"><el-icon><WarningFilled /></el-icon><span>{{ un.error }}</span></div>
          <div v-else-if="un.data?.note" class="rp-note">{{ un.data.note }}</div>
          <div class="rp-table">
            <el-table :data="un.data?.rows ?? []" height="100%" size="small" border stripe v-loading="un.loading">
              <el-table-column type="index" label="#" width="56" align="right" />
              <el-table-column label="类" min-width="320" show-overflow-tooltip>
                <template #default="{ row }"><span class="mono">{{ row.class_name }}</span></template>
              </el-table-column>
              <el-table-column label="对象数" width="110" align="right" prop="count" sortable />
              <el-table-column label="浅堆" width="120" align="right" prop="shallow_bytes" sortable>
                <template #default="{ row }"><span class="warn">{{ formatBytes(row.shallow_bytes) }}</span></template>
              </el-table-column>
              <template #empty><div class="rp-empty">没有不可达对象（或该 dump 已被 GC 清理）</div></template>
            </el-table>
          </div>
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { Refresh, Search, WarningFilled } from '@element-plus/icons-vue'
import { useMatStore } from '../../store'
import {
  fetchClassLoaders,
  fetchThreads,
  fetchUnreachable,
  fetchWasteReport,
  formatBytes,
  formatInt,
} from '../../api/matApi'
import type {
  ClassLoadersResult,
  ThreadsResult,
  UnreachableResult,
  WasteResult,
} from '../../api/matApi'
import type { MatDump } from '../../types'
import type { ExportColumn } from '../../utils/export'
import ExportButton from '../ExportButton.vue'

/**
 * 专用报告合集：线程 / 类加载器 / 内存浪费 / 不可达对象。
 *
 * 四个报告各自独立、成本不同（线程与浪费要扫文件，不可达复用引用会话），
 * 因此**按子 tab 惰性加载**：切到哪个才发哪个请求，切走不重复拉（除非点刷新）。
 * 这是与「泄漏检测」一次性加载的区别 —— 没人会同时看四份报告。
 */
type ReportKey = 'threads' | 'loaders' | 'waste' | 'unreachable'

const props = defineProps<{ dump: MatDump }>()
const store = useMatStore()

const active = ref<ReportKey>('threads')

interface PaneState<T> {
  loading: boolean
  error: string
  data: T | null
}
const th = reactive<PaneState<ThreadsResult>>({ loading: false, error: '', data: null })
const cl = reactive<PaneState<ClassLoadersResult>>({ loading: false, error: '', data: null })
const wa = reactive<PaneState<WasteResult>>({ loading: false, error: '', data: null })
const un = reactive<PaneState<UnreachableResult>>({ loading: false, error: '', data: null })

const thQuery = ref('')
const clQuery = ref('')

const thRows = computed(() => {
  const all = th.data?.rows ?? []
  const q = thQuery.value.trim().toLowerCase()
  if (!q) return all
  return all.filter(
    r => r.name.toLowerCase().includes(q) || r.class_name.toLowerCase().includes(q),
  )
})

const clRows = computed(() => {
  const all = cl.data?.rows ?? []
  const q = clQuery.value.trim().toLowerCase()
  if (!q) return all
  return all.filter(r => (r.loader_class || '').toLowerCase().includes(q))
})

const threadColumns: ExportColumn[] = [
  { key: 'name', label: 'Thread Name' },
  { key: 'class_name', label: 'Class' },
  { key: 'obj_id', label: 'Object ID' },
  { key: 'shallow_bytes', label: 'Shallow (bytes)' },
  { key: 'thread_status', label: 'threadStatus' },
  { key: 'is_root', label: 'GC Root' },
]
const loaderColumns: ExportColumn[] = [
  { key: 'loader_class', label: 'Loader Class' },
  { key: 'loader_obj_id', label: 'Loader Object ID' },
  { key: 'classes', label: 'Classes' },
  { key: 'duplicate_classes', label: 'Duplicate Classes' },
  { key: 'sample', label: 'Sample Classes' },
]
const dupStringColumns: ExportColumn[] = [
  { key: 'value', label: 'Value' },
  { key: 'count', label: 'Copies' },
  { key: 'shallow_bytes', label: 'Shallow (bytes)' },
  { key: 'wasted_bytes', label: 'Wasted (bytes)' },
]
const collectionColumns: ExportColumn[] = [
  { key: 'class_name', label: 'Collection Class' },
  { key: 'instances', label: 'Instances' },
  { key: 'empty_instances', label: 'Empty' },
  { key: 'avg_fill', label: 'Avg Size' },
  { key: 'max_size', label: 'Max Size' },
  { key: 'shallow_bytes', label: 'Shallow (bytes)' },
]
const unreachableColumns: ExportColumn[] = [
  { key: 'class_name', label: 'Class' },
  { key: 'count', label: 'Objects' },
  { key: 'shallow_bytes', label: 'Shallow (bytes)' },
]

/** 通用装载骨架：只置位自己那一份状态，四份报告互不影响 */
async function loadInto<T>(
  st: PaneState<T>,
  force: boolean,
  load: () => Promise<T>,
  label: string,
  /** 需要对象图 / 支配树的报告先等后台进程（大 dump 否则会在主进程里现算） */
  gate?: (id: string, needDom: boolean) => Promise<unknown>,
): Promise<void> {
  if (st.loading) return
  if (st.data && !force) return
  st.loading = true
  st.error = ''
  try {
    if (gate) await gate(props.dump.id, false)
    st.data = await load()
  } catch (e: any) {
    st.error = `${label}失败：` + (e?.message || String(e))
  } finally {
    st.loading = false
  }
}

function loadThreads(force = false): void {
  void loadInto(th, force, () => fetchThreads(props.dump.path, 500), '线程报告')
}
function loadLoaders(force = false): void {
  void loadInto(cl, force, () => fetchClassLoaders(props.dump.path, 500), '类加载器报告')
}
function loadWaste(force = false): void {
  void loadInto(wa, force, () => fetchWasteReport(props.dump.path, 200), '内存浪费报告')
}
function loadUnreachable(force = false): void {
  // 不可达查询按「可达标记」筛对象 ⇒ 要对象图（不需要支配树）
  void loadInto(
    un,
    force,
    () => fetchUnreachable(props.dump.path, 200),
    '不可达对象查询',
    store.ensureAnalysisReady,
  )
}

const LOADERS: Record<ReportKey, (force?: boolean) => void> = {
  threads: loadThreads,
  loaders: loadLoaders,
  waste: loadWaste,
  unreachable: loadUnreachable,
}

/** 只加载当前激活的那一份 */
function ensureActive(): void {
  LOADERS[active.value]()
}

function onTabChange(name: string | number): void {
  active.value = name as ReportKey
  ensureActive()
}

function track(objId: string, className: string): void {
  if (!objId) return
  store.openRefs(props.dump.id, { kind: 'object', objId, className })
}

/** dump 变化：清空四份结果（不同文件数据完全不同），只重查当前激活的 */
function resetAll(): void {
  for (const st of [th, cl, wa, un]) {
    st.data = null
    st.error = ''
    st.loading = false
  }
  thQuery.value = ''
  clQuery.value = ''
}

onMounted(ensureActive)

watch(() => props.dump.id, () => {
  resetAll()
  ensureActive()
})
</script>

<style scoped lang="scss">
.reports {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.rp-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.rp-tabs :deep(.el-tabs__header) {
  margin: 0;
  padding: 0 12px;
  flex-shrink: 0;
}

.rp-tabs :deep(.el-tabs__content) {
  flex: 1;
  min-height: 0;
  padding: 0;
}

.rp-tabs :deep(.el-tab-pane) {
  height: 100%;
}

.rp-pane {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.rp-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
  flex-wrap: wrap;
}

.rp-search { width: 200px; }

.rp-meta {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.rp-meta .warn,
.warn { color: var(--stateWarningFg); }

.rp-error {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  font-size: var(--font-size-sm);
  color: var(--stateErrorFg);
  flex-shrink: 0;
}

.rp-note {
  padding: 6px 12px;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}

.rp-table {
  flex: 1;
  min-height: 0;
}

/** 上方标题 + 下方表格的分段容器（两段等分，表格高度由 .rp-table 的 flex 决定） */
.rp-split {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;

  & + & { border-top: 1px solid var(--el-border-color-lighter); }
}

.rp-section-title {
  padding: 4px 12px;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-lighter);
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}

.rp-dup {
  flex-shrink: 0;
  border-top: 1px solid var(--el-border-color-lighter);
}

.rp-dup-head {
  display: flex;
  align-items: baseline;
  gap: 10px;
  padding: 6px 12px;
  font-size: var(--font-size-xs);
  background: var(--el-fill-color-lighter);
  flex-wrap: wrap;
}

.rp-tag {
  margin-left: 6px;
  padding: 0 4px;
  font-size: var(--font-size-xs);
  color: var(--stateInfoFg);
  border: 1px solid var(--el-color-info-light-5);
  border-radius: var(--radius-xs);

  &.sampled {
    color: var(--stateWarningFg);
    border-color: var(--el-color-warning-light-5);
  }
}

.rp-link {
  margin-left: 6px;
  font-size: var(--font-size-xs);
  color: var(--el-color-primary);
  cursor: pointer;

  &:hover { text-decoration: underline; }
}

.mono { font-family: var(--font-family-mono); }

.rp-empty {
  padding: 16px;
  text-align: center;
  color: var(--el-text-color-secondary);
  font-size: var(--font-size-sm);
}
</style>
