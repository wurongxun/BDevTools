<template>
  <!-- 全局标准 TabBar（Chrome 风格，与 database/ssh/files 一致） -->
  <TabBarContainer ref="barRef" class="log-tabs"  aria-label="日志源标签">
    <TabBarItem
      v-for="s in store.sources"
      :key="s.id"
      :active="s.id === store.activeSourceId"
      :title="s.path"
      draggable
      :dragging="s.id === dragId"
      :drag-over="dragOverId === s.id"
      @click="store.setActiveSource(s.id)"
      @close="store.closeSource(s.id)"
      @dragstart="onDragStart(s.id, $event)"
      @dragover.prevent="onDragOver(s.id)"
      @drop="onDrop(s.id, $event)"
      @dragend="onDragEnd"
      @contextmenu.prevent="onTabContextMenu(s.id, $event)"
    >
      <span class="tab-dot" :style="{ background: s.color }"></span>
      <el-icon class="tab-icon">
        <Document v-if="s.kind === 'local'" />
        <Connection v-else-if="s.kind === 'ssh'" />
        <VideoPlay v-else />
      </el-icon>
      <span class="tab-name">{{ s.name }}</span>
      <span v-if="s.kind === 'stream'" class="tab-badge" :class="`st-${s.streamState}`">
        {{ streamLabel(s.streamState) }}
      </span>
      <span v-if="store.isSourceLoading(s.id)" class="tab-loading" title="加载中">
        <el-icon class="is-loading"><Loading /></el-icon>
      </span>
      <TabCloseButton v-else title="关闭" @close="store.closeSource(s.id)" />
    </TabBarItem>
  </TabBarContainer>

  <!-- tab 右键菜单 -->
  <teleport to="body">
    <div
      v-if="ctxVisible"
      class="tab-ctx-menu"
      :style="{ left: ctxPos.x + 'px', top: ctxPos.y + 'px' }"
      @click.stop
    >
      <div class="tab-ctx-item" @click="ctxClose">关闭</div>
      <div class="tab-ctx-item" @click="ctxCloseOthers">关闭其它</div>
      <div class="tab-ctx-item" @click="ctxCloseToRight">关闭右侧</div>
      <div class="tab-ctx-item" @click="ctxCloseAll">关闭全部</div>
      <div class="tab-ctx-item" @click="ctxReload">
        {{ ctxSource?.kind === 'stream' ? '重新连接' : '重新加载' }}
      </div>
      <div class="tab-ctx-sep"></div>
      <div class="tab-ctx-item" @click="ctxCopyPath">复制路径</div>
      <div
        v-if="ctxSource && ctxSource.kind !== 'stream'"
        class="tab-ctx-item"
        @click="ctxReveal"
      >
        在文件管理器中显示
      </div>
    </div>
  </teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { Document, Connection, VideoPlay, Loading } from '@element-plus/icons-vue'
import { TabBarContainer, TabBarItem, TabCloseButton } from '@/components/TabBar'
import { call } from '@/api/rpc'
import { useLogStore } from '../store/logs'

const store = useLogStore()

/** 标准 TabBar 容器实例（暴露 tabListRef，用于把激活 tab 滚动到可见区域） */
const barRef = ref<InstanceType<typeof TabBarContainer> | null>(null)

/** 正在拖拽的 tab（用于置灰） */
const dragId = ref('')
/** 拖拽悬停落点的 tab（显示落点指示线） */
const dragOverId = ref('')

// 激活 tab 变化后，若其超出横向可视区则自动滚动到可见（避免多 tab 时需要手动找）
watch(
  () => store.activeSourceId,
  () => {
    nextTick(() => {
      const list = barRef.value?.tabListRef
      const el = list?.querySelector('.tab-bar-item-active') as HTMLElement | null
      el?.scrollIntoView({ inline: 'nearest', block: 'nearest' })
    })
  },
)

function onDragStart(id: string, e: DragEvent) {
  dragId.value = id
  e.dataTransfer?.setData('text/plain', id)
  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
}

function onDragOver(id: string) {
  dragOverId.value = id
}

function onDrop(targetId: string, e: DragEvent) {
  e.preventDefault()
  if (dragId.value && dragId.value !== targetId) {
    store.moveSource(dragId.value, targetId)
  }
  dragId.value = ''
  dragOverId.value = ''
}

function onDragEnd() {
  dragId.value = ''
  dragOverId.value = ''
}

// ==================== 右键菜单 ====================
const ctxVisible = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxId = ref('')

/** 当前右键菜单针对的源（用于条件显示"在文件管理器中显示"等） */
const ctxSource = computed(() => store.sources.find(s => s.id === ctxId.value) ?? null)

function onTabContextMenu(id: string, e: MouseEvent) {
  ctxId.value = id
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxVisible.value = true
}

function closeContextMenu() {
  ctxVisible.value = false
  ctxId.value = ''
}

function onDocumentClick() {
  closeContextMenu()
}

async function ctxClose() {
  const id = ctxId.value
  closeContextMenu()
  if (id) await store.closeSource(id)
}
async function ctxCloseOthers() {
  const id = ctxId.value
  closeContextMenu()
  if (id) await store.closeOthers(id)
}
async function ctxCloseToRight() {
  const id = ctxId.value
  closeContextMenu()
  if (id) await store.closeToRight(id)
}
async function ctxCloseAll() {
  closeContextMenu()
  await store.closeAll()
}

/** 重新加载 / 重新连接当前右键源 */
async function ctxReload() {
  const id = ctxId.value
  closeContextMenu()
  if (id) await store.reloadSource(id)
}

/** 复制源路径到剪贴板 */
function ctxCopyPath() {
  const s = ctxSource.value
  closeContextMenu()
  if (!s) return
  const text = s.path
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

/** 在文件管理器中定位该日志文件（流源无本地路径，已对菜单项隐藏） */
async function ctxReveal() {
  const s = ctxSource.value
  closeContextMenu()
  if (!s || s.kind === 'stream') return
  try {
    await call('file_manager_open_path', { path: s.path })
  } catch {
    /* ignore */
  }
}

onMounted(() => document.addEventListener('click', onDocumentClick))
onUnmounted(() => document.removeEventListener('click', onDocumentClick))

function streamLabel(state?: string): string {
  switch (state) {
    case 'live':
      return '在线'
    case 'connecting':
      return '连接中'
    case 'paused':
      return '已暂停'
    case 'error':
      return '错误'
    default:
      return ''
  }
}
</script>

<style scoped lang="scss">
// tab 外壳（底色/圆角/凹脚/关闭按钮）由标准 TabBar 组件负责，这里只写内容元素
.tab-dot {
  flex: none;
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.tab-icon {
  flex: none;
  font-size: var(--font-size-md);
  color: var(--el-text-color-secondary);
}

.tab-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tab-badge {
  flex: none;
  font-size: var(--font-size-xs);
  padding: 0 5px;
  border-radius: var(--radius-xl);
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color);

  &.st-live {
    color: var(--el-color-success);
    background: var(--el-color-success-light-9);
  }
  &.st-error {
    color: var(--el-color-danger);
    background: var(--el-color-danger-light-9);
  }
  &.st-connecting {
    color: var(--el-color-warning);
    background: var(--el-color-warning-light-9);
  }
}

.tab-loading {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  color: var(--el-color-primary);

  .el-icon {
    font-size: var(--font-size-lg);
  }
}

.tab-ctx-menu {
  position: fixed;
  z-index: 1600;
  min-width: 132px;
  padding: 4px 0;
  background: var(--el-bg-color-overlay);
  border: 1px solid var(--el-border-color-light);
  border-radius: var(--radius-md);
  box-shadow: var(--el-box-shadow-light);
}

.tab-ctx-item {
  padding: 5px 12px;
  font-size: var(--font-size-md);
  cursor: pointer;
  color: var(--el-text-color-regular);

  &:hover {
    background: var(--el-fill-color-light);
  }
}

.tab-ctx-sep {
  height: 1px;
  margin: 4px 0;
  background: var(--el-border-color-lighter);
}
</style>
