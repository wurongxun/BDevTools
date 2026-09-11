<template>
  <div class="sql-tab">
    <div class="sql-editor-wrap">
      <SqlEditor
        :config-id="tab.configId"
        :database="tab.database"
        :initial-content="tab.content"
        :saved-file-path="tab.filePath"
        @executed="onSqlExecuted"
        @saved="onSqlSaved"
      />
    </div>
    <div
      class="resize-handle"
      :class="{ dragging: isDragging }"
      @mousedown.prevent="onResizeStart"
    ></div>
    <div
      ref="resultPanelRef"
      class="sql-result-wrap"
      :style="{ height: resultHeight + 'px', transition: isDragging ? 'none' : '' }"
    >
      <SqlResultPanel :tab-id="tab.id" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useEditorStore, useScriptsStore } from '../store'
import SqlEditor from './SqlEditor.vue'
import SqlResultPanel from './SqlResultPanel.vue'
import type { EditorTab } from '../types'

/**
 * SQL 编辑器 Tab 内容
 * 由 MainPanel 的 KeepAlive 按 tab.id 缓存实例：切走/切回时
 * 编辑内容、查询结果、结果面板高度均保持，无需重新执行。
 */
const props = defineProps<{
  tab: EditorTab
}>()

const editorStore = useEditorStore()
const scriptsStore = useScriptsStore()

// ==================== 垂直 resize（编辑器 / 结果面板） ====================
const RESULT_HEIGHT_DEFAULT = 250
const RESULT_HEIGHT_MIN = 100

const resultPanelRef = ref<HTMLElement>()
const resultHeight = ref(RESULT_HEIGHT_DEFAULT)
const isDragging = ref(false)

function onResizeStart(e: MouseEvent) {
  const sqlTab = resultPanelRef.value?.parentElement
  if (!sqlTab) return
  const tabRect = sqlTab.getBoundingClientRect()
  const startY = e.clientY
  const startH = resultHeight.value
  isDragging.value = true
  document.body.style.cursor = 'ns-resize'
  document.body.style.userSelect = 'none'

  const resultEl = resultPanelRef.value!

  const onMouseMove = (moveEvent: MouseEvent) => {
    const newH = Math.max(RESULT_HEIGHT_MIN, Math.min(tabRect.height - 150, startH - (moveEvent.clientY - startY)))
    resultEl.style.height = newH + 'px'
  }
  const onMouseUp = () => {
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    // 读取 DOM 最终值，交给 Vue 接管
    resultHeight.value = parseInt(resultEl.style.height) || RESULT_HEIGHT_DEFAULT
    isDragging.value = false
  }
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}

function onSqlExecuted(entry: { sql: string; result: any }) {
  if (props.tab.id && entry) {
    editorStore.appendTabResult(props.tab.id, { sql: entry.sql, result: entry.result })
  }
}

async function onSqlSaved(filePath: string, configId: string, database: string) {
  if (props.tab.id) {
    editorStore.setTabFilePath(props.tab.id, filePath)
    // 保存时自动落绑定：该文件默认对应的连接与数据库
    await scriptsStore.bindFile(filePath, configId, database)
    await scriptsStore.refresh()
  }
}
</script>

<style scoped>
.sql-tab {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.sql-editor-wrap {
  flex: 1;
  min-height: 150px;
  overflow: hidden;
}
.resize-handle {
  height: var(--resize-handle-h-line-height);
  background: transparent;
  cursor: ns-resize;
  flex-shrink: 0;
  position: relative;
}
/* 扩大命中热区（透明、不占布局）：1px 的线难以精确点中，向上下各扩 hit-area */
.resize-handle::before {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  top: calc(-1 * var(--resize-handle-hit-area));
  bottom: calc(-1 * var(--resize-handle-hit-area));
}
/* 视觉线：绝对定位居中，常态 1px，悬停/拖拽中增粗到 active 高度 */
.resize-handle::after {
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
.resize-handle:hover::after,
.resize-handle.dragging::after {
  height: var(--resize-handle-h-line-height-active);
  background: var(--resize-handle-color-active);
}
.sql-result-wrap {
  flex-shrink: 0;
  overflow: hidden;
  border-top: 1px solid var(--el-border-color);
}
</style>
