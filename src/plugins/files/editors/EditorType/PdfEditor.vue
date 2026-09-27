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
import BookmarkTree from './Pdf/BookmarkTree.vue'
import { useDocumentManagerStore } from '../../store/documents'
import { ArrowRight, ArrowDown, FolderOpened } from '@element-plus/icons-vue'

defineOptions({
  name: 'PdfPreview'
})

// 常量定义
const SCALE_STEP = 0.2
const MIN_SCALE = 0.2
const MAX_SCALE = 5
const CONTAINER_PADDING = 40
const MAX_FIT_SCALE = 1.5
const docStore = useDocumentManagerStore()

const props = defineProps<{
  /** 本实例负责的文档路径（父级 FileEditor 按文档 key 复用实例，一个实例只对应一个文档） */
  path: string
}>()

const emit = defineEmits<{
  /** 视图状态上报（页码 / 缩放 / 滚动位置）：由 FileEditor 收口写进 docStore.viewState */
  'update-state': [state: Record<string, unknown>]
}>()

// 当前激活的文档路径
const activeDocumentPath = computed(() => docStore.activeDocumentPath)

/**
 * 本实例真正负责的文档：优先父级传入的 path。
 * 不能只用 activeDocumentPath —— 切 tab 时它已经指向新文档，而旧实例（即将 deactivate）
 * 上的 watcher 会先一步触发，把新文档的 PDF 加载进旧实例，并把旧文档的状态写到新文档上。
 */
const docPath = computed(() => props.path || activeDocumentPath.value)

const scale = ref<number>(1)
const page_num = ref<number>(1)
const page_count = ref<number>(0)
const loading = ref<boolean>(true)
const error = ref<string | null>(null)
const controlsPosition = ref<{ top: number; left: number }>({ top: 0, left: 0 })

// 书签栏相关
const showBookmarkPanel = ref<boolean>(false)
const bookmarks = ref<any[]>([])

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

// ==================== 视图状态（页码 / 缩放 / 滚动位置） ====================
//
// 真源只有一个：docStore.viewState（由 FileEditor 收口写入；会话快照 openfile.json 的
// scroll_top 就是它）。组件内不再另存一份 Map —— 双份数据必然分叉，排查成本远高于收益。
//
// 为什么不能在 onDeactivated 里读 DOM 的 scrollTop（原实现就是这么写的）：
// 本组件被两层 KeepAlive 包裹（插件级 AppEditorMain、文档级 FileEditor）。切走时 Vue 把
// 组件根元素 move 到**脱离文档**的 storageContainer，此刻 el.scrollTop 立刻变 0
// （clientHeight / scrollHeight 也一起变 0），移回文档后不会自动恢复 —— 读 DOM 只能拿到 0。
// 所以位置必须在用户滚动过程中就记进 JS（liveScrollTop），detach 之后只用这个值。

/** 用户滚动时记录的真实位置：KeepAlive detach 之后读 DOM 恒为 0，只能靠它上报/恢复 */
const liveScrollTop = ref(0)
/**
 * 待恢复的滚动位置：切换页码 / 缩放会重建 canvas（scrollHeight 随之变化），
 * 位置必须等渲染完成后再落，否则会被浏览器截断到旧的（更小的）内容高度内。
 */
let pendingScrollTop = 0
/**
 * 用户手动缩放过、或本次是从会话快照恢复的缩放：为真时容器尺寸变化不再强制「适配宽度」。
 * 否则切回标签页时 ResizeObserver（detach → 尺寸 0 → 移回 → 尺寸恢复）会把缩放冲掉，
 * canvas 尺寸一变，刚恢复的滚动位置也会被一起截断。
 */
let keepScale = false
/** 视图状态上报去抖定时器 */
let reportTimer: number | null = null

/**
 * 读某文档在 store 里记录的视图状态（页码 / 缩放 / 滚动位置）。
 *
 * 只有 PdfEditor 会写 page / pageScale（会话快照 openfile.json 只存 cursor_* / scroll_top 与
 * scroll_left），所以「有 page」就等于「这份记录是完整的」。反过来：重启后拿到的记录只有
 * scrollTop，而页码必然回落第 1 页、缩放回落适配宽度，若还按旧位置落滚动条就是
 * 「页码与位置错位」，所以残缺记录一律不恢复（结果与此前一致：PDF 每次打开从第 1 页顶部开始）。
 */
function readViewState(filePath: string) {
  const vs = docStore.documents.get(filePath)?.viewState || {}
  const page = vs.page
  if (typeof page !== 'number' || page <= 0) return { scrollTop: 0, page: 1, scale: 0 }
  return {
    scrollTop: typeof vs.scrollTop === 'number' && vs.scrollTop > 0 ? vs.scrollTop : 0,
    page: page as number,
    scale: typeof vs.pageScale === 'number' && vs.pageScale > 0 ? vs.pageScale : 0
  }
}

/** 上报视图状态：必须带上自己的 path，父级按上报者收口（用 activeDocumentPath 收会串号） */
function reportViewState() {
  if (!props.path) return
  // 文档还没加载完就别写：此刻 scale 还是初值 1、pageNum 还是 1，会把「适配宽度」覆盖成 1，
  // 下次切回来就按 scale=1 渲染（PDF 宽度对不上容器）
  if (!pdfDoc || page_count.value <= 0) return
  emit('update-state', {
    path: props.path,
    scrollTop: liveScrollTop.value,
    page: pageNum,
    pageScale: scale.value
  })
}

function scheduleReportViewState() {
  if (reportTimer !== null) window.clearTimeout(reportTimer)
  reportTimer = window.setTimeout(() => {
    reportTimer = null
    reportViewState()
  }, 250)
}

/** 容器滚动：实时记录位置（detach 后读不到）+ 维护悬浮控件位置 */
function onContainerScroll() {
  const container = containerRef.value
  if (container) liveScrollTop.value = container.scrollTop
  updateControlsPosition()
  scheduleReportViewState()
}

/**
 * 把滚动位置落到真实滚动条上。
 * 元素刚插回文档时 scrollHeight 可能还没算出来，赋值会被截断 → 下一帧补一次。
 */
function applyScrollTop(target: number) {
  if (target <= 0) return
  const put = (attempt: number) => {
    const el = containerRef.value
    if (!el || !el.isConnected) return
    el.scrollTop = target
    if (Math.abs(el.scrollTop - target) <= 1 || attempt >= 2) {
      liveScrollTop.value = el.scrollTop
      return
    }
    requestAnimationFrame(() => put(attempt + 1))
  }
  void nextTick(() => put(0))
}

/** 有等待恢复的位置则立刻落一次（canvas 渲染完成后调用；无待恢复值时是空操作） */
function flushPendingScroll() {
  if (pendingScrollTop <= 0) return
  const target = pendingScrollTop
  pendingScrollTop = 0
  applyScrollTop(target)
}

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

/**
 * 取本实例的滚动容器。
 * 此前用 `document.querySelector('.pdf-container')` 查全局：多个 PDF 文档同时打开时
 * DOM 里存在多个同 class 容器，querySelector 只命中第一个（很可能是别的文档的），
 * 于是滚动位置读写、悬浮控件定位、适配宽度计算全部算在别人身上。
 */
const getContainer = (): HTMLElement | null => containerRef.value

const updateControlsPosition = () => {
  const container = getContainer()
  if (!container || !container.isConnected) return

  const rect = container.getBoundingClientRect()
  // KeepAlive detach 期间元素不在文档里，rect 全是 0 —— 会把悬浮控件甩到窗口左上角
  if (rect.width === 0 && rect.height === 0) return
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
    if (!pdfDoc) return
    // 手动缩放（或从快照恢复的缩放）优先：否则切回标签页时 RO 会按「适配宽度」把它冲掉
    if (!keepScale) {
      scale.value = await calculateFitScale()
    }
    renderPage(pageNum)
  }, 150)
}

// 组件被激活：恢复上次的页码 / 缩放 / 滚动位置
onActivated(() => {
  isDeactivated = false
  const filePath = props.path
  if (!filePath) return

  const prevScale = scale.value
  const prevPage = page_num.value
  const vs = readViewState(filePath)
  if (vs.scale > 0) {
    scale.value = vs.scale
    keepScale = true
  }
  if (vs.page > 1) pageNum = vs.page
  liveScrollTop.value = vs.scrollTop
  pendingScrollTop = vs.scrollTop

  if (!pdfDoc) return
  const willRebuild = pageNum !== prevPage || scale.value !== prevScale
  queueRenderPage(pageNum)
  if (!willRebuild) {
    // canvas 尺寸不变 → 渲染回调里的 flushPendingScroll 不会触发，这里补一次
    void nextTick(() => flushPendingScroll())
  }
})

// 组件被停用：上报状态。位置只能用组件内记录值 —— 此刻 DOM 已被 KeepAlive 移出文档，读 scrollTop 恒为 0
onDeactivated(() => {
  isDeactivated = true
  if (reportTimer !== null) {
    window.clearTimeout(reportTimer)
    reportTimer = null
  }
  reportViewState()
})


// 缩放控制（手动缩放后锁定：窗口尺寸变化不再强制「适配宽度」，否则缩放会被 RO 冲掉）
const onMagnify = () => {
  if (!pdfDoc || !ctx) return
  if (scale.value < MAX_SCALE) {
    keepScale = true
    scale.value = Math.min(scale.value + SCALE_STEP, MAX_SCALE)
    renderPage(pageNum)
  }
}

const onReduce = () => {
  if (!pdfDoc || !ctx) return
  if (scale.value > MIN_SCALE) {
    keepScale = true
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

    if (!docPath.value) {
      throw new Error('No active document selected')
    }

    const doc = docStore.documents.get(docPath.value)
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

    // 恢复上次的页码 / 缩放 / 位置（会话快照里有），没有记录才用「适配宽度」
    const vs = readViewState(props.path)
    keepScale = vs.scale > 0
    scale.value = vs.scale > 0 ? vs.scale : await calculateFitScale()
    if (vs.page > 1) pageNum = vs.page
    liveScrollTop.value = vs.scrollTop
    pendingScrollTop = vs.scrollTop

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

// 监听激活文档变化：只有「激活文档变成本实例负责的文档」才加载。
// 否则切 tab 时旧实例的 watcher 会先于 deactivate 触发，把新文档的 PDF 加载进旧实例。
watch(
  activeDocumentPath,
  async (newId, oldId) => {
    if (isDeactivated) return
    if (oldId === undefined) return
    if (!newId || newId !== props.path) return
    await loadPdfFile()
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
        // canvas 尺寸已按新视口设置 → scrollHeight 就绪，可以把待恢复的位置落下去
        flushPendingScroll()
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
  // 翻页/缩放后立刻把页码与缩放同步进 store（不必等用户滚动或切走）
  scheduleReportViewState()
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
    // KeepAlive 淘汰（:max=8）走的是 unmount 而非 deactivate，这里兜底上报一次
    if (reportTimer !== null) {
      window.clearTimeout(reportTimer)
      reportTimer = null
    }
    reportViewState()
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
    <div class="pdf-container" ref="containerRef" @scroll="onContainerScroll">
      <!-- 加载状态 -->
      <div v-if="loading" class="loading-overlay">
        <div class="loading-spinner">Loading PDF...</div>
      </div>
      <!-- 错误状态 -->
      <div v-else-if="error" class="error-overlay">
        <div class="error-message">{{ error }}</div>
      </div>
      <canvas ref="canvasRef" class="pdf-canvas"></canvas>

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
