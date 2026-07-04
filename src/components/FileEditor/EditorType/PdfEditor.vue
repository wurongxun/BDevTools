<script setup lang="ts">
import {
  onMounted,
  onUnmounted,
  ref,
  nextTick,
  watch,
  computed,
  ComputedRef,
  onActivated,
  onDeactivated
} from 'vue'
import * as pdfjsLib from 'pdfjs-dist'
import { useRoute } from 'vue-router'
import pdfjsWorker from 'pdfjs-dist/build/pdf.worker.min?url'
import { ElMessage } from 'element-plus'
import { convertFileSrc } from '@tauri-apps/api/core'
import useStore from '@/store'
import { DocumentState } from '@/store/modules/documents'

defineOptions({
  name: 'PdfPreview'
})

// 常量定义
const SCALE_STEP = 0.2
const MIN_SCALE = 0.2
const MAX_SCALE = 5
const CONTAINER_PADDING = 40
const MAX_FIT_SCALE = 3

const { appConfig, system, useDocumentManagerStore } = useStore()
// 当前激活的文档路径
const activeDocumentId = computed(() => useDocumentManagerStore.activeDocumentId)
const route = useRoute()
const scale = ref<number>(1)
const page_num = ref<number>(1)
const page_count = ref<number>(0)
const loading = ref<boolean>(true)
const error = ref<string | null>(null)
const controlsPosition = ref<{ top: number; left: number }>({ top: 0, left: 0 })

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

// 保存组件状态
const savedState = new Map<
  string,
  {
    pageNum: number
    scale: number
    filePath: string
  }
>()

// 主题切换功能 - 提前定义
const toggleDarkMode = (isDarkMode: boolean) => {
  const canvas = document.getElementById('the-canvas')
  if (isDarkMode) {
    canvas?.classList.add('dark-mode')
  } else {
    canvas?.classList.remove('dark-mode')
  }
}

// 获取当前文件路径
const getCurrentFilePath = (): string => {
  if (route.meta.filePath) {
    return decodeURIComponent(route.meta.filePath as string)
  } else if (route.query.file) {
    return route.query.file as string
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
  const currentPath = route.path
  const state = savedState.get(currentPath)

  if (state && state.filePath === getCurrentFilePath()) {
    pageNum = state.pageNum
    scale.value = state.scale

    if (pdfDoc) {
      renderPage(pageNum)
    }
  }
})

// 组件被停用时保存状态
onDeactivated(() => {
  console.log('[PdfPreview] Deactivated, saving state')
  const currentPath = route.path
  const filePath = getCurrentFilePath()

  if (filePath) {
    savedState.set(currentPath, {
      pageNum,
      scale: scale.value,
      filePath
    })
  }
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

    if (!activeDocumentId.value) {
      throw new Error('No active document selected')
    }

    const doc = useDocumentManagerStore.documents.get(activeDocumentId.value)
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
      pdfDoc.destroy()
      pdfDoc = null
    }

    const loadingTask = pdfjsLib.getDocument({ url: url })
    pdfDoc = await loadingTask.promise
    page_count.value = pdfDoc.numPages

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

// 监听激活文档变化
watch(
  activeDocumentId,
  async (newId) => {
    console.log('Active document changed:', newId)
    if (newId) {
      await loadPdfFile()
    }
  },
  { immediate: true }
)

// 监听暗黑模式变化
let isDark: ComputedRef<boolean> = computed(() => system.isDark)
watch(
  () => system.isDark,
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

onMounted(() => {
  console.log('pdfPreview mounted', route)

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

    if (resizeObserver) {
      resizeObserver.disconnect()
    }

    // 清理 PDF 文档
    if (pdfDoc) {
      pdfDoc.destroy()
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
      <button @click="onReduce" :disabled="scale <= 0.2 || loading" title="缩小 (Ctrl+-)">-</button>
      <button @click="onMagnify" :disabled="scale >= 5 || loading" title="放大 (Ctrl++)">+</button>
      <button @click="onPrevPage" :disabled="page_num <= 1 || loading" title="上一页 (←)">
        Previous
      </button>
      <span>Page: {{ page_num }} / {{ page_count }}</span>
      <button @click="onNextPage" :disabled="page_num >= page_count || loading" title="下一页 (→)">
        Next
      </button>
    </div>
  </div>
</template>

<style scoped>
.pdf-container {
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
  border-radius: 4px;
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
  font-size: 18px;
  color: #409eff;
}

.error-message {
  font-size: 18px;
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
  border-radius: 30px;
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
  border-radius: 20px;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.2s ease;
  min-width: 60px;
  box-shadow: 0 2px 3px rgba(0, 0, 0, 0.2);
  font-size: 14px;
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
  font-size: 14px;
  min-width: 80px;
  text-align: center;
}

@media (max-width: 768px) {
  .pdf-controls {
    flex-wrap: wrap;
    width: 90%;
    justify-content: center;
    gap: 8px;
    padding: 8px 16px;
  }

  .pdf-controls button {
    padding: 6px 12px;
    min-width: 50px;
    font-size: 14px;
  }

  .pdf-controls span {
    font-size: 12px;
    min-width: 60px;
  }
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
