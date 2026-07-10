<template>
  <div class="log-viewer">
    <div class="toolbar">
      <el-input v-model="keyword" placeholder="搜索关键词" @keyup.enter="doSearch" />
      <el-button @click="prevMatch" :disabled="currentMatch <= 1">上一个</el-button>
      <el-button @click="nextMatch" :disabled="currentMatch >= totalMatches">下一个</el-button>
      <el-input-number v-model="jumpTo" :min="1" :max="totalMatches" @change="jumpToMatch" />
      <span>{{ currentMatch }} / {{ totalMatches }}</span>
      <span v-if="totalLines > 0" class="line-info">共 {{ totalLines }} 行</span>
      <span v-if="isLoadingChunk" class="loading-tip">加载中...</span>
    </div>

    <div class="code-viewer" ref="viewerRef" @scroll="onScroll">
      <div
        v-for="(line, idx) in lines"
        :key="firstLineNumber + idx"
        class="code-line"
        :class="{
          highlight: highlightSet.has(firstLineNumber + idx),
          target: firstLineNumber + idx === targetLineNumber
        }"
        :ref="(el) => (firstLineNumber + idx === targetLineNumber ? (targetRef = el) : null)"
      >
        <span class="line-number">{{ firstLineNumber + idx }}</span>
        <span class="line-content" v-html="highlightText(line)"></span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts" name="TextView">
import { ref, watch, nextTick, onMounted, onActivated, onDeactivated } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const CHUNK_SIZE = 1000
const OVERLAP = 100
const SCROLL_THRESHOLD = 50

const props = defineProps<{
  path: string
}>()

const keyword = ref('')
const searchId = ref('')
const currentMatch = ref(0)
const totalMatches = ref(0)
const jumpTo = ref(1)
const totalLines = ref(0)

const lines = ref<string[]>([])
const firstLineNumber = ref(1)
const highlightSet = ref(new Set<number>())
const targetLineNumber = ref(0)
const targetRef = ref<HTMLElement | null>(null)
const viewerRef = ref<HTMLElement | null>(null)

let isDeactivated = false
let isLoaded = false
let chunkLoading = false
const isLoadingChunk = ref(false)

let currentStartLine = 1
let currentEndLine = 0

let scrollTimer: ReturnType<typeof setTimeout> | null = null

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
    startLine: currentStartLine,
    endLine: currentEndLine
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
  currentStartLine = state.startLine
  currentEndLine = state.endLine
  isLoaded = true
  nextTick(() => {
    if (viewerRef.value) {
      viewerRef.value.scrollTop = state.scrollTop
    }
  })
  return true
}

async function loadChunk(filePath: string, startLine: number) {
  if (isLoadingChunk) return
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
      currentStartLine = startLine
      currentEndLine = startLine + newLines.length - 1
      if (!data.has_more) {
        currentEndLine = data.total_lines
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
  if (currentEndLine >= totalLines.value) return

  const nextStart = currentEndLine - OVERLAP + 1
  if (nextStart <= 0) return

  const el = viewerRef.value
  const oldScrollHeight = el?.scrollHeight || 0

  const ok = await loadChunk(props.path, nextStart)
  if (ok) {
    await nextTick()
    if (el) {
      const newScrollHeight = el.scrollHeight
      const heightDiff = newScrollHeight - oldScrollHeight
      el.scrollTop = el.scrollTop + heightDiff
    }
  }
}

async function loadPrevChunk() {
  if (!props.path || chunkLoading) return
  if (currentStartLine <= 1) return

  let prevStart = currentStartLine - CHUNK_SIZE + OVERLAP
  if (prevStart < 1) prevStart = 1

  const ok = await loadChunk(props.path, prevStart)
  if (ok) {
    await nextTick()
    if (viewerRef.value) {
      viewerRef.value.scrollTop = 0
    }
  }
}

function onScroll() {
  if (chunkLoading || !viewerRef.value) return

  if (scrollTimer) clearTimeout(scrollTimer)
  scrollTimer = setTimeout(() => {
    const el = viewerRef.value
    if (!el) return

    const isAtBottom = el.scrollHeight - el.scrollTop - el.clientHeight < SCROLL_THRESHOLD
    const isAtTop = el.scrollTop < SCROLL_THRESHOLD

    if (isAtBottom) {
      loadNextChunk()
    } else if (isAtTop) {
      loadPrevChunk()
    }
  }, 150)
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
  targetRef.value?.scrollIntoView({ block: 'center', behavior: 'smooth' })
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

onMounted(async () => {
  if (props.path) {
    await loadFirstChunk(props.path)
  }
})

onActivated(() => {
  isDeactivated = false
  if (props.path && !restoreState(props.path)) {
    if (!isLoaded) {
      loadFirstChunk(props.path)
    }
  }
})

onDeactivated(() => {
  isDeactivated = true
  if (scrollTimer) {
    clearTimeout(scrollTimer)
    scrollTimer = null
  }
  saveCurrentState()
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as *;

.log-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
}
.toolbar {
  display: flex;
  gap: 8px;
  padding: 8px;
  align-items: center;
  @include useTheme {
    border-bottom: 1px solid getVar(borderColor);
    background-color: getVar(bgColor);
  }
}
.line-info {
  font-size: 12px;
  white-space: nowrap;
  @include useTheme {
    color: getVar(textColor);
    opacity: 0.5;
  }
}
.loading-tip {
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
