<template>
  <div ref="rootRef" class="log-view" @scroll.passive="onScroll">
    <!-- 空态 -->
    <div v-if="!lineCount" class="log-empty">
      <p v-if="store.loading" class="log-empty-text">加载中…</p>
      <template v-else>
        <p class="log-empty-text">从左侧打开一个日志文件开始分析</p>
        <p class="log-empty-hint">支持大文件分块懒加载 · 关键词/正则搜索 · 级别配色 · 堆栈高亮</p>
      </template>
    </div>

    <!-- 虚拟滚动区 -->
    <div v-else class="log-scroll" :style="{ height: totalHeight + 'px' }">
      <div class="log-window" :style="{ transform: `translateY(${offsetY}px)` }">
        <div
          v-for="row in visibleLines"
          :key="row.line.index"
          class="log-row"
          :class="[
            `log-row-${row.line.level.toLowerCase()}`,
            {
              'is-hit': hitLineSet.has(row.line.index),
              'is-current': row.line.index === currentLineNo,
              'is-stack': row.line.isStack,
            },
          ]"
          :style="{ height: LINE_HEIGHT + 'px' }"
          @mouseenter="onRowEnter($event, row.line)"
          @mouseleave="onRowLeave"
          @contextmenu.prevent="onRowContextMenu($event, row.line)"
        >
          <span class="log-dot" :style="{ background: sourceColor }"></span>
          <span class="log-gutter" :style="{ width: gutterWidth + 'px' }">{{ row.line.index }}</span>
          <span class="log-badge" :class="`lv-${row.line.level.toLowerCase()}`">{{ levelText(row.line) }}</span>
          <span class="log-text" v-html="row.html"></span>
        </div>
      </div>
    </div>

    <!-- 加载更多提示 -->
    <div v-if="store.loadingMore" class="log-more-tip">加载更多…</div>

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
      store.setScroll(lastActiveId, el.scrollTop)
    }
    pendingScrollRestore.value = newId
    lastActiveId = newId
    if (el) {
      const target = store.getScroll(newId) ?? 0
      el.scrollTop = target
      scrollTop.value = target
    }
  },
)

// 内容就绪（含异步筛选结果）后，再次把记忆的滚动位置应用到新源
watch(
  () => store.displayLines.length,
  len => {
    if (pendingScrollRestore.value && len > 0) {
      const el = rootRef.value
      if (el) {
        const target = store.getScroll(pendingScrollRestore.value) ?? 0
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

// ==================== 虚拟滚动 ====================
/** 实际渲染的行集：筛选态下是命中行（行号不连续），否则为全量加载行 */
const lineCount = computed(() => store.displayLines.length)
const totalHeight = computed(() => lineCount.value * LINE_HEIGHT)
const gutterWidth = computed(() => Math.max(48, String(store.activeSource?.totalLines ?? 0).length * 8 + 16))
const visibleStart = computed(() => Math.max(0, Math.floor(scrollTop.value / LINE_HEIGHT) - 5))
const visibleCount = computed(() => Math.ceil(viewportHeight.value / LINE_HEIGHT) + 10)
const offsetY = computed(() => visibleStart.value * LINE_HEIGHT)

interface RowItem {
  line: ParsedLine
  html: string
}

const visibleLines = computed<RowItem[]>(() => {
  const arr = store.displayLines
  const start = visibleStart.value
  const end = Math.min(arr.length, start + visibleCount.value)
  const out: RowItem[] = []
  for (let i = start; i < end; i++) {
    const line = arr[i]
    out.push({ line, html: htmlFor(line) })
  }
  return out
})

const sourceColor = computed(() => store.activeSource?.color || 'transparent')

function levelText(line: ParsedLine): string {
  if (line.isStack) return '└'
  return line.level === 'UNKNOWN' ? '' : line.level
}

function onScroll() {
  const el = rootRef.value
  if (!el) return
  scrollTop.value = el.scrollTop
  // 记忆当前源的滚动位置，切回该 tab 时恢复
  store.setScroll(store.activeSourceId, el.scrollTop)
  // 触底预取下一块
  if (el.scrollHeight - el.scrollTop - el.clientHeight < 240) {
    void store.loadMore()
  }
}

// ==================== 悬停浮层 ====================
function onRowEnter(e: MouseEvent, line: ParsedLine) {
  const el = rootRef.value
  if (!el) return
  const target = e.currentTarget as HTMLElement
  const rowTop = target.offsetTop
  hoverLine.value = line
  hoverTop.value = rowTop - el.scrollTop + LINE_HEIGHT + 4
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

// ==================== 对外方法 ====================
/**
 * 滚动到指定行（命中导航 / 时间轴联动用），居中显示。
 * 全量态：行号与数组下标一一对应；
 * 筛选态：命中行号不连续，用 store.positionOfLine 映射到 displayLines 下标。
 */
function scrollToLine(lineNo: number) {
  const el = rootRef.value
  if (!el || lineNo <= 0) return
  let displayIdx: number
  if (store.filterActive) {
    displayIdx = store.positionOfLine(lineNo)
    if (displayIdx < 0) return
  } else {
    displayIdx = lineNo - 1
  }
  const target = Math.max(0, displayIdx * LINE_HEIGHT - el.clientHeight / 2 + LINE_HEIGHT)
  el.scrollTop = target
  scrollTop.value = target
}

defineExpose({ scrollToLine })

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
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.log-view {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow: auto;
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
  position: sticky;
  bottom: 0;
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
