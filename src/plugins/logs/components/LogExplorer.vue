<template>
  <div class="log-explorer">
    <div class="explorer-header">
      <el-tooltip content="刷新" placement="top">
        <app-button bare :icon="Refresh" :disabled="!store.rootDir" @click="store.refresh()" />
      </el-tooltip>
      <el-tooltip content="在文件管理器中打开" placement="top">
        <app-button bare :icon="FolderOpened" :disabled="!store.rootDir" @click="store.revealRoot()" />
      </el-tooltip>
      <el-tooltip content="浏览器设置" placement="top">
        <app-button bare :icon="Setting" :class="{ active: settingsOpen }" @click="toggleSettings" />
      </el-tooltip>
    </div>

    <div class="explorer-root-path" :title="store.rootDir">
      <el-icon class="root-icon"><Folder /></el-icon>
      <span class="root-text">{{ store.rootDir || '未设置目录' }}</span>
      <span v-if="store.usingWorkspaceDir" class="root-badge">工作区</span>
    </div>

    <div class="explorer-body" v-loading="store.loading">
      <el-tree
        v-if="store.tree.length"
        :data="store.tree"
        :props="{ label: 'name', children: 'children' }"
        node-key="path"
        :expand-on-click-node="true"
        :default-expanded-keys="defaultExpanded"
        @node-click="onNodeClick"
        @node-contextmenu.prevent="onNodeContextmenu"
      >
        <template #default="{ data }">
          <span class="node-row" :class="{ 'is-active': !data.isDir && data.path === activePath }">
            <el-icon class="node-icon">
              <Folder v-if="data.isDir" />
              <Document v-else />
            </el-icon>
            <span class="node-name" :title="data.path">{{ data.name }}</span>
          </span>
        </template>
      </el-tree>
      <div v-else-if="store.errorMsg" class="empty-tip error">{{ store.errorMsg }}</div>
      <div v-else class="empty-tip">该目录下没有匹配的日志文件</div>
    </div>

    <!-- 浏览器设置（齿轮按钮展开/折叠） -->
    <el-collapse-transition>
      <div v-show="settingsOpen" class="explorer-settings">
        <div class="setting-block">
          <div class="setting-title">日志目录</div>
          <div class="setting-row">
            <span class="setting-value" :title="store.rootDir">{{ store.rootDir || '未设置' }}</span>
          </div>
          <div class="setting-actions">
            <app-button size="small" @click="store.pickDirectory()">
              <el-icon><FolderAdd /></el-icon>
              选择目录
            </app-button>
            <app-button size="small" :disabled="store.usingWorkspaceDir" @click="store.resetToWorkspace()">
              重置为工作区目录
            </app-button>
          </div>
          <div class="setting-hint" v-if="!store.usingWorkspaceDir">
            已单独设置目录，不再读取工作区设置的目录
          </div>
        </div>

        <div class="setting-block">
          <div class="setting-title">过滤文件类型</div>
          <div class="setting-row">
            <el-input
              v-model="typesText"
              size="small"
              placeholder=".log .txt .out none"
              @keyup.enter="applyTypes"
            />
            <app-button size="small" type="primary" plain @click="applyTypes">应用</app-button>
          </div>
          <div class="setting-hint">
            空格或逗号分隔；<code>none</code> 表示包含「无后缀名文件」。默认已包含常见日志类型与无后缀文件。
          </div>
        </div>
      </div>
    </el-collapse-transition>

    <!-- 右键菜单 -->
    <teleport to="body">
      <div
        v-if="ctxVisible"
        class="ctx-menu"
        :style="{ left: ctxPos.x + 'px', top: ctxPos.y + 'px' }"
        @click.stop
      >
        <div class="ctx-item" @click="openCtxNode">打开</div>
        <div class="ctx-item" @click="revealCtxNode">在文件管理器中显示</div>
      </div>
    </teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { Refresh, FolderOpened, Setting, Folder, Document, FolderAdd } from '@element-plus/icons-vue'
import { call } from '@/api/rpc'
import { useLogExplorerStore, type LogFileNode } from '../store/explorer'
import { useLogStore } from '../store/logs'

const store = useLogExplorerStore()
const logStore = useLogStore()
const settingsOpen = ref(false)
const typesText = ref(store.fileTypesText)

/** 当前激活日志源的路径（仅本地/SSH 文件有对应树节点，用于树内高亮） */
const activePath = computed(() => logStore.activeSource?.path ?? '')

/** 默认展开「顶层根 + 其下一级目录」，打开面板即见两层结构，无需逐个手点 */
const defaultExpanded = computed(() => {
  const out: string[] = []
  for (const root of store.tree) {
    out.push(root.path)
    for (const child of root.children) {
      if (child.isDir) out.push(child.path)
    }
  }
  return out
})

function toggleSettings() {
  settingsOpen.value = !settingsOpen.value
}

function applyTypes() {
  void store.setFileTypesText(typesText.value)
}

let lastClick = { path: '', time: 0 }

function onNodeClick(data: LogFileNode) {
  const now = Date.now()
  // 双击文件打开（与数据库 ScriptExplorer 一致）
  if (!data.isDir && lastClick.path === data.path && now - lastClick.time < 400) {
    store.openFile(data.path, data.name)
  }
  lastClick = { path: data.path, time: now }
}

// ==================== 右键菜单 ====================
const ctxVisible = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxNode = ref<LogFileNode | null>(null)

function onNodeContextmenu(event: MouseEvent, data: LogFileNode) {
  ctxNode.value = data
  ctxPos.value = { x: event.clientX, y: event.clientY }
  ctxVisible.value = true
}

function closeContextMenu() {
  ctxVisible.value = false
  ctxNode.value = null
}

function onDocumentClick() {
  closeContextMenu()
}

async function openCtxNode() {
  const n = ctxNode.value
  closeContextMenu()
  if (n && !n.isDir) store.openFile(n.path, n.name)
}

async function revealCtxNode() {
  const n = ctxNode.value
  closeContextMenu()
  if (!n) return
  try {
    await call('file_manager_open_path', { path: n.path })
  } catch {
    /* ignore */
  }
}

// 工作区切换时同步文件类型输入框
watch(
  () => store.fileTypesText,
  v => {
    typesText.value = v
  },
)

onMounted(() => {
  void store.init()
  document.addEventListener('click', onDocumentClick)
})

onUnmounted(() => {
  document.removeEventListener('click', onDocumentClick)
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.log-explorer {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.explorer-header {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 2px;
  padding: 4px 8px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}

/* 齿轮按钮：面板展开时高亮主色 */
.explorer-header {
  :deep(.is-bare.active) {
    color: var(--el-color-primary);
    background-color: var(--el-fill-color-light);
  }
}

.explorer-root-path {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  flex-shrink: 0;
  // 头部路径条用 sidebarBg（工具栏条），与 TabBar/LogToolbar/FilterBar 同源
  @include theme.useTheme {
    background: theme.getVar('sidebarBg');
    border-bottom: 1px solid theme.getVar('borderColor');
  }

  .root-icon {
    flex-shrink: 0;
    @include theme.useTheme {
      color: theme.getVar('textSecondary');
    }
  }
  .root-text {
    flex: 1;
    min-width: 0;
    font-size: var(--font-size-sm);
    color: var(--el-text-color-regular);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-family-mono);
  }
  .root-badge {
    flex-shrink: 0;
    font-size: var(--font-size-xs);
    padding: 0 5px;
    border-radius: var(--radius-xl);
    color: var(--el-text-color-secondary);
    background: var(--el-fill-color-light);
  }
}

.explorer-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 4px 0;
}

.empty-tip {
  padding: 12px;
  font-size: var(--font-size-base);
  color: var(--el-text-color-secondary);
  text-align: center;

  &.error {
    color: var(--stateErrorFg);
  }
}

.node-row {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: 1;
  overflow: hidden;
  font-size: var(--font-size-md);
  padding: 1px 4px;
  border-radius: var(--radius-sm);

  &.is-active {
    color: var(--el-color-primary);
    background: var(--el-color-primary-light-9);
    font-weight: 600;
  }
}

.node-icon {
  flex-shrink: 0;
  color: var(--el-text-color-secondary);
}

.node-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.explorer-settings {
  flex-shrink: 0;
  padding: 6px 8px;
  max-height: 50%;
  overflow: auto;
  // 底部设置面板用 sidebarBg（工具栏条），与顶部路径条/TabBar 同源
  @include theme.useTheme {
    background: theme.getVar('sidebarBg');
    border-top: 1px solid theme.getVar('borderColor');
  }
}

.setting-block {
  margin-bottom: 8px;

  &:last-child {
    margin-bottom: 0;
  }
}

.setting-title {
  font-size: var(--font-size-md);
  font-weight: 600;
  color: var(--el-text-color-primary);
  margin-bottom: 4px;
}

.setting-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.setting-value {
  flex: 1;
  min-width: 0;
  font-size: var(--font-size-sm);
  font-family: var(--font-family-mono);
  color: var(--el-text-color-regular);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.setting-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.setting-hint {
  margin-top: 4px;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  line-height: 1.5;

  code {
    font-family: var(--font-family-mono);
    padding: 0 3px;
    border-radius: var(--radius-xs);
    background: var(--el-fill-color);
    color: var(--el-text-color-regular);
  }
}

.ctx-menu {
  position: fixed;
  z-index: 1600;
  min-width: 150px;
  padding: 4px 0;
  background: var(--el-bg-color-overlay);
  border: 1px solid var(--el-border-color-light);
  border-radius: var(--radius-md);
  box-shadow: var(--el-box-shadow-light);
}

.ctx-item {
  padding: 5px 12px;
  font-size: var(--font-size-md);
  cursor: pointer;
  color: var(--el-text-color-regular);

  &:hover {
    background: var(--el-fill-color-light);
  }
}
</style>
