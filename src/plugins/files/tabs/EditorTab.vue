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

      <!-- 收纳按钮 - 显示所有文件下拉菜单 -->
      <div class="dropdown-wrapper" ref="dropdownWrapper">
        <button
          class="dropdown-btn"
          @click="toggleDropdown"
          title="显示所有打开的文件"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M4 6h8M4 8h8M4 10h8" stroke="currentColor" stroke-width="1.5"/>
          </svg>
        </button>

        <!-- 下拉菜单 -->
        <div
          v-show="showDropdown"
          class="dropdown-menu"
          :style="dropdownStyle"
        >
          <div class="dropdown-header">
            <span>打开的文件 ({{ documentList.length }})</span>
            <button
              v-if="documentList.length > 1"
              class="close-all-btn"
              @click="closeAllTabs"
              title="关闭全部"
            >
              关闭全部
            </button>
          </div>
          <div class="dropdown-list">
            <div
              v-for="doc in documentList"
              :key="doc.id"
              class="dropdown-item"
              :class="{
                'dropdown-item-active': doc.id === activeDocumentId,
                'dropdown-item-modified': doc.isDirty
              }"
              @click="selectFromDropdown(doc.id)"
            >
              <span class="dropdown-icon">
                <FileIcon :type="doc.type" />
              </span>
              <span class="dropdown-name">{{ doc.name }}</span>
              <span v-if="doc.isDirty" class="dropdown-dirty">●</span>
              <button
                class="dropdown-close"
                @click.stop="closeFromDropdown(doc.id)"
                title="关闭"
              >
                <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
                  <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.5"/>
                </svg>
              </button>
            </div>
          </div>
        </div>
      </div>

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
      确定要关闭所有打开的文件吗？未保存的更改会先自动保存。
    </div>
    <template #footer>
      <app-button size="small" @click="showCloseAllConfirm = false">取消</app-button>
      <app-button size="small" type="danger" plain @click="confirmCloseAllTabs">全部关闭</app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { useDocumentManagerStore } from '@/store/modules/documents'
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { TabBarContainer, TabBarItem, TabCloseButton } from '@/components/TabBar'
import FileIcon from './FileIcon.vue'
import GlobalDialog from '@/components/GlobalDialog.vue'

const docStore = useDocumentManagerStore();

// 「关闭全部」确认弹窗
const showCloseAllConfirm = ref(false)

// TabBar 容器引用
const tabBarContainer = ref<InstanceType<typeof TabBarContainer> | null>(null)
const canScrollLeft = ref(false)
const canScrollRight = ref(false)
const showDropdown = ref(false)
const dropdownWrapper = ref<HTMLElement | null>(null)
const dropdownStyle = ref<any>({})

// Tooltip 相关
const tooltipVisible = ref(false)
const tooltipText = ref('')
const tooltipStyle = ref<any>({})
let tooltipTimer: number | null = null

// 计算属性
const documentList = computed(() => docStore.documentList)
const activeDocumentId = computed(() => docStore.activeDocumentPath)

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

// 处理标签页点击
function handleTabClick(docId: string) {
  docStore.setActiveDocument(docId)
  scrollToTab(docId)
}

// 处理标签页关闭
function handleTabClose(docId: string) {
  docStore.closeDocument(docId)
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

// 切换下拉菜单
function toggleDropdown() {
  showDropdown.value = !showDropdown.value
  if (showDropdown.value) {
    calculateDropdownPosition()
  }
}

// 计算下拉菜单位置
function calculateDropdownPosition() {
  nextTick(() => {
    const wrapper = dropdownWrapper.value
    if (!wrapper) return

    const rect = wrapper.getBoundingClientRect()
    const menuWidth = Math.min(350, window.innerWidth - rect.right - 20)

    dropdownStyle.value = {
      right: '0',
      top: `${rect.bottom}px`,
      maxWidth: `${menuWidth}px`
    }
  })
}

// 从下拉菜单选择文件
function selectFromDropdown(docId: string) {
  docStore.setActiveDocument(docId)
  showDropdown.value = false
  scrollToTab(docId)
}

// 从下拉菜单关闭文件
function closeFromDropdown(docId: string) {
  docStore.closeDocument(docId)
  if (documentList.value.length === 0) {
    showDropdown.value = false
  }
}

// 关闭所有标签页（先弹确认，避免误触；确认后走批量关闭，快照只同步一次）
function closeAllTabs() {
  showCloseAllConfirm.value = true
}

function confirmCloseAllTabs() {
  showCloseAllConfirm.value = false
  docStore.closeAllDocuments()
  showDropdown.value = false
}

// 点击外部关闭下拉菜单
function handleClickOutside(event: MouseEvent) {
  if (!showDropdown.value) return

  const target = event.target as HTMLElement
  if (!target.closest('.dropdown-wrapper')) {
    showDropdown.value = false
  }
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

  // 添加全局点击事件监听
  document.addEventListener('click', handleClickOutside)

  // 窗口大小改变时重新计算菜单位置
  window.addEventListener('resize', calculateDropdownPosition)
})

onUnmounted(() => {
  const container = getTabListEl()
  if (container) {
    container.removeEventListener('scroll', updateScrollButtons)
  }

  document.removeEventListener('click', handleClickOutside)
  window.removeEventListener('resize', calculateDropdownPosition)

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

/* 下拉菜单包装器 */
.dropdown-wrapper {
  position: relative;
  flex-shrink: 0;
}

/* 下拉按钮 */
.dropdown-btn {
  width: 32px;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #2d2d2d;
  border: none;
  border-left: 1px solid #1e1e1e;
  color: #cccccc;
  cursor: pointer;
  transition: background-color 0.2s;

  @include theme.useTheme {
     background-color: color.adjust(theme.getVar('bgColor'), $lightness:5%);
    border-left-color: theme.getVar('borderColor');
    color: theme.getVar('textColor');
  }
}

.dropdown-btn:hover {
  background-color: #3e3e3e;
  color: #ffffff;

  @include theme.useTheme {
     background-color: color.adjust(theme.getVar('bgColor'), $lightness:10%);
    color: theme.getVar('textColor');
  }
}

/* 下拉菜单 */
.dropdown-menu {
  position: fixed;
  background-color: #252526;
  border: 1px solid #3e3e3e;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  z-index: 1000;
  max-height: calc(100vh - 100px);
  display: flex;
  flex-direction: column;
  animation: dropdownFadeIn 0.15s ease-out;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-color: theme.getVar('borderColor');
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  }
}

@keyframes dropdownFadeIn {
  from {
    opacity: 0;
    transform: translateY(-8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 下拉菜单头部 */
.dropdown-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background-color: #2d2d2d;
  border-bottom: 1px solid #1e1e1e;
  font-size: theme.getVar('fontSizeSmall');
  color: #cccccc;
  font-weight: 500;

  @include theme.useTheme {
     background-color: color.adjust(theme.getVar('bgColor'), $lightness:5%);
    border-bottom-color: theme.getVar('borderColor');
    color: theme.getVar('textColor');
  }
}

/* 关闭全部按钮 */
.close-all-btn {
  padding: 2px 8px;
  background: transparent;
  border: 1px solid #3e3e3e;
  border-radius: var(--radius-sm);
  color: #cccccc;
  font-size: theme.getVar('fontSizeSm');
  cursor: pointer;
  transition: all 0.15s;

  @include theme.useTheme {
    border-color: theme.getVar('borderColor');
    color: theme.getVar('textColor');
  }
}

.close-all-btn:hover {
  background-color: #c42b1c;
  border-color: #c42b1c;
  color: #ffffff;
}

/* 下拉列表 */
.dropdown-list {
  overflow-y: auto;
  overflow-x: hidden;
  max-height: 400px;
}

/* 下拉项 */
.dropdown-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  transition: background-color 0.15s;
  border-bottom: 1px solid #1e1e1e;

  @include theme.useTheme {
    border-bottom-color: theme.getVar('borderColor');
  }
}

.dropdown-item:last-child {
  border-bottom: none;
}

.dropdown-item:hover {
  background-color: #2a2d2e;

  @include theme.useTheme {
     background-color: color.adjust(theme.getVar('bgColor'), $lightness:8%);
  }
}

// 激活项高亮统一走主题令牌 hoverColor（与菜单 hover、活动栏一致），
// 不再硬编码 VS Code 蓝 #094771/#0e5f94（审查报告 #22：亮色主题下同样生效、与浅色体系冲突）
.dropdown-item-active {
  @include theme.useTheme {
    background-color: theme.getVar('hoverColor');
    color: theme.getVar('textColor');
  }
}

.dropdown-item-active:hover {
  @include theme.useTheme {
    background-color: theme.getVar('activityBarHover');
    color: theme.getVar('textColor');
  }
}

/* 下拉项图标 */
.dropdown-icon {
  flex-shrink: 0;
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 下拉项名称 */
.dropdown-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: theme.getVar('fontSize');
}

/* 下拉项修改指示器 */
.dropdown-dirty {
  flex-shrink: 0;
  color: #e2b714;
  font-size: theme.getVar('fontSize');
}

/* 下拉项关闭按钮：命中区 24×24（WCAG 2.5.8），始终可见以保证键盘/触屏可发现（审查报告 #26） */
.dropdown-close {
  flex-shrink: 0;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background-color 0.15s;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.dropdown-item:hover .dropdown-close {
  opacity: 1;
}

.dropdown-close:hover {
  background-color: rgba(255, 255, 255, 0.1);
  color: #ffffff;

  @include theme.useTheme {
    background-color: rgba(255, 255, 255, 0.15);
    color: theme.getVar('textColor');
  }
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
