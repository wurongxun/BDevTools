<template>
  <div class="rf">
    <!-- 会话构建失败 -->
    <div v-if="buildState.error" class="rf-error">
      <el-icon><WarningFilled /></el-icon>
      <span>{{ buildState.error }}</span>
      <app-button size="small" text @click="store.rebuildRefs(props.dump.id)">
        <el-icon><Refresh /></el-icon>
      </app-button>
    </div>

    <!-- 首次构建（大 dump 较慢，明确提示） -->
    <div v-else-if="buildState.loading" v-loading="true" class="rf-loading">
      <div class="rf-loading-tip">正在解析对象引用图并构建索引（大 dump 首次需数十秒，之后查询为毫秒级）…</div>
    </div>

    <template v-else-if="buildInfo">
      <!-- 规模超限 -->
      <div v-if="buildInfo.truncated" class="rf-notes">
        <el-icon class="rf-notes-ic"><InfoFilled /></el-icon>
        <div class="rf-notes-body">
          <div v-for="(n, i) in buildInfo.notes" :key="i" class="rf-notes-line">{{ n }}</div>
          <app-button size="small" type="primary" plain class="rf-notes-act" @click="store.openSettings()">
            调整分析预算并重试
          </app-button>
        </div>
      </div>

      <template v-else>
        <!-- 工具条 -->
        <div class="rf-toolbar">
          <span class="rf-title">对象引用追踪</span>
          <span class="rf-meta">
            {{ formatInt(buildInfo.reachable_nodes) }} 可达对象 · {{ formatInt(buildInfo.classes) }} 类 · {{ formatInt(buildInfo.edge_count) }} 引用边
          </span>
          <el-tag v-if="buildInfo.cached" size="small" type="success" effect="plain">会话缓存命中</el-tag>
          <span class="rf-elapsed">索引构建 {{ buildInfo.elapsed_ms }} ms</span>
        </div>

        <!-- 主区：左（搜索 → 实例样本）右（对象详情） -->
        <div class="rf-body">
          <aside class="rf-left">
            <div class="rf-left-head">
              <el-input
                v-model="keyword"
                size="small"
                clearable
                placeholder="搜索类名，如 byte[] / HashMap / Demo"
                aria-label="搜索类名"
                @keyup.enter="doSearch"
                @clear="doSearch"
              >
                <template #prefix><el-icon><Search /></el-icon></template>
              </el-input>
              <app-button size="small" @click="doSearch">搜索</app-button>
            </div>

            <div class="rf-left-sub">
              <template v-if="mode === 'classes'">
                <span v-if="classState.loading">搜索中…</span>
                <span v-else-if="classHits.length">{{ classHits.length }} 个匹配类</span>
                <span v-else>输入关键字搜索堆内类（无实例的类不在此列）</span>
              </template>
              <template v-else>
                <app-button size="small" text class="rf-back" @click="backToClasses">
                  <el-icon><Back /></el-icon>返回类列表
                </app-button>
                <span class="rf-sub-meta">{{ curClassName }} · {{ inst?.total ? formatInt(inst.total) : 0 }} 个实例</span>
              </template>
            </div>

            <!-- 类列表 / 实例列表 -->
            <div v-loading="classState.loading || instLoading" class="rf-left-list">
              <!-- classes -->
              <template v-if="mode === 'classes'">
                <div v-if="classState.error" class="rf-list-err">{{ classState.error }}</div>
                <button
                  v-for="hit in classHits"
                  :key="hit.class_name"
                  type="button"
                  class="rf-class"
                  @click="selectClass(hit.class_name)"
                >
                  <span class="rf-class-name mono" :title="hit.class_name">{{ hit.class_name }}</span>
                  <span class="rf-class-nums">
                    <span class="mono">{{ formatInt(hit.count) }}</span> 对象 · <b>{{ formatBytes(hit.shallow_bytes) }}</b>
                  </span>
                </button>
                <div v-if="!classHits.length && !classState.loading" class="rf-list-empty">
                  <el-icon><Search /></el-icon>
                  <span>按类名搜索实例（支持子串，忽略大小写；按浅堆合计降序）</span>
                </div>
              </template>

              <!-- instances -->
              <template v-else>
                <div v-if="instErr" class="rf-list-err">{{ instErr }}</div>
                <div v-else-if="inst && inst.total === 0" class="rf-list-empty">
                  <el-icon><InfoFilled /></el-icon><span>该类没有实例（纯静态持有 / 数组类等场景请查泄漏检测）</span>
                </div>
                <button
                  v-for="row in inst?.rows ?? []"
                  :key="row.obj_id"
                  type="button"
                  class="rf-inst"
                  :class="{ active: isTraceTail(row.obj_id) }"
                  @click="openObj(row.obj_id, `${curClassName} @ ${row.obj_id}`)"
                >
                  <span class="mono rf-inst-id">{{ row.obj_id }}</span>
                  <span class="rf-inst-shallow mono">{{ formatBytes(row.shallow_bytes) }}</span>
                </button>
              </template>
            </div>
          </aside>

          <!-- 右侧详情 -->
          <section class="rf-right">
            <div v-if="!focusObjId" class="rf-empty">
              <el-icon class="rf-empty-ic"><Position /></el-icon>
              <div class="rf-empty-main">
                <b>选择一个对象查看引用详情</b>
                <span>左侧选中实例，或点击任意引用对象继续向下追踪；路径显示「GC Root → 对象」最短引用链与各跳引用点。</span>
              </div>
            </div>

            <template v-else>
              <!-- 追踪面包屑 -->
              <div class="rf-trace">
                <el-icon class="rf-trace-ic"><Link /></el-icon>
                <template v-for="(t, i) in trace" :key="i">
                  <button
                    v-if="i"
                    type="button"
                    class="rf-trace-sep"
                    :disabled="i === trace.length - 1"
                    @click="goTrace(i)"
                  >
                    <el-icon><Right /></el-icon>
                  </button>
                  <button
                    type="button"
                    class="rf-trace-node mono"
                    :class="{ active: i === trace.length - 1 }"
                    :title="t.label"
                    @click="goTrace(i)"
                  >
                    {{ t.short }}
                  </button>
                </template>
                <span v-if="detail?.elapsed_ms != null" class="rf-detail-elapsed">查询 {{ detail.elapsed_ms }} ms</span>
              </div>

              <!-- 详情主体 -->
              <div v-loading="detLoading" class="rf-detail">
                <template v-if="detState.error">
                  <div class="rf-list-err">{{ detState.error }}</div>
                </template>
                <template v-else-if="detail">
                  <!-- 对象信息头 -->
                  <div class="rf-head">
                    <span class="rf-cls mono" :title="detail.class_name">{{ detail.class_name }}</span>
                    <el-tag v-if="detail.is_root" size="small" type="danger" effect="dark">GC Root · {{ rootLabel(detail.root_kind) }}</el-tag>
                    <el-tag v-else-if="!detail.reachable" size="small" type="info" effect="plain">不可达（垃圾）</el-tag>
                    <el-tag v-else size="small" type="success" effect="plain">可达</el-tag>
                    <span class="rf-oid mono">{{ detail.obj_id }}</span>
                  </div>
                  <div class="rf-sub">
                    <span>浅堆 {{ formatBytes(detail.shallow_bytes) }}</span>
                    <span class="rf-sub-sep" />
                    <span>{{ kindLabel(detail.kind) }}</span>
                    <span class="rf-sub-sep" />
                    <span>出引用 {{ detail.outgoing_total }} · 入引用 {{ detail.incoming_total }}</span>
                  </div>

                  <!-- Path to GC Roots -->
                  <div class="rf-sec">
                    <div class="rf-sec-title">
                      <el-icon><Flag /></el-icon>
                      Path to GC Roots
                      <span class="rf-sec-hint">最短引用链 · 每跳标注引用点</span>
                    </div>

                    <div v-if="detail.path_state === 'is-root'" class="rf-path-note ok">
                      该对象自身是 GC Root（{{ rootLabel(detail.root_kind) }}），无上层引用链。
                    </div>
                    <div v-else-if="detail.path_state === 'unreachable'" class="rf-path-note warn">
                      <el-icon><WarningFilled /></el-icon>{{ detail.path_note }}
                    </div>
                    <div v-else-if="detail.path_state === 'truncated'" class="rf-path-note warn">
                      <el-icon><WarningFilled /></el-icon>{{ detail.path_note }}
                    </div>
                    <div v-else class="rf-path">
                      <template v-for="(p, i) in detail.path" :key="i">
                        <div class="rf-path-hop">
                          <button
                            type="button"
                            class="rf-path-node mono"
                            :class="{ root: p.is_root, focus: i === detail.path.length - 1 }"
                            :title="`${p.obj_id} · 点击继续追踪`"
                            @click="p.is_root || openObj(p.obj_id, `${p.class_name} @ ${p.obj_id}`, p.via)"
                          >
                            <span class="rf-node-name">{{ p.class_name }}</span>
                            <span v-if="p.is_root" class="rf-node-root">ROOT</span>
                          </button>
                          <span v-if="i < detail.path.length - 1" class="rf-path-via mono">{{ viaText(p.via, detail.path[i + 1]) }}</span>
                        </div>
                        <el-icon v-if="i < detail.path.length - 1" class="rf-path-arrow"><ArrowDown /></el-icon>
                      </template>
                    </div>
                  </div>

                  <!-- incoming -->
                  <div class="rf-sec">
                    <div class="rf-sec-title">
                      <el-icon><Download /></el-icon>
                      Incoming · 谁引用它
                      <span class="rf-sec-hint">按引用者类聚合，共 {{ formatInt(detail.incoming_total) }} 条引用</span>
                    </div>
                    <div v-if="detail.incoming_total === 0" class="rf-sec-empty">无任何对象引用它</div>
                    <template v-else>
                      <div class="rf-inc-list">
                        <div v-for="(g, gi) in detail.incoming_by_class" :key="gi" class="rf-inc-group">
                          <div class="rf-inc-head">
                            <span class="mono rf-inc-cls" :title="g.class_name">{{ g.class_name }}</span>
                            <span class="rf-inc-count mono">{{ formatInt(g.count) }} ×</span>
                            <span class="rf-inc-shallow mono">{{ formatBytes(g.shallow_bytes) }}</span>
                          </div>
                          <div v-if="detail.incoming_samples.length" class="rf-inc-samples">
                            <button
                              v-for="(s, si) in detail.incoming_samples.filter(x => x.referrer_class === g.class_name)"
                              :key="si"
                              type="button"
                              class="rf-inc-sample mono"
                              :title="`引用点：${s.via}`"
                              @click="openObj(s.referrer_obj_id, `${s.referrer_class} @ ${s.referrer_obj_id}`, s.via)"
                            >
                              <span class="rf-inc-sample-via">{{ s.via || '—' }}</span>
                              <span class="rf-inc-sample-id">{{ s.referrer_obj_id }}</span>
                              <span class="rf-inc-sample-sh">{{ formatBytes(s.referrer_shallow) }}</span>
                            </button>
                          </div>
                        </div>
                      </div>
                      <div v-if="detail.incoming_truncated" class="rf-more">
                        聚合组过多，仅显示引用数最多的 {{ detail.incoming_by_class.length }} 类；点击引用对象可继续展开
                      </div>
                    </template>
                  </div>

                  <!-- outgoing -->
                  <div class="rf-sec">
                    <div class="rf-sec-title">
                      <el-icon><Top /></el-icon>
                      Outgoing · 它引用了谁
                      <span class="rf-sec-hint">共 {{ formatInt(detail.outgoing_total) }} 条引用</span>
                    </div>
                    <div v-if="detail.outgoing_total === 0" class="rf-sec-empty">
                      {{ detail.kind === 'primitive-array' ? '基本类型数组不持有对象引用' : '无对象引用' }}
                    </div>
                    <template v-else>
                      <div class="rf-out-list">
                        <div v-for="(o, oi) in detail.outgoing" :key="oi" class="rf-out-row">
                          <span class="rf-out-label mono" :title="o.label">{{ o.label }}</span>
                          <button type="button" class="rf-out-target mono" @click="openObj(o.target_obj_id, `${o.target_class} @ ${o.target_obj_id}`, o.label)">
                            <span class="rf-out-cls">{{ o.target_class }}</span>
                            <span class="rf-out-id">{{ o.target_obj_id }}</span>
                            <span class="rf-out-sh">{{ formatBytes(o.target_shallow) }}</span>
                            <span v-if="o.count > 1" class="rf-out-count">× {{ formatInt(o.count) }}</span>
                          </button>
                        </div>
                      </div>
                      <div v-if="detail.outgoing_truncated" class="rf-more">
                        明细过多已截断（仅显示前 {{ detail.outgoing.length }} 项）——大数组已按目标类聚合，点击样例对象查看详情
                      </div>
                    </template>
                  </div>
                </template>
              </div>
            </template>
          </section>
        </div>
      </template>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import {
  ArrowDown,
  Back,
  Download,
  Flag,
  InfoFilled,
  Link,
  Position,
  Refresh,
  Right,
  Search,
  Top,
  WarningFilled,
} from '@element-plus/icons-vue'
import { useMatStore } from '../../store'
import type { RefIntentPreset } from '../../store'
import { formatBytes, formatInt } from '../../api/matApi'
import {
  fetchRefInspect,
  fetchRefInstances,
  fetchRefSearchClasses,
} from '../../api/matApi'
import type { RefClassHit, RefInstances, RefObjectDetail } from '../../api/matApi'
import type { MatDump } from '../../types'

/**
 * 对象引用追踪（Reference Tracking）：
 * 左侧 类名搜索 → 该类实例样本（浅堆降序）→ 选中实例；
 * 右侧 对象详情 = Path to GC Roots（最短链 + 每跳引用点）+ Incoming（谁引用我，按类聚合）
 *      + Outgoing（我引用谁，字段级标注）。
 * 任何引用对象可点击继续追踪（顶部面包屑可回退），支持直方图 / 泄漏检测跳转预设。
 */
const props = defineProps<{
  dump: MatDump
}>()

const store = useMatStore()

// ---------- 会话构建状态 ----------
const buildState = computed(() => store.refsCache[props.dump.id] ?? { loading: false, error: '', data: null })
const buildInfo = computed(() => buildState.value.data)

// ---------- 搜索 / 实例列表（左侧） ----------
const keyword = ref('')
const mode = ref<'classes' | 'instances'>('classes')
const classState = reactive({ loading: false, error: '', searched: false })
const classHits = ref<RefClassHit[]>([])
const curClassName = ref('')
const inst = ref<RefInstances | null>(null)
const instLoading = ref(false)
const instErr = ref('')

// ---------- 详情 / 追踪栈（右侧） ----------
interface TraceItem {
  objId: string
  label: string
  short: string
}
const trace = ref<TraceItem[]>([])
const focusObjId = computed(() => (trace.value.length ? trace.value[trace.value.length - 1].objId : ''))
const detailMap = reactive<Record<string, { loading: boolean; error: string; data: RefObjectDetail | null }>>({})
const detState = computed(() => detailMap[focusObjId.value] ?? { loading: false, error: '', data: null })
const detLoading = computed(() => trace.value.length > 0 && detState.value.loading)
const detail = computed(() => (focusObjId.value ? detState.value.data : null))

const KEEP_TRACE = 60

// ---------- 动作 ----------
/** 执行类名搜索（keyword 空 = 列出浅堆 Top 类） */
async function runSearch(q: string) {
  if (!props.dump.path) return
  classState.loading = true
  classState.error = ''
  try {
    const res = await fetchRefSearchClasses(props.dump.path, q)
    classHits.value = res.hits
    classState.searched = true
  } catch (e: any) {
    classHits.value = []
    classState.error = e?.message || String(e)
  } finally {
    classState.loading = false
  }
}

function doSearch() {
  void runSearch(keyword.value.trim())
}

watch(
  () => buildInfo.value,
  () => {
    if (buildInfo.value && !classState.searched) {
      void runSearch(keyword.value.trim())
    }
  }
)

function backToClasses() {
  mode.value = 'classes'
  instErr.value = ''
}

async function selectClass(name: string) {
  curClassName.value = name
  mode.value = 'instances'
  instLoading.value = true
  instErr.value = ''
  inst.value = null
  try {
    inst.value = await fetchRefInstances(props.dump.path, name)
  } catch (e: any) {
    instErr.value = e?.message || String(e)
  } finally {
    instLoading.value = false
  }
  // 自动查看该类浅堆最大的实例（多数场景用户想直接看持有大对象的引用链）
  if (inst.value && inst.value.rows.length) {
    const top = inst.value.rows[0]
    openObj(top.obj_id, `${name} @ ${top.obj_id}`)
  }
}

function isTraceTail(objId: string): boolean {
  return objId === focusObjId.value
}

/** 打开一个对象（追踪栈入栈 + 拉取详情）；via 仅作展示备注，不参与跳转 */
async function openObj(objId: string, label: string, _via?: string) {
  // 已在栈顶则不重复入栈
  if (trace.value.length && trace.value[trace.value.length - 1].objId === objId) return
  const short = label.length > 44 ? label.slice(0, 42) + '…' : label
  trace.value.push({ objId, label, short })
  if (trace.value.length > KEEP_TRACE) trace.value.splice(0, trace.value.length - KEEP_TRACE)
  await loadDetail(objId)
}

/** 面包屑回跳：截断追踪栈 */
function goTrace(i: number) {
  if (i >= trace.value.length - 1) return
  trace.value.splice(i + 1)
}

async function loadDetail(objId: string) {
  const exist = detailMap[objId]
  if (exist?.data || exist?.loading) return
  detailMap[objId] = { loading: true, error: '', data: null }
  try {
    detailMap[objId] = { loading: false, error: '', data: await fetchRefInspect(props.dump.path, objId) }
  } catch (e: any) {
    detailMap[objId] = { loading: false, error: (e?.message || String(e)), data: null }
  }
}

// ---------- 跨视图跳转预设（直方图 / 泄漏检测入口） ----------
let lastIntentSeq = 0
function applyIntent(preset: RefIntentPreset | null) {
  if (!preset) return
  if (preset.kind === 'class') {
    keyword.value = preset.query
    mode.value = 'classes'
    void selectClass(preset.query) // 精确类名直接展示实例
    // 同时刷新命中列表，便于横向查看相关类
    void runSearch(preset.query)
  } else {
    // object：直接定位详情
    openObj(preset.objId, `${preset.className ?? preset.objId} @ ${preset.objId}`)
    if (preset.className) {
      keyword.value = preset.className
      void runSearch(preset.className)
    }
  }
}

watch(
  () => store.refsIntent,
  (it) => {
    if (!props.dump || it.dumpId !== props.dump.id || it.seq === lastIntentSeq) return
    lastIntentSeq = it.seq
    if (it.preset) applyIntent(it.preset)
  },
  { deep: true }
)

// ---------- dump 切换重置 ----------
function resetAll() {
  lastIntentSeq = store.refsIntent.seq
  keyword.value = ''
  mode.value = 'classes'
  classHits.value = []
  classState.searched = false
  classState.error = ''
  inst.value = null
  instErr.value = ''
  trace.value = []
  for (const k of Object.keys(detailMap)) delete detailMap[k]
}

watch(
  () => props.dump.id,
  () => {
    resetAll()
    if (buildInfo.value) void doSearch()
  }
)

onMounted(async () => {
  await store.ensureRefsBuild(props.dump.id)
  // 跨视图跳转（openRefs）触发本视图首次挂载时，setup 期注册的 watch 收不到已发生的 intent，
  // 这里等构建完成后补消费，否则「追踪」跳过来会落在空状态
  const it = store.refsIntent
  if (it.dumpId === props.dump.id && it.seq !== lastIntentSeq) {
    lastIntentSeq = it.seq
    if (it.preset) applyIntent(it.preset)
  }
})

// ---------- 文案 ----------
const ROOT_LABELS: Record<string, string> = {
  thread: '线程',
  'jni-global': 'JNI 全局引用',
  'jni-local': 'JNI 局部引用',
  'stack-frame': 'Java 栈帧',
  'native-stack': 'Native 栈',
  'system-class': '系统类',
  'thread-block': '线程阻塞对象',
  monitor: '监视器',
  unknown: '未知根',
  'gc-root': 'GC Root',
}
function rootLabel(kind: string): string {
  return ROOT_LABELS[kind] || kind
}
function kindLabel(kind: string): string {
  switch (kind) {
    case 'instance':
      return '实例对象'
    case 'object-array':
      return '对象数组'
    case 'primitive-array':
      return '基本类型数组'
    case 'class':
      return '类对象'
    default:
      return kind
  }
}
/** via 展示：向上传播“引用点”字段，如 (ref) → 下一条 */
function viaText(via: string, _next: unknown): string {
  if (!via) return '—'
  return `(${via})`
}
</script>

<style scoped lang="scss">
.rf {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.rf-error {
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

.rf-loading {
  flex: 1;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}

.rf-loading-tip {
  position: absolute;
  top: 58%;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}

// ---------- 超限说明 ----------
.rf-notes {
  display: flex;
  gap: 8px;
  padding: 6px 12px;
  font-size: var(--font-size-sm);
  color: var(--el-color-warning);
  background: var(--el-color-warning-light-9);
  border-bottom: 1px solid var(--el-color-warning-light-7);
  flex-shrink: 0;
}

.rf-notes-ic {
  margin-top: 1px;
  flex-shrink: 0;
}

.rf-notes-line {
  line-height: 1.7;
}

// ---------- 工具条 ----------
.rf-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
  flex-wrap: wrap;
}

.rf-title {
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.rf-meta {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.rf-elapsed {
  margin-left: auto;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
}

// ---------- 主体两栏 ----------
.rf-body {
  flex: 1;
  min-height: 0;
  display: flex;
}

.rf-left {
  width: 320px;
  min-width: 260px;
  max-width: 46%;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--el-border-color-lighter);
  background: var(--el-fill-color-lighter);
}

.rf-left-head {
  display: flex;
  gap: 6px;
  padding: 8px;
  flex-shrink: 0;
}

.rf-left-sub {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 2px 10px 6px;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}

.rf-sub-meta {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.rf-left-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 0 6px 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

// 类行 / 实例行（button 重置默认样式）
.rf-class,
.rf-inst {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 6px 8px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--el-text-color-primary);
  text-align: left;
  cursor: pointer;

  &:hover {
    background: var(--el-fill-color);
  }

  &.active {
    background: var(--el-color-primary-light-8);
  }
}

.rf-class-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-size-sm);
}

.rf-class-nums {
  flex-shrink: 0;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);

  b {
    color: var(--el-text-color-primary);
  }
}

.rf-inst {
  font-size: var(--font-size-sm);
}

.rf-inst-id {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.rf-inst-shallow {
  flex-shrink: 0;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  font-variant-numeric: tabular-nums;
}

.rf-list-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 28px 16px;
  text-align: center;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);

  .el-icon {
    font-size: 20px;
    color: var(--el-text-color-placeholder);
  }
}

.rf-list-err {
  padding: 10px;
  font-size: var(--font-size-sm);
  color: var(--el-color-danger);
}

// ---------- 右侧 ----------
.rf-right {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.rf-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 14px;
  padding: 40px;
}

.rf-empty-ic {
  font-size: 34px;
  color: var(--el-color-primary-light-5);
}

.rf-empty-main {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-width: 420px;

  b {
    font-size: var(--font-size-md);
    color: var(--el-text-color-primary);
  }

  span {
    font-size: var(--font-size-sm);
    color: var(--el-text-color-secondary);
    line-height: 1.7;
  }
}

// 追踪面包屑
.rf-trace {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
  overflow: hidden;
}

.rf-trace-ic {
  color: var(--el-text-color-placeholder);
  margin-right: 4px;
  flex-shrink: 0;
}

.rf-trace-node {
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  border: none;
  background: transparent;
  padding: 2px 6px;
  border-radius: var(--radius-xs);
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
  cursor: pointer;

  &:hover {
    background: var(--el-fill-color);
  }

  &.active {
    color: var(--el-text-color-primary);
    font-weight: 600;
  }
}

.rf-trace-sep {
  border: none;
  background: transparent;
  color: var(--el-text-color-placeholder);
  cursor: default;
  display: flex;
}

.rf-detail-elapsed {
  margin-left: auto;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
  flex-shrink: 0;
}

// 详情滚动容器
.rf-detail {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 10px 12px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

// 对象头
.rf-head {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.rf-cls {
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--el-text-color-primary);
  word-break: break-all;
}

.rf-oid {
  margin-left: auto;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
}

.rf-sub {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.rf-sub-sep {
  width: 1px;
  height: 10px;
  background: var(--el-border-color);
}

// 分节
.rf-sec {
  border: 1px solid var(--el-border-color-lighter);
  border-radius: var(--radius-md);
  background: var(--el-fill-color-lighter);
  padding: 8px 10px;
}

.rf-sec-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--el-text-color-primary);
  margin-bottom: 8px;

  .el-icon {
    color: var(--el-color-primary);
  }
}

.rf-sec-hint {
  font-weight: 400;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
}

.rf-sec-empty {
  padding: 10px 4px;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}

// path
.rf-path-note {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);

  &.ok {
    color: var(--el-color-success);
    background: var(--el-color-success-light-9);
  }

  &.warn {
    color: var(--el-color-warning);
    background: var(--el-color-warning-light-9);
  }
}

.rf-path {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 2px;
}

.rf-path-hop {
  display: flex;
  align-items: center;
  gap: 8px;
}

.rf-path-node {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: none;
  border-radius: var(--radius-sm);
  background: var(--el-fill-color);
  color: var(--el-text-color-primary);
  font-size: var(--font-size-sm);
  padding: 4px 10px;
  cursor: pointer;
  max-width: 70%;
  overflow: hidden;

  &:hover {
    background: var(--el-fill-color-dark);
  }

  &.root {
    background: var(--el-color-primary);
    color: #fff;
    cursor: default;
  }

  &.focus {
    outline: 1px solid var(--el-color-primary-light-5);
  }
}

.rf-node-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.rf-node-root {
  font-size: 10px;
  letter-spacing: 0.5px;
  opacity: 0.9;
}

.rf-path-via {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
  flex-shrink: 0;
}

.rf-path-arrow {
  color: var(--el-text-color-placeholder);
  margin-left: 18px;
}

// incoming
.rf-inc-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.rf-inc-group {
  border-radius: var(--radius-sm);
  background: var(--el-fill-color-light);
  padding: 5px 8px;
}

.rf-inc-head {
  display: flex;
  align-items: center;
  gap: 10px;
}

.rf-inc-cls {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-size-sm);
  font-weight: 600;
}

.rf-inc-count {
  color: var(--el-color-danger);
  font-size: var(--font-size-sm);
}

.rf-inc-shallow {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.rf-inc-samples {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 4px;
}

.rf-inc-sample {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: var(--radius-xs);
  background: transparent;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  padding: 2px 8px;
  cursor: pointer;

  &:hover {
    background: var(--el-color-primary-light-9);
    color: var(--el-color-primary);
  }
}

.rf-inc-sample-via {
  font-weight: 600;
}

.rf-inc-sample-sh {
  opacity: 0.8;
}

// outgoing
.rf-out-list {
  display: flex;
  flex-direction: column;
}

.rf-out-row {
  display: flex;
  align-items: center;
  gap: 8px;
  border-bottom: 1px dashed var(--el-border-color-lighter);
  padding: 3px 2px;

  &:last-child {
    border-bottom: none;
  }
}

.rf-out-label {
  width: 120px;
  flex-shrink: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-size-sm);
  color: var(--el-color-primary);
  text-align: right;
}

.rf-out-target {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  border: none;
  background: transparent;
  color: var(--el-text-color-primary);
  font-size: var(--font-size-sm);
  cursor: pointer;
  padding: 2px 4px;
  border-radius: var(--radius-xs);

  &:hover {
    background: var(--el-fill-color);
  }
}

.rf-out-cls {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
}

.rf-out-id {
  color: var(--el-text-color-placeholder);
  flex-shrink: 0;
}

.rf-out-sh {
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}

.rf-out-count {
  color: var(--el-color-warning);
  flex-shrink: 0;
}

.rf-more {
  margin-top: 6px;
  font-size: var(--font-size-xs);
  color: var(--el-color-warning);
}

.rf-notes-act {
  margin-top: 6px;
}
</style>
