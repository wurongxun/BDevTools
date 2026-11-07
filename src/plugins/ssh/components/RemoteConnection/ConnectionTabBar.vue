<template>
  <TabBarContainer>
    <TabBarItem
      v-for="tab in tabs"
      :key="tab.id"
      :active="tab.id === activeTabId"
      :draggable="true"
      :dragging="tab.id === dragId"
      :drag-over="tab.id === dragOverId"
      @click="handleTabClick(tab.id)"
      @close="handleTabClose(tab.id)"
      @dragstart="onDragStart(tab.id, $event)"
      @dragover.prevent="dragOverId = tab.id"
      @drop="onDrop(tab.id, $event)"
      @dragend="onDragEnd"
      @contextmenu.prevent="onTabContextMenu(tab.id, $event)"
    >
      <!-- 连接状态指示点 -->
      <span class="conn-status-dot" :class="`status-${tab.status}`" />

      <!-- Tab 图标 -->
      <span class="conn-tab-icon">
        <svg v-if="tab.type === 'terminal'" width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
          <path d="M2 3l5 5-5 5" stroke="currentColor" stroke-width="1.5" fill="none"/>
          <line x1="8" y1="13" x2="14" y2="13" stroke="currentColor" stroke-width="1.5"/>
        </svg>
        <svg v-else-if="tab.type === 'tunnel'" width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.2">
          <path d="M2 8h12M2 8l3-3M2 8l3 3M14 8l-3-3M14 8l-3 3" />
        </svg>
        <svg v-else width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
          <path d="M2 3h12v10H2V3z" fill="none" stroke="currentColor" stroke-width="1"/>
          <line x1="2" y1="6" x2="14" y2="6" stroke="currentColor" stroke-width="1"/>
          <circle cx="4" cy="4.5" r="0.5" fill="currentColor"/>
          <circle cx="6" cy="4.5" r="0.5" fill="currentColor"/>
        </svg>
      </span>

      <span class="conn-tab-label">{{ tab.title }}</span>

      <TabCloseButton v-if="tab.closable" @close="handleTabClose(tab.id)" title="关闭连接" />
    </TabBarItem>

    <template #append>
      <div class="conn-tab-actions">
        <button
          class="tab-action-btn"
          :class="{ 'is-on': connectionStore.splitEnabled }"
          :title="connectionStore.splitEnabled ? '关闭分屏' : '开启分屏（左右双窗格）'"
          aria-label="切换分屏"
          @click="connectionStore.toggleSplit()"
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.2">
            <rect x="1.5" y="2.5" width="13" height="11" rx="1" />
            <line x1="8" y1="2.5" x2="8" y2="13.5" />
          </svg>
        </button>
        <button
          v-if="connectionStore.splitEnabled"
          class="tab-action-btn"
          :title="connectionStore.splitOrientation === 'vertical' ? '切换为上下分屏' : '切换为左右分屏'"
          aria-label="切换分屏方向"
          @click="toggleOrientation"
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.2">
            <rect x="1.5" y="2.5" width="13" height="11" rx="1" />
            <line x1="1.5" y1="8" x2="14.5" y2="8" />
          </svg>
        </button>
        <button
          class="tab-action-btn"
          :class="{ 'is-on': connectionStore.composeVisible }"
          title="撰写栏（可广播到所有终端）"
          aria-label="切换撰写栏"
          @click="connectionStore.composeVisible = !connectionStore.composeVisible"
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.2">
            <path d="M2 3h12M2 7h12M2 11h8" />
          </svg>
        </button>
      </div>
    </template>
  </TabBarContainer>

  <!-- tab 右键菜单 -->
  <teleport to="body">
    <div v-if="ctxVisible" class="conn-tab-ctx" :style="{ left: ctxX + 'px', top: ctxY + 'px' }" @click.stop>
      <div class="ctx-item" @click="duplicateCtxTab">新建同类型标签</div>
      <div class="ctx-item" @click="splitCtxTab">在副窗格打开</div>
      <div class="ctx-sep" />
      <div class="ctx-item" @click="closeCtxTab">关闭标签</div>
    </div>
    <div v-if="ctxVisible" class="ctx-overlay" @click="closeCtx" />
  </teleport>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { TabBarContainer, TabBarItem, TabCloseButton } from '@/components/TabBar'
import { useConnectionStore } from '../../store'

const connectionStore = useConnectionStore()

const tabs = computed(() => connectionStore.tabs)
const activeTabId = computed(() => connectionStore.activeTabId)

const dragId = ref('')
const dragOverId = ref('')

function onDragStart(id: string, e: DragEvent) {
  dragId.value = id
  e.dataTransfer?.setData('text/plain', id)
  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
}

function onDrop(targetId: string, e: DragEvent) {
  e.preventDefault()
  if (dragId.value && dragId.value !== targetId) {
    connectionStore.moveTab(dragId.value, targetId)
  }
  dragId.value = ''
  dragOverId.value = ''
}

function onDragEnd() {
  dragId.value = ''
  dragOverId.value = ''
}

function handleTabClick(tabId: string) {
  connectionStore.switchTab(tabId)
}

function toggleOrientation() {
  const next = connectionStore.splitOrientation === 'vertical' ? 'horizontal' : 'vertical'
  connectionStore.setSplit(true, next)
}

function handleTabClose(tabId: string) {
  connectionStore.closeTab(tabId)
}

// ==================== 右键菜单 ====================
const ctxVisible = ref(false)
const ctxX = ref(0)
const ctxY = ref(0)
const ctxId = ref('')

function onTabContextMenu(id: string, e: MouseEvent) {
  ctxId.value = id
  ctxX.value = e.clientX
  ctxY.value = e.clientY
  ctxVisible.value = true
}

function closeCtx() {
  ctxVisible.value = false
}

function duplicateCtxTab() {
  connectionStore.duplicateTab(ctxId.value)
  closeCtx()
}

function splitCtxTab() {
  const id = ctxId.value
  if (!connectionStore.splitEnabled) connectionStore.setSplit(true)
  connectionStore.switchTab(id, 'secondary')
  closeCtx()
}

function closeCtxTab() {
  connectionStore.closeTab(ctxId.value)
  closeCtx()
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.conn-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  &.status-disconnected { background-color: #888; }
  &.status-connecting { background-color: #e2b714; animation: pulse 1s infinite; }
  &.status-connected { background-color: #4ec9b0; }
  &.status-error { background-color: #f44747; }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.conn-tab-icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  color: inherit;
}

.conn-tab-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
}

.conn-tab-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 0 6px;
  flex-shrink: 0;
}

.tab-action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 20px;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  background: transparent;
  cursor: pointer;
  @include theme.useTheme { color: theme.getVar('textSecondary'); }
  &:hover { @include theme.useTheme { background: theme.getVar('activityBarHover'); color: theme.getVar('textColor'); } }
  &.is-on { @include theme.useTheme { color: var(--el-color-primary); } }
}
</style>

<style lang="scss">
@use '@/styles/theme.scss' as theme;

// Teleport 节点不受 scoped 影响
.conn-tab-ctx {
  position: fixed;
  z-index: 3002;
  min-width: 170px;
  padding: 4px;
  border-radius: var(--radius-md);
  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderStrong');
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.18);
    color: theme.getVar('textColor');
  }
}

.ctx-overlay {
  position: fixed;
  inset: 0;
  z-index: 3001;
}

.conn-tab-ctx .ctx-item {
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: var(--font-size-base);
  &:hover { @include theme.useTheme { background-color: theme.getVar('activityBarHover'); } }
}

.conn-tab-ctx .ctx-sep {
  height: 1px;
  margin: 4px 0;
  @include theme.useTheme { background-color: theme.getVar('borderColor'); }
}
</style>
