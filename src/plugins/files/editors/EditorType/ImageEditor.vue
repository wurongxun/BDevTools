<template>
  <div class="image-wrapper">
    <!-- 工具栏 -->
    <div class="img-toolbar">
      <el-tooltip content="缩小" placement="bottom">
        <app-button size="small" :disabled="!ready" @click="zoomOut">
          <el-icon><ZoomOut /></el-icon>
        </app-button>
      </el-tooltip>
      <span class="img-scale" title="点击适应窗口" @click="resetFit">{{ scaleText }}</span>
      <el-tooltip content="放大" placement="bottom">
        <app-button size="small" :disabled="!ready" @click="zoomIn">
          <el-icon><ZoomIn /></el-icon>
        </app-button>
      </el-tooltip>
      <el-tooltip content="适应窗口" placement="bottom">
        <app-button size="small" :disabled="!ready" @click="resetFit">适应</app-button>
      </el-tooltip>
      <el-tooltip content="原始大小" placement="bottom">
        <app-button size="small" :disabled="!ready" @click="actualSize">1:1</app-button>
      </el-tooltip>
      <el-tooltip content="旋转 90°" placement="bottom">
        <app-button size="small" :disabled="!ready" @click="rotateBy(90)">
          <el-icon><RefreshRight /></el-icon>
        </app-button>
      </el-tooltip>
      <span class="img-info">{{ infoText }}</span>
    </div>

    <!-- 图片区 -->
    <div ref="stageRef" class="img-stage">
      <div v-if="loading" class="img-placeholder">加载中…</div>
      <div v-else-if="error" class="img-placeholder is-error">{{ error }}</div>
      <img
        v-else-if="src"
        ref="imgRef"
        :src="src"
        :style="imgStyle"
        class="img-canvas"
        draggable="false"
        @load="onImgLoad"
        @error="onImgError"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { ZoomIn, ZoomOut, RefreshRight } from '@element-plus/icons-vue'
import { convertFileSrc } from '@/core/runtime'
import { useDocumentManagerStore } from '@/store/modules/documents'

defineOptions({ name: 'ImageEditor' })

const props = defineProps<{
  /** 本实例负责的文档路径（父级 FileEditor 按文档 key 复用实例，一个实例只对应一个文档） */
  path: string
}>()

const MIN_SCALE = 0.05
const MAX_SCALE = 20
const ZOOM_STEP = 1.25

/** 扩展名 → MIME。缺了 MIME 浏览器不会解码图片（直接当下载），所以这份表是必需的 */
const MIME_MAP: Record<string, string> = {
  png: 'image/png',
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  gif: 'image/gif',
  bmp: 'image/bmp',
  webp: 'image/webp',
  avif: 'image/avif',
  svg: 'image/svg+xml',
  ico: 'image/x-icon',
  tif: 'image/tiff',
  tiff: 'image/tiff',
}

function mimeOf(name: string): string {
  const ext = name.split('.').pop()?.toLowerCase() ?? ''
  return MIME_MAP[ext] ?? 'application/octet-stream'
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / 1024 / 1024).toFixed(2)} MB`
}

const docStore = useDocumentManagerStore()

const stageRef = ref<HTMLElement | null>(null)
const imgRef = ref<HTMLImageElement | null>(null)
const src = ref('')
const loading = ref(true)
const error = ref('')
const rotate = ref(0)
/** 'fit' = 跟随容器；'manual' = 用户显式缩放 */
const mode = ref<'fit' | 'manual'>('fit')
const manualScale = ref(1)
const natural = ref<{ w: number; h: number } | null>(null)
const bytes = ref(0)
const box = ref({ w: 0, h: 0 })

/** objectURL 必须显式回收：KeepAlive 会缓存实例，泄漏的 Blob 会一直占着内存 */
let objectUrl: string | null = null
function releaseUrl() {
  if (objectUrl) {
    URL.revokeObjectURL(objectUrl)
    objectUrl = null
  }
}

const doc = computed(() => docStore.documents.get(props.path))

/** 旋转后参与布局的「视觉尺寸」：90°/270° 时宽高互换 */
const oriented = computed(() => {
  const n = natural.value
  if (!n) return null
  return rotate.value % 180 === 0 ? { w: n.w, h: n.h } : { w: n.h, h: n.w }
})

const fitScale = computed(() => {
  const o = oriented.value
  if (!o || !box.value.w || !box.value.h || !o.w || !o.h) return 1
  return Math.min(box.value.w / o.w, box.value.h / o.h, 1)
})

const currentScale = computed(() => (mode.value === 'fit' ? fitScale.value : manualScale.value))

const imgStyle = computed(() => {
  const style: Record<string, string> = { transform: `rotate(${rotate.value}deg)` }
  const n = natural.value
  if (!n || !n.w || !n.h) {
    // 无固有尺寸（SVG 只有 viewBox 时 naturalWidth 可能是 0）：交给 CSS 约束，
    // 否则会算出 width: 0px —— 图直接看不见
    style.maxWidth = '100%'
    style.maxHeight = '100%'
    return style
  }
  style.width = `${Math.round(n.w * currentScale.value)}px`
  return style
})

const ready = computed(() => !!natural.value && !error.value)
const scaleText = computed(() => `${Math.round(currentScale.value * 100)}%`)
const infoText = computed(() => {
  const n = natural.value
  if (!n) return ''
  const parts = [`${n.w} × ${n.h}`]
  if (bytes.value) parts.push(formatBytes(bytes.value))
  return parts.join(' · ')
})

function buildSource() {
  releaseUrl()
  const d = doc.value
  if (!d) {
    error.value = '文档不存在'
    loading.value = false
    return
  }
  loading.value = true
  error.value = ''
  natural.value = null
  rotate.value = 0
  mode.value = 'fit'

  const buf = d.content
  if (buf instanceof ArrayBuffer && buf.byteLength > 0) {
    // 主路径：后端 read_file_bytes 给的裸字节（前端直接拿到 ArrayBuffer）
    bytes.value = buf.byteLength
    objectUrl = URL.createObjectURL(new Blob([buf], { type: mimeOf(d.name) }))
    src.value = objectUrl
    return
  }
  // 兜底：content 没加载（超过后端 64MB 上限 / 读取失败）→ 走 asset 协议让浏览器直接读文件，
  // 这样超大图依然能看，代价是依赖 assetProtocol 的 scope。
  bytes.value = 0
  const p = d.isRemote ? d.localTempPath || d.id : d.id
  if (!p) {
    error.value = '无法定位文件路径'
    loading.value = false
    return
  }
  src.value = convertFileSrc(p)
}

function onImgLoad() {
  const el = imgRef.value
  natural.value = el ? { w: el.naturalWidth, h: el.naturalHeight } : null
  loading.value = false
  error.value = ''
  measure()
}

function onImgError() {
  loading.value = false
  natural.value = null
  error.value = '图片解码失败（文件可能已损坏或不是图片）'
}

function measure() {
  const el = stageRef.value
  if (el) box.value = { w: el.clientWidth, h: el.clientHeight }
}

function setScale(next: number) {
  if (!natural.value) return
  mode.value = 'manual'
  manualScale.value = Math.min(MAX_SCALE, Math.max(MIN_SCALE, next))
}
function zoomIn() {
  setScale(currentScale.value * ZOOM_STEP)
}
function zoomOut() {
  setScale(currentScale.value / ZOOM_STEP)
}
function resetFit() {
  mode.value = 'fit'
}
function actualSize() {
  mode.value = 'manual'
  manualScale.value = 1
}
function rotateBy(deg: number) {
  rotate.value = (rotate.value + deg) % 360
}

let ro: ResizeObserver | null = null
function attachObserver() {
  ro?.disconnect()
  const el = stageRef.value
  if (!el || typeof ResizeObserver === 'undefined') return
  ro = new ResizeObserver(() => measure())
  ro.observe(el)
}

// 文档内容异步到位后再建源；path 变化（KeepAlive 复用实例）时重建
watch([() => props.path, () => doc.value?.content], buildSource, { immediate: true })
watch(stageRef, attachObserver)

onBeforeUnmount(() => {
  ro?.disconnect()
  ro = null
  releaseUrl()
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.image-wrapper {
  width: 100%;
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;

  @include theme.useTheme {
    background: theme.getVar('appMainBgColor');
  }
}

.img-toolbar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  flex-shrink: 0;

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.img-scale {
  min-width: 48px;
  text-align: center;
  cursor: pointer;
  font-size: var(--font-size-sm);
  font-variant-numeric: tabular-nums;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.img-info {
  margin-left: auto;
  font-size: var(--font-size-sm);
  font-family: var(--font-family-mono);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.img-stage {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 12px;
}

.img-canvas {
  display: block;
  // 变换以中心为轴，旋转时才不会甩出容器
  transform-origin: center center;
  user-select: none;
}

.img-placeholder {
  font-size: var(--font-size-base);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &.is-error {
    @include theme.useTheme {
      color: theme.getVar('dangerText');
    }
  }
}
</style>
