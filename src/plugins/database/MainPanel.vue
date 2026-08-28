<template>
  <div class="db-main-panel">
    <!-- Tab 栏 -->
    <div class="tab-bar" v-if="editorStore.tabs.length > 0">
      <div
        v-for="tab in editorStore.tabs"
        :key="tab.id"
        class="tab-item"
        :class="{ active: tab.id === editorStore.activeTabId }"
        @click="editorStore.activeTabId = tab.id"
      >
        <span class="tab-title">{{ tab.title }}</span>
        <el-icon class="tab-close" @click.stop="editorStore.closeTab(tab.id)"><Close /></el-icon>
      </div>
    </div>

    <!-- 主内容区 -->
    <div class="main-content">
      <template v-if="editorStore.activeTab">
        <!-- SQL 编辑器 Tab -->
        <div v-if="editorStore.activeTab.type === 'sql'" class="sql-tab">
          <div class="sql-editor-wrap">
            <SqlEditor
              :config-id="editorStore.activeTab.configId"
              :database="editorStore.activeTab.database"
              :initial-content="editorStore.activeTab.content"
              :saved-file-path="editorStore.activeTab.filePath"
              @executed="onSqlExecuted"
              @saved="onSqlSaved"
            />
          </div>
          <div
            class="resize-handle"
            :class="{ dragging: isDragging }"
            @mousedown.prevent="onResizeStart"
          ></div>
          <div ref="resultPanelRef" class="sql-result-wrap" :style="{ height: resultHeight + 'px', transition: isDragging ? 'none' : '' }">
            <SqlResultPanel :result="editorStore.activeTabResult" />
          </div>
        </div>

        <!-- 表数据 Tab -->
        <div v-else-if="editorStore.activeTab.type === 'table'" class="table-tab">
          <TableDataView
            v-if="editorStore.activeTab.configId && editorStore.activeTab.database && editorStore.activeTab.table"
            :key="editorStore.activeTab.id"
            :config-id="editorStore.activeTab.configId"
            :database="editorStore.activeTab.database"
            :table="editorStore.activeTab.table"
          />
        </div>

        <!-- 表结构 Tab -->
        <div v-else-if="editorStore.activeTab.type === 'metadata'" class="metadata-tab">
          <TableMetadata
            v-if="editorStore.activeTab.configId && editorStore.activeTab.database && editorStore.activeTab.table"
            :key="editorStore.activeTab.id"
            :config-id="editorStore.activeTab.configId"
            :database="editorStore.activeTab.database"
            :table="editorStore.activeTab.table"
          />
        </div>
      </template>

      <!-- 无 Tab 时的欢迎页 -->
      <div v-else class="welcome">
        <el-empty description="数据库管理工具">
          <template #image>
            <el-icon :size="64" color="#409eff"><Coin /></el-icon>
          </template>
          <p style="color: #909399; font-size: var(--font-size-lg);">从左侧选择连接，双击表查看数据，或右键新建查询</p>
        </el-empty>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { Close, Coin } from '@element-plus/icons-vue'
import { useEditorStore, useScriptsStore } from './store'
import SqlEditor from './components/SqlEditor.vue'
import SqlResultPanel from './components/SqlResultPanel.vue'
import TableDataView from './components/TableDataView.vue'
import TableMetadata from './components/TableMetadata.vue'

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

function onSqlExecuted(result: any) {
  if (editorStore.activeTabId) {
    editorStore.setTabResult(editorStore.activeTabId, result)
  }
}

async function onSqlSaved(filePath: string, configId: string, database: string) {
  if (editorStore.activeTabId) {
    editorStore.setTabFilePath(editorStore.activeTabId, filePath)
    // 保存时自动落绑定：该文件默认对应的连接与数据库
    await scriptsStore.bindFile(filePath, configId, database)
    await scriptsStore.refresh()
  }
}
</script>

<style scoped>
.db-main-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--el-bg-color);
}
.tab-bar {
  display: flex;
  background: #252526;
  border-bottom: 1px solid #3c3c3c;
  overflow-x: auto;
  min-height: 35px;
}
.tab-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
  cursor: pointer;
  color: #969696;
  font-size: var(--font-size-md);
  border-right: 1px solid #3c3c3c;
  white-space: nowrap;
  min-width: 80px;
}
.tab-title {
  /* 表名/查询标题用等宽字体，避免阿里巴巴普惠体 13px 下 _ 不可见 */
  font-family: var(--font-family-mono);
}
.tab-item:hover {
  color: #fff;
  background: #2d2d2d;
}
.tab-item.active {
  color: #fff;
  background: #1e1e1e;
  border-bottom: 2px solid #007acc;
}
.tab-close {
  font-size: var(--font-size-base);
  border-radius: var(--radius-sm);
  padding: 2px;
}
.tab-close:hover {
  background: #505050;
}
.main-content {
  flex: 1;
  overflow: hidden;
}
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
  height: 4px;
  background: #3c3c3c;
  cursor: ns-resize;
  flex-shrink: 0;
  position: relative;
}
.resize-handle:hover,
.resize-handle.dragging {
  background: #007acc;
}
.sql-result-wrap {
  flex-shrink: 0;
  overflow: hidden;
  border-top: 1px solid #3c3c3c;
}
.table-tab, .metadata-tab {
  height: 100%;
  overflow: auto;
}
.welcome {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
