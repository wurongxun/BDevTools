<script setup lang="ts">
import {
  onMounted,
  onUnmounted,
  ref,
  nextTick,
  watch,
  computed,
  onActivated,
  onDeactivated
} from 'vue'
import * as pdfjsLib from 'pdfjs-dist'
import pdfjsWorker from 'pdfjs-dist/build/pdf.worker.min?url'
import { ElMessage } from 'element-plus'
import { convertFileSrc } from '@/core/runtime'
import { isDark } from '@/composables/useDark'
import useStore from '@/store'
import BookmarkTree from './Pdf/BookmarkTree.vue'
import { DocumentState } from '../../store/documents'
import { useDocumentManagerStore } from '../../store/documents'
import { ArrowRight, ArrowDown, FolderOpened, Document } from '@element-plus/icons-vue'

defineOptions({
  name: 'PdfPreview'
})

// 常量定义
const SCALE_STEP = 0.2
const MIN_SCALE = 0.2
const MAX_SCALE = 5
const CONTAINER_PADDING = 40
const MAX_FIT_SCALE = 1.5
const { appConfig, system } = useStore()
const docStore = useDocumentManagerStore()
// 当前激活的文档路径
const activeDocumentPath = computed(() => docStore.activeDocumentPath)
const scale = ref<number>(1)
const page_num = ref<number>(1)
const page_count = ref<number>(0)
const loading = ref<boolean>(true)
const error = ref<string | null>(null)
const controlsPosition = ref<{ top: number; left: number }>({ top: 0, left: 0 })

// 书签栏相关
const showBookmarkPanel = ref<boolean>(false)
const bookmarks = ref<any[]>([])
const expandedBookmarks = ref<Set<string>>(new Set())

// 使用 shallowRef 存储大型对象，避免不必要的响应式开销
let pdfDoc: any = null
let pageNum = 1
let pageRendering = false
let pageNumPending: number | null = null
// 使用模板引用
const canvasRef = ref<HTMLCanvasElement | null>(null)
const containerRef = ref<HTMLElement | null>(null)
let canvas: HTMLCanvasElement | null = null
let ctx: CanvasRenderingContext2D | null = null

pdfjsLib.GlobalWorkerOptions.workerSrc = pdfjsWorker

// KeepAlive 缓存标志：防止 watch 在组件恢复时重复加载
let isDeactivated = false
// 保存组件状态（以文档ID为key，而非路由路径）
const savedState = new Map<
  string,
  {
    pageNum: number
    scale: number
    scrollTop: number
    filePath: string
  }
>()

// 主题切换功能 - 提前定义
// 用组件自身的 canvasRef 而非 document.getElementById('the-canvas')：
// 多 PDF 文档同时打开时模板存在重复 id，getElementById 只会命中第一个实例
const toggleDarkMode = (isDarkMode: boolean) => {
  const canvas = canvasRef.value
  if (!canvas) return
  if (isDarkMode) {
    canvas.classList.add('dark-mode')
  } else {
    canvas.classList.remove('dark-mode')
  }
}

// 获取当前文件路径
const getCurrentFilePath = (): string => {
  if (activeDocumentPath.value) {
    return decodeURIComponent(activeDocumentPath.value as string)
  }
  return ''
}

// 缓存容器元素，避免重复查询
let cachedContainer: HTMLElement | null = null
const getContainer = (): HTMLElement | null => {
  if (!cachedContainer) {
    cachedContainer = document.querySelector('.pdf-container') as HTMLElement
  }
  return cachedContainer
}

const updateControlsPosition = () => {
  const container = getContainer()
  if (!container) return

  const rect = container.getBoundingClientRect()
  const containerCenterY = rect.top + rect.height - 40
  const containerCenterX = rect.left + rect.width / 2

  controlsPosition.value = {
    top: containerCenterY,
    left: containerCenterX
  }
}

const calculateFitScale = async (): Promise<number> => {
  if (!pdfDoc || !canvasRef.value) return 1

  const container = getContainer()
  if (!container) return 1

  const containerWidth = container.clientWidth - CONTAINER_PADDING

  try {
    const firstPage = await pdfDoc.getPage(1)
    const viewport = firstPage.getViewport({ scale: 1 })
    const fitScale = containerWidth / viewport.width
    return Math.min(fitScale, MAX_FIT_SCALE)
  } catch (err) {
    console.error('计算适配缩放失败:', err)
    return 1
  }
}

// 防抖函数
let resizeTimer: number | null = null
const debouncedResize = () => {
  if (resizeTimer) {
    clearTimeout(resizeTimer)
  }
  resizeTimer = window.setTimeout(async () => {
    updateControlsPosition()
    if (pdfDoc) {
      const fitScale = await calculateFitScale()
      scale.value = fitScale
      renderPage(pageNum)
    }
  }, 150)
}

// 组件被激活时恢复状态
onActivated(() => {
  console.log('[PdfPreview] Activated, restoring state')
  isDeactivated = false
  const docId = activeDocumentPath.value
  if (!docId) return

  const state = savedState.get(docId)
  if (state && state.filePath === docId) {
    pageNum = state.pageNum
    scale.value = state.scale

    if (pdfDoc) {
      renderPage(pageNum)
      nextTick(() => {
        const container = getContainer()
        if (container) {
          container.scrollTop = state.scrollTop
        }
      })
    }
  }
})

// 组件被停用时保存状态
onDeactivated(() => {
  console.log('[PdfPreview] Deactivated, saving state')
  isDeactivated = true
  const docId = activeDocumentPath.value
  if (!docId) return

  const container = getContainer()
  const scrollTop = container ? container.scrollTop : 0

  savedState.set(docId, {
    pageNum,
    scale: scale.value,
    scrollTop,
    filePath: docId
  })
})


// 缩放控制
const onMagnify = () => {
  if (!pdfDoc || !ctx) return
  if (scale.value < MAX_SCALE) {
    scale.value = Math.min(scale.value + SCALE_STEP, MAX_SCALE)
    renderPage(pageNum)
  }
}

const onReduce = () => {
  if (!pdfDoc || !ctx) return
  if (scale.value > MIN_SCALE) {
    scale.value = Math.max(scale.value - SCALE_STEP, MIN_SCALE)
    renderPage(pageNum)
  }
}

function scrollToTop() {
  nextTick(() => {
    const pdfContainer = getContainer()
    if (pdfContainer) {
      pdfContainer.scrollTo({ top: 0, behavior: 'smooth' })
    }
  })
}

// 加载 PDF 文件
const loadPdfFile = async () => {
  try {
    loading.value = true
    error.value = null

    await nextTick()

    if (!canvasRef.value) {
      console.warn('Canvas element not ready, retrying...')
      await new Promise((resolve) => setTimeout(resolve, 100))
      await nextTick()

      if (!canvasRef.value) {
        throw new Error('Canvas element not found')
      }
    }

    if (!activeDocumentPath.value) {
      throw new Error('No active document selected')
    }

    const doc = docStore.documents.get(activeDocumentPath.value)
    if (!doc || !doc.id) {
      throw new Error('No PDF file path provided')
    }

    if (doc.type !== 'pdf') {
      return
    }

    let url = convertFileSrc(doc.id)
    canvas = canvasRef.value
    ctx = canvas.getContext('2d')

    if (!ctx) {
      throw new Error('Failed to get canvas context')
    }

    console.log('[PdfPreview] Loading PDF:', url)

    // 如果已有 PDF 文档，先销毁
    if (pdfDoc) {
      //pdfDoc.destroy()
      pdfDoc = null
    }

    const loadingTask = pdfjsLib.getDocument({ url: url })
    pdfDoc = await loadingTask.promise
    page_count.value = pdfDoc.numPages

    // 提取书签
    await extractBookmarks()

    const fitScale = await calculateFitScale()
    scale.value = fitScale

    renderPage(pageNum)
  } catch (err: any) {
    console.error('PDF 加载失败:', err)
    error.value = getErrorMessage(err)
    ElMessage.error(error.value)
  } finally {
    loading.value = false
  }
}

// 提取 PDF 书签
const extractBookmarks = async () => {
  try {
    const outline = await pdfDoc.getOutline()
    if (outline && Array.isArray(outline)) {
      bookmarks.value = processBookmarks(outline)
      console.log('提取到书签:', bookmarks.value.length, '个')
    } else {
      bookmarks.value = []
      console.log('此 PDF 没有书签')
    }
  } catch (err) {
    console.error('提取书签失败:', err)
    bookmarks.value = []
  }
}

// 处理书签数据，添加唯一标识和层级信息
const processBookmarks = (outline: any[], level = 0): any[] => {
  return outline.map((item, index) => {
    const id = `${level}-${index}-${Date.now()}`
    const processed = {
      ...item,
      id,
      level,
      expanded: false,
      children: item.items ? processBookmarks(item.items, level + 1) : []
    }
    return processed
  })
}

// 切换书签展开/折叠
const toggleBookmarkExpand = (bookmark: any) => {
  bookmark.expanded = !bookmark.expanded
}

// 跳转到指定页码
const goToBookmarkPage = (dest: any) => {
  if (!pdfDoc) return

  try {
    // dest 可能是数组或对象，需要解析页码
    let targetPage = 1

    if (Array.isArray(dest)) {
      // dest[0] 通常是页面引用
      const pageRef = dest[0]
      if (pageRef && typeof pageRef === 'object') {
        // 通过页面引用获取页码
        pdfDoc.getPageIndex(pageRef).then((pageIndex: number) => {
          const pageNumber = pageIndex + 1
          jumpToPage(pageNumber)
        }).catch((err: any) => {
          console.error('获取页码失败:', err)
        })
      }
    } else if (typeof dest === 'number') {
      targetPage = dest
      jumpToPage(targetPage)
    }
  } catch (err) {
    console.error('跳转失败:', err)
    ElMessage.error('跳转失败')
  }
}

// 执行页面跳转
const jumpToPage = (pageNumber: number) => {
  if (pageNumber >= 1 && pageNumber <= page_count.value) {
    pageNum = pageNumber
    queueRenderPage(pageNum)
    scrollToTop()
    ElMessage.success(`已跳转到第 ${pageNumber} 页`)
  }
}

// 错误消息格式化
const getErrorMessage = (err: any): string => {
  if (err.name === 'PasswordException') {
    return '此 PDF 文件需要密码'
  } else if (err.message?.includes('Invalid PDF')) {
    return '无效的 PDF 文件格式'
  } else if (err.message?.includes('Missing PDF')) {
    return 'PDF 文件不存在'
  }
  return err.message || 'Failed to load PDF'
}

// 监听激活文档变化 - 仅在组件处于激活状态时响应
watch(
  activeDocumentPath,
  async (newId, oldId) => {
    if (isDeactivated) return
    if (oldId === undefined) return
    console.log('[PdfPreview] Active document changed:', newId)
    if (newId) {
      await loadPdfFile()
    }
  }
)


// 监听暗黑模式变化：跟随全局主题单例（此前依赖 system store 的 isDark，
// 其初始值仅读 localStorage 旧键且只在切换时被同步——「启动即暗色」场景下 PDF 会一直保持亮色）
watch(
  isDark,
  (newValue, oldValue) => {
    console.log('Dark mode changed:', newValue, oldValue)
    toggleDarkMode(newValue)
  }
)

function renderPage(num: number) {
  if (!pdfDoc || !ctx || !canvas) return

  pageRendering = true
  pdfDoc
    .getPage(num)
    .then(function (page: any) {
      const viewport = page.getViewport({ scale: scale.value })

      if (canvas) {
        canvas.style.width = viewport.width + 'px'
        canvas.style.height = viewport.height + 'px'
        canvas.width = Math.floor(viewport.width)
        canvas.height = Math.floor(viewport.height)
        canvas.style.display = 'block'
        canvas.style.margin = '0 auto'
      }

      if (canvas && ctx) {
        ctx.clearRect(0, 0, canvas.width, canvas.height)

        const renderContext = {
          canvasContext: ctx,
          viewport: viewport,
          background: 'rgba(0,0,0,0)'
        }

        const renderTask = page.render(renderContext)
        renderTask.promise
          .then(function () {
            pageRendering = false
            if (pageNumPending !== null) {
              renderPage(pageNumPending)
              pageNumPending = null
            }
          })
          .catch((error: any) => {
            console.error('渲染页面出错:', error)
            pageRendering = false
          })
      }
    })
    .catch((error: any) => {
      console.error('获取页面出错:', error)
      pageRendering = false
    })

  page_num.value = num
}

function queueRenderPage(num: number) {
  if (pageRendering) {
    pageNumPending = num
  } else {
    renderPage(num)
  }
}

function onPrevPage() {
  if (pageNum <= 1 || !pdfDoc) return
  pageNum--
  queueRenderPage(pageNum)
  scrollToTop()
}

function onNextPage() {
  if (!pdfDoc || pageNum >= pdfDoc.numPages) return
  pageNum++
  queueRenderPage(pageNum)
  scrollToTop()
}

const goToPage = (page: number) => {
  if (page >= 1 && page <= page_count.value && pdfDoc) {
    pageNum = page
    queueRenderPage(pageNum)
    scrollToTop()
  }
}

// 切换书签面板显示
const toggleBookmarkPanel = () => {
  showBookmarkPanel.value = !showBookmarkPanel.value
}
// 全部展开
const expandAllBookmarks = () => {
  const expandRecursive = (items: any[]) => {
    items.forEach(item => {
      item.expanded = true
      if (item.children && item.children.length > 0) {
        expandRecursive(item.children)
      }
    })
  }
  expandRecursive(bookmarks.value)
}

// 全部折叠
const collapseAllBookmarks = () => {
  const collapseRecursive = (items: any[]) => {
    items.forEach(item => {
      item.expanded = false
      if (item.children && item.children.length > 0) {
        collapseRecursive(item.children)
      }
    })
  }
  collapseRecursive(bookmarks.value)
}

onMounted(() => {
  console.log('pdfPreview mounted', activeDocumentPath.value)

  const handleKeydown = (e: KeyboardEvent) => {
    if ((e.target as HTMLElement).tagName === 'INPUT') return

    if (e.key === 'ArrowLeft') {
      onPrevPage()
    } else if (e.key === 'ArrowRight') {
      onNextPage()
    } else if (e.key === '+' || e.key === '=') {
      if (e.ctrlKey) onMagnify()
    } else if (e.key === '-') {
      if (e.ctrlKey) onReduce()
    }
  }

  window.addEventListener('keydown', handleKeydown)

  let resizeObserver: ResizeObserver | null = null

  onUnmounted(() => {
    console.log('[PdfPreview] Unmounting, cleaning up...')
    window.removeEventListener('keydown', handleKeydown)
    // 移除捕获级 scroll 监听（组件卸载后不清理会持续持有组件闭包，导致内存无法回收）
    window.removeEventListener('scroll', updateControlsPosition, true)

    if (resizeObserver) {
      resizeObserver.disconnect()
    }

    // 清理 PDF 文档
    if (pdfDoc) {
      //pdfDoc.destroy()
      pdfDoc = null
    }

    // 清除定时器
    if (resizeTimer) {
      clearTimeout(resizeTimer)
      resizeTimer = null
    }

    // 清理缓存
    cachedContainer = null
  })

  toggleDarkMode(isDark.value)

  nextTick(() => {
    loadPdfFile()

    setTimeout(() => {
      updateControlsPosition()
      const container = getContainer()
      if (container) {
        resizeObserver = new ResizeObserver(debouncedResize)
        resizeObserver.observe(container)
      }

      window.addEventListener('scroll', updateControlsPosition, true)
    }, 200)
  })
})
</script>

<!-- ... existing code ... -->

<template>
  <div class="pdf-wrapper" :class="{ 'with-bookmark': showBookmarkPanel }">
    <!-- 书签栏 -->
    <div v-if="showBookmarkPanel" class="bookmark-panel">
      <div class="bookmark-header">
        <span class="bookmark-title">
          <el-icon><FolderOpened /></el-icon>
          书签
        </span>
        <div class="bookmark-actions">
          <div class="el-button-group">
            <el-tooltip content="全部展开" placement="top">
              <app-button size="small" @click="expandAllBookmarks">
                <el-icon><ArrowDown /></el-icon>
              </app-button>
            </el-tooltip>
            <el-tooltip content="全部折叠" placement="top">
              <app-button size="small" @click="collapseAllBookmarks">
                <el-icon><ArrowRight /></el-icon>
              </app-button>
            </el-tooltip>
          </div>
          <app-button size="small" @click="toggleBookmarkPanel" :icon="'Close'" />
        </div>
      </div>

      <div class="bookmark-content">
        <div v-if="bookmarks.length === 0" class="no-bookmarks">
          <el-empty description="此 PDF 没有书签" :image-size="80" />
        </div>

        <div v-else class="bookmark-tree">
          <BookmarkTree
            :bookmarks="bookmarks"
            :current-page="page_num"
            @go-to-page="goToBookmarkPage"
            @toggle-expand="toggleBookmarkExpand"
          />
        </div>
      </div>
    </div>

    <!-- PDF 预览区域 -->
    <div class="pdf-container" ref="containerRef">
      <!-- 加载状态 -->
      <div v-if="loading" class="loading-overlay">
        <div class="loading-spinner">Loading PDF...</div>
      </div>
      <!-- 错误状态 -->
      <div v-else-if="error" class="error-overlay">
        <div class="error-message">{{ error }}</div>
      </div>
      <canvas ref="canvasRef" class="pdf-canvas" id="the-canvas"></canvas>

      <div
        class="pdf-controls"
        :style="{
          position: 'fixed',
          top: `${controlsPosition.top}px`,
          left: `${controlsPosition.left}px`,
          transform: 'translate(-50%, -50%)'
        }"
      >
        <!-- 书签按钮 -->
        <app-button
          @click="toggleBookmarkPanel"
          :type="showBookmarkPanel ? 'primary' : 'default'"
          size="small"
          title="书签"
        >
          <el-icon><FolderOpened /></el-icon>
        </app-button>

        <button @click="onReduce" :disabled="scale <= 0.2 || loading" title="缩小 (Ctrl+-)">
          -
        </button>
        <button @click="onMagnify" :disabled="scale >= 5 || loading" title="放大 (Ctrl++)">
          +
        </button>
        <button @click="onPrevPage" :disabled="page_num <= 1 || loading" title="上一页 (←)">
          Previous
        </button>
        <span>Page: {{ page_num }} / {{ page_count }}</span>
        <button
          @click="onNextPage"
          :disabled="page_num >= page_count || loading"
          title="下一页 (→)"
        >
          Next
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;
.pdf-wrapper {
  display: flex;
  width: 100%;
  height: 100%;
  position: relative;
}

.pdf-wrapper.with-bookmark {
  padding-left: 300px;
}

/* 书签栏样式 */
.bookmark-panel {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 300px;
  background-color: var(--el-bg-color);
  border-right: 1px solid var(--el-border-color);
  display: flex;
  flex-direction: column;
  z-index: 50;
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.1);
}

.bookmark-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  background-color: var(--el-fill-color-light);
}

.bookmark-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: theme.getVar('fontSize');
  color: var(--el-text-color-primary);
}

.bookmark-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.bookmark-content {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 8px;
}

.no-bookmarks {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--el-text-color-secondary);
}

.bookmark-tree {
  user-select: none;
}

/* 书签项样式 */
.bookmark-item {
  margin-bottom: 2px;
}

.bookmark-node {
  display: flex;
  align-items: center;
  padding: 8px 12px;
  cursor: pointer;
  border-radius: var(--radius-md);
  transition: all 0.2s;
  gap: 8px;
}

.bookmark-node:hover {
  background-color: var(--el-fill-color-light);
}

.bookmark-node.active {
  background-color: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
}

.bookmark-toggle {
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}

.bookmark-icon {
  color: var(--el-color-warning);
  flex-shrink: 0;
}

.bookmark-text {
  flex: 1;
  font-size: theme.getVar('fontSize');
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.bookmark-page {
  font-size: theme.getVar('fontSizeSmall');
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
  min-width: 30px;
  text-align: right;
}

.bookmark-children {
  padding-left: 20px;
}

.pdf-container {
  flex: 1;
  width: 100%;
  height: 100%;
  position: relative;
  padding: 1px 0;
  box-sizing: border-box;
  overflow-y: auto;
  overflow-x: hidden;
}

.pdf-canvas {
  width: 100%;
  max-width: 100%;
  display: block;
  margin: 0 auto;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
  border-radius: var(--radius-md);
}

.loading-overlay,
.error-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
  background-color: var(--el-fill-color-light);
  z-index: 200;
}

.loading-spinner {
  font-size: theme.getVar('fontSizeLg');
  color: #409eff;
}

.error-message {
  font-size: theme.getVar('fontSizeLg');
  color: #f56c6c;
  text-align: center;
  padding: 20px;
}

.pdf-controls {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 10px;
  padding: 8px 16px;
  border-radius: var(--radius-round);
  box-shadow: 0 4px 15px rgba(0, 0, 0, 0.3);
  z-index: 100;
  backdrop-filter: blur(10px);
  background-color: rgba(30, 30, 30, 0.95);
  border: 1px solid rgba(255, 255, 255, 0.1);
  transition: all 0.2s ease;
}

.pdf-controls button {
  padding: 8px 16px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  background-color: rgba(255, 255, 255, 0.1);
  color: #ffffff;
  border-radius: var(--radius-round);
  cursor: pointer;
  font-weight: 500;
  transition: all 0.2s ease;
  min-width: 60px;
  box-shadow: 0 2px 3px rgba(0, 0, 0, 0.2);
  font-size: theme.getVar('fontSize');
}

.pdf-controls button:hover:not(:disabled) {
  background-color: rgba(255, 255, 255, 0.2);
  color: #ffffff;
  border-color: rgba(255, 255, 255, 0.3);
  transform: translateY(-2px);
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.3);
}

.pdf-controls button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
  transform: none;
  box-shadow: 0 2px 5px rgba(0, 0, 0, 0.2);
  color: rgba(255, 255, 255, 0.5);
}

.pdf-controls span {
  font-weight: 500;
  color: #ffffff;
  font-size: theme.getVar('fontSize');
  min-width: 80px;
  text-align: center;
}


/* 给PDF渲染容器添加CSS滤镜来实现暗黑效果： */
.pdf-container.dark-mode .pdf-canvas {
  filter: invert(1) hue-rotate(180deg);
}

/* CSS中添加暗黑模式样式 */
.pdf-canvas.dark-mode {
  filter: invert(1) hue-rotate(180deg);
}

/* 或者使用CSS变量控制 */
:root {
  --pdf-bg: #ffffff;
}

:root.dark {
  --pdf-bg: #1a1a1a;
}

.pdf-container {
  background-color: var(--pdf-bg);
}
</style>
