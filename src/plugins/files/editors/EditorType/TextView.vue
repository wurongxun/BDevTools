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
            @keyup.enter="onSearchKeyup"
            @compositionstart="onCompositionStart()"
            @compositionend="onCompositionEnd()"
            @keydown.escape="showSearchBar = false"
            class="search-input"
          >
            <template #prefix>
              <el-icon><Search /></el-icon>
            </template>
          </el-input>
          <div class="search-actions">
            <app-button size="small" @click="prevMatch" :disabled="currentMatch <= 1">
              <el-icon><ArrowUp /></el-icon>
            </app-button>
            <el-input v-model="jumpTo" :min="1" :max="totalMatches" size="small" @change="jumpToMatch" controls-position="compact" />
            <app-button size="small" @click="nextMatch" :disabled="currentMatch >= totalMatches">
              <el-icon><ArrowDown /></el-icon>
            </app-button>
            <span class="match-count">{{ currentMatch }} / {{ totalMatches }}</span>
          </div>
          <app-button size="small" text @click="showSearchBar = false">
            <el-icon><Close /></el-icon>
          </app-button>
        </div>
      </div>
    </transition>
    <!-- 加载/搜索动画遮罩 -->
    <div v-if="showOverlay" class="loading-overlay">
      <div class="loading-spinner">
        <div class="spinner-ring"></div>
        <span class="loading-text">{{ overlayText }}</span>
      </div>
    </div>
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
    <div class="block-nav">
      <app-button size="small" @click="goToFirstBlock" :disabled="currentBlock <= 1 || totalBlocks <= 1 || isSearching" title="第一块 (Home)">
        <el-icon><DArrowLeft /></el-icon>
      </app-button>
      <app-button size="small" @click="goToPrevBlock" :disabled="currentBlock <= 1 || isSearching" title="上一块 (PageUp)">
        <el-icon><ArrowLeft /></el-icon>
      </app-button>
      <div class="block-jump">
        <el-input
          v-model="jumpBlockInput"
          type="number"
          size="small"
          :disabled="totalBlocks <= 1 || isSearching"
          @keyup.enter="onJumpBlockEnter"
          class="jump-input"
        />
        <span class="block-total">/{{ totalBlocks }}</span>
      </div>
      <app-button size="small" @click="goToNextBlock" :disabled="currentBlock >= totalBlocks || isSearching" title="下一块 (PageDown)">
        <el-icon><ArrowRight /></el-icon>
      </app-button>
      <app-button size="small" @click="goToLastBlock" :disabled="currentBlock >= totalBlocks || isSearching" title="最后一块 (End)">
        <el-icon><DArrowRight /></el-icon>
      </app-button>
      <span class="line-range">行 {{ currentStartLine }}-{{ currentEndLine }}</span>
      <el-dropdown trigger="click" @command="onEncodingChange" class="encoding-selector">
        <span class="encoding-label">{{ currentEncoding }}</span>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item v-for="enc in encodingOptions" :key="enc" :command="enc"
                              :class="{ 'is-active': enc === currentEncoding }">
              {{ enc }}
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
      <span v-if="totalLines > 0" class="line-info">共 {{ totalLines }} 行</span>
      <span v-if="isLoadingChunk" class="loading-tip">加载中...</span>
    </div>
  </div>
</template>

<script setup lang="ts" name="TextView">import { ref, watch, nextTick, onMounted, onActivated, onDeactivated,onBeforeUnmount, computed } from 'vue'
import { invoke } from '@/api/rpc'
import { Search, ArrowUp, ArrowDown, Close, ArrowLeft, ArrowRight, DArrowLeft, DArrowRight } from '@element-plus/icons-vue'
import NotificationUtil from '@/utils/NotificationUtil'
import useStore from '@/store'
import { useDocumentManagerStore } from '@/store/modules/documents'
const {appConfig } = useStore();
const docStore = useDocumentManagerStore()
/**
 * 虚拟滚动与分块加载核心配置
 * - CHUNK_SIZE:    每次从后端加载的行数，控制单次 IO 与内存占用
 * - OVERLAP:       相邻块之间的重叠行数，防止滚动切换时内容断层
 * - LINE_HEIGHT:   每行固定像素高度，用于虚拟滚动中计算偏移量与可视范围
 * - VIRTUAL_BUFFER: 可视区域上下额外渲染的行数，避免快速滚动时出现空白
 */
let CHUNK_SIZE = computed(() => appConfig.read_lines)

const OVERLAP = 50

const LINE_HEIGHT = 20
//渲染行数，用于虚拟滚动计算与渲染
const VIRTUAL_BUFFER = 15

const props = defineProps<{
  path: string
}>()

const emit = defineEmits<{
  /** 视图状态上报（滚动位置）：由 FileEditor 收口写进 docStore.viewState —— 切 tab / 切插件 / 重启都能恢复 */
  'update-state': [state: Record<string, unknown>]
}>()


/** 搜索功能状态：关键词、搜索会话ID、匹配序号与总数、跳转目标、搜索栏显隐 */
const keyword = ref('')
const searchId = ref('')
const currentMatch = ref(0)
const totalMatches = ref(0)
const jumpTo = ref(1)
const totalLines = ref(0)
const showSearchBar = ref(false)
const matchLineNumbers = ref<number[]>([])
/** 搜索功能状态：是否正在搜索 */
const isSearching = ref(false)
/** IME 输入法组合状态 */
let compositionEndTime = 0
/** 内容渲染与虚拟滚动状态：行数据、起始行号、高亮行集合、目标行、DOM 引用 */
const lines = ref<string[]>([])
const firstLineNumber = ref(1)
const highlightSet = ref(new Set<number>())
const targetLineNumber = ref(0)
const viewerRef = ref<HTMLElement | null>(null)
const searchInputRef = ref()

/** 加载与生命周期标志：KeepAlive 停用标记、首次加载标记、块加载互斥锁 */
let isDeactivated = false
/** 是否经历过 deactivate：用于区分「首次挂载后的 onActivated」与「切回来的 onActivated」 */
let hasDeactivated = false
let isLoaded = false
let chunkLoading = false
const isLoadingChunk = ref(false)
const MAX_CACHED_STATES = 5


// ========== 块预取缓存 ==========
interface ChunkCacheEntry {
  lines: string[]
  firstLineNumber: number
  startLine: number
  endLine: number
  hasMore: boolean
}
const chunkCache = new Map<string, ChunkCacheEntry>()
const MAX_CHUNK_CACHE = 3

function getCacheKey(path: string, startLine: number): string {
  return `${path}::${startLine}`
}

function getCachedChunk(path: string, startLine: number): ChunkCacheEntry | undefined {
  return chunkCache.get(getCacheKey(path, startLine))
}

function setCacheChunk(path: string, startLine: number, entry: ChunkCacheEntry) {
  const key = getCacheKey(path, startLine)
  if (chunkCache.has(key)) chunkCache.delete(key)
  chunkCache.set(key, entry)
  if (chunkCache.size > MAX_CHUNK_CACHE) {
    const oldest = chunkCache.keys().next().value
    if (oldest !== undefined) chunkCache.delete(oldest)
  }
}

function clearChunkCache() {
  chunkCache.clear()
}

/** 当前块的起止行号（1-based），用于块导航与重叠计算 */
const currentStartLine = ref(1)
const currentEndLine = ref(0)

/** 虚拟滚动状态：滚动防抖定时器、当前滚动偏移、容器可视高度、内容总高度 */
let scrollTimer: ReturnType<typeof setTimeout> | null = null
const scrollTopValue = ref(0)
const containerHeight = ref(0)
let scrollGuard = false
/** 观察滚动容器尺寸：虚拟窗口的可见行数依赖它 */
let ro: ResizeObserver | null = null

/** 加载动画遮罩状态 */
const showOverlay = ref(false)
const overlayText = ref('加载中...')
/** 块跳转输入值（回车才执行跳转） */
const jumpBlockInput = ref('1')

// ========== 编码相关 ==========
const currentEncoding = ref('UTF-8')
const encodingOptions = ['UTF-8', 'GBK', 'GB18030', 'BIG5', 'UTF-16LE', 'UTF-16BE', 'Shift_JIS', 'EUC-JP', 'EUC-KR', 'ASCII']


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
  return Math.ceil(totalLines.value / CHUNK_SIZE.value)
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
    const startLine = (val - 1) * CHUNK_SIZE.value + 1
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
  return Math.ceil(currentStartLine.value / CHUNK_SIZE.value)
})


// watch(currentBlock, (val) => {
//   jumpBlockInput.value = String(val)
// })

function onJumpBlockEnter() {
  const val = parseInt(jumpBlockInput.value)
  if (isNaN(val) || val < 1 || val > totalBlocks.value) {
    jumpBlockInput.value = String(currentBlock.value)
    return
  }
  jumpBlock.value = val
}

// 文件内容相关:
//
//   lines - 文件的所有行内容
// firstLineNumber - 第一行的行号(用于分块加载时的行号显示)
// totalLines - 文件总行数
// 搜索相关:
//
//   highlightSet - 需要高亮的行号集合(搜索匹配的行)
// keyword - 搜索关键词
// searchId - 搜索会话ID
// currentMatch - 当前匹配的序号
// totalMatches - 匹配总数
// targetLineNumber - 目标行号
// 视图位置相关:
//
//   jumpTo - 要跳转到的行号
// scrollTop - 滚动条位置
// startLine - 当前显示范围的起始行
// endLine - 当前显示范围的结束行

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
    startLine: currentStartLine.value,
    endLine: currentEndLine.value
  }
  savedState.set(filePath, state)
  if (savedState.size > MAX_CACHED_STATES) {
    const oldestKey = savedState.keys().next().value
    if (oldestKey !== undefined) {
      savedState.delete(oldestKey)
    }
  }
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
  // 滚动位置不在这里恢复：统一走 applyScrollTop —— 它还要同步虚拟窗口基准值，
  // 只设 DOM 会出现「渲染窗口在下方、视口在顶部」的空白（见 restoreView 注释）
  return true
}

async function loadChunk(filePath: string, startLine: number) {
  if (isLoadingChunk.value) return

  const cached = getCachedChunk(filePath, startLine)
  if (cached) {
    lines.value = cached.lines
    firstLineNumber.value = cached.firstLineNumber
    currentStartLine.value = cached.startLine
    currentEndLine.value = cached.endLine
    jumpBlockInput.value = String(Math.ceil(startLine / CHUNK_SIZE.value))
    isLoaded = true
    return true
  }

  chunkLoading = true;
  isLoadingChunk.value = true
  try {
    const res: any = await invoke('read_file_lines', {
      filePath,
      startLine,
      lineCount: Number(CHUNK_SIZE.value),
      encoding: currentEncoding.value
    })
    if (res && res.code === 200) {
      const data = res.data
      const newLines = data.content.split('\n').filter((l: string) => l.length > 0 || true)
      lines.value = newLines
      firstLineNumber.value = data.first_line_number
      if (data.total_lines > 0) {
        totalLines.value = data.total_lines
      }
      currentStartLine.value = startLine
      currentEndLine.value = startLine + newLines.length - 1
      if (!data.has_more) {
        currentEndLine.value = startLine + newLines.length - 1
        if (totalLines.value <= 0) {
          totalLines.value = startLine + newLines.length - 1
        }
      }
      jumpBlockInput.value = String(Math.ceil(startLine / CHUNK_SIZE.value))
      isLoaded = true

      setCacheChunk(filePath, startLine, {
        lines: newLines,
        firstLineNumber: data.first_line_number,
        startLine: currentStartLine.value,
        endLine: currentEndLine.value,
        hasMore: data.has_more
      })

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

async function loadFirstChunk(filePath: string): Promise<boolean> {
  showOverlay.value = true
  overlayText.value = '加载中...'
  const ok = await loadChunk(filePath, 1)
  showOverlay.value = false
  if (ok) {
    highlightSet.value = new Set()
    targetLineNumber.value = 0
    currentMatch.value = 0
    totalMatches.value = 0
    prefetchAdjacent(filePath, 1)
  }
  return !!ok
}



// ==================== 滚动位置的真源与恢复 ====================
//
// 为什么不能在 onDeactivated 里读 DOM 的 scrollTop（原实现就是这么写的）：
// 本组件被两层 KeepAlive 包裹（插件级 AppEditorMain、文档级 FileEditor）。切走时 Vue 把
// 组件根元素 move 到**脱离文档**的 storageContainer，实测此刻 el.scrollTop 立刻变 0
// （clientHeight / scrollHeight 也一起变 0），移回文档后**不会**自动恢复。
// 于是「切走再切回」读 DOM 只能拿到 0 —— 表现就是「默认回到顶部」。
// 对照：display:none（v-show）是可逆的，恢复显示后 scrollTop 会回到原值；detach 不可逆。
//
// 结论：位置必须在**用户滚动过程中**就记进 JS（scrollTopValue，onVirtualScroll 里维护），
// detach 之后只用这个值，并统一写进 docStore.viewState.scrollTop —— 与 CodeEditor 同一套管道，
// 切 tab / 切插件 / 重启应用三种场景一次解决（会话快照里的 scroll_top 就是它）。

/** 恢复位置期间置位：此刻 DOM 的值可能是被截断的 0，不能让它回写 scrollTopValue */
let restoringScroll = false

/** 读文档在 store 里记录的滚动位置（会话快照恢复的也是它） */
function storedScrollTop(filePath: string): number {
  const v = docStore.documents.get(filePath)?.viewState.scrollTop
  return typeof v === 'number' && v > 0 ? v : 0
}

/** 上报滚动位置（去抖）：写进 docStore.viewState，供切 tab / 切插件 / 重启恢复 */
let reportTimer: ReturnType<typeof setTimeout> | null = null
function reportScrollTop() {
  if (!props.path) return
  // 必须带上自己的 path：FileEditor 若用 activeDocumentPath 收口，切 tab 时会把位置写到新文档上（串号）
  emit('update-state', { path: props.path, scrollTop: scrollTopValue.value })
}
function scheduleScrollReport() {
  if (reportTimer) clearTimeout(reportTimer)
  reportTimer = setTimeout(() => {
    reportTimer = null
    reportScrollTop()
  }, 250)
}

/**
 * 把滚动位置应用到真实滚动条，并保持与虚拟窗口基准值同步。
 *
 * 顺序很重要：**先同步 scrollTopValue（它决定渲染哪一段行），再落 DOM**。
 * 只落 DOM、不同步 ref 正是「切回空白」的成因 —— ref 还停在切走前的位置，
 * 渲染窗口在下方、视口在顶部，顶部一片空白，滚一下才重新算对。
 */
function applyScrollTop(target: number) {
  if (target <= 0) return
  scrollTopValue.value = target
  restoringScroll = true
  const put = (attempt: number) => {
    const el = viewerRef.value
    if (!el) {
      restoringScroll = false
      return
    }
    el.scrollTop = target
    // 元素刚插回文档时 scrollHeight 可能还没算出来，赋值会被截断 → 下一帧补一次
    if (Math.abs(el.scrollTop - target) <= 1 || attempt >= 2) {
      scrollTopValue.value = el.scrollTop
      restoringScroll = false
      return
    }
    requestAnimationFrame(() => put(attempt + 1))
  }
  void nextTick(() => put(0))
}

/**
 * 恢复该文档的内容与滚动位置（onMounted / onActivated / path 变化统一走这里）。
 *
 * 内容来源三档：
 * 1. 组件内 savedState 命中（KeepAlive 保住了实例）→ 内容与当前块都在内存里，直接落滚动条；
 * 2. 未命中但有 store 记录的位置 → **按位置反算目标块**再加载。不能只加载第 1 块了事：
 *    第 1 块只含前 CHUNK_SIZE 行，深位置会被浏览器截断到内容高度内，等于没恢复；
 * 3. 都没有 → 正常加载第 1 块。
 */
async function restoreView(filePath: string) {
  const target = storedScrollTop(filePath)
  if (restoreState(filePath) || isLoaded) {
    applyScrollTop(target)
    return
  }

  const chunk = Math.max(1, Number(CHUNK_SIZE.value) || 1)
  if (target > 0) {
    // 位置对应的文件行号 → 它所在的块首行
    const targetLine = Math.floor(target / LINE_HEIGHT) + 1
    const startLine = Math.floor((targetLine - 1) / chunk) * chunk + 1
    const ok = startLine === 1
      ? await loadFirstChunk(filePath)
      : await loadChunk(filePath, startLine)
    if (ok) {
      // 抑制「触底加载下一块」：位置恰在块底部时会被误判成用户滚到底，加载新块后
      // scrollTop 会被重置到 0（分块设计如此），位置就又丢了
      scrollGuard = true
      applyScrollTop(startLine === 1 ? target : target - (startLine - 1) * LINE_HEIGHT)
      setTimeout(() => {
        scrollGuard = false
      }, 400)
    }
    return
  }

  await loadFirstChunk(filePath)
}

async function loadNextChunk() {
  if (!props.path || chunkLoading) return
  if (totalLines.value > 0 && currentEndLine.value >= totalLines.value) return

  const nextStart = currentEndLine.value - OVERLAP + 1
  if (nextStart <= 0) return

  scrollGuard = true
  showOverlay.value = true
  overlayText.value = '加载中...'
  const ok = await loadChunk(props.path, nextStart)
  showOverlay.value = false
  if (ok) {
    await nextTick()
    if (viewerRef.value) {
      viewerRef.value.scrollTop = 0
    }
    prefetchAdjacent(props.path, nextStart)
  }
  setTimeout(() => { scrollGuard = false }, 300)
}

async function loadPrevChunk() {
  if (!props.path || chunkLoading) return
  if (currentStartLine.value <= 1) return

  let prevStart = currentStartLine.value - CHUNK_SIZE.value + OVERLAP
  if (prevStart < 1) prevStart = 1

  scrollGuard = true
  showOverlay.value = true
  overlayText.value = '加载中...'
  const ok = await loadChunk(props.path, prevStart)
  showOverlay.value = false
  if (ok) {
    await nextTick()
    if (viewerRef.value) {
      viewerRef.value.scrollTop = viewerRef.value.scrollHeight
    }
    prefetchAdjacent(props.path, prevStart)
  }
  setTimeout(() => { scrollGuard = false }, 300)
}

function goToFirstBlock() {
  if (currentBlock.value <= 1) return
  scrollGuard = true
  showOverlay.value = true
  overlayText.value = '加载中...'
  loadChunk(props.path, 1).then((ok) => {
    showOverlay.value = false
    if (ok && viewerRef.value) viewerRef.value.scrollTop = 0
    if (ok) prefetchAdjacent(props.path, 1)
    setTimeout(() => { scrollGuard = false }, 300)
  })
}

function goToLastBlock() {
  if (currentBlock.value >= totalBlocks.value) return
  const startLine = (totalBlocks.value - 1) * CHUNK_SIZE.value + 1
  scrollGuard = true
  showOverlay.value = true
  overlayText.value = '加载中...'
  loadChunk(props.path, startLine).then((ok) => {
    showOverlay.value = false
    if (ok && viewerRef.value) viewerRef.value.scrollTop = 0
    if (ok) prefetchAdjacent(props.path, startLine)
    setTimeout(() => { scrollGuard = false }, 300)
  })
}

function goToPrevBlock() {
  if (currentBlock.value <= 1) return
  const startLine = Math.max(1, (currentBlock.value - 2) * CHUNK_SIZE.value + 1)
  scrollGuard = true
  showOverlay.value = true
  overlayText.value = '加载中...'
  loadChunk(props.path, startLine).then((ok) => {
    showOverlay.value = false
    if (ok && viewerRef.value) viewerRef.value.scrollTop = 0
    if (ok) prefetchAdjacent(props.path, startLine)
    setTimeout(() => { scrollGuard = false }, 300)
  })
}

function goToNextBlock() {
  if (currentBlock.value >= totalBlocks.value) return
  const startLine = currentBlock.value * CHUNK_SIZE.value + 1
  scrollGuard = true
  showOverlay.value = true
  overlayText.value = '加载中...'
  loadChunk(props.path, startLine).then((ok) => {
    showOverlay.value = false
    if (ok && viewerRef.value) viewerRef.value.scrollTop = 0
    if (ok) prefetchAdjacent(props.path, startLine)
    setTimeout(() => { scrollGuard = false }, 300)
  })
}

// ========== 预取相邻块（后台静默加载，不阻塞UI） ==========
async function prefetchAdjacent(filePath: string, currentStart: number) {
  const nextStart = currentStart + CHUNK_SIZE.value
  const prevStart = Math.max(1, currentStart - CHUNK_SIZE.value)

  const tasks: Promise<void>[] = []

  if (totalLines.value <= 0 || nextStart <= totalLines.value) {
    if (!getCachedChunk(filePath, nextStart)) {
      tasks.push(prefetchOneChunk(filePath, nextStart))
    }
  }

  if (prevStart < currentStart && !getCachedChunk(filePath, prevStart)) {
    tasks.push(prefetchOneChunk(filePath, prevStart))
  }

  await Promise.all(tasks)
}

async function prefetchOneChunk(filePath: string, startLine: number) {
  try {
    const res: any = await invoke('read_file_lines', {
      filePath,
      startLine,
      lineCount: Number(CHUNK_SIZE.value),
      encoding: currentEncoding.value
    })
    if (res && res.code === 200) {
      const data = res.data
      const newLines = data.content.split('\n').filter((l: string) => l.length > 0 || true)
      if (data.total_lines > 0) {
        totalLines.value = data.total_lines
      }
      const endLine = startLine + newLines.length - 1
      setCacheChunk(filePath, startLine, {
        lines: newLines,
        firstLineNumber: data.first_line_number,
        startLine,
        endLine: data.has_more ? endLine : data.total_lines || endLine,
        hasMore: data.has_more
      })
    }
  } catch (e) {
    console.warn('预取块失败:', startLine, e)
  }
}



function onVirtualScroll() {
  if (!viewerRef.value) return

  const el = viewerRef.value
  // 恢复位置期间不回写：此刻 DOM 的值可能是被截断的 0，回写会让虚拟窗口跳回顶部又变空白
  if (!restoringScroll) scrollTopValue.value = el.scrollTop
  containerHeight.value = el.clientHeight
  scheduleScrollReport()

  if (chunkLoading || isSearching.value || scrollGuard) return

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

function onCompositionStart() {
  compositionEndTime = 0
}

function onCompositionEnd() {
  compositionEndTime = Date.now()
}

function onSearchKeyup(e: KeyboardEvent) {
  if (compositionEndTime > 0 && Date.now() - compositionEndTime < 500) return
  doSearch()
}

async function doSearch() {
  if (!keyword.value || !props.path) return
  isSearching.value = true
  showOverlay.value = true
  overlayText.value = '搜索中...'
  try {
    // 迁移到统一的 log_query_lines（§2.7）：无筛选 + 关键词，恒忽略大小写（与旧口径一致）
    const res: any = await invoke('logs_query_lines', {
      filePath: props.path,
      encoding: currentEncoding.value,
      keyword: keyword.value,
      mode: 'text',
      ignoreCase: true,
      maxMatches: 200000
    })
    if (res && res.code === 200) {
      totalMatches.value = res.data.matched
      matchLineNumbers.value = res.data.lines || []
      if (res.data.matches_truncated) {
        NotificationUtil.warn(`匹配过多，仅保留前 ${matchLineNumbers.value.length} 条可跳转`)
      }
      if (totalMatches.value > 0) {
        currentMatch.value = 1
        await loadPreview(1)
      } else {
        NotificationUtil.warn('未找到匹配结果')
        isSearching.value = false
      }
    } else {
      NotificationUtil.error(res?.msg || '搜索失败')
      isSearching.value = false
    }
  } catch (e: any) {
    console.error('搜索异常:', e)
    NotificationUtil.error(e?.message || '搜索异常，请重试')
    isSearching.value = false
  } finally {
    showOverlay.value = false
  }
}


function navigateInBlock(targetLine: number, matchIndex: number) {
  const kw = keyword.value.toLowerCase()
  const startLine = Math.max(currentStartLine.value, targetLine - CHUNK_SIZE.value)
  const endLine = Math.min(currentEndLine.value, targetLine + CHUNK_SIZE.value)
  const startIdx = startLine - firstLineNumber.value
  const endIdx = endLine - firstLineNumber.value + 1
  lines.value = lines.value.slice(startIdx, endIdx)
  firstLineNumber.value = startLine
  currentStartLine.value = startLine
  currentEndLine.value = startLine + lines.value.length - 1
  const newHighlight = new Set<number>()
  for (let i = 0; i < lines.value.length; i++) {
    if (lines.value[i].toLowerCase().includes(kw)) {
      newHighlight.add(startLine + i)
    }
  }
  highlightSet.value = newHighlight
  targetLineNumber.value = targetLine
  currentMatch.value = matchIndex
  jumpTo.value = matchIndex
  jumpBlockInput.value = String(Math.ceil(startLine / CHUNK_SIZE.value))
  nextTick(() => scrollToLineNumber(targetLine))
}

async function loadPreview(index: number) {
  if (matchLineNumbers.value.length === 0 || index < 1 || index > matchLineNumbers.value.length) return
  const targetLine = matchLineNumbers.value[index - 1]
  if (targetLine >= currentStartLine.value && targetLine <= currentEndLine.value) {
    navigateInBlock(targetLine, index)
    return
  }
  showOverlay.value = true
  overlayText.value = '搜索中...'
  try {
    // 迁移后不再有 search_log_preview：按行号直接读一段（L2 已把取段改成 O(1) 定位），
    // 高亮改由前端本地计算（与 navigateInBlock 同一口径：小写包含匹配）
    const startLine = Math.max(1, targetLine - CHUNK_SIZE.value)
    const res: any = await invoke('read_file_lines', {
      filePath: props.path,
      startLine,
      lineCount: Number(CHUNK_SIZE.value) * 2 + 1,
      encoding: currentEncoding.value
    })
    if (res && res.code === 200) {
      const data = res.data
      const newLines: string[] = data.content.split('\n')
      lines.value = newLines
      firstLineNumber.value = data.first_line_number
      if (data.total_lines > 0) {
        totalLines.value = data.total_lines
      }
      currentStartLine.value = startLine
      currentEndLine.value = startLine + newLines.length - 1
      const kw = keyword.value.toLowerCase()
      const newHighlight = new Set<number>()
      for (let i = 0; i < newLines.length; i++) {
        if (newLines[i].toLowerCase().includes(kw)) {
          newHighlight.add(startLine + i)
        }
      }
      highlightSet.value = newHighlight
      targetLineNumber.value = targetLine
      currentMatch.value = index
      jumpTo.value = index
      jumpBlockInput.value = String(Math.ceil(startLine / CHUNK_SIZE.value))

      await nextTick()
      scrollToLineNumber(targetLineNumber.value)
    } else {
      NotificationUtil.error(res?.msg || '加载预览失败')
    }
  } catch (e: any) {
    console.error('加载预览异常:', e)
    NotificationUtil.error(e?.message || '加载预览异常')
  } finally {
    showOverlay.value = false
  }
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

function jumpToMatch(val: number | string) {
  loadPreview(Number(val))
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
    clearChunkCache()
    currentEncoding.value = 'UTF-8'
    isLoaded = false
    await restoreView(newPath)
    countTotalLinesAsync(newPath)
    detectFileEncoding(newPath)
  }
)

watch(showSearchBar, (val) => {
  if (!val) {
    isSearching.value = false
  }
})
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
function removeKeydownListener() {
  window.removeEventListener('keydown', onKeydown)
}

function addKeydownListener() {
  removeKeydownListener()
  window.addEventListener('keydown', onKeydown)
}
onMounted(async () => {
  //初始化行数
  if (props.path) {
    await restoreView(props.path)
    countTotalLinesAsync(props.path)
    detectFileEncoding(props.path)
  }
  const el = viewerRef.value
  if (el) {
    containerHeight.value = el.clientHeight
    // 面板尺寸变化（侧栏拖拽 / 沉浸式切换 / 窗口缩放）后必须重算可见行数，
    // 否则虚拟窗口仍按旧高度渲染，可视区底部会出现未渲染的空白带
    ro = new ResizeObserver(entries => {
      for (const entry of entries) containerHeight.value = entry.contentRect.height
    })
    ro.observe(el)
  }
  addKeydownListener()
  window.addEventListener('encoding-change', onGlobalEncodingChange);

})

onActivated(() => {
  isDeactivated = false
  // 只有「真正被切走过」才恢复：首次挂载后 Vue 也会调一次 onActivated，那次归 onMounted 管
  if (hasDeactivated && props.path) {
    // 切回时 DOM 已被移回文档（clientHeight/scrollHeight 恢复），但 scrollTop 是 0 —— 必须主动落回去
    void restoreView(props.path)
  }
  if (viewerRef.value) {
    containerHeight.value = viewerRef.value.clientHeight
  }
  addKeydownListener()
})

onDeactivated(() => {
  isDeactivated = true
  hasDeactivated = true
  if (scrollTimer) {
    clearTimeout(scrollTimer)
    scrollTimer = null
  }
  showSearchBar.value = false
  isSearching.value = false
  saveCurrentState()
  // 位置必须用组件内记录值：此刻 DOM 已被 KeepAlive 移出文档，读 el.scrollTop 只会得到 0
  if (reportTimer) {
    clearTimeout(reportTimer)
    reportTimer = null
  }
  reportScrollTop()
  removeKeydownListener()
})

onBeforeUnmount(() => {
  removeKeydownListener()
  // KeepAlive 淘汰（:max=8）走的是 unmount 而非 deactivate，这里再兜底上报一次
  if (reportTimer) {
    clearTimeout(reportTimer)
    reportTimer = null
  }
  reportScrollTop()
  if (ro) {
    ro.disconnect()
    ro = null
  }
  savedState.clear()
  clearChunkCache()
  window.removeEventListener('encoding-change', onGlobalEncodingChange)
})

// ========== 异步统计总行数（不阻塞首次显示） ==========
async function countTotalLinesAsync(filePath: string) {
  try {
    const res: any = await invoke('count_file_lines', { filePath })
    if (res && res.code === 200 && res.data > 0) {
      totalLines.value = res.data
    }
  } catch (e) {
    console.warn('统计行数失败:', e)
  }
}

// ========== 编码检测与切换 ==========
async function detectFileEncoding(filePath: string) {
  try {
    const res: any = await invoke('detect_file_encoding', { path: filePath })
    if (res && res.code === 200 && res.data) {
      const detected = normalizeEncodingName(res.data)
      currentEncoding.value = detected
    }
  } catch (e) {
    console.warn('编码检测失败:', e)
  }
}

function normalizeEncodingName(name: string): string {
  const upper = name.toUpperCase().replace(/[^A-Z0-9]/g, '')
  if (upper.includes('UTF8')) return 'UTF-8'
  if (upper.includes('GB18030')) return 'GB18030'
  if (upper.includes('GBK') || upper === 'GB2312' || upper === 'CP936') return 'GBK'
  if (upper.includes('BIG5')) return 'BIG5'
  if (upper.includes('UTF16LE')) return 'UTF-16LE'
  if (upper.includes('UTF16BE')) return 'UTF-16BE'
  if (upper.includes('SHIFTJIS') || upper.includes('SJIS')) return 'Shift_JIS'
  if (upper.includes('EUCJP')) return 'EUC-JP'
  if (upper.includes('EUCKR')) return 'EUC-KR'
  if (upper.includes('ASCII')) return 'ASCII'
  return 'UTF-8'
}

function onEncodingChange(enc: string) {
  if (enc === currentEncoding.value) return
  switchEncoding(enc)
}

function onGlobalEncodingChange(e: Event) {
  const enc = (e as CustomEvent).detail.encoding
  if (enc && enc !== currentEncoding.value) {
    switchEncoding(enc)
  }
}

async function switchEncoding(enc: string) {
  currentEncoding.value = enc
  clearChunkCache()
  if (props.path) {
    showOverlay.value = true
    overlayText.value = '切换编码中...'
    await loadChunk(props.path, currentStartLine.value > 1 ? currentStartLine.value : 1)
    showOverlay.value = false
  }
}
</script>

<style scoped lang="scss">@use '@/styles/theme.scss' as *;

.log-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
  position: relative;
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
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
  padding: 8px 12px;
  max-width: calc(100vw - 32px);
  border-radius: var(--radius-xl);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
  @include useTheme {
    background-color: getVar(bgColor);
    border: 1px solid getVar(borderColor);
  }
}

.search-input {
  width: 220px;
  flex-shrink: 1;
  min-width: 120px;
}

.search-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.match-count {
  font-size: getVar(fontSizeSmall);
  min-width: 90px;
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
  flex-wrap: wrap;
  gap: 4px;
  padding: 6px 12px;
  @include useTheme {
    background-color: getVar(bgColor);
    border-top: 1px solid getVar(borderColor);
  }
  flex-shrink: 0;
}

.block-jump {
  display: flex;
  align-items: center;
  gap: 4px;
}

.block-total {
  font-size: getVar(fontSizeSmall);
  @include useTheme {
    color: getVar(textColor);
    opacity: 0.6;
  }
}

.line-range {
  font-size: getVar(fontSizeSmall);
  margin-left: 8px;
  @include useTheme {
    color: getVar(textColor);
    opacity: 0.5;
  }
}

.encoding-selector {
  margin-left: auto;
  cursor: pointer;
}

.encoding-label {
  font-size: getVar(fontSizeSmall);
  @include useTheme {
    color: getVar(textColor);
    opacity: 0.7;
  }
  &:hover {
    @include useTheme {
      color: getVar(textColor);
      opacity: 1;
    }
  }
}

.line-info {
  font-size: getVar(fontSizeSmall);
  white-space: nowrap;
  margin-left: 8px;
  @include useTheme {
    color: getVar(textColor);
    opacity: 0.5;
  }
}

.loading-tip {
  font-size: getVar(fontSizeSmall);
  color: #f5c542;
  white-space: nowrap;
  margin-left: 4px;
}

.code-viewer {
  flex: 1;
  overflow-x: scroll;
  overflow-y: auto;
  font-family: var(--font-family-mono);
  font-size: getVar(fontSize);
  @include useTheme {
    background: getVar(appMainBgColor);
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
  border-radius: var(--radius-xs);
  padding: 0 2px;
}

.loading-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 90;
  display: flex;
  align-items: center;
  justify-content: center;
  @include useTheme {
    background: rgba(0, 0, 0, 0.35);
  }
}

.loading-spinner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.spinner-ring {
  width: 36px;
  height: 36px;
  border: 3px solid rgba(255, 255, 255, 0.15);
  border-top-color: #409eff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loading-text {
  font-size: getVar(fontSize);
  @include useTheme {
    color: getVar(textColor);
  }
  opacity: 0.8;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

</style>
