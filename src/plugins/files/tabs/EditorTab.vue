<template>
  <TabBarContainer ref="tabBarContainer">
    <!-- 左滚动按钮 -->
    <template #prepend>
      <button
        v-show="canScrollLeft"
        class="scroll-btn scroll-btn-left"
        @click="scrollLeft"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M10.5 3L5.5 8l5 5" stroke="currentColor" stroke-width="1.5" fill="none"/>
        </svg>
      </button>
    </template>

    <!-- Tab 列表 -->
    <TabBarItem
      v-for="doc in documentList"
      :key="doc.id"
      :data-doc-id="doc.id"
      :active="doc.id === activeDocumentId"
      :draggable="true"
      :dragging="dragState.draggedTab === doc.id"
      :drag-over="dragState.dragOverTab === doc.id"
      @click="handleTabClick(doc.id)"
      @close="handleTabClose(doc.id)"
      @dragstart="handleDragStart($event, doc.id)"
      @dragover="handleDragOver($event, doc.id)"
      @drop="handleDrop($event, doc.id)"
      @dragend="handleDragEnd"
      @mouseenter="showTooltip($event, doc.id)"
      @mouseleave="hideTooltip"
      @contextmenu="onTabContextMenu($event, doc.id)"
    >
      <!-- 文件图标 -->
      <span class="tab-icon">
        <FileIcon :type="doc.type" />
      </span>
      <!-- 文件名 -->
      <span class="tab-label">{{ doc.name }}</span>
      <!-- 修改指示器 -->
      <span v-if="doc.isDirty" class="tab-dirty-indicator">●</span>
      <!-- 关闭按钮 -->
      <TabCloseButton @close="handleTabClose(doc.id)" />
    </TabBarItem>

    <!-- 右滚动按钮 + 下拉菜单 -->
    <template #append>
      <button
        v-show="canScrollRight"
        class="scroll-btn scroll-btn-right"
        @click="scrollRight"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M5.5 3l5 5-5 5" stroke="currentColor" stroke-width="1.5" fill="none"/>
        </svg>
      </button>

      <!-- 收纳菜单：复用宿主 TabBarOverflowMenu（M3 移除 files 自建的第二套下拉） -->
      <TabBarOverflowMenu
        :items="overflowItems"
        :active-id="activeDocumentId || ''"
        button-title="显示所有打开的文件"
        header-label="打开的文件"
        @select="selectFromOverflow"
        @close="closeFromOverflow"
        @close-all="closeAllTabs"
      />

      <!-- Tooltip 提示框 -->
      <div
        v-show="tooltipVisible"
        class="file-path-tooltip"
        :style="tooltipStyle"
      >
        {{ tooltipText }}
      </div>
    </template>
  </TabBarContainer>

  <!-- 关闭全部确认（Tauri WebView2 不支持原生 confirm，统一走 GlobalDialog） -->
  <GlobalDialog v-model="showCloseAllConfirm" title="关闭全部" width="400px">
    <div class="close-all-message">
      确定要关闭所有打开的文件吗？<template v-if="dirtyCount > 0">其中有 {{ dirtyCount }} 个文件有未保存的更改，会一并保存。</template>
    </div>
    <template #footer>
      <app-button size="small" @click="showCloseAllConfirm = false">取消</app-button>
      <app-button size="small" type="danger" plain @click="confirmCloseAllTabs">全部关闭</app-button>
    </template>
  </GlobalDialog>

  <!-- 单个文档关闭确认：禁止「关闭即静默写回」，由用户显式选择 -->
  <GlobalDialog v-model="showCloseConfirm" title="未保存的更改" width="420px">
    <div class="close-all-message">
      「{{ pendingCloseName }}」有未保存的更改，请选择如何处理。
    </div>
    <template #footer>
      <app-button size="small" @click="cancelPendingClose">取消</app-button>
      <app-button size="small" plain @click="discardAndClose">不保存</app-button>
      <app-button size="small" type="primary" @click="saveAndClose">保存并关闭</app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { useDocumentManagerStore } from '@/store/modules/documents'
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { TabBarContainer, TabBarItem, TabCloseButton, TabBarOverflowMenu } from '@/components/TabBar'
import FileIcon from './FileIcon.vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { showContextMenu } from '@/components/context-menu/useContextMenu'
import { call } from '@/api/rpc'
import NotificationUtil from '@/utils/NotificationUtil'
import { openSearchPanel } from '../search/useFileSearch'
import { useSplitView } from '../view/useSplitView'
import { openInTerminal, runCurrentFile } from '../terminal/useTerminalLaunch'

const docStore = useDocumentManagerStore();
const split = useSplitView()

// 「关闭全部」确认弹窗
const showCloseAllConfirm = ref(false)

// TabBar 容器引用
const tabBarContainer = ref<InstanceType<typeof TabBarContainer> | null>(null)
const canScrollLeft = ref(false)
const canScrollRight = ref(false)

// Tooltip 相关
const tooltipVisible = ref(false)
const tooltipText = ref('')
const tooltipStyle = ref<any>({})
let tooltipTimer: number | null = null

// 计算属性
const documentList = computed(() => docStore.documentList)
const activeDocumentId = computed(() => docStore.activeDocumentPath)

/** 宿主收纳菜单只吃展示数据，关闭/激活的业务语义仍由本组件决定。 */
const overflowItems = computed(() =>
  documentList.value.map((doc) => ({
    id: doc.id,
    label: doc.name,
    title: doc.id,
    dirty: !!doc.isDirty
  }))
)

/**
 * 按文档 id 取标签页的真实 DOM 节点。
 *
 * 必须走 DOM 查询、不能存模板 ref：`<script setup>` 组件的模板 ref 拿到的是
 * **组件实例**（脚本内绑定默认不对外暴露），对它调 `getBoundingClientRect()`
 * 会抛 `TypeError: tab.getBoundingClientRect is not a function`。因为调用点在
 * `nextTick` 回调里，控制台表现为「Uncaught (in promise)」。
 * 标签页没有虚拟滚动、全部常驻 DOM，查询开销可忽略。
 */
function getTabEl(docId: string): HTMLElement | null {
  const container = getTabListEl()
  if (!container) return null
  return (
    Array.from(container.querySelectorAll<HTMLElement>('[data-doc-id]')).find(
      (el) => el.dataset.docId === docId
    ) ?? null
  )
}

// 显示 tooltip
function showTooltip(event: MouseEvent, docId: string) {
  const doc = docStore.documents.get(docId)
  if (!doc) return

  // 延迟显示，避免快速移动时闪烁
  tooltipTimer = window.setTimeout(() => {
    tooltipText.value = doc.id
    tooltipVisible.value = true

    // 计算 tooltip 位置（在鼠标下方）
    const target = event.target as HTMLElement
    const rect = target.getBoundingClientRect()

    tooltipStyle.value = {
      left: `${rect.left}px`,
      top: `${rect.bottom + 8}px`,
      maxWidth: '600px'
    }
  }, 500)
}

// 隐藏 tooltip
function hideTooltip() {
  if (tooltipTimer) {
    clearTimeout(tooltipTimer)
    tooltipTimer = null
  }
  tooltipVisible.value = false
}

// 处理标签页点击：分屏时把文档放进当前 pane（同路径双开会被 useSplitView 拒绝）
function handleTabClick(docId: string) {
  split.activateDocument(docId)
  scrollToTab(docId)
}

// ===== 关闭前确认：禁止「关闭即静默写回」，由用户显式选择 =====
const showCloseConfirm = ref(false)
const pendingCloseId = ref<string | null>(null)
const pendingCloseName = computed(() => {
  if (!pendingCloseId.value) return ''
  return docStore.documents.get(pendingCloseId.value)?.name || ''
})
const dirtyCount = computed(() => documentList.value.filter(d => d.isDirty).length)

/** 标签右键菜单：复用宿主 showContextMenu（Tauri WebView2 已屏蔽原生右键菜单）。 */
function onTabContextMenu(e: MouseEvent, docId: string) {
  e.preventDefault()
  hideTooltip()
  showContextMenu({
    x: e.clientX,
    y: e.clientY,
    items: [
      { label: '关闭', onClick: () => requestClose(docId) },
      { label: '关闭其他', onClick: () => docStore.closeOtherDocuments(docId) },
      {
        label: '关闭全部',
        onClick: () => { showCloseAllConfirm.value = true }
      },
      { divider: true },
      { label: '在右侧分屏打开', onClick: () => split.openInRightPane(docId) },
      {
        label: '与当前文件比较',
        onClick: () => {
          const current = docStore.activeDocumentPath || ''
          if (!current || current === docId) {
            NotificationUtil.warn('请先激活另一份文件再比较')
            return
          }
          split.openDiff(current, docId)
        }
      },
      { label: '复制路径', onClick: () => void copyTabPath(docId) },
      { label: '在文件管理器中显示', onClick: () => void revealTabPath(docId) },
      { label: '在集成终端打开所在目录', onClick: () => { openInTerminal(docId) } },
      { label: '在集成终端运行当前脚本', onClick: () => { runCurrentFile(docId) } },
      { divider: true },
      { label: '在文件中查找', onClick: () => openSearchPanel() }
    ]
  })
}

async function copyTabPath(docId: string) {
  try {
    await navigator.clipboard.writeText(docId)
  } catch {
    // WebView2 下 clipboard API 可能不可用，退回 execCommand（与 logs LogTabs 同范式）
    const ta = document.createElement('textarea')
    ta.value = docId
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    ta.remove()
  }
  NotificationUtil.success('已复制文件路径')
}

async function revealTabPath(docId: string) {
  try {
    const res = await call<{ code: number; msg: string }>('file_manager_open_path', { path: docId })
    if (res?.code !== 200) NotificationUtil.error(res?.msg || '打开所在目录失败')
  } catch (e: any) {
    console.error('[files/tabs] 打开所在目录失败', e)
    NotificationUtil.error(e?.message || '打开所在目录失败')
  }
}

function requestClose(docId: string) {
  const doc = docStore.documents.get(docId)
  if (doc?.isDirty) {
    pendingCloseId.value = docId
    showCloseConfirm.value = true
    return
  }
  docStore.closeDocument(docId)
}

function cancelPendingClose() {
  showCloseConfirm.value = false
  pendingCloseId.value = null
}

function discardAndClose() {
  const id = pendingCloseId.value
  cancelPendingClose()
  if (id) docStore.closeDocument(id, { skipSave: true })
}

async function saveAndClose() {
  const id = pendingCloseId.value
  if (!id) { cancelPendingClose(); return }
  const ok = await docStore.saveDocument(id)
  cancelPendingClose()
  // 已显式保存，关闭时跳过 store 里那条历史自动保存
  if (ok) docStore.closeDocument(id, { skipSave: true })
}

// 处理标签页关闭
function handleTabClose(docId: string) {
  // 关闭标签页  Tooltip也同时关闭
  hideTooltip()
  requestClose(docId)
}

// 获取 tab 列表容器
function getTabListEl(): HTMLElement | null {
  return tabBarContainer.value?.tabListRef || null
}

// 滚动到指定标签页
function scrollToTab(docId: string) {
  nextTick(() => {
    const container = getTabListEl()
    const tab = getTabEl(docId)
    if (!container || !tab) return

    const tabRect = tab.getBoundingClientRect()
    const containerRect = container.getBoundingClientRect()

    // 如果标签页在可视区域外，滚动使其可见
    if (tabRect.left < containerRect.left) {
      container.scrollLeft -= (containerRect.left - tabRect.left) + 8
    } else if (tabRect.right > containerRect.right) {
      container.scrollLeft += (tabRect.right - containerRect.right) + 8
    }

    updateScrollButtons()
  })
}

// 向左滚动
function scrollLeft() {
  const container = getTabListEl()
  if (!container) return

  container.scrollBy({
    left: -200,
    behavior: 'smooth'
  })
}

// 向右滚动
function scrollRight() {
  const container = getTabListEl()
  if (!container) return

  container.scrollBy({
    left: 200,
    behavior: 'smooth'
  })
}

// 更新滚动按钮状态
function updateScrollButtons() {
  const container = getTabListEl()
  if (!container) return

  const { scrollLeft, scrollWidth, clientWidth } = container

  canScrollLeft.value = scrollLeft > 0
  canScrollRight.value = scrollLeft + clientWidth < scrollWidth - 1
}

// 宿主收纳菜单的三个意图上报：激活 / 关闭单个 / 关闭全部
function selectFromOverflow(docId: string) {
  split.activateDocument(docId)
  scrollToTab(docId)
}

function closeFromOverflow(docId: string) {
  requestClose(docId)
}

// 关闭所有标签页（先弹确认，避免误触；确认后走批量关闭，快照只同步一次）
function closeAllTabs() {
  showCloseAllConfirm.value = true
}

async function confirmCloseAllTabs() {
  showCloseAllConfirm.value = false
  // 用户点「全部关闭」即授权保存全部脏文档；显式保存后再关闭，避免 store 里那条兜底自动保存
  if (dirtyCount.value > 0) await docStore.saveAllDocuments()
  docStore.closeAllDocuments()
  split.disableSplit()
}

// 监听文档列表变化，自动滚动到新激活的标签页
watch(activeDocumentId, (newId) => {
  if (newId) {
    scrollToTab(newId)
  }
})

// 监听滚动事件
onMounted(() => {
  const container = getTabListEl()
  if (container) {
    container.addEventListener('scroll', updateScrollButtons, { passive: true })
    updateScrollButtons()
  }

})

onUnmounted(() => {
  const container = getTabListEl()
  if (container) {
    container.removeEventListener('scroll', updateScrollButtons)
  }

  // 清理 tooltip 定时器
  if (tooltipTimer) {
    clearTimeout(tooltipTimer)
  }
})

// 拖拽相关状态
const dragState = ref({
  isDragging: false,
  draggedTab: null as string | null,
  dragOverTab: null as string | null,
  startX: 0,
  currentX: 0
})

// 处理拖拽开始
function handleDragStart(event: DragEvent, docId: string) {
  dragState.value.isDragging = true
  dragState.value.draggedTab = docId

  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
    event.dataTransfer.setData('text/plain', docId)

    // 创建拖拽预览
    const dragImage = event.target as HTMLElement
    event.dataTransfer.setDragImage(dragImage, 0, 0)
  }
}

// 处理拖拽经过
function handleDragOver(event: DragEvent, docId: string) {
  event.preventDefault()
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'move'
  }
  dragState.value.dragOverTab = docId
}

// 处理放置
function handleDrop(event: DragEvent, targetDocId: string) {
  event.preventDefault()

  const draggedDocId = dragState.value.draggedTab
  if (!draggedDocId || draggedDocId === targetDocId) return

  // 重新排序文档
  const docs = [...docStore.documentOrder]
  const draggedIndex = docs.indexOf(draggedDocId)
  const targetIndex = docs.indexOf(targetDocId)

  if (draggedIndex !== -1 && targetIndex !== -1) {
    docs.splice(draggedIndex, 1)
    docs.splice(targetIndex, 0, draggedDocId)
    docStore.reorderDocuments(docs)
    //docStore.reorderDocuments（
  }

  // 重置拖拽状态
  dragState.value = {
    isDragging: false,
    draggedTab: null,
    dragOverTab: null,
    startX: 0,
    currentX: 0
  }
}

// 处理拖拽结束
function handleDragEnd() {
  dragState.value = {
    isDragging: false,
    draggedTab: null,
    dragOverTab: null,
    startX: 0,
    currentX: 0
  }
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;
@use 'sass:color';

/* 滚动按钮样式 */
.scroll-btn {
  flex-shrink: 0;
  width: 28px;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #2d2d2d;
  border: none;
  color: #cccccc;
  cursor: pointer;
  transition: background-color 0.2s;
  z-index: 10;

  @include theme.useTheme {
     background-color: color.adjust(theme.getVar('bgColor'), $lightness:5%);
    color: theme.getVar('textColor');
  }
}

.scroll-btn:hover {
  background-color: #3e3e3e;
  color: #ffffff;

  @include theme.useTheme {
     background-color: color.adjust(theme.getVar('bgColor'), $lightness:10%);
    color: theme.getVar('textColor');
  }
}

.scroll-btn:active {
  background-color: #4a4a4a;

  @include theme.useTheme {
     background-color: color.adjust(theme.getVar('bgColor'), $lightness:15%);
  }
}

/* 文件图标 */
.tab-icon {
  flex-shrink: 0;
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 文件名标签 */
.tab-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 修改指示器 */
.tab-dirty-indicator {
  flex-shrink: 0;
  color: #e2b714;
  font-size: theme.getVar('fontSizeLg');
  line-height: 1;
}

/* Tooltip 提示框样式 */
.file-path-tooltip {
  position: fixed;
  background-color: #252526;
  color: #cccccc;
  padding: 6px 10px;
  border: 1px solid #3e3e3e;
  border-radius: var(--radius-sm);
  font-size: theme.getVar('fontSizeSmall');
  font-family: var(--font-family-mono);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  z-index: 9999;
  pointer-events: none;
  white-space: nowrap;
  max-width: 600px;
  overflow: hidden;
  text-overflow: ellipsis;
  animation: tooltipFadeIn 0.15s ease-out;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    color: theme.getVar('textColor');
    border-color: theme.getVar('borderColor');
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.5);
  }
}

@keyframes tooltipFadeIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

</style>
