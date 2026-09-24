<template>
  <div class="log-view">
    <!-- 空态 -->
    <div v-if="!logicalTotal" class="log-empty">
      <p v-if="store.loading" class="log-empty-text">加载中…</p>
      <template v-else>
        <p class="log-empty-text">从左侧打开一个日志文件开始分析</p>
        <p class="log-empty-hint">支持大文件分块懒加载 · 关键词/正则搜索 · 级别配色 · 堆栈高亮</p>
      </template>
    </div>

    <!-- 虚拟滚动区（原生滚动条隐藏：需要在轨道上画命中刻度，只能自绘） -->
    <div v-else ref="rootRef" class="log-scroll-area" @scroll.passive="onScroll">
      <div class="log-scroll" :style="{ height: totalHeight + 'px' }">
        <div class="log-window" :style="{ transform: `translateY(${offsetY}px)` }">
        <div
          v-for="row in visibleLines"
          :key="row.idx"
          class="log-row"
          :class="
            row.line
              ? [
                  `log-row-${row.line.level.toLowerCase()}`,
                  {
                    'is-hit': hitLineSet.has(row.line.index),
                    'is-current': row.line.index === currentLineNo,
                    'is-stack': row.line.isStack,
                  },
                ]
              : ['is-pending']
          "
          :style="{ height: rowHeight + 'px', lineHeight: rowHeight + 'px' }"
          @mouseenter="row.line && onRowEnter($event, row.line)"
          @mouseleave="onRowLeave"
          @contextmenu.prevent="row.line && onRowContextMenu($event, row.line)"
        >
          <span class="log-dot" :style="{ background: row.line ? sourceColor : 'transparent' }"></span>
          <span class="log-gutter" :style="{ width: gutterWidth + 'px' }">{{ row.lineNo }}</span>
          <template v-if="row.line">
            <span class="log-badge" :class="`lv-${row.line.level.toLowerCase()}`">{{ levelText(row.line) }}</span>
            <span class="log-text" v-html="row.html"></span>
          </template>
        </div>
        </div>
      </div>
    </div>

    <!-- 自绘滚动条：轨道 + 命中刻度 + 滑块 -->
    <div
      v-if="logicalTotal"
      class="log-scrollbar"
      :class="{ 'is-dragging': dragging }"
      @mousedown.self.prevent="onTrackDown"
    >
      <svg class="log-ticks" width="100%" height="100%" aria-hidden="true">
        <path :d="tickPath" fill="none" stroke-width="1"></path>
      </svg>
      <div
        class="log-thumb"
        :style="{ height: thumbHeight + 'px', transform: `translateY(${thumbTop}px)` }"
        @mousedown.stop.prevent="onThumbDown"
      ></div>
    </div>

    <!-- 底部提示：加载中 / 超大文件滚动条受限 -->
    <div v-if="store.loadingMore || scrollCapped" class="log-more-tip">
      {{
        scrollCapped
          ? `文件过大（${logicalTotal.toLocaleString()} 行），行高已压缩到底，滚动条无法覆盖全部行——建议先用筛选缩小范围`
          : '加载中…'
      }}
    </div>

    <!-- 悬停元信息浮层 -->
    <div
      v-if="hoverLine"
      class="log-hover"
      :style="{ top: hoverTop + 'px', left: hoverLeft + 'px' }"
    >
      <div class="log-hover-row"><span class="k">行号</span><span class="v">#{{ hoverLine.index }}</span></div>
      <div v-if="hoverLine.tsText" class="log-hover-row">
        <span class="k">时间</span><span class="v">{{ hoverLine.tsText }}</span>
      </div>
      <div class="log-hover-row">
        <span class="k">级别</span>
        <span class="v" :class="`lv-text-${hoverLine.level.toLowerCase()}`">{{ hoverLine.level }}</span>
      </div>
      <div v-if="hoverLine.thread" class="log-hover-row">
        <span class="k">线程</span><span class="v">{{ hoverLine.thread }}</span>
      </div>
      <div v-if="hoverLine.logger" class="log-hover-row">
        <span class="k">来源</span><span class="v">{{ hoverLine.logger }}</span>
      </div>
      <div v-if="hoverLine.stack?.signature" class="log-hover-row">
        <span class="k">堆栈</span><span class="v">{{ hoverLine.stack.signature }}</span>
      </div>
    </div>
  </div>

    <!-- 行右键菜单：复制本行 / 复制内容 / 复制行号 -->
    <teleport to="body">
      <div
        v-if="ctxVisible"
        class="log-ctx-menu"
        :style="{ left: ctxPos.x + 'px', top: ctxPos.y + 'px' }"
        @click.stop
      >
        <div class="log-ctx-item" @click="ctxCopyLine">复制本行（含行号）</div>
        <div class="log-ctx-item" @click="ctxCopyContent">复制内容</div>
        <div class="log-ctx-item" @click="ctxCopyNumber">复制行号</div>
        <!-- 仅筛选态有意义：回到完整日志看上下文（行号恒为原始行号，无需反查） -->
        <template v-if="store.filterActive">
          <div class="log-ctx-sep"></div>
          <div class="log-ctx-item" @click="ctxRevealInFull">在完整日志中定位</div>
        </template>
      </div>
    </teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useLogStore } from '../store/logs'
import { renderLine } from '../core/highlight'
import type { ParsedLine, TextRange } from '../types'

const LINE_HEIGHT = 20
const store = useLogStore()

const rootRef = ref<HTMLElement | null>(null)
const scrollTop = ref(0)
const viewportHeight = ref(600)
const hoverLine = ref<ParsedLine | null>(null)
const hoverTop = ref(0)
const hoverLeft = ref(0)

/** 上一次激活的源 ID（用于切换时把滚动位置记到旧源） */
let lastActiveId = store.activeSourceId
/** 待恢复的滚动位置对应的源 ID（内容异步就绪后再应用一次，避免被裁切） */
const pendingScrollRestore = ref<string | null>(null)

let ro: ResizeObserver | null = null

// ==================== 每个 tab 记忆滚动位置 ====================
/**
 * 切换激活源时：先把当前（旧源）滚动位置记入 scrollMap，再把新源记忆的位置恢复。
 * 内容若是异步加载（如筛选态），pendingScrollRestore 会在 displayLines 就绪后再应用一次。
 */
watch(
  () => store.activeSourceId,
  newId => {
    const el = rootRef.value
    if (lastActiveId && lastActiveId !== newId && el) {
      store.setScroll(lastActiveId, Math.round(el.scrollTop / rowHeight.value))
    }
    pendingScrollRestore.value = newId
    lastActiveId = newId
    if (el) {
      const target = (store.getScroll(newId) ?? 0) * rowHeight.value
      el.scrollTop = target
      scrollTop.value = target
    }
  },
)

// 内容就绪（含异步筛选结果）后，再次把记忆的滚动位置应用到新源
watch(
  () => store.logicalTotal,
  len => {
    if (pendingScrollRestore.value && len > 0) {
      const el = rootRef.value
      if (el) {
        const target = (store.getScroll(pendingScrollRestore.value) ?? 0) * rowHeight.value
        el.scrollTop = target
        scrollTop.value = target
      }
      pendingScrollRestore.value = null
    }
  },
)

// ==================== 命中数据 ====================
/** 命中行 → 命中区间数组（构建一次，供渲染与标记使用） */
const hitMap = computed(() => {
  const m = new Map<number, TextRange[]>()
  for (const h of store.result.hits) m.set(h.lineIndex, h.ranges)
  return m
})
const hitLineSet = computed(() => store.result.hitLines)
const currentLineNo = computed(() => {
  const hit = store.currentHit
  return hit ? hit.lineIndex : 0
})

// ==================== 渲染缓存 ====================
/**
 * 缓存版本：仅在搜索条件或规则变化时整体失效。
 * 当前命中行不进缓存（每次实时重算），因此导航上/下一个只重算一行。
 */
// 关键：把当前源 ID 与编码纳入缓存版本，切换源 / 切换编码后强制失效，
// 否则改编码 re-parse 后行号相同会命中旧的（乱码）渲染结果，导致“编码修改无效果”。
const cacheVersion = computed(() => {
  const s = store.search
  const rulesSig = store.rules
    .map(r => `${r.id}:${r.pattern}:${r.useRegex ? 1 : 0}:${r.ignoreCase ? 1 : 0}:${r.color}:${r.enabled ? 1 : 0}`)
    .join('|')
  return `${store.activeSourceId} ${store.encoding} ${s.keyword} ${s.mode} ${s.ignoreCase ? 1 : 0} ${rulesSig}`
})


/**
 * 切换源或切换编码时，旧的行渲染缓存已失效，立即清空避免显示旧内容。
 * 即便 cacheVersion 已含这两个维度，这里再做一次显式清空作为双保险。
 */
watch(
  () => [store.activeSourceId, store.encoding],
  () => htmlCache.clear(),
)
const htmlCache = new Map<string, string>()

function htmlFor(line: ParsedLine): string {
  const ranges = hitMap.value.get(line.index)
  const cur = store.currentHit
  // 当前聚焦命中所在行：实时渲染以反映 currentRangeIndex
  if (cur && cur.lineIndex === line.index) {
    return renderLine(line, {
      hitRanges: ranges,
      currentHit: cur.ranges[store.currentRangeIndex] ?? cur.ranges[0] ?? null,
      rules: store.rules,
    })
  }
  const key = `${cacheVersion.value}#${line.index}`
  const cached = htmlCache.get(key)
  if (cached !== undefined) return cached
  const html = renderLine(line, { hitRanges: ranges, rules: store.rules })
  if (htmlCache.size > 8000) htmlCache.clear()
  htmlCache.set(key, html)
  return html
}

// ==================== 比例虚拟滚动 ====================
/**
 * 滚动条高度必须按**逻辑总行数**算，不能按已加载行数（§3.1）。
 *
 * 按已加载行数算的话，滚动条量的是"加载进度"而不是"数据位置"：
 * 拖到底只是当前加载边界，松手触发加载后边界又往前跑、滚动条同时变长，
 * 于是表现为"滚到底还在加载"，且永远到不了文件尾部。
 */
const logicalTotal = computed(() => store.logicalTotal)

/**
 * 浏览器 `scrollHeight` 有上限（Chromium / Firefox 约 3355 万 px），
 * 超过后滚动位置会失准。行数大到 `LINE_HEIGHT × total` 顶到该上限时，
 * 只能按比例压缩行高——绝大多数日志（150 万行内）仍是完整的 20px，不受影响。
 */
const MAX_SCROLL_PX = 33_000_000
/** 压缩下限：再矮就无法阅读，此时宁可提示"请先筛选"也不继续压 */
const MIN_ROW_HEIGHT = 8
const rowHeight = computed(() => {
  const t = logicalTotal.value
  if (!t) return LINE_HEIGHT
  return Math.max(MIN_ROW_HEIGHT, Math.min(LINE_HEIGHT, MAX_SCROLL_PX / t))
})
/** 压缩到下限后仍放不下：滚不到最后一行，必须让用户知道（由工具栏提示） */
const scrollCapped = computed(() => logicalTotal.value * MIN_ROW_HEIGHT > MAX_SCROLL_PX)

const totalHeight = computed(() => logicalTotal.value * rowHeight.value)
const gutterWidth = computed(() =>
  Math.max(48, String(store.activeSource?.totalLines ?? logicalTotal.value).length * 8 + 16),
)
const visibleStart = computed(() => Math.max(0, Math.floor(scrollTop.value / rowHeight.value) - 5))
const visibleCount = computed(() => Math.ceil(viewportHeight.value / rowHeight.value) + 10)
const offsetY = computed(() => visibleStart.value * rowHeight.value)

interface RowItem {
  /** 逻辑下标（滚动条上的第几个槽）——用作 key，稳定且不随加载变化 */
  idx: number
  /** 原始行号；未加载的槽也知道自己的行号，可以灰显出来 */
  lineNo: number
  /** 行内容；null = 尚未加载，渲染成等高空槽 */
  line: ParsedLine | null
  html: string
}

const visibleLines = computed<RowItem[]>(() => {
  const total = logicalTotal.value
  const start = visibleStart.value
  const end = Math.min(total, start + visibleCount.value)
  if (end <= start) return []
  const arr = store.linesForRange(start, end)
  const out: RowItem[] = []
  for (let i = 0; i < arr.length; i++) {
    const idx = start + i
    const line = arr[i]
    out.push({
      idx,
      lineNo: line ? line.index : store.lineNoAt(idx),
      line,
      html: line ? htmlFor(line) : '',
    })
  }
  return out
})

const sourceColor = computed(() => store.activeSource?.color || 'transparent')

function levelText(line: ParsedLine): string {
  if (line.isStack) return '└'
  return line.level === 'UNKNOWN' ? '' : line.level
}

/**
 * 滚动补载用**防抖 + 只认最后一次**：拖动滚动条会连续触发几十次 scroll 事件，
 * 每次都发一次读盘请求会把后端和 UI 一起拖垮。
 */
let loadTimer: number | null = null
function scheduleRangeLoad() {
  if (loadTimer !== null) clearTimeout(loadTimer)
  loadTimer = window.setTimeout(() => {
    loadTimer = null
    void store.ensureRangeLoaded(visibleStart.value, visibleStart.value + visibleCount.value)
  }, 120)
}

function onScroll() {
  const el = rootRef.value
  if (!el) return
  scrollTop.value = el.scrollTop
  // 记忆当前源滚到的**逻辑下标**（不是像素：行高会随总行数压缩，记像素会错位）
  store.setScroll(store.activeSourceId, Math.round(scrollTop.value / rowHeight.value))
  // 按比例滚动后不需要"触底续读 / 触顶回补"：视口落在哪就按需搬窗口
  scheduleRangeLoad()
}

// ==================== 自绘滚动条 ====================
/** 刻度线长度（轨道宽 10px，刻度画在左侧） */
const TICK_W = 6

/**
 * 命中刻度：把命中行号映射成滚动条轨道上的 y（klogg 风格）。
 *
 * **只在全量态画**——筛选态下滚动条上每一条都是命中，画满等于没画，没有信息量。
 * 命中可能上百万条，按步长采样后去重到轨道像素级（一个 y 最多一条刻度）。
 */
const hitTicks = computed<number[]>(() => {
  if (store.filterActive) return []
  const nav = store.navLines
  const total = logicalTotal.value
  const h = viewportHeight.value
  if (!nav.length || !total || !h) return []
  const step = Math.max(1, Math.ceil(nav.length / 4000))
  const seen = new Set<number>()
  const out: number[] = []
  for (let i = 0; i < nav.length; i += step) {
    const y = Math.floor(((nav[i] - 1) / total) * h)
    if (y >= 0 && y < h && !seen.has(y)) {
      seen.add(y)
      out.push(y)
    }
  }
  return out
})
const tickPath = computed(() => hitTicks.value.map(y => `M1 ${y + 0.5}h${TICK_W}`).join(''))

const thumbHeight = computed(() => {
  const t = totalHeight.value
  const h = viewportHeight.value
  if (!t || !h) return 0
  return Math.max(24, Math.min(h, (h / t) * h))
})
const thumbTop = computed(() => {
  const t = totalHeight.value
  const h = viewportHeight.value
  if (!t || !h) return 0
  const raw = (scrollTop.value / t) * h
  return Math.min(Math.max(0, raw), Math.max(0, h - thumbHeight.value))
})

const dragging = ref(false)
/** 拖动结束时解绑 window 监听；组件卸载时也要用它兜底 */
let stopDrag: (() => void) | null = null

function onThumbDown(e: MouseEvent) {
  const el = rootRef.value
  if (!el) return
  dragging.value = true
  const startY = e.clientY
  const startTop = el.scrollTop
  // 轨道像素 → 内容像素的放大倍率（内容比视口长多少倍）
  const ratio = totalHeight.value / Math.max(1, viewportHeight.value)
  const move = (ev: MouseEvent) => {
    el.scrollTop = Math.max(0, startTop + (ev.clientY - startY) * ratio)
  }
  const up = () => {
    dragging.value = false
    stopDrag = null
    window.removeEventListener('mousemove', move)
    window.removeEventListener('mouseup', up)
  }
  stopDrag = up
  window.addEventListener('mousemove', move)
  window.addEventListener('mouseup', up)
}

/** 点击轨道：把该位置**居中**（直接跳到顶部对齐会让视线丢失参照） */
function onTrackDown(e: MouseEvent) {
  const el = rootRef.value
  if (!el) return
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
  const y = e.clientY - rect.top
  const contentH = totalHeight.value
  const trackH = rect.height
  el.scrollTop = Math.max(0, Math.min(contentH - trackH, (y / trackH) * contentH - trackH / 2))
}

// ==================== 悬停浮层 ====================
function onRowEnter(e: MouseEvent, line: ParsedLine) {
  const el = rootRef.value
  if (!el) return
  const target = e.currentTarget as HTMLElement
  const rowTop = target.offsetTop
  hoverLine.value = line
  hoverTop.value = rowTop - el.scrollTop + rowHeight.value + 4
  hoverLeft.value = Math.min(120, Math.max(0, el.clientWidth - 360))
}
function onRowLeave() {
  hoverLine.value = null
}

// ==================== 行右键菜单（复制） ====================
const ctxVisible = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxLine = ref<ParsedLine | null>(null)

function onRowContextMenu(e: MouseEvent, line: ParsedLine) {
  ctxLine.value = line
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxVisible.value = true
}

function closeCtx() {
  ctxVisible.value = false
  ctxLine.value = null
}

/** 复制文本到剪贴板（优先 Clipboard API，回退到 execCommand） */
function copyText(text: string) {
  try {
    if (navigator.clipboard?.writeText) {
      void navigator.clipboard.writeText(text)
      return
    }
  } catch {
    /* 回退 */
  }
  const ta = document.createElement('textarea')
  ta.value = text
  ta.style.position = 'fixed'
  ta.style.opacity = '0'
  document.body.appendChild(ta)
  ta.select()
  try {
    document.execCommand('copy')
  } catch {
    /* ignore */
  }
  document.body.removeChild(ta)
}

function ctxCopyLine() {
  const l = ctxLine.value
  closeCtx()
  if (l) copyText(`${l.index}\t${l.raw}`)
}
function ctxCopyContent() {
  const l = ctxLine.value
  closeCtx()
  if (l) copyText(l.raw)
}
function ctxCopyNumber() {
  const l = ctxLine.value
  closeCtx()
  if (l) copyText(String(l.index))
}

/**
 * 在完整日志中定位当前行（右键菜单）。
 *
 * 筛选态下看到的是不连续的子集，"这一行前后发生了什么"必须回到完整日志才看得见。
 * 行号恒为原始行号，所以退出筛选后按同一个行号定位即可，不需要反查映射。
 */
async function ctxRevealInFull() {
  const l = ctxLine.value
  closeCtx()
  if (!l) return
  await store.revealInFullLog(l.index)
  // 退出筛选会整体换掉 displayLines，必须等 DOM 高度重算后再滚动，否则被旧高度裁掉
  await nextTick()
  scrollToLine(l.index)
}

// ==================== 对外方法 ====================
/**
 * 滚动到指定行（命中导航 / 时间轴联动用），居中显示。
 *
 * 比例滚动下"滚动位置"与"行号"是同一个量的两种表示，跳转只需反算 scrollTop：
 * 全量态 `idx = lineNo - 1`；筛选态行号不连续，用 `indexOfLineNo` 查下标。
 * 调用方负责先 `ensureLineLoaded`，否则这里只改 scrollTop、内容由滚动补载去填。
 */
function scrollToLine(lineNo: number) {
  const el = rootRef.value
  if (!el || lineNo <= 0) return
  const idx = store.indexOfLineNo(lineNo)
  if (idx < 0) return
  const target = Math.max(0, idx * rowHeight.value - el.clientHeight / 2 + rowHeight.value)
  el.scrollTop = target
  scrollTop.value = target
  scheduleRangeLoad()
}

defineExpose({ scrollToLine, scrollCapped })

onMounted(() => {
  const el = rootRef.value
  if (!el) return
  viewportHeight.value = el.clientHeight || 600
  ro = new ResizeObserver(entries => {
    for (const entry of entries) viewportHeight.value = entry.contentRect.height
  })
  ro.observe(el)
  document.addEventListener('click', closeCtx)
})

onBeforeUnmount(() => {
  ro?.disconnect()
  ro = null
  document.removeEventListener('click', closeCtx)
  if (loadTimer !== null) clearTimeout(loadTimer)
  stopDrag?.()
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.log-view {
  position: relative;
  flex: 1;
  min-height: 0;
  // 自身不滚动：滚动交给内层 .log-scroll-area，滚动条才能待在外层不随内容滚走
  overflow: hidden;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-base);
  line-height: 20px;
  // 主面板根背景跟随 appMainBgColor，与 Layout/.app-main 同源
  @include theme.useTheme {
    background: theme.getVar('appMainBgColor');
  }

  :deep(.log-row) {
    display: flex;
    align-items: center;
    white-space: pre;
    padding-right: 12px;

    &:hover {
      background: var(--log-row-hover-bg);
    }

    &.is-current {
      background: var(--log-row-current-bg);
    }
  }
}

/** 滚动内容区：原生滚动条隐藏，由下方 .log-scrollbar 代劳（要在轨道上画命中刻度） */
.log-scroll-area {
  // 用绝对铺满而非 height:100%：父级是 min-height:0 的 flex item，百分比高度在此不可靠
  position: absolute;
  inset: 0;
  overflow: auto;
  scrollbar-width: none;

  &::-webkit-scrollbar {
    width: 0;
    height: 0;
  }
}

// ==================== 自绘滚动条 ====================
.log-scrollbar {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 10px;
  z-index: 6;
  opacity: 0;
  transition: opacity 0.12s;
  background: var(--el-fill-color-lighter);

  // 默认隐藏，避免遮挡行尾内容；鼠标进入视图或拖动时才现身
  .log-view:hover &,
  &:hover,
  &.is-dragging {
    opacity: 1;
  }
}

.log-ticks {
  position: absolute;
  inset: 0;
  pointer-events: none;

  path {
    stroke: var(--el-color-primary);
    opacity: 0.85;
  }
}

.log-thumb {
  position: absolute;
  top: 0;
  left: 1px;
  right: 1px;
  border-radius: 4px;
  background: var(--el-border-color-darker);
  opacity: 0.8;

  .log-scrollbar:hover &,
  .log-scrollbar.is-dragging & {
    opacity: 1;
    background: var(--el-color-info);
  }
}

.log-scroll {
  position: relative;
  width: 100%;
}

.log-window {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  will-change: transform;
}

.log-row {
  display: flex;
  align-items: center;
  overflow: hidden;
  white-space: pre;

  &:hover {
    background: var(--log-row-hover-bg);
  }

  &.is-current {
    background: var(--log-row-current-bg);
  }

  &.is-stack {
    opacity: 0.92;
  }

  /** 尚未加载的槽：只画等高占位 + 灰行号，使滚动条高度恒定、拖动时不白屏 */
  &.is-pending {
    .log-gutter {
      color: var(--el-text-color-placeholder);
      opacity: 0.45;
    }
  }

  &.log-row-error,
  &.log-row-fatal {
    background: var(--log-level-error-bg);
    color: var(--log-level-error-fg);
  }

  &.log-row-warn {
    background: var(--log-level-warn-bg);
    color: var(--log-level-warn-fg);
  }

  &.log-row-info {
    color: var(--log-level-info-fg);
  }

  &.log-row-debug {
    color: var(--log-level-debug-fg);
  }

  &.log-row-trace {
    color: var(--log-level-trace-fg);
  }
}

.log-dot {
  flex: none;
  width: 3px;
  height: 100%;
  margin-right: 4px;
  opacity: 0.85;
}

.log-gutter {
  flex: none;
  padding-right: 8px;
  text-align: right;
  color: var(--log-gutter-fg);
  user-select: none;
  font-size: var(--font-size-sm);
}

.log-badge {
  flex: none;
  width: 44px;
  font-size: var(--font-size-xs);
  color: var(--log-gutter-fg);
  user-select: none;
}

.log-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.log-empty {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
}

.log-empty-text {
  margin: 0;
  color: var(--el-text-color-secondary);
}

.log-empty-hint {
  margin: 0;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-placeholder);
}

.log-more-tip {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 7;
  padding: 3px 8px;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
  background: var(--el-bg-color-overlay);
  border-top: 1px solid var(--el-border-color-lighter);
}

.log-hover {
  position: absolute;
  z-index: 20;
  min-width: 240px;
  max-width: 420px;
  padding: 6px 8px;
  border-radius: var(--radius-md);
  background: var(--el-bg-color-overlay);
  border: 1px solid var(--el-border-color-light);
  box-shadow: var(--shadow-md);
  font-family: var(--font-family-base);
  font-size: var(--font-size-sm);
  line-height: 1.7;
  pointer-events: none;
}

.log-hover-row {
  display: flex;
  gap: 8px;

  .k {
    flex: none;
    width: 36px;
    color: var(--el-text-color-secondary);
  }

  .v {
    flex: 1;
    min-width: 0;
    word-break: break-all;
    color: var(--el-text-color-primary);
  }
}

.log-ctx-menu {
  position: fixed;
  z-index: 1600;
  min-width: 150px;
  padding: 4px 0;
  background: var(--el-bg-color-overlay);
  border: 1px solid var(--el-border-color-light);
  border-radius: var(--radius-md);
  box-shadow: var(--el-box-shadow-light);
}

.log-ctx-sep {
  height: 1px;
  margin: 4px 0;
  background: var(--el-border-color-lighter);
}

.log-ctx-item {
  padding: 5px 12px;
  font-size: var(--font-size-md);
  cursor: pointer;
  color: var(--el-text-color-regular);

  &:hover {
    background: var(--el-fill-color-light);
  }
}
</style>
