<template>
  <div class="tab-bar-container">
    <!-- 左滚动按钮 -->
    <button
      v-show="canScrollLeft"
      class="scroll-btn scroll-btn-left"
      @click="scrollLeft"
    >
      <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
        <path d="M10.5 3L5.5 8l5 5" stroke="currentColor" stroke-width="1.5" fill="none"/>
      </svg>
    </button>

    <!-- 标签页容器 -->
    <div
      ref="tabContainer"
      class="tab-list"
      @wheel.prevent="handleWheel"
    >
      <div
        v-for="doc in documentList"
        :key="doc.id"
        :ref="el => setTabRef(doc.id, el)"
        class="tab-item"
        :class="{
      'tab-active': doc.id === activeDocumentId,
      'tab-modified': doc.isDirty,
      'tab-dragging': dragState.draggedTab === doc.id,
      'tab-drag-over': dragState.dragOverTab === doc.id
    }"
        draggable="true"
        @click="handleTabClick(doc.id)"
        @mousedown.middle.prevent="handleTabClose(doc.id)"
        @dragstart="handleDragStart($event, doc.id)"
        @dragover="handleDragOver($event, doc.id)"
        @drop="handleDrop($event, doc.id)"
        @dragend="handleDragEnd"
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
        <button
          class="tab-close"
          @click.stop="handleTabClose(doc.id)"
          title="关闭"
        >
          <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
            <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.5"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- 右滚动按钮 -->
    <button
      v-show="canScrollRight"
      class="scroll-btn scroll-btn-right"
      @click="scrollRight"
    >
      <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
        <path d="M5.5 3l5 5-5 5" stroke="currentColor" stroke-width="1.5" fill="none"/>
      </svg>
    </button>
  </div>
</template>

<script setup lang="ts">
import useStore from '@/store'
const { routerNoteStore, tagview, useDocumentManagerStore } = useStore()
import { ref, computed, watch, onMounted, nextTick } from 'vue'
import FileIcon from './FileIcon.vue'

const docStore = useDocumentManagerStore;

// 响应式数据
const tabContainer = ref<HTMLElement | null>(null)
const tabRefs = ref<Map<string, HTMLElement>>(new Map())
const canScrollLeft = ref(false)
const canScrollRight = ref(false)

// 计算属性
const documentList = computed(() => docStore.documentList)
const activeDocumentId = computed(() => docStore.activeDocumentId)

// 设置 tab 引用
function setTabRef(id: string, el: any) {
  if (el) {
    tabRefs.value.set(id, el)
  } else {
    tabRefs.value.delete(id)
  }
}

// 处理标签页点击
function handleTabClick(docId: string) {
  docStore.setActiveDocument(docId)
  scrollToTab(docId)
}

// 处理标签页关闭
function handleTabClose(docId: string) {
  docStore.closeDocument(docId)
}

// 滚动到指定标签页
function scrollToTab(docId: string) {
  nextTick(() => {
    const tab = tabRefs.value.get(docId)
    const container = tabContainer.value
    if (!tab || !container) return

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
  const container = tabContainer.value
  if (!container) return

  container.scrollBy({
    left: -200,
    behavior: 'smooth'
  })
}

// 向右滚动
function scrollRight() {
  const container = tabContainer.value
  if (!container) return

  container.scrollBy({
    left: 200,
    behavior: 'smooth'
  })
}

// 处理鼠标滚轮
function handleWheel(event: WheelEvent) {
  const container = tabContainer.value
  if (!container) return

  // 将垂直滚动转换为水平滚动
  container.scrollLeft += event.deltaY

  updateScrollButtons()
}

// 更新滚动按钮状态
function updateScrollButtons() {
  const container = tabContainer.value
  if (!container) return

  const { scrollLeft, scrollWidth, clientWidth } = container

  canScrollLeft.value = scrollLeft > 0
  canScrollRight.value = scrollLeft + clientWidth < scrollWidth - 1
}

// 监听文档列表变化，自动滚动到新激活的标签页
watch(activeDocumentId, (newId) => {
  if (newId) {
    scrollToTab(newId)
  }
})

// 监听滚动事件
onMounted(() => {
  const container = tabContainer.value
  if (container) {
    container.addEventListener('scroll', updateScrollButtons, { passive: true })
    updateScrollButtons()
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

<style scoped>
.tab-bar-container {
  display: flex;
  align-items: stretch;
  height: 36px;
  background-color: #252526;
  border-bottom: 1px solid #1e1e1e;
  user-select: none;
  position: relative;
}

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
}

.scroll-btn:hover {
  background-color: #3e3e3e;
  color: #ffffff;
}

.scroll-btn:active {
  background-color: #4a4a4a;
}

/* 标签页列表容器 */
.tab-list {
  flex: 1;
  display: flex;
  align-items: stretch;
  overflow-x: auto;
  overflow-y: hidden;
  scroll-behavior: smooth;

  /* 隐藏滚动条 - 兼容多种浏览器 */
  scrollbar-width: none; /* Firefox */
  -ms-overflow-style: none; /* IE/Edge */
}

/* WebKit 浏览器隐藏滚动条 */
.tab-list::-webkit-scrollbar {
  display: none;
  width: 0;
  height: 0;
}

/* 单个标签页 */
.tab-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
  min-width: 120px;
  max-width: 200px;
  height: 100%;
  background-color: #2d2d2d;
  border-right: 1px solid #1e1e1e;
  color: #969696;
  font-size: 13px;
  cursor: pointer;
  transition: background-color 0.15s;
  white-space: nowrap;
  position: relative;
}

/* 活跃标签页样式 */
.tab-active {
  background-color: #1e1e1e;
  color: #ffffff;
  border-bottom: 2px solid #007acc;
}

.tab-active::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 2px;
  background-color: #007acc;
}

/* 非活跃标签页悬停效果 */
.tab-item:not(.tab-active):hover {
  background-color: #2a2d2e;
  color: #cccccc;
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
  font-size: 16px;
  line-height: 1;
}

/* 关闭按钮 */
.tab-close {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  border-radius: 4px;
  color: #969696;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s, background-color 0.15s;
}

.tab-item:hover .tab-close {
  opacity: 1;
}

.tab-close:hover {
  background-color: rgba(255, 255, 255, 0.1);
  color: #ffffff;
}

/* 拖拽时的占位符样式 */
.tab-drag-placeholder {
  width: 2px;
  height: 100%;
  background-color: #007acc;
  flex-shrink: 0;
}

/* 响应式调整 */
@media (max-width: 768px) {
  .tab-item {
    min-width: 80px;
    max-width: 150px;
    padding: 0 8px;
  }
}

/* 添加拖拽相关样式 */
.tab-dragging {
  opacity: 0.5;
  background-color: #3e3e3e;
}

.tab-drag-over {
  border-left: 2px solid #007acc;
}
</style>
