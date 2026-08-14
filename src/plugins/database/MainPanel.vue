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
          <SqlEditor
            :config-id="editorStore.activeTab.configId"
            :initial-content="editorStore.activeTab.content"
            @executed="onSqlExecuted"
          />
          <SqlResultPanel :result="sqlResult" />
        </div>

        <!-- 表数据 Tab -->
        <div v-else-if="editorStore.activeTab.type === 'table'" class="table-tab">
          <TableDataView
            v-if="editorStore.activeTab.configId && editorStore.activeTab.database && editorStore.activeTab.table"
            :config-id="editorStore.activeTab.configId"
            :database="editorStore.activeTab.database"
            :table="editorStore.activeTab.table"
          />
        </div>

        <!-- 表结构 Tab -->
        <div v-else-if="editorStore.activeTab.type === 'metadata'" class="metadata-tab">
          <TableMetadata
            v-if="editorStore.activeTab.configId && editorStore.activeTab.database && editorStore.activeTab.table"
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
          <p style="color: #909399; font-size: 14px;">从左侧选择连接，双击表查看数据，或右键新建查询</p>
        </el-empty>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { Close, Coin } from '@element-plus/icons-vue'
import { useEditorStore } from './store'
import SqlEditor from './components/SqlEditor.vue'
import SqlResultPanel from './components/SqlResultPanel.vue'
import TableDataView from './components/TableDataView.vue'
import TableMetadata from './components/TableMetadata.vue'
import type { SqlResult } from './types'

const editorStore = useEditorStore()
const sqlResult = ref<SqlResult | null>(null)

function onSqlExecuted(result: SqlResult) {
  sqlResult.value = result
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
  font-size: 13px;
  border-right: 1px solid #3c3c3c;
  white-space: nowrap;
  min-width: 80px;
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
  font-size: 12px;
  border-radius: 3px;
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
}
.sql-tab > :first-child {
  flex: 1;
  min-height: 200px;
}
.sql-tab > :last-child {
  height: 250px;
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
