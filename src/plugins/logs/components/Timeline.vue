<template>
  <div ref="rootRef" class="timeline">
    <div v-if="!buckets.length" class="tl-empty">该日志不含可解析的时间戳，暂无法生成时间轴</div>

    <template v-else>
      <div class="tl-head">
        <span class="tl-title">时间轴</span>
        <span class="tl-meta">
          共 {{ formatCount(totalCount) }} 条 · ERROR
          <b class="tl-err">{{ formatCount(errorCount) }}</b>
          · 每柱 ≈ {{ granularityLabel }}
        </span>
      </div>

      <svg
        ref="svgRef"
        class="tl-svg"
        :viewBox="`0 0 ${W} ${H}`"
        preserveAspectRatio="none"
        @mousedown="onDown"
        @mousemove="onMove"
        @mouseup="onUp"
        @mouseleave="onLeave"
      >
        <!-- 基线 -->
        <line :x1="PAD_L" :y1="baseY" :x2="W - PAD_R" :y2="baseY" class="tl-axis" />

        <!-- 柱状：total（中性）+ error（红色叠加在顶部） -->
        <template v-for="(b, i) in buckets" :key="i">
          <g v-if="b.total > 0">
            <rect
              :x="barX(i)"
              :y="barY(b.total)"
              :width="barW"
              :height="barH(b.total)"
              class="tl-bar"
            />
            <rect
              v-if="b.error > 0"
              :x="barX(i)"
              :y="barY(b.total)"
              :width="barW"
              :height="barH(b.error)"
              class="tl-bar-err"
            />
          </g>
        </template>

        <!-- 拖拽选区 -->
        <rect
          v-if="sel"
          :x="Math.min(sel.x0, sel.x1)"
          :y="PAD_T"
          :width="Math.abs(sel.x1 - sel.x0)"
          :height="plotH"
          class="tl-brush"
        />

      </svg>

      <!-- 时间刻度（HTML 渲染，避免 SVG preserveAspectRatio="none" 非等比拉伸导致字号过小/变形） -->
      <div class="tl-ticks">
        <span
          v-for="t in ticks"
          :key="t.x"
          class="tl-tick"
          :style="{ left: `${(t.x / W) * 100}%` }"
        >
          {{ t.label }}
        </span>
      </div>

      <!-- 悬停浮层 -->
      <div
        v-if="hover"
        class="tl-tip"
        :style="{ left: hover.px + 'px', top: hover.py + 'px' }"
      >
        <div class="tl-tip-row">
          <span>{{ hover.start }} ~</span>
        </div>
        <div class="tl-tip-row"><span>{{ hover.end }}</span></div>
        <div class="tl-tip-row">
          总量 <b>{{ formatCount(hover.total) }}</b> · 警告
          <b>{{ formatCount(hover.warn) }}</b> · 错误 <b class="tl-err">{{ formatCount(hover.error) }}</b>
        </div>
        <div v-if="hover.firstLine" class="tl-tip-row tl-tip-hint">点击跳转到第 {{ hover.firstLine }} 行</div>
        <div v-else class="tl-tip-row tl-tip-hint">空时段</div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useLogStore } from '../store/logs'
import { formatCount, formatTs } from '../core/filter'
import type { TimeBucket } from '../core/indexApi'

const emit = defineEmits<{ (e: 'jump', lineNo: number): void }>()

const store = useLogStore()

// ==================== 几何常量 ====================
const W = 1000
const H = 200
const PAD_L = 12
const PAD_R = 12
const PAD_T = 10
const PAD_B = 24
const plotW = W - PAD_L - PAD_R
const plotH = H - PAD_T - PAD_B
const baseY = PAD_T + plotH

const buckets = computed<TimeBucket[]>(() => store.indexMeta?.buckets ?? [])
const nBuckets = computed(() => buckets.value.length)
const slot = computed(() => (nBuckets.value ? plotW / nBuckets.value : plotW))
const barW = computed(() => Math.max(1, slot.value - 1.2))
const maxTotal = computed(() => buckets.value.reduce((m, b) => Math.max(m, b.total), 1))

const totalCount = computed(() => buckets.value.reduce((s, b) => s + b.total, 0))
const errorCount = computed(() => buckets.value.reduce((s, b) => s + b.error, 0))

// ==================== 粒度标签 ====================
const granularityLabel = computed(() => {
  const n = nBuckets.value
  if (!n) return '—'
  const span = (store.indexMeta?.ts_max ?? 0) - (store.indexMeta?.ts_min ?? 0)
  const widthMs = span / n
  if (widthMs < 2 * 60_000) return `${Math.max(1, Math.round(widthMs / 1000))} 秒`
  if (widthMs < 2 * 3_600_000) return `${Math.max(1, Math.round(widthMs / 60_000))} 分钟`
  if (widthMs < 2 * 86_400_000) return `${Math.max(1, Math.round(widthMs / 3_600_000))} 小时`
  return `${Math.max(1, Math.round(widthMs / 86_400_000))} 天`
})

// ==================== 坐标换算 ====================
function barX(i: number): number {
  return PAD_L + i * slot.value + (slot.value - barW.value) / 2
}
function barH(v: number): number {
  return Math.max(1.5, (v / maxTotal.value) * plotH)
}
function barY(v: number): number {
  return baseY - barH(v)
}

/** clientX → viewBox x（SVG 按 CSS 宽度缩放，需换算） */
function clientToVbX(clientX: number): number {
  const el = svgRef.value
  if (!el) return PAD_L
  const rect = el.getBoundingClientRect()
  const ratio = rect.width ? (clientX - rect.left) / rect.width : 0
  return PAD_L + ratio * plotW
}

/** viewBox x → UTC 毫秒（时间桶等宽，直接线性映射） */
function vbXToTime(x: number): number {
  const tsMin = store.indexMeta?.ts_min ?? 0
  const tsMax = store.indexMeta?.ts_max ?? 0
  const span = Math.max(1, tsMax - tsMin)
  const r = Math.min(1, Math.max(0, (x - PAD_L) / plotW))
  return Math.round(tsMin + r * span)
}

function bucketIndexAt(x: number): number {
  const i = Math.floor((x - PAD_L) / slot.value)
  return Math.min(nBuckets.value - 1, Math.max(0, i))
}

// ==================== 时间刻度 ====================
const ticks = computed(() => {
  const n = nBuckets.value
  if (!n || !store.indexMeta) return []
  const tsMin = store.indexMeta.ts_min!
  const tsMax = store.indexMeta.ts_max!
  const count = 5
  const out: { x: number; label: string }[] = []
  for (let k = 0; k < count; k++) {
    const r = k / (count - 1)
    const x = PAD_L + r * plotW
    const t = Math.round(tsMin + r * (tsMax - tsMin))
    out.push({ x, label: formatTs(t).slice(5) }) // MM-DD HH:mm:ss → 去掉年
  }
  return out
})

// ==================== 交互：悬停 / 点击跳转 / 拖拽选区间 ====================
const rootRef = ref<HTMLElement | null>(null)
const svgRef = ref<SVGSVGElement | null>(null)
const hover = ref<{
  px: number
  py: number
  start: string
  end: string
  total: number
  warn: number
  error: number
  firstLine: number | null
} | null>(null)
const drag = ref<{ active: boolean; x0: number; x1: number; moved: boolean }>({
  active: false,
  x0: 0,
  x1: 0,
  moved: false,
})
const sel = ref<{ x0: number; x1: number } | null>(null)

function updateHover(clientX: number, clientY: number) {
  const vbX = clientToVbX(clientX)
  const i = bucketIndexAt(vbX)
  const b = buckets.value[i]
  if (!b) {
    hover.value = null
    return
  }
  const rootRect = rootRef.value?.getBoundingClientRect()
  hover.value = {
    px: (rootRect ? clientX - rootRect.left : clientX) + 14,
    py: (rootRect ? clientY - rootRect.top : clientY) + 14,
    start: formatTs(b.start),
    end: formatTs(b.end),
    total: b.total,
    warn: b.warn,
    error: b.error,
    firstLine: b.first_line,
  }
}

function onDown(e: MouseEvent) {
  const x = clientToVbX(e.clientX)
  drag.value = { active: true, x0: x, x1: x, moved: false }
  sel.value = { x0: x, x1: x }
}
function onMove(e: MouseEvent) {
  updateHover(e.clientX, e.clientY)
  if (drag.value.active) {
    const x = clientToVbX(e.clientX)
    if (Math.abs(x - drag.value.x0) > 4) drag.value.moved = true
    drag.value.x1 = x
    sel.value = { x0: drag.value.x0, x1: x }
  }
}
function onUp() {
  if (!drag.value.active) return
  const d = drag.value
  drag.value = { active: false, x0: 0, x1: 0, moved: false }
  sel.value = null
  if (d.moved) {
    const from = vbXToTime(Math.min(d.x0, d.x1))
    const to = vbXToTime(Math.max(d.x0, d.x1))
    store.applyFilter({ tsFrom: from, tsTo: to })
  } else {
    const i = bucketIndexAt(d.x0)
    const fl = buckets.value[i]?.first_line
    if (fl && fl > 0) emit('jump', fl)
  }
}
function onLeave() {
  hover.value = null
  if (drag.value.active) {
    drag.value = { active: false, x0: 0, x1: 0, moved: false }
    sel.value = null
  }
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.timeline {
  position: relative;
  padding: 4px 8px;
  // 时间轴条用 sidebarBg（与 TabBar/LogToolbar/FilterBar 同源：工具栏条）
  @include theme.useTheme {
    background: theme.getVar('sidebarBg');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.tl-empty {
  padding: 12px 0;
  text-align: center;
  color: var(--el-text-color-secondary);
  font-size: var(--font-size-sm);
}

.tl-head {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 2px;
}

.tl-title {
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.tl-meta {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.tl-err {
  color: var(--stateErrorFg);
}

.tl-svg {
  display: block;
  width: 100%;
  height: 96px;
  cursor: crosshair;
  user-select: none;
}

.tl-axis {
  stroke: var(--el-border-color);
  stroke-width: 1;
}

.tl-bar {
  fill: var(--el-color-primary);
  opacity: 0.45;
}

.tl-bar-err {
  fill: var(--el-color-danger);
  opacity: 0.85;
}

.tl-brush {
  fill: var(--el-color-primary);
  opacity: 0.18;
}

.tl-ticks {
  position: relative;
  height: 14px;
  margin-top: 2px;
}

.tl-tick {
  position: absolute;
  transform: translateX(-50%);
  white-space: nowrap;
  font-size: var(--font-size-sm);
  font-family: var(--font-family-mono);
  color: var(--el-text-color-secondary);
}

/* 首尾刻度向内展开，避免文字溢出面板左右边缘 */
.tl-tick:first-child {
  transform: translateX(0);
}

.tl-tick:last-child {
  transform: translateX(-100%);
}

.tl-tip {
  position: absolute;
  z-index: 30;
  pointer-events: none;
  min-width: 180px;
  max-width: 260px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  background: var(--el-bg-color-overlay);
  border: 1px solid var(--el-border-color-light);
  box-shadow: var(--shadow-md);
  font-size: var(--font-size-xs);
  line-height: 1.7;
  color: var(--el-text-color-primary);
}

.tl-tip-row {
  white-space: nowrap;
}

.tl-tip-hint {
  color: var(--el-color-primary);
  margin-top: 2px;
}
</style>
