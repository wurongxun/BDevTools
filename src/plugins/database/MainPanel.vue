<template>
  <div class="db-main-panel">
    <!-- Tab 栏（复用通用 TabBar：tab 多时横向滚动、固定最小宽度不挤压，与文件插件一致） -->
    <TabBarContainer v-if="editorStore.tabs.length > 0">
      <TabBarItem
        v-for="tab in editorStore.tabs"
        :key="tab.id"
        :active="tab.id === editorStore.activeTabId"
        @click="editorStore.activeTabId = tab.id"
        @close="editorStore.closeTab(tab.id)"
      >
        <span class="tab-title" :title="tab.title">{{ tab.title }}</span>
        <TabCloseButton @close="editorStore.closeTab(tab.id)" />
      </TabBarItem>
    </TabBarContainer>

    <!-- 主内容区 -->
    <div class="main-content">
      <!--
        tab 内容按 tab.id 缓存实例。每个 tab 独立 KeepAlive（v-for 常驻，v-if 在 component 上）：
        切走 deactivated 卸载 DOM、切回 activated 恢复，编辑内容/查询结果/表格/筛选条件均保持；
        关闭 tab 时对应 KeepAlive 随 v-for 条目移除而卸载，内部缓存的组件实例（Monaco/结果表格）
        随之真正销毁并释放内存——避免单一 KeepAlive 缓存关闭 tab 实例直到超 max 才 LRU 淘汰
        导致的「关闭后内存不降」。
      -->
      <template v-if="editorStore.tabs.length > 0">
        <template v-for="tab in editorStore.tabs" :key="tab.id">
          <KeepAlive>
            <component
              :is="tabComponentFor(tab)"
              v-if="tab.id === editorStore.activeTabId"
              :key="tab.id"
              v-bind="tabPropsFor(tab)"
            />
          </KeepAlive>
        </template>
      </template>

      <!-- 无 Tab 时的欢迎页 -->
      <div v-else class="welcome">
        <el-empty description="数据库管理工具">
          <template #image>
            <el-icon :size="64"><Coin /></el-icon>
          </template>
          <p class="welcome-hint">从左侧选择连接，双击表查看数据，或右键新建查询</p>
        </el-empty>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Coin } from '@element-plus/icons-vue'
import { TabBarContainer, TabBarItem, TabCloseButton } from '@/components/TabBar'
import { useEditorStore } from './store'
import SqlTab from './components/SqlTab.vue'
import TableViewer from './components/TableViewer.vue'
import TableMetadata from './components/TableMetadata.vue'
import type { EditorTab } from './types'

const editorStore = useEditorStore()

/** 按 tab 类型选择内容组件（sql → SqlTab；table → TableViewer 数据/属性/预留；metadata → 详细元数据） */
function tabComponentFor(tab: EditorTab): any {
  const type = tab.type
  if (type === 'sql') return SqlTab
  if (type === 'table') return TableViewer
  if (type === 'metadata') return TableMetadata
  return null
}

/** 按 tab 类型透传 props（SqlTab 接收 tab；TableViewer/TableMetadata 接收 configId/database/table） */
function tabPropsFor(tab: EditorTab): Record<string, any> {
  if (tab.type === 'sql') return { tab }
  return {
    configId: tab.configId,
    database: tab.database,
    table: tab.table,
  }
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.db-main-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  // 主面板根背景用 appMainBgColor（与 Layout/.app-main 同源），
  // 让 side / tab / main 三段色板协调。
  @include theme.useTheme {
    background: theme.getVar('appMainBgColor');
  }
}
.tab-title {
  /* 表名/查询标题用等宽字体，避免阿里巴巴普惠体 13px 下 _ 不可见 */
  font-family: var(--font-family-mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 160px;
}
.main-content {
  flex: 1;
  overflow: hidden;
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
.welcome-hint {
  font-size: var(--font-size-lg);
  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}
</style>
