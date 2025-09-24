<script setup lang="ts">
import { onMounted, onUnmounted, ref, nextTick, watch, computed, ComputedRef } from "vue"
import * as pdfjsLib from "pdfjs-dist"
import { useRoute } from 'vue-router'
import pdfjsWorker from 'pdfjs-dist/build/pdf.worker.min?url'
import { ElMessage } from 'element-plus'
import { convertFileSrc } from '@tauri-apps/api/core'
import useStore from '@/store/index'

const { appConfig, system } = useStore()

const route = useRoute()
const scale = ref<number>(3)
const page_num = ref<number>(1)
const page_count = ref<number>(0)
const loading = ref<boolean>(true)
const error = ref<string | null>(null)


let isDark: ComputedRef<boolean> = computed(() =>
  system.isDark
)
watch(() => system.isDark, (newValue, oldValue) => {
  console.log("watch  toggleDarkMode", newValue, oldValue)
  toggleDarkMode(newValue)
})
let pdfDoc: any = null
let pageNum = 1
let pageRendering = false
let pageNumPending: number | null = null
// 使用模板引用
const canvasRef = ref<HTMLCanvasElement | null>(null)
let canvas: HTMLCanvasElement | null = null
let ctx: CanvasRenderingContext2D | null = null

pdfjsLib.GlobalWorkerOptions.workerSrc = pdfjsWorker


// 添加加载状态管理
const onMagnify = () => {
  if (!pdfDoc || !ctx) return
  if (scale.value < 5) {
    scale.value += 0.2
    renderPage(pageNum)
  }
}

const onReduce = () => {
  if (!pdfDoc || !ctx) return
  if (scale.value > 0.2) {
    scale.value -= 0.2
    renderPage(pageNum)
  }
}

function scrollToTop() {
  nextTick(() => {
    const pdfContainer = document.querySelector('.pdf-container')
    if (pdfContainer) {
      pdfContainer.scrollTo({ top: 0, behavior: 'smooth' })
    }
  })
}

// 改进错误处理
const loadPdfFile = async () => {
  try {
    loading.value = true
    error.value = null
   // 确保 canvas 已经渲染
    await nextTick()
      if (!canvasRef.value) {
      throw new Error('Canvas element not found')
    }
    let filePath = ""
    if (route.meta.filePath) {
      filePath = decodeURIComponent(route.meta.filePath as string)
    } else if (route.query.file) {
      filePath = route.query.file as string
    }

    const url = convertFileSrc(filePath)
    // canvas = document.getElementById('the-canvas') as HTMLCanvasElement
    // ctx = canvas.getContext('2d')
      canvas = canvasRef.value
    ctx = canvas.getContext('2d')

    const loadingTask = pdfjsLib.getDocument(url)

    // 添加加载进度监听
    loadingTask.onProgress = (progressData) => {
      console.log(`Loading: ${progressData.loaded / progressData.total * 100}%`)
    }

    pdfDoc = await loadingTask.promise
    page_count.value = pdfDoc.numPages
    renderPage(pageNum)
  } catch (err: any) {
    error.value = err.message || 'Failed to load PDF'
    ElMessage.error(error.value)
  } finally {
    loading.value = false
  }
}
function renderPage(num: number) {
  if (!pdfDoc || !ctx || !canvas) return

  pageRendering = true
  pdfDoc.getPage(num).then(function (page: any) {
    const viewport = page.getViewport({ scale: scale.value })
    
    // 确保在设置 canvas 尺寸前 canvas 元素存在
    if (canvas) {
      // 先设置 canvas 的显示尺寸
      canvas.style.width = viewport.width + 'px'
      canvas.style.height = viewport.height + 'px'
      
      // 再设置 canvas 的绘图尺寸（必须是整数）
      canvas.width = Math.floor(viewport.width)
      canvas.height = Math.floor(viewport.height)
      
      // 设置 CSS 样式以确保正确显示
      canvas.style.display = 'block'
      canvas.style.margin = '0 auto'
    }

    if (canvas && ctx) {
      // 清除画布
      ctx.clearRect(0, 0, canvas.width, canvas.height)
      
      const renderContext = {
        canvasContext: ctx,
        viewport: viewport,
        background: 'rgba(0,0,0,0)'
      }
      
      const renderTask = page.render(renderContext)
      renderTask.promise.then(function () {
        pageRendering = false
        if (pageNumPending !== null) {
          renderPage(pageNumPending)
          pageNumPending = null
        }
      }).catch((error: any) => {
        console.error('渲染页面出错:', error)
        pageRendering = false
      })
    }
  }).catch((error: any) => {
    console.error('获取页面出错:', error)
    pageRendering = false
  })

  page_num.value = num
}
// 修改 renderPage 函数，添加安全检查
// function renderPage(num: number) {
//   if (!pdfDoc || !ctx || !canvas) return

//   pageRendering = true
//   pdfDoc.getPage(num).then(function (page: any) {
//     const viewport = page.getViewport({ scale: scale.value })
//     // 添加安全检查
//     if (canvas && ctx) {
//       canvas.height = viewport.height
//       canvas.width = viewport.width

//       const renderContext = {
//         canvasContext: ctx,
//         viewport: viewport,
//         background: 'rgba(0,0,0,0)'
//       }
      
//       const renderTask = page.render(renderContext)
//       renderTask.promise.then(function () {
//         pageRendering = false
//         if (pageNumPending !== null) {
//           renderPage(pageNumPending)
//           pageNumPending = null
//         }
//       })
//     }
//   })

//   page_num.value = num
// }
// function renderPage(num: number) {
//   if (!pdfDoc || !ctx || !canvas) return

//   pageRendering = true
//   pdfDoc.getPage(num).then(function (page: any) {
//     const viewport = page.getViewport({ scale: scale.value })
//     if (canvas && ctx) {
//       canvas.height = viewport.height
//       canvas.width = viewport.width

//       const renderContext = {
//         canvasContext: ctx,
//         viewport: viewport,
//         background: 'rgba(0,0,0,0)' // 透明背景
//       }

//       const renderTask = page.render(renderContext)
//       renderTask.promise.then(function () {
//         pageRendering = false
//         if (pageNumPending !== null) {
//           renderPage(pageNumPending)
//           pageNumPending = null
//         }
//       })
//     }
//   })

//   page_num.value = num
// }

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

// 添加跳转到指定页面功能
const goToPage = (page: number) => {
  if (page >= 1 && page <= page_count.value && pdfDoc) {
    pageNum = page
    queueRenderPage(pageNum)
    scrollToTop()
  }
}

// 监听路由参数变化
watch(
  () => route.path,
  () => {
    loadPdfFile()
  },
  { immediate: true }
)

// 添加键盘快捷键支持
onMounted(() => {
  const handleKeydown = (e: KeyboardEvent) => {
    // 防止在输入框中触发
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
  onUnmounted(() => {
    window.removeEventListener('keydown', handleKeydown)
  })
  toggleDarkMode(isDark.value)
  loadPdfFile();
})
// const renderContext = {
//   canvasContext: ctx,
//   viewport: viewport,
//   // 设置透明背景，然后在容器上设置暗色背景
//   background: 'rgba(0,0,0,0)' // 透明背景
// };
// 添加主题切换功能
const toggleDarkMode = (isDarkMode: boolean) => {
  const canvas = document.getElementById('the-canvas')
  if (isDarkMode) {
    canvas?.classList.add('dark-mode')
  } else {
    canvas?.classList.remove('dark-mode')
  }
}
</script>

<template>
  <div class="pdf-container">
    <!-- 加载状态 -->
    <div v-if="loading" class="loading-overlay">
      <div class="loading-spinner">Loading PDF...</div>
    </div>
    <!-- 错误状态 -->
    <div v-else-if="error" class="error-overlay">
      <div class="error-message">{{ error }}</div>
    </div>
    <!-- <canvas class="pdf-canvas" id="the-canvas"></canvas> -->
         <canvas ref="canvasRef" class="pdf-canvas" id="the-canvas"></canvas>

    <div class="pdf-controls">
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
  ;
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
  position: fixed;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 16px;
  border-radius: 30px;
  box-shadow: 0 4px 15px rgba(0, 0, 0, 0.15);
  z-index: 100;
  backdrop-filter: blur(5px);
  /* 使用hsla设置背景色 */
  background-color: hsla(0, 0%, 100%, 0.1);
  /* 最后一个参数0.5控制透明度 */
  border: 1px solid rgba(0, 0, 0, 0.05);
}

.pdf-controls button {
  padding: 8px 16px;
  border: 1px solid var(--el-border-color-light);
  background-color: var(--el-fill-color-light);
  ;
  border-radius: 20px;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.2s ease;
  min-width: 60px;
  box-shadow: 0 2px 3px rgba(0, 0, 0, 0.05);
}

.pdf-controls button:hover:not(:disabled) {
  background-color: var(--el-fill-color-light);
  ;
  color: var(--el-text-color-regular);
  border-color: var(--el-border-color-light);
  transform: translateY(-2px);
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.1);
}

.pdf-controls button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
  box-shadow: 0 2px 5px rgba(0, 0, 0, 0.05);
}

.pdf-controls span {
  font-weight: 500;
  color: var(--el-text-color-regular);
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