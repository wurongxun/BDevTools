<template>
  <div class="leak">
    <!-- 分析失败 -->
    <div v-if="state.error" class="lk-error">
      <el-icon><WarningFilled /></el-icon>
      <span>{{ state.error }}</span>
      <app-button size="small" text @click="load">
        <el-icon><Refresh /></el-icon>
      </app-button>
    </div>

    <!-- 分析中（支配树计算较重，明确提示耗时） -->
    <div v-else-if="state.loading" v-loading="true" class="lk-loading">
      <div class="lk-loading-tip">正在解析对象引用图并计算支配树（Retained Heap）…</div>
    </div>

    <template v-else-if="result">
      <!-- 工具条 -->
      <div class="lk-toolbar">
        <span class="lk-title">Leak Suspects</span>
        <span class="lk-meta">
          {{ formatInt(result.summary.reachable_nodes) }} 个可达对象 · 可达浅堆 {{ formatBytes(result.summary.reachable_shallow) }}
        </span>
        <el-tag v-if="result.summary.truncated" size="small" type="warning" effect="plain">规模超限：已跳过支配树分析</el-tag>
        <span class="lk-elapsed">分析耗时 {{ result.summary.elapsed_ms }} ms</span>
        <app-button size="small" text class="lk-budget" @click="store.openSettings()">
          <el-icon class="lk-budget-ic"><Setting /></el-icon>分析预算
        </app-button>
      </div>

      <!-- 堆统计条 -->
      <div class="lk-stats">
        <div v-for="c in statCells" :key="c.k" class="lk-stat" :class="{ muted: c.muted }" :title="c.tip">
          <span class="v">{{ c.v }}</span>
          <span class="k">{{ c.k }}</span>
        </div>
      </div>

      <!-- 分析说明（规模保护 / 畸形数据提示等） -->
      <div v-if="result.notes.length" class="lk-notes">
        <el-icon class="lk-notes-ic"><InfoFilled /></el-icon>
        <div class="lk-notes-body">
          <div v-for="(n, i) in result.notes" :key="i" class="lk-notes-line">{{ n }}</div>
        </div>
      </div>

      <!-- Top 嫌疑列表 -->
      <template v-if="result.suspects.length">
        <div class="lk-list-head">
          <span>Top {{ result.suspects.length }} 嫌疑对象 · 按 Retained Heap 降序</span>
          <span class="lk-list-hint">Retained Heap = 仅由该对象支配的对象浅堆之和（MAT 口径），即该对象被回收后能释放的内存</span>
        </div>

        <div class="lk-list">
          <div v-for="sp in result.suspects" :key="sp.rank" class="lk-suspect">
            <div class="lk-rank" :class="rankClass(sp.rank)">{{ sp.rank }}</div>

            <div class="lk-main">
              <!-- 头部：类名 + 标识 -->
              <div class="lk-head">
                <span class="lk-cls mono" :title="sp.class_name">{{ sp.class_name }}</span>
                <el-tag v-if="sp.is_class" size="small" type="warning" effect="dark" class="lk-tag">类对象 · 静态持有</el-tag>
                <app-button size="small" text class="lk-jump" @click="track(sp)">
                  <el-icon class="lk-jump-ic"><Aim /></el-icon>追踪
                </app-button>
                <span class="lk-oid mono" :title="'对象 id：' + sp.obj_id">{{ sp.obj_id }}</span>
              </div>

              <!-- Retained 强调区 + 占比条 -->
              <div class="lk-ret-row">
                <span class="lk-ret" :class="retClass(sp.pct)">{{ formatBytes(sp.retained_bytes) }}</span>
                <span class="lk-ret-pct" :class="retClass(sp.pct)">{{ sp.pct.toFixed(1) }}%</span>
                <div class="lk-meter">
                  <div class="lk-meter-bar" :style="{ width: pctWidth(sp.pct) }"></div>
                </div>
                <span class="lk-meter-label">占可达堆</span>
              </div>

              <!-- 次级指标 -->
              <div class="lk-sub">
                <span class="lk-sub-item">浅堆 {{ formatBytes(sp.shallow_bytes) }}</span>
                <span class="lk-sub-sep" />
                <span class="lk-sub-item">支配 {{ formatInt(sp.retained_objects) }} 个对象</span>
                <span class="lk-sub-sep" />
                <span class="lk-sub-item">
                  最近 GC Root：
                  <el-tag size="small" effect="plain" class="lk-root-tag" :title="'root kind: ' + sp.root_kind">
                    {{ rootLabel(sp.root_kind) }}
                  </el-tag>
                </span>
              </div>

              <!-- 引用路径 -->
              <div class="lk-path mono">
                <template v-if="sp.path.length">
                  <template v-for="(p, i) in sp.path" :key="i">
                    <span v-if="i" class="lk-arrow">→</span>
                    <span class="lk-node" :class="{ root: p.is_root }" :title="p.is_root ? 'GC Root 对象' : p.class_name">
                      {{ p.class_name }}
                    </span>
                  </template>
                </template>
                <span v-else class="lk-node">（无支配链信息）</span>
              </div>
            </div>
          </div>
        </div>
      </template>

      <!-- 无嫌疑 -->
      <div v-else class="lk-none">
        <el-icon class="lk-none-ic"><CircleCheck /></el-icon>
        <template v-if="result.summary.truncated">
          <span>对象图规模超过当前分析上限，已跳过支配树分析，无法给出泄漏嫌疑（阈值与规模见上方说明）。</span>
          <app-button size="small" type="primary" plain class="lk-none-act" @click="store.openSettings()">
            调整分析预算并重试
          </app-button>
        </template>
        <template v-else>
          <span>未发现明显的泄漏嫌疑 —— 无可达对象被单一对象大规模支配。</span>
        </template>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { Aim, CircleCheck, InfoFilled, Refresh, Setting, WarningFilled } from '@element-plus/icons-vue'
import { useMatStore } from '../../store'
import { formatBytes, formatInt } from '../../api/matApi'
import type { MatLeakSummary, MatLeakSuspect } from '../../api/matApi'
import type { MatDump } from '../../types'

/**
 * 泄漏检测（Leak Suspects）：后端基于对象引用图 → Lengauer-Tarjan 支配树
 * → Retained Heap 精确聚合，输出 Top 嫌疑对象（占用 / 占比 / 支配对象数）
 * 与「GC Root → … → 嫌疑对象」支配链。
 *
 * 结果按 dump 缓存于 store（切子 tab / 切 dump 返回不重复分析）。
 * 大堆文件（可达对象 / 引用边超限）时后端返回 summary + notes + 空 suspects。
 */
const props = defineProps<{
  dump: MatDump
}>()

const store = useMatStore()

const state = computed(() => store.leakCache[props.dump.id] ?? { loading: false, error: '', data: null })
const result = computed(() => state.value.data)

/** 加载（缓存命中 / 分析中不重复请求） */
async function load() {
  await store.ensureLeakSuspects(props.dump.id)
}

/** 跳到引用追踪并定位该嫌疑对象（双向引用 + Path to GC Roots） */
function track(sp: MatLeakSuspect) {
  store.openRefs(props.dump.id, { kind: 'object', objId: sp.obj_id, className: sp.class_name })
}

/** GC Root 分类 → 中文标签（未知分类原样展示） */
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

/** 堆统计条（summary → 展示格） */
const statCells = computed<{ k: string; v: string; tip?: string; muted?: boolean }[]>(() => {
  const s: MatLeakSummary | undefined = result.value?.summary
  if (!s) return []
  return [
    { k: '堆对象总数', v: formatInt(s.total_nodes) },
    { k: '全部浅堆', v: formatBytes(s.total_shallow) },
    { k: 'GC Roots', v: formatInt(s.root_nodes) },
    {
      k: '引用边',
      v: formatInt(s.edge_count),
      tip: s.skipped_edge_count ? `另有 ${formatInt(s.skipped_edge_count)} 条指向图外对象的畸形引用已忽略` : undefined,
    },
    { k: '可达对象', v: formatInt(s.reachable_nodes) },
    { k: '不可达对象', v: formatInt(s.unreachable_nodes), muted: true },
    { k: '不可达浅堆', v: formatBytes(s.unreachable_shallow), muted: true },
  ]
})

/** 排名徽标样式（Top3 渐弱强调） */
function rankClass(rank: number): string {
  if (rank === 1) return 'r1'
  if (rank === 2) return 'r2'
  if (rank === 3) return 'r3'
  return 'rn'
}

/** 占比条宽度（占可达堆百分比，夹到 0-100） */
function pctWidth(pct: number): string {
  const p = Math.min(Math.max(pct, 0), 100)
  return p.toFixed(2) + '%'
}

/** Retained 数值颜色分级：>50% 高危（红），>10% 关注（橙），其余常规 */
function retClass(pct: number): string {
  if (pct >= 50) return 'high'
  if (pct >= 10) return 'mid'
  return ''
}

watch(
  () => props.dump.id,
  () => void load()
)

onMounted(() => {
  void load()
})
</script>

<style scoped lang="scss">
.leak {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: auto; // 内容（统计条 + 嫌疑卡片）整体超高时滚动
}

.lk-error {
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

.lk-loading {
  flex: 1;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}

.lk-loading-tip {
  position: absolute;
  top: 58%;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}

// ---------- 工具条 ----------
.lk-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
  flex-wrap: wrap;
}

.lk-title {
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.lk-meta {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.lk-elapsed {
  margin-left: auto;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
}

// ---------- 堆统计条 ----------
.lk-stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(108px, 1fr));
  gap: 1px;
  background: var(--el-border-color-lighter);
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}

.lk-stat {
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

  &.muted .v {
    color: var(--el-text-color-secondary);
    font-weight: 400;
  }
}

// ---------- 说明 ----------
.lk-notes {
  display: flex;
  gap: 8px;
  padding: 6px 12px;
  font-size: var(--font-size-sm);
  color: var(--el-color-warning);
  background: var(--el-color-warning-light-9);
  border-bottom: 1px solid var(--el-color-warning-light-7);
  flex-shrink: 0;
}

.lk-notes-ic {
  margin-top: 1px;
  flex-shrink: 0;
}

.lk-notes-line {
  line-height: 1.7;
}

// ---------- 嫌疑列表 ----------
.lk-list-head {
  display: flex;
  align-items: baseline;
  gap: 12px;
  padding: 8px 12px 0;
  flex-shrink: 0;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-primary);
}

.lk-list-hint {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
}

.lk-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px 12px 14px;
}

.lk-suspect {
  display: flex;
  gap: 10px;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: var(--radius-md);
  padding: 10px 12px;
  background: var(--el-fill-color-lighter);
}

.lk-rank {
  flex-shrink: 0;
  width: 22px;
  height: 22px;
  margin-top: 1px;
  border-radius: var(--radius-full);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--font-size-sm);
  font-weight: 600;
  font-variant-numeric: tabular-nums;

  &.r1 {
    background: var(--el-color-danger);
    color: #fff;
  }

  &.r2 {
    background: var(--el-color-warning);
    color: #fff;
  }

  &.r3 {
    background: var(--el-color-primary);
    color: #fff;
  }

  &.rn {
    background: var(--el-fill-color);
    color: var(--el-text-color-secondary);
  }
}

.lk-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 5px;
}

// 头部行
.lk-head {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.lk-cls {
  font-size: var(--font-size-md);
  font-weight: 600;
  color: var(--el-text-color-primary);
  word-break: break-all;
}

.lk-tag {
  flex-shrink: 0;
}

.lk-jump {
  flex-shrink: 0;
  color: var(--el-color-primary);
}

.lk-jump-ic {
  margin-right: 2px;
}

.lk-oid {
  margin-left: auto;
  flex-shrink: 0;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
}

// Retained 强调行
.lk-ret-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.lk-ret {
  font-size: var(--font-size-lg);
  font-weight: 600;
  font-variant-numeric: tabular-nums;

  &.high {
    color: var(--el-color-danger);
  }

  &.mid {
    color: var(--el-color-warning);
  }
}

.lk-ret-pct {
  font-size: var(--font-size-sm);
  font-variant-numeric: tabular-nums;

  &.high {
    color: var(--el-color-danger);
  }

  &.mid {
    color: var(--el-color-warning);
  }
}

.lk-meter {
  flex: 1;
  height: 6px;
  border-radius: var(--radius-full);
  background: var(--el-fill-color);
  overflow: hidden;
}

.lk-meter-bar {
  height: 100%;
  border-radius: var(--radius-full);
  background: var(--el-color-danger);
  opacity: 0.75;
}

.lk-meter-label {
  flex-shrink: 0;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
}

// 次级指标行
.lk-sub {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.lk-sub-sep {
  width: 1px;
  height: 10px;
  background: var(--el-border-color);
}

.lk-root-tag {
  margin-left: 2px;
}

// 引用路径
.lk-path {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 2px 6px;
  padding: 5px 8px;
  background: var(--el-fill-color-light);
  border-radius: var(--radius-xs);
  font-size: var(--font-size-sm);
  line-height: 1.6;
  word-break: break-all;
}

.lk-arrow {
  color: var(--el-text-color-placeholder);
}

.lk-node {
  color: var(--el-text-color-primary);

  &.root {
    font-weight: 600;
    color: var(--el-color-primary);
  }
}

// 无嫌疑
.lk-none {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 36px 24px;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
  text-align: center;
}

.lk-none-ic {
  font-size: 18px;
  color: var(--el-color-success);
}

.lk-budget {
  margin-left: auto;
  color: var(--el-color-primary);
}

.lk-budget-ic {
  margin-right: 4px;
}

.lk-none-act {
  margin-top: 6px;
}
</style>
