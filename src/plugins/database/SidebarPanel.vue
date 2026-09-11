<template>
  <div ref="rootRef" class="db-sidebar">
    <!-- 数据库连接区 -->
    <div ref="connRef" class="connections-section" :style="connStyle">
      <div class="panel-header">
        <span class="panel-title">数据库连接</span>
        <div class="header-actions">
          <app-button
            bare
            :icon="dbSidebar.scriptsExpanded ? CaretTop : CaretBottom"
            :class="{ active: dbSidebar.scriptsExpanded }"
            :title="dbSidebar.scriptsExpanded ? '收起查询脚本' : '展开查询脚本'"
            @click="dbSidebar.toggleScripts()"
          />
          <app-button bare :icon="Plus" title="新建连接" @click="showConnectionDialog = true" />
          <app-button bare :icon="Refresh" title="刷新" @click="refreshConnections" />
        </div>
      </div>
      <DatabaseTree ref="treeRef" />
    </div>

    <!-- 上下分区拖拽分隔线（仅脚本区展开时显示；粗细走 --resize-handle-h-line-height） -->
    <div
      v-show="dbSidebar.scriptsExpanded"
      ref="splitterRef"
      class="resize-handle-line"
      :class="{ 'is-dragging': splitterDragging }"
      role="separator"
      aria-orientation="horizontal"
      title="拖拽调整上下占比"
      @mousedown.prevent="onSplitterDown"
    ></div>

    <!-- 查询文件管理器（含底部「插件设置」折叠区，由 ScriptExplorer 内部管理） -->
    <div
      v-show="dbSidebar.scriptsExpanded"
      ref="scriptsRef"
      class="scripts-section"
      :style="{ minHeight: MIN_SCRIPT_PX + 'px' }"
    >
      <ScriptExplorer />
    </div>

    <ConnectionDialog
      v-model="showConnectionDialog"
      :edit-config="editingConfig"
      @submit="handleConnectionSubmit"
    />
  </div>
</template>
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { Plus, Refresh, CaretTop, CaretBottom } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { useConnectionStore, useDbSidebarStore } from './store'
import DatabaseTree from './components/DatabaseTree.vue'
import ConnectionDialog from './components/ConnectionDialog.vue'
import ScriptExplorer from './components/ScriptExplorer.vue'
import type { DbConnectionConfig } from './types'

const connectionStore = useConnectionStore()
const dbSidebar = useDbSidebarStore()
const showConnectionDialog = ref(false)
const editingConfig = ref<DbConnectionConfig | null>(null)
const treeRef = ref<InstanceType<typeof DatabaseTree>>()

onMounted(() => {
  connectionStore.loadConnections()
})

function refreshConnections() {
  connectionStore.loadConnections()
  treeRef.value?.refreshTree()
}

// ==================== 上下分区拖拽 ====================
// 连接区 flex-grow 由拖拽动态更新（脚本区恒为 1），默认 1.2 ≈ 原 flex 比例
const MIN_CONN_PX = 80 // 连接区最小可视高度（容纳 header + 若干节点）
const MIN_SCRIPT_PX = 96 // 脚本区最小可视高度
const rootRef = ref<HTMLElement>()
const connRef = ref<HTMLElement>()
const splitterRef = ref<HTMLElement>()
const scriptsRef = ref<HTMLElement>()
const connGrow = ref(1.2)
const splitterDragging = ref(false)
/** 连接区 flex-grow 以字符串输出，避免 Vue 对数字属性误加 px */
const connStyle = computed(() => ({ flexGrow: String(connGrow.value) }))

let dragState: { startY: number; connPx: number; dividerPx: number; totalPx: number } | null = null

function clamp(v: number, min: number, max: number): number {
  return Math.min(Math.max(v, min), max)
}

function onSplitterDown(e: MouseEvent) {
  const rootEl = rootRef.value
  const connEl = connRef.value
  const splitterEl = splitterRef.value
  if (!rootEl || !connEl || !splitterEl) return
  splitterDragging.value = true
  dragState = {
    startY: e.clientY,
    connPx: connEl.getBoundingClientRect().height,
    dividerPx: splitterEl.getBoundingClientRect().height,
    totalPx: rootEl.getBoundingClientRect().height,
  }
  document.body.style.cursor = 'ns-resize'
  document.body.style.userSelect = 'none'
  window.addEventListener('mousemove', onSplitterMove)
  window.addEventListener('mouseup', onSplitterUp)
}

function onSplitterMove(e: MouseEvent) {
  const d = dragState
  if (!d) return
  const maxConnPx = d.totalPx - d.dividerPx - MIN_SCRIPT_PX
  const connPx = clamp(d.connPx + (e.clientY - d.startY), MIN_CONN_PX, Math.max(MIN_CONN_PX, maxConnPx))
  const scriptsPx = d.totalPx - d.dividerPx - connPx
  if (scriptsPx > 0) connGrow.value = connPx / scriptsPx
}

function onSplitterUp() {
  splitterDragging.value = false
  dragState = null
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  window.removeEventListener('mousemove', onSplitterMove)
  window.removeEventListener('mouseup', onSplitterUp)
}

onUnmounted(() => {
  // 组件卸载时若仍在拖拽，清理全局监听与 body 样式
  if (dragState) {
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    window.removeEventListener('mousemove', onSplitterMove)
    window.removeEventListener('mouseup', onSplitterUp)
    dragState = null
  }
})

async function handleConnectionSubmit(config: DbConnectionConfig) {
  try {
    if (config.id) {
      const res = await connectionStore.updateConnection(config)
      if (res.code === 200) {
        ElMessage.success('连接配置已更新')
      } else {
        ElMessage.error(res.msg)
      }
    } else {
      const res = await connectionStore.createConnection(config)
      if (res.code === 200) {
        ElMessage.success('连接配置已创建')
      } else {
        ElMessage.error(res.msg)
      }
    }
    showConnectionDialog.value = false
    treeRef.value?.refreshTree()
  } catch (e: any) {
    ElMessage.error('操作失败: ' + (e.message || e))
  }
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;
.db-sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  // 侧边栏根背景用 sidebarBg（与 Layout/.panel-area 同源），保持 side 色板协调
  @include theme.useTheme {
    background: theme.getVar('sidebarBg');
  }
}
.connections-section {
  flex: 1 1 0%; // flex-grow 由拖拽状态 connGrow 内联驱动，默认 1.2
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.connections-section .database-tree {
  flex: 1;
  min-height: 0;
  height: auto;
}
// ===== 上下分区拖拽分隔线 =====
.resize-handle-line {
  flex-shrink: 0;
  width: 100%;
  height: var(--resize-handle-h-line-height);
  cursor: ns-resize;
  background: transparent;
  position: relative;
  user-select: none;
}
// 扩大命中热区（透明、不占布局）
.resize-handle-line::before {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  top: calc(-1 * var(--resize-handle-hit-area));
  bottom: calc(-1 * var(--resize-handle-hit-area));
}
// 视觉线：绝对定位居中，常态 1px，悬停/拖拽中增粗到 active 高度
.resize-handle-line::after {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  top: 50%;
  height: var(--resize-handle-h-line-height);
  transform: translateY(-50%);
  background: var(--resize-handle-color);
  transition: height 0.15s ease, background-color 0.15s ease;
}
.resize-handle-line:hover::after,
.resize-handle-line.is-dragging::after {
  height: var(--resize-handle-h-line-height-active);
  background: var(--resize-handle-color-active);
}
.scripts-section {
  flex: 1 1 0%; // min-height 由内联 MIN_SCRIPT_PX 约束
  min-height: 0;
  overflow: hidden;
}


// ===== 头部 =====
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 12px;
  height: 35px;
  flex-shrink: 0;
}
.header-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}
.panel-title {
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 1px;
  @include theme.useTheme {
    color: theme.getVar('textColor');
    font-size: theme.getVar('fontSizeSmall');
  }
}

.header-actions {
  :deep(.is-bare.active) {
    color: var(--el-color-primary);
    background-color: var(--el-fill-color-light);
  }
}
</style>
