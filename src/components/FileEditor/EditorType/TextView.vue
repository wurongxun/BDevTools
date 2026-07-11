<template>
  <div class="log-viewer" @keydown="onKeydown" tabindex="0">
    <!-- 悬浮搜索框 -->
    <transition name="search-fade">
      <div v-if="showSearchBar" class="search-bar-overlay">
        <div class="search-bar">
          <el-input
            ref="searchInputRef"
            v-model="keyword"
            placeholder="搜索关键词"
            size="small"
            @keyup.enter="doSearch"
            @keydown.escape="showSearchBar = false"
            class="search-input"
          >
            <template #prefix>
              <el-icon><Search /></el-icon>
            </template>
          </el-input>
          <div class="search-actions">
            <el-button size="small" @click="prevMatch" :disabled="currentMatch <= 1">
              <el-icon><ArrowUp /></el-icon>
            </el-button>
            <el-button size="small" @click="nextMatch" :disabled="currentMatch >= totalMatches">
              <el-icon><ArrowDown /></el-icon>
            </el-button>
            <el-input-number v-model="jumpTo" :min="1" :max="totalMatches" size="small" @change="jumpToMatch" controls-position="compact" />
            <span class="match-count">{{ currentMatch }} / {{ totalMatches }}</span>
          </div>
          <el-button size="small" text @click="showSearchBar = false">
            <el-icon><Close /></el-icon>
          </el-button>
        </div>
      </div>
    </transition>
    <!-- 虚拟滚动容器 -->
    <div class="code-viewer" ref="viewerRef" @scroll="onVirtualScroll">
      <div class="virtual-spacer" :style="{ height: totalContentHeight + 'px' }">
        <div
          v-for="(item, idx) in visibleItems"
          :key="item.lineNumber"
          class="code-line virtual-line"
          :class="{
            highlight: highlightSet.has(item.lineNumber),
            target: item.lineNumber === targetLineNumber
          }"
          :style="{ transform: 'translateY(' + item.offset + 'px)' }"
        >
          <span class="line-number">{{ item.lineNumber }}</span>
          <span class="line-content" v-html="highlightText(item.text)"></span>
        </div>
      </div>
    </div>
    <!-- 块导航栏 -->
    <div v-if="totalBlocks > 1" class="block-nav">
      <el-button size="small" @click="goToFirstBlock" :disabled="currentBlock <= 1" title="第一块 (Home)">
        <el-icon><DArrowLeft /></el-icon>
      </el-button>
      <el-button size="small" @click="goToPrevBlock" :disabled="currentBlock <= 1" title="上一块 (PageUp)">
        <el-icon><ArrowLeft /></el-icon>
      </el-button>
      <div class="block-jump">
        <el-input-number
          v-model="jumpBlock"
          :min="1"
          :max="totalBlocks"
          size="small"
          controls-position="compact"
        />
        <span class="block-total">/ {{ totalBlocks }}</span>
      </div>
      <el-button size="small" @click="goToNextBlock" :disabled="currentBlock >= totalBlocks" title="下一块 (PageDown)">
        <el-icon><ArrowRight /></el-icon>
      </el-button>
      <el-button size="small" @click="goToLastBlock" :disabled="currentBlock >= totalBlocks" title="最后一块 (End)">
        <el-icon><DArrowRight /></el-icon>
      </el-button>
      <span class="line-range">行 {{ currentStartLine }}-{{ currentEndLine }}</span>
      <span v-if="totalLines > 0" class="line-info">共 {{ totalLines }} 行</span>
      <span v-if="isLoadingChunk" class="loading-tip">加载中...</span>
    </div>

  </div>
</template>

<script setup lang="ts" name="TextView">
import { ref, watch, nextTick, onMounted, onActivated, onDeactivated, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Search, ArrowUp, ArrowDown, Close, ArrowLeft, ArrowRight, DArrowLeft, DArrowRight } from '@element-plus/icons-vue'

/**
 * 虚拟滚动与分块加载核心配置
 * - CHUNK_SIZE:    每次从后端加载的行数，控制单次 IO 与内存占用
 * - OVERLAP:       相邻块之间的重叠行数，防止滚动切换时内容断层
 * - LINE_HEIGHT:   每行固定像素高度，用于虚拟滚动中计算偏移量与可视范围
 * - VIRTUAL_BUFFER: 可视区域上下额外渲染的行数，避免快速滚动时出现空白
 */
const CHUNK_SIZE = 10000
const OVERLAP = 50
const LINE_HEIGHT = 20
const VIRTUAL_BUFFER = 15

const props = defineProps<{
  path: string
}>()

// ... existing code ...

/** 搜索功能状态：关键词、搜索会话ID、匹配序号与总数、跳转目标、搜索栏显隐 */
const keyword = ref('')
const searchId = ref('')
const currentMatch = ref(0)
const totalMatches = ref(0)
const jumpTo = ref(1)
const totalLines = ref(0)
const showSearchBar = ref(false)

/** 内容渲染与虚拟滚动状态：行数据、起始行号、高亮行集合、目标行、DOM 引用 */
const lines = ref<string[]>([])
const firstLineNumber = ref(1)
const highlightSet = ref(new Set<number>())
const targetLineNumber = ref(0)
const viewerRef = ref<HTMLElement | null>(null)
const searchInputRef = ref()

/** 加载与生命周期标志：KeepAlive 停用标记、首次加载标记、块加载互斥锁 */
let isDeactivated = false
let isLoaded = false
let chunkLoading = false
const isLoadingChunk = ref(false)

/** 当前块的起止行号（1-based），用于块导航与重叠计算 */
const currentStartLine = ref(1)
const currentEndLine = ref(0)

/** 虚拟滚动状态：滚动防抖定时器、当前滚动偏移、容器可视高度、内容总高度 */
let scrollTimer: ReturnType<typeof setTimeout> | null = null
const scrollTopValue = ref(0)
const containerHeight = ref(0)

const totalContentHeight = computed(() => lines.value.length * LINE_HEIGHT)


interface VisibleItem {
  lineNumber: number
  text: string
  offset: number
}

const visibleItems = computed<VisibleItem[]>(() => {
  const total = lines.value.length
  if (total === 0) return []

  const startIdx = Math.max(0, Math.floor(scrollTopValue.value / LINE_HEIGHT) - VIRTUAL_BUFFER)
  const visibleCount = Math.ceil(containerHeight.value / LINE_HEIGHT)
  const endIdx = Math.min(total, startIdx + visibleCount + VIRTUAL_BUFFER * 2)

  const items: VisibleItem[] = []
  for (let i = startIdx; i < endIdx; i++) {
    items.push({
      lineNumber: firstLineNumber.value + i,
      text: lines.value[i],
      offset: i * LINE_HEIGHT
    })
  }
  return items
})



/**
 * 块导航计算属性组
 * 基于 CHUNK_SIZE 将文件划分为多个逻辑块，支持块号显示、跳转与双向绑定
 */

/**
 * 计算文件总块数
 * @returns {number} 总块数，文件为空时返回 1
 */
const totalBlocks = computed(() => {
  if (totalLines.value <= 0) return 1
  return Math.ceil(totalLines.value / CHUNK_SIZE)
})

/**
 * 块号双向绑定计算属性
 * 读取时返回当前所在块号，写入时跳转到指定块并将滚动条重置到顶部
 * @param {number} val - 目标块号（1-based）
 */
const jumpBlock = computed({
  get: () => currentBlock.value,
  set: (val: number) => {
    if (val < 1 || val > totalBlocks.value) return
    const startLine = (val - 1) * CHUNK_SIZE + 1
    loadChunk(props.path, startLine).then(() => {
      if (viewerRef.value) viewerRef.value.scrollTop = 0
    })
  }
})

/**
 * 计算当前所在块号
 * @returns {number} 当前块号（1-based），起始行为第 1 行时返回 1
 */
const currentBlock = computed(() => {
  if (currentStartLine.value <= 1) return 1
  return Math.ceil(currentStartLine.value / CHUNK_SIZE)
})




interface FileState {
  lines: string[]
  firstLineNumber: number
  highlightSet: Set<number>
  targetLineNumber: number
  currentMatch: number
  totalMatches: number
  jumpTo: number
  totalLines: number
  keyword: string
  searchId: string
  scrollTop: number
  startLine: number
  endLine: number
}

const savedState = new Map<string, FileState>()

function saveCurrentState() {
  const filePath = props.path
  if (!filePath) return
  const state: FileState = {
    lines: [...lines.value],
    firstLineNumber: firstLineNumber.value,
    highlightSet: new Set(highlightSet.value),
    targetLineNumber: targetLineNumber.value,
    currentMatch: currentMatch.value,
    totalMatches: totalMatches.value,
    jumpTo: jumpTo.value,
    totalLines: totalLines.value,
    keyword: keyword.value,
    searchId: searchId.value,
    scrollTop: viewerRef.value?.scrollTop || 0,
    startLine: currentStartLine.value,
    endLine: currentEndLine.value
  }
  savedState.set(filePath, state)
}

function restoreState(filePath: string): boolean {
  const state = savedState.get(filePath)
  if (!state) return false
  lines.value = state.lines
  firstLineNumber.value = state.firstLineNumber
  highlightSet.value = new Set(state.highlightSet)
  targetLineNumber.value = state.targetLineNumber
  currentMatch.value = state.currentMatch
  totalMatches.value = state.totalMatches
  jumpTo.value = state.jumpTo
  totalLines.value = state.totalLines
  keyword.value = state.keyword
  searchId.value = state.searchId
  currentStartLine.value = state.startLine
  currentEndLine.value = state.endLine
  isLoaded = true
  nextTick(() => {
    if (viewerRef.value) {
      viewerRef.value.scrollTop = state.scrollTop
    }
  })
  return true
}

async function loadChunk(filePath: string, startLine: number) {
  if (isLoadingChunk.value) return
  chunkLoading = true;
  isLoadingChunk.value = true
  try {
    const res: any = await invoke('read_file_lines', {
      filePath,
      startLine,
      lineCount: CHUNK_SIZE
    })
    if (res && res.code === 200) {
      const data = res.data
      const newLines = data.content.split('\n').filter((l: string) => l.length > 0 || true)
      lines.value = newLines
      firstLineNumber.value = data.first_line_number
      totalLines.value = data.total_lines
      currentStartLine.value = startLine
      currentEndLine.value = startLine + newLines.length - 1
      if (!data.has_more) {
        currentEndLine.value = data.total_lines
      }
      isLoaded = true
      return true
    }
  } catch (e) {
    console.error('加载文件块失败:', e)
  } finally {
    chunkLoading = false
    isLoadingChunk.value = false
  }
  return false
}

async function loadFirstChunk(filePath: string) {
  const ok = await loadChunk(filePath, 1)
  if (ok) {
    highlightSet.value = new Set()
    targetLineNumber.value = 0
    currentMatch.value = 0
    totalMatches.value = 0
  }
}



async function loadNextChunk() {
  if (!props.path || chunkLoading) return
  if (currentEndLine.value >= totalLines.value) return

  const nextStart = currentEndLine.value - OVERLAP + 1
  if (nextStart <= 0) return

  const ok = await loadChunk(props.path, nextStart)
  if (ok) {
    await nextTick()
    if (viewerRef.value) {
      viewerRef.value.scrollTop = 0
    }
  }
}
async function loadPrevChunk() {
  if (!props.path || chunkLoading) return
  if (currentStartLine.value <= 1) return

  let prevStart = currentStartLine.value - CHUNK_SIZE + OVERLAP
  if (prevStart < 1) prevStart = 1

  const ok = await loadChunk(props.path, prevStart)
  if (ok) {
    await nextTick()
    if (viewerRef.value) {
      viewerRef.value.scrollTop = 0
    }
  }
}

function goToFirstBlock() {
  if (currentBlock.value <= 1) return
  loadChunk(props.path, 1).then(() => {
    if (viewerRef.value) viewerRef.value.scrollTop = 0
  })
}

function goToLastBlock() {
  if (currentBlock.value >= totalBlocks.value) return
  const startLine = (totalBlocks.value - 1) * CHUNK_SIZE + 1
  loadChunk(props.path, startLine).then(() => {
    if (viewerRef.value) viewerRef.value.scrollTop = 0
  })
}

function goToPrevBlock() {
  if (currentBlock.value <= 1) return
  const startLine = Math.max(1, (currentBlock.value - 2) * CHUNK_SIZE + 1)
  loadChunk(props.path, startLine).then(() => {
    if (viewerRef.value) viewerRef.value.scrollTop = 0
  })
}

function goToNextBlock() {
  if (currentBlock.value >= totalBlocks.value) return
  const startLine = currentBlock.value * CHUNK_SIZE + 1
  loadChunk(props.path, startLine).then(() => {
    if (viewerRef.value) viewerRef.value.scrollTop = 0
  })
}


function onVirtualScroll() {
  if (!viewerRef.value) return

  const el = viewerRef.value
  scrollTopValue.value = el.scrollTop
  containerHeight.value = el.clientHeight

  if (chunkLoading) return

  if (scrollTimer) clearTimeout(scrollTimer)
  scrollTimer = setTimeout(() => {
    const SCROLL_THRESHOLD = 50
    const isAtBottom = el.scrollHeight - el.scrollTop - el.clientHeight < SCROLL_THRESHOLD
    const isAtTop = el.scrollTop < SCROLL_THRESHOLD

    if (isAtBottom) {
      loadNextChunk()
    } else if (isAtTop) {
      loadPrevChunk()
    }
  }, 150)
}

function scrollToLineNumber(lineNumber: number) {
  if (!viewerRef.value) return
  const idx = lineNumber - firstLineNumber.value
  if (idx < 0 || idx >= lines.value.length) return
  const targetScroll = idx * LINE_HEIGHT - containerHeight.value / 2
  viewerRef.value.scrollTop = Math.max(0, targetScroll)
}

async function doSearch() {
  if (!keyword.value || !props.path) return
  const res: any = await invoke('search_log_index', {
    filePath: props.path,
    keyword: keyword.value
  })
  if (res && res.code === 200) {
    searchId.value = res.data.search_id
    totalMatches.value = res.data.total_matches
    currentMatch.value = 1
    await loadPreview(1)
  }
}

async function loadPreview(index: number) {
  const res: any = await invoke('search_log_preview', {
    searchId: searchId.value,
    targetIndex: index,
    contextLines: 50
  })
  if (res && res.code === 200) {
    const data = res.data
    lines.value = data.content.split('\n')
    firstLineNumber.value = data.first_line_number
    highlightSet.value = new Set(data.highlight_lines)
    targetLineNumber.value = data.target_line_number
    currentMatch.value = data.current_match_index
    jumpTo.value = data.current_match_index
  }

  await nextTick()
  scrollToLineNumber(targetLineNumber.value)
}

function nextMatch() {
  if (currentMatch.value < totalMatches.value) {
    loadPreview(currentMatch.value + 1)
  }
}

function prevMatch() {
  if (currentMatch.value > 1) {
    loadPreview(currentMatch.value - 1)
  }
}

function jumpToMatch(val: number) {
  loadPreview(val)
}

function highlightText(line: string): string {
  if (!keyword.value) return escapeHtml(line)
  const escaped = escapeHtml(line)
  const kw = escapeHtml(keyword.value)
  const regex = new RegExp(`(${kw.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi')
  return escaped.replace(regex, '<mark class="match-hl">$1</mark>')
}

function escapeHtml(str: string): string {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

watch(
  () => props.path,
  async (newPath) => {
    if (isDeactivated) return
    if (!newPath) return
    if (!restoreState(newPath)) {
      await loadFirstChunk(newPath)
    }
  }
)

function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
    e.preventDefault()
    showSearchBar.value = true
    nextTick(() => {
      searchInputRef.value?.focus()
    })
    return
  }

  if (showSearchBar.value) return

  if (e.key === 'Home') {
    e.preventDefault()
    goToFirstBlock()
  } else if (e.key === 'End') {
    e.preventDefault()
    goToLastBlock()
  } else if (e.key === 'PageUp') {
    e.preventDefault()
    goToPrevBlock()
  } else if (e.key === 'PageDown') {
    e.preventDefault()
    goToNextBlock()
  }
}

onMounted(async () => {
  if (props.path) {
    await loadFirstChunk(props.path)
  }
  if (viewerRef.value) {
    containerHeight.value = viewerRef.value.clientHeight
  }
  window.addEventListener('keydown', onKeydown)
})

onActivated(() => {
  isDeactivated = false
  if (props.path && !restoreState(props.path)) {
    if (!isLoaded) {
      loadFirstChunk(props.path)
    }
  }
  if (viewerRef.value) {
    containerHeight.value = viewerRef.value.clientHeight
  }
  window.addEventListener('keydown', onKeydown)
})

onDeactivated(() => {
  isDeactivated = true
  if (scrollTimer) {
    clearTimeout(scrollTimer)
    scrollTimer = null
  }
  showSearchBar.value = false
  saveCurrentState()
  window.removeEventListener('keydown', onKeydown)
})
</script>

<style scoped lang="scss">@use '@/styles/theme.scss' as *;

.log-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
  position: relative;
  outline: none;
}

.search-bar-overlay {
  position: absolute;
  top: 8px;
  right: 16px;
  z-index: 100;
}

.search-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
  @include useTheme {
    background-color: getVar(bgColor);
    border: 1px solid getVar(borderColor);
  }
}

.search-input {
  width: 220px;
}

.search-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.match-count {
  font-size: 12px;
  min-width: 60px;
  text-align: center;
  @include useTheme {
    color: getVar(textColor);
    opacity: 0.6;
  }
}

.search-fade-enter-active,
.search-fade-leave-active {
  transition: all 0.2s ease;
}

.search-fade-enter-from {
  opacity: 0;
  transform: translateY(-10px);
}

.search-fade-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}

.block-nav {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px;
  @include useTheme {
    background-color: getVar(bgColor);
    border-bottom: 1px solid getVar(borderColor);
  }
  flex-shrink: 0;
}

.block-jump {
  display: flex;
  align-items: center;
  gap: 4px;
}

.block-total {
  font-size: 12px;
  @include useTheme {
    color: getVar(textColor);
    opacity: 0.6;
  }
}

.line-range {
  font-size: 12px;
  margin-left: 8px;
  @include useTheme {
    color: getVar(textColor);
    opacity: 0.5;
  }
}

.line-info {
  position: absolute;
  bottom: 8px;
  right: 16px;
  font-size: 12px;
  white-space: nowrap;
  @include useTheme {
    color: getVar(textColor);
    opacity: 0.5;
  }
}

.loading-tip {
  position: absolute;
  bottom: 8px;
  left: 16px;
  font-size: 12px;
  color: #f5c542;
  white-space: nowrap;
}

.code-viewer {
  flex: 1;
  overflow: auto;
  font-family: 'Consolas', monospace;
  font-size: 13px;
  @include useTheme {
    background: getVar(bgColor);
  }
}

.virtual-spacer {
  position: relative;
  width: 100%;
}

.virtual-line {
  position: absolute;
  left: 0;
  right: 0;
  height: 20px;
  line-height: 20px;
  will-change: transform;
}

.code-line {
  display: flex;
  padding: 1px 8px;
  @include useTheme {
    color: getVar(textColor);
  }
}

.code-line.highlight {
  @include useTheme {
    background: rgba(255, 200, 0, 0.08);
  }
}

.code-line.target {
  @include useTheme {
    background: rgba(255, 200, 0, 0.2);
    border-left: 3px solid getVar(borderColor);
  }
}

.line-number {
  min-width: 60px;
  text-align: right;
  padding-right: 16px;
  user-select: none;
  @include useTheme {
    color: getVar(textColor);
    opacity: 0.4;
  }
}

.line-content {
  white-space: pre;
}

:deep(.match-hl) {
  background: #f5c542;
  color: #1e1e1e;
  border-radius: 2px;
  padding: 0 2px;
}
</style>
