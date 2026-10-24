<template>
  <div class="db-main-panel">
    <!-- Tab 栏（复用通用 TabBar：tab 多时横向滚动、固定最小宽度不挤压，与文件插件一致） -->
    <TabBarContainer v-if="editorStore.tabs.length > 0" ref="tabBarRef">
      <TabBarItem
        v-for="tab in editorStore.tabs"
        :key="tab.id"
        :data-tab-id="tab.id"
        :active="tab.id === editorStore.activeTabId"
        @click="editorStore.activeTabId = tab.id"
        @close="editorStore.closeTab(tab.id)"
      >
        <span class="tab-title" :title="tab.title">{{ tab.title }}</span>
        <TabCloseButton @close="editorStore.closeTab(tab.id)" />
      </TabBarItem>

      <!--
        最右侧「收纳菜单」：标签横向溢出后，滚动按钮一个个找很低效，
        这里能直接跳到任意标签 / 单独关掉它 / 一键关全部。
      -->
      <template #append>
        <TabBarOverflowMenu
          :items="tabMenuItems"
          :active-id="editorStore.activeTabId"
          button-title="显示所有标签"
          header-label="打开的标签"
          @select="selectFromMenu"
          @close="editorStore.closeTab"
          @close-all="showCloseAllConfirm = true"
        />
      </template>
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

    <!-- 关闭全部确认（Tauri WebView2 不支持原生 confirm，统一走 GlobalDialog） -->
    <GlobalDialog v-model="showCloseAllConfirm" title="关闭全部" width="400px">
      <div class="close-all-msg">
        确定要关闭所有标签页吗？未保存的 SQL 内容不会自动保存，将随之丢失。
      </div>
      <template #footer>
        <app-button size="small" @click="showCloseAllConfirm = false">取消</app-button>
        <app-button size="small" type="danger" plain @click="confirmCloseAll">全部关闭</app-button>
      </template>
    </GlobalDialog>
  </div>
</template>

<script setup lang="ts">
import { Coin } from '@element-plus/icons-vue'
import { computed, nextTick, ref } from 'vue'
import { TabBarContainer, TabBarItem, TabBarOverflowMenu, TabCloseButton } from '@/components/TabBar'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { useEditorStore } from './store'
import SqlTab from './components/SqlTab.vue'
import TableViewer from './components/TableViewer.vue'
import ViewViewer from './components/ViewViewer.vue'
import RoutineViewer from './components/RoutineViewer.vue'
import TableMetadata from './components/TableMetadata.vue'
import ObjectListViewer from './components/ObjectListViewer.vue'
import SystemInfoViewer from './components/SystemInfoViewer.vue'
import type { EditorTab } from './types'

const editorStore = useEditorStore()

// ==================== Tab 栏收纳菜单 / 关闭全部 ====================
const tabBarRef = ref<InstanceType<typeof TabBarContainer> | null>(null)
const showCloseAllConfirm = ref(false)

/** 菜单项：标题截断由菜单内的 CSS 负责，这里额外给一份全文做悬浮提示 */
const tabMenuItems = computed(() =>
  editorStore.tabs.map(t => ({
    id: t.id,
    label: t.title,
    title: t.database ? `${t.title}（${t.database}）` : t.title,
    dirty: !!t.dirty,
  }))
)

/** 从菜单跳转：该标签可能正横向溢出在可视区外，把标签本身滚回来 */
function selectFromMenu(tabId: string) {
  editorStore.activeTabId = tabId
  nextTick(() => {
    const list = tabBarRef.value?.tabListRef
    const el = list?.querySelector<HTMLElement>(`[data-tab-id="${CSS.escape(tabId)}"]`)
    el?.scrollIntoView({ inline: 'nearest', block: 'nearest' })
  })
}

function confirmCloseAll() {
  showCloseAllConfirm.value = false
  editorStore.closeAllTabs()
}


/**
 * 按 tab 类型选择内容组件
 * - sql → SqlTab
 * - table → TableViewer（数据 / 属性）
 * - view → ViewViewer（数据 / 字段 / 源码）
 * - routine → RoutineViewer（存储过程 / 函数 / 触发器 的源码与属性）
 * - metadata → TableMetadata（表结构）
 * - object-list → ObjectListViewer（用户 / 管理员 / 视图 / 索引 / 存储过程 / 触发器 / 事件）
 * - system-info → SystemInfoViewer（会话·全局 状态与变量）
 */
function tabComponentFor(tab: EditorTab): any {
  const type = tab.type
  if (type === 'sql') return SqlTab
  if (type === 'table') return TableViewer
  if (type === 'view') return ViewViewer
  if (type === 'routine') return RoutineViewer
  if (type === 'metadata') return TableMetadata
  if (type === 'object-list') return ObjectListViewer
  if (type === 'system-info') return SystemInfoViewer
  return null
}

/** 按 tab 类型透传 props */
function tabPropsFor(tab: EditorTab): Record<string, any> {
  if (tab.type === 'sql') return { tab }
  if (tab.type === 'object-list') {
    return {
      configId: tab.configId,
      database: tab.database,
      objectType: tab.objectType,
    }
  }
  if (tab.type === 'view') {
    return { configId: tab.configId, database: tab.database, name: tab.objectName }
  }
  if (tab.type === 'routine') {
    return {
      configId: tab.configId,
      database: tab.database,
      name: tab.objectName,
      kind: tab.objectType,
    }
  }
  if (tab.type === 'system-info') {
    return {
      configId: tab.configId,
      subType: tab.subType,
    }
  }
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
.close-all-msg {
  line-height: 1.6;
  @include theme.useTheme {
    font-size: theme.getVar('fontSizeSmall');
  }
}
</style>
