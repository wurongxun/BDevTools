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
      <WorkerProgress
        v-if="worker && !worker.finished"
        :dump-id="dump.id"
        hint="分析跑在独立子进程里，主窗口可以正常操作；完成后本页自动显示结果。"
        counters
      />
      <div v-else class="lk-loading-tip">正在解析对象引用图并计算支配树（Retained Heap）…</div>
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
        <ExportButton :rows="csvRows" :columns="csvColumns" :filename="`${dump.name}-leak-suspects.csv`" label="CSV" />
        <app-button size="small" text :disabled="!result.suspects.length" @click="exportHtmlReport">导出报告</app-button>
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

      <!-- 问题叙述 + 占比饼图 -->
      <div v-if="result.suspects.length" class="lk-overview">
        <div class="lk-narr">
          <div class="lk-narr-head">
            <b>问题叙述</b>
            <span class="lk-narr-hint">由报告自动生成的结论摘要，便于直接贴进问题单</span>
          </div>
          <ol class="lk-narr-list">
            <li v-for="(n, i) in narratives" :key="i">
              <b>{{ n.head }}</b>{{ n.body }}
            </li>
          </ol>
        </div>
        <div class="lk-pie">
          <div class="lk-pie-chart" :style="{ background: pie.gradient }" role="img" aria-label="可达堆占比饼图"></div>
          <div class="lk-pie-legend">
            <div v-for="s in pie.segs" :key="s.label" class="lk-pie-item">
              <i class="lk-pie-dot" :style="{ background: s.color }"></i>
              <span class="lk-pie-name" :title="s.label">{{ s.label }}</span>
              <span class="lk-pie-pct">{{ s.pct.toFixed(2) }}%</span>
            </div>
          </div>
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

              <!-- 引用路径（长链默认折叠，避免把卡片撑高） -->
              <div class="lk-path mono">
                <template v-if="sp.path.length">
                  <template v-for="(p, i) in visiblePath(sp)" :key="i">
                    <span v-if="i" class="lk-arrow">→</span>
                    <span class="lk-node" :class="{ root: p.is_root }" :title="p.is_root ? 'GC Root 对象' : p.class_name">
                      {{ p.class_name }}
                    </span>
                  </template>
                  <button
                    v-if="sp.path.length > PATH_FOLD"
                    type="button"
                    class="lk-fold"
                    :aria-label="expanded.has(sp.rank) ? '收起支配链' : '展开完整支配链'"
                    @click="togglePath(sp.rank)"
                  >{{ expanded.has(sp.rank) ? '收起' : `…+${sp.path.length - PATH_FOLD}` }}</button>
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
import { computed, onMounted, ref, watch } from 'vue'
import { Aim, CircleCheck, InfoFilled, Refresh, Setting, WarningFilled } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { useMatStore } from '../../store'
import WorkerProgress from '../WorkerProgress.vue'
import { formatBytes, formatInt } from '../../api/matApi'
import type { MatLeakSummary, MatLeakSuspect } from '../../api/matApi'
import type { MatDump } from '../../types'
import { exportHtml, reportHtml, escapeHtml } from '../../utils/export'
import type { ExportColumn } from '../../utils/export'
import ExportButton from '../ExportButton.vue'

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
/** 后台分析作业（有就显示进度，别让用户对着一个转圈猜） */
const worker = computed(() => store.workerJobOf(props.dump.id))
const result = computed(() => state.value.data)

/** 加载（缓存命中 / 分析中不重复请求） */
async function load() {
  await store.ensureLeakSuspects(props.dump.id)
}

/** 跳到引用追踪并定位该嫌疑对象（双向引用 + Path to GC Roots） */
function track(sp: MatLeakSuspect) {
  store.openRefs(props.dump.id, { kind: 'object', objId: sp.obj_id, className: sp.class_name })
}

// ---------- 支配链折叠 ----------
/** 折叠时保留的节点数：GC Root 在链首、嫌疑对象在链尾，中间才是噪音 */
const PATH_FOLD = 6
/** 已展开的嫌疑（按 rank 记，rank 在同一次分析里稳定） */
const expanded = ref<Set<number>>(new Set())

function togglePath(rank: number): void {
  const next = new Set(expanded.value)
  if (next.has(rank)) next.delete(rank)
  else next.add(rank)
  expanded.value = next
}

/** 折叠态保留「头 2 + 尾 4」，跨过中间的委托/包装层 */
function visiblePath(sp: MatLeakSuspect): MatLeakSuspect['path'] {
  const p = sp.path
  if (p.length <= PATH_FOLD || expanded.value.has(sp.rank)) return p
  return [...p.slice(0, 2), ...p.slice(p.length - 4)]
}

// ---------- 问题叙述（模板化结论） ----------
/**
 * 按「类名 / 持有形态」给一条定位建议。
 * 这是启发式规则（不做类型推断），命中就给出方向，不命中只报事实 ——
 * 报告的价值在「哪些对象值得看」，不在替用户下判断，因此文案里不写「就是泄漏」。
 */
function hintOf(sp: MatLeakSuspect): string {
  const c = sp.class_name
  if (sp.is_class) return '类对象被静态字段持有 ⇒ 检查该类的 static 集合/缓存是否有上限与清理'
  if (/ClassLoader|^sun\..*Loader/.test(c)) return '类加载器未被释放 ⇒ 检查热部署/动态代理/线程上下文类加载器'
  if (/Thread|Executor|Pool/.test(c)) return '由线程/线程池持有 ⇒ 检查 ThreadLocal、任务队列与线程生命周期'
  if (/Map|Cache|Dictionary|Table|Set$/.test(c)) return '集合/缓存类 ⇒ 检查容量上限、过期策略与 key 是否被外部强引用'
  if (/\[\]$|^\[/.test(c)) return '数组对象 ⇒ 检查批量读取、缓冲区与扩容后的旧数组引用'
  if (/String|CharSequence/.test(c)) return '字符串聚集 ⇒ 检查重复构造与 substring 保留大底串'
  if (/Connection|Session|Client|Channel|Stream/.test(c)) return '资源类对象 ⇒ 检查是否关闭/归还，连接池是否回收'
  return '按支配链逐层定位持有者，确认该引用是否仍应存在'
}

interface Narrative { head: string; body: string }

const narratives = computed<Narrative[]>(() => {
  const r = result.value
  const sps = r?.suspects ?? []
  if (!r || !sps.length) return []
  const out: Narrative[] = []
  const reach = r.summary.reachable_shallow

  const top = sps[0]
  const share = reach > 0 ? ((top.retained_bytes / reach) * 100).toFixed(1) : top.pct.toFixed(1)
  out.push({
    head: `${top.class_name}：`,
    body:
      `仅该对象就支配 ${formatBytes(top.retained_bytes)}，占可达堆（${formatBytes(reach)}）的 ${share}%，`
      + `合计 ${formatInt(top.retained_objects)} 个对象；最近 GC Root 为「${rootLabel(top.root_kind)}」。`
      + `${hintOf(top)}。`,
  })

  const rest = sps.slice(1, 3)
  if (rest.length) {
    const sum = sps.reduce((a, s) => a + s.retained_bytes, 0)
    const shareAll = reach > 0 ? ((sum / reach) * 100).toFixed(1) : '—'
    out.push({
      head: `Top ${sps.length} 合计：`,
      body:
        `共支配 ${formatBytes(sum)}，占可达堆 ${shareAll}%。`
        + rest.map((s, i) => `第 ${i + 2} 名 ${s.class_name}（${s.pct.toFixed(1)}%，${hintOf(s)}）`).join('；')
        + '。',
    })
  }

  const un = r.summary.unreachable_shallow
  if (un > 0) {
    out.push({
      head: '不可达对象：',
      body:
        `${formatInt(r.summary.unreachable_nodes)} 个对象（${formatBytes(un)}）已不可达但仍占文件体积 —— `
        + '这部分是 dump 时刻尚未被 GC 回收的垃圾，重启/GC 后会释放，不计入泄漏。',
    })
  }
  return out
})

// ---------- 占比饼图（纯 CSS conic-gradient，无图表库） ----------
/**
 * 只画 Top 5 + 其余可达对象。用 CSS 变量当色值，深色主题自动跟随 ——
 * 引 echarts 需要额外处理 useDark 的类名时序（先改 ref 后加类），收益不抵成本。
 */
const PIE_COLORS = [
  'var(--el-color-danger)',
  'var(--el-color-warning)',
  'var(--el-color-primary)',
  'var(--el-color-success)',
  'var(--el-color-info)',
]
const pie = computed<{ segs: { label: string; pct: number; color: string }[]; gradient: string }>(() => {
  const sps = result.value?.suspects ?? []
  const top = sps.slice(0, 5)
  const segs = top.map((s, i) => ({
    label: s.class_name,
    pct: Math.max(0, s.pct),
    color: PIE_COLORS[i],
  }))
  const used = segs.reduce((a, s) => a + s.pct, 0)
  const restPct = Math.max(0, 100 - used)
  if (restPct > 0.05) segs.push({ label: '其他可达对象', pct: restPct, color: 'var(--el-fill-color-dark)' })

  let acc = 0
  const stops = segs.map((s) => {
    const from = acc
    acc = Math.min(100, acc + s.pct)
    return `${s.color} ${from.toFixed(2)}% ${acc.toFixed(2)}%`
  })
  return { segs, gradient: `conic-gradient(${stops.join(', ')})` }
})


// ---------- 导出 ----------
const csvColumns: ExportColumn[] = [
  { key: 'rank', label: 'Rank' },
  { key: 'obj_id', label: 'Object ID' },
  { key: 'class_name', label: 'Class Name' },
  { key: 'shallow_bytes', label: 'Shallow (bytes)' },
  { key: 'retained_bytes', label: 'Retained (bytes)' },
  { key: 'retained_objects', label: 'Retained Objects' },
  { key: 'pct', label: 'Retained %' },
  { key: 'root_kind', label: 'GC Root Kind' },
  { key: 'path', label: 'Dominator Path' },
]
const csvRows = computed(() =>
  (result.value?.suspects ?? []).map(sp => ({
    rank: sp.rank,
    obj_id: sp.obj_id,
    class_name: sp.class_name,
    shallow_bytes: sp.shallow_bytes,
    retained_bytes: sp.retained_bytes,
    retained_objects: sp.retained_objects,
    pct: sp.pct.toFixed(2),
    root_kind: sp.root_kind,
    path: sp.path.map(p => p.class_name).join(' → '),
  }))
)

/** HTML 报告：内联样式、可脱离应用独立打开 */
function exportHtmlReport(): void {
  const r = result.value
  if (!r) return
  const s = r.summary
  const meta = [
    `文件：${props.dump.path}`,
    `格式：${props.dump.format} · 大小：${formatBytes(props.dump.size)}`,
    `可达对象：${formatInt(s.reachable_nodes)} · 可达浅堆：${formatBytes(s.reachable_shallow)}`,
    `支配树：${s.truncated ? '因规模限制跳过' : '已计算'}`,
  ]
  const parts: string[] = []
  parts.push('<h2>堆概览</h2><table>')
  const stats: [string, string][] = [
    ['堆对象总数', formatInt(s.total_nodes)],
    ['全部浅堆', formatBytes(s.total_shallow)],
    ['GC Roots', formatInt(s.root_nodes)],
    ['引用边', formatInt(s.edge_count)],
    ['可达对象 / 浅堆', `${formatInt(s.reachable_nodes)} / ${formatBytes(s.reachable_shallow)}`],
    ['不可达对象 / 浅堆', `${formatInt(s.unreachable_nodes)} / ${formatBytes(s.unreachable_shallow)}`],
  ]
  for (const [k, v] of stats) parts.push(`<tr><th>${escapeHtml(k)}</th><td class="num">${escapeHtml(v)}</td></tr>`)
  parts.push('</table>')
  if (r.notes.length) {
    parts.push('<h2>分析说明</h2><ul>' + r.notes.map(n => `<li>${escapeHtml(n)}</li>`).join('') + '</ul>')
  }
  if (narratives.value.length) {
    parts.push(
      '<h2>问题叙述</h2><ol>'
      + narratives.value.map(n => `<li><b>${escapeHtml(n.head)}</b>${escapeHtml(n.body)}</li>`).join('')
      + '</ol>',
    )
  }
  parts.push(`<h2>Top ${r.suspects.length} 泄漏嫌疑</h2>`)
  for (const sp of r.suspects) {
    const pct = Math.min(100, Math.max(0, sp.pct))
    parts.push('<div class="card">')
    parts.push(`<b>#${sp.rank} ${escapeHtml(sp.class_name)}</b> <span class="muted">${escapeHtml(sp.obj_id)}</span>`)
    parts.push(`<p>Retained Heap：${escapeHtml(formatBytes(sp.retained_bytes))}（${sp.pct.toFixed(2)}% 可达堆，${formatInt(sp.retained_objects)} 个对象） · Shallow：${escapeHtml(formatBytes(sp.shallow_bytes))} · GC Root：${escapeHtml(sp.root_kind)}</p>`)
    parts.push(`<div class="bar"><i style="width:${pct}%"></i></div>`)
    parts.push(`<p class="muted">支配链：${sp.path.map(p => escapeHtml(p.class_name)).join(' → ') || '—'}</p>`)
    parts.push('</div>')
  }
  try {
    exportHtml(`${props.dump.name}-leak-report.html`, reportHtml('Leak Suspects 报告', meta, parts.join('')))
    ElMessage.success('泄漏报告已导出')
  } catch (e: any) {
    ElMessage.error('导出失败：' + (e?.message || String(e)))
  }
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
  () => {
    expanded.value = new Set()
    void load()
  }
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
  color: var(--stateErrorFg);
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
  color: var(--stateWarningFg);
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
    color: var(--stateErrorFg);
  }

  &.mid {
    color: var(--stateWarningFg);
  }
}

.lk-ret-pct {
  font-size: var(--font-size-sm);
  font-variant-numeric: tabular-nums;

  &.high {
    color: var(--stateErrorFg);
  }

  &.mid {
    color: var(--stateWarningFg);
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

// ---------- 问题叙述 + 饼图 ----------
.lk-overview {
  display: flex;
  gap: 12px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
  flex-wrap: wrap;
}

.lk-narr {
  flex: 1;
  min-width: 320px;
}

.lk-narr-head {
  display: flex;
  align-items: baseline;
  gap: 10px;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-primary);
  flex-wrap: wrap;
}

.lk-narr-hint {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
}

.lk-narr-list {
  margin: 6px 0 0;
  padding-left: 20px;
  font-size: var(--font-size-sm);
  line-height: 1.8;
  color: var(--el-text-color-regular);

  li { word-break: break-word; }

  b { color: var(--el-text-color-primary); }
}

.lk-pie {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.lk-pie-chart {
  width: 88px;
  height: 88px;
  border-radius: var(--radius-full);
  border: 1px solid var(--el-border-color-lighter);
  // 中心留白，做成环形，露出面板底色
  mask: radial-gradient(circle, transparent 44%, #000 45%);
}

.lk-pie-legend {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: var(--font-size-xs);
  max-width: 240px;
}

.lk-pie-item {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.lk-pie-dot {
  width: 8px;
  height: 8px;
  border-radius: var(--radius-xs);
  flex-shrink: 0;
}

.lk-pie-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-family-mono);
  color: var(--el-text-color-regular);
}

.lk-pie-pct {
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
  color: var(--el-text-color-secondary);
}

// ---------- 支配链折叠 ----------
.lk-fold {
  border: 1px solid var(--el-border-color-lighter);
  background: transparent;
  color: var(--el-color-primary);
  border-radius: 999px;
  padding: 0 6px;
  font-size: var(--font-size-xs);
  line-height: 16px;
  cursor: pointer;

  &:hover { border-color: var(--el-color-primary); }
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
  color: var(--stateSuccessFg);
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
