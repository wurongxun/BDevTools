<template>
  <div class="script-explorer">
    <div class="explorer-header">
      <el-tooltip content="新建查询" placement="top">
        <el-button :icon="Plus" size="small" circle @click="askNewFile(scriptsStore.scriptsDir)" />
      </el-tooltip>
      <el-tooltip content="新建文件夹" placement="top">
        <el-button :icon="FolderAdd" size="small" circle @click="askNewFolder(scriptsStore.scriptsDir)" />
      </el-tooltip>
      <el-tooltip content="刷新" placement="top">
        <el-button :icon="Refresh" size="small" circle @click="scriptsStore.refresh()" />
      </el-tooltip>
      <el-tooltip content="打开目录" placement="top">
        <el-button :icon="FolderOpened" size="small" circle @click="openScriptsDir" />
      </el-tooltip>
    </div>

    <!-- 按连接过滤 -->
    <el-select
      v-model="filterConfigId"
      size="small"
      clearable
      placeholder="全部连接"
      class="filter-select"
    >
      <el-option v-for="conn in connectionStore.connections" :key="conn.id" :label="conn.name" :value="conn.id" />
    </el-select>

    <div class="explorer-body" v-loading="scriptsStore.loading">
      <el-tree
        v-if="filteredTree.length > 0"
        :data="filteredTree"
        :props="{ label: 'name', children: 'children' }"
        node-key="path"
        :expand-on-click-node="false"
        @node-click="onNodeClick"
        @node-contextmenu.prevent="onNodeContextmenu"
      >
        <template #default="{ data }">
          <span class="node-row">
            <el-icon class="node-icon">
              <Folder v-if="data.isDir" />
              <Document v-else />
            </el-icon>
            <span class="node-name" :title="data.path">{{ data.name }}</span>
            <span
              v-if="!data.isDir"
              class="bind-badge"
              :class="{ unavailable: isBindingUnavailable(data.path) }"
            >{{ badgeText(data.path) }}</span>
          </span>
        </template>
      </el-tree>
      <div v-else class="empty-tip">暂无查询文件，点击 + 新建</div>
    </div>

    <!-- 右键菜单 -->
    <div
      v-if="ctxVisible"
      class="ctx-menu"
      :style="{ left: ctxPos.x + 'px', top: ctxPos.y + 'px' }"
      @click.stop
    >
      <template v-if="!ctxNode?.isDir">
        <div class="ctx-item" @click="openFile(ctxNode)">打开</div>
        <div class="ctx-item" @click="askRename(ctxNode)">重命名</div>
        <div class="ctx-item" @click="askChangeBinding(ctxNode)">修改关联连接</div>
        <div class="ctx-item" @click="revealInFileManager(ctxNode)">在文件管理器中显示</div>
        <div class="ctx-item danger" @click="askDelete(ctxNode)">删除</div>
      </template>
      <template v-else>
        <div class="ctx-item" @click="askNewFile(ctxNode.path)">新建查询</div>
        <div class="ctx-item" @click="askNewFolder(ctxNode.path)">新建文件夹</div>
        <div class="ctx-item" @click="askRename(ctxNode)">重命名</div>
        <div class="ctx-item danger" @click="askDelete(ctxNode)">删除</div>
      </template>
    </div>

    <!-- 新建查询文件 -->
    <GlobalPromptDialog
      v-if="newFileDialogVisible"
      v-model="newFileDialogVisible"
      title="新建查询文件"
      placeholder="查询文件名称"
      confirm-button-text="创建"
      :validator="validateName"
      @confirm="confirmNewFile"
    />

    <!-- 新建文件夹 -->
    <GlobalPromptDialog
      v-if="newFolderDialogVisible"
      v-model="newFolderDialogVisible"
      title="新建文件夹"
      placeholder="文件夹名称"
      confirm-button-text="创建"
      :validator="validateName"
      @confirm="confirmNewFolder"
    />

    <!-- 重命名 -->
    <GlobalPromptDialog
      v-if="renameDialogVisible"
      v-model="renameDialogVisible"
      title="重命名"
      :initial-value="renameInitial"
      :validator="validateName"
      @confirm="confirmRename"
    />

    <!-- 删除确认 -->
    <GlobalDialog v-model="deleteDialogVisible" title="确认删除" width="400px">
      <div class="delete-message">
        {{ deleteMessage }}
        <div class="delete-sub">该操作不可恢复。</div>
      </div>
      <template #footer>
        <el-button size="small" @click="deleteDialogVisible = false">取消</el-button>
        <el-button size="small" type="danger" plain @click="confirmDelete">删除</el-button>
      </template>
    </GlobalDialog>

    <!-- 修改关联连接 -->
    <GlobalDialog v-model="bindingDialogVisible" title="修改关联连接" width="420px">
      <div class="binding-form">
        <div class="binding-row">
          <span class="binding-label">连接</span>
          <el-select
            v-model="bindingConfigId"
            size="small"
            clearable
            placeholder="不关联连接"
            style="flex: 1"
            @change="onBindingConnectionChange"
          >
            <el-option v-for="conn in connectionStore.connections" :key="conn.id" :label="conn.name" :value="conn.id" />
          </el-select>
        </div>
        <div class="binding-row">
          <span class="binding-label">数据库</span>
          <el-select
            v-model="bindingDatabase"
            size="small"
            clearable
            placeholder="不指定数据库"
            style="flex: 1"
            :disabled="!bindingConfigId"
          >
            <el-option v-for="db in bindingDatabases" :key="db.name" :label="db.name" :value="db.name" />
          </el-select>
        </div>
      </div>
      <template #footer>
        <el-button size="small" @click="bindingDialogVisible = false">取消</el-button>
        <el-button size="small" type="primary" @click="confirmChangeBinding">确定</el-button>
      </template>
    </GlobalDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { Plus, FolderAdd, Refresh, FolderOpened, Folder, Document } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { tauri_request } from '@/utils/tauri_request'
import type { ResponseData } from '@/Types/entitys'
import GlobalDialog from '@/components/GlobalDialog.vue'
import GlobalPromptDialog from '@/components/GlobalPromptDialog.vue'
import { useConnectionStore, useEditorStore, useScriptsStore } from '../store'
import { dbListDatabases } from '../api/metadata'
import type { ScriptFileNode, DatabaseInfo } from '../types'

const scriptsStore = useScriptsStore()
const editorStore = useEditorStore()
const connectionStore = useConnectionStore()

// ==================== 过滤 ====================
const filterConfigId = ref('')

/** 按连接过滤后的树（文件夹保留包含匹配文件的分支） */
const filteredTree = computed<ScriptFileNode[]>(() => {
  if (!filterConfigId.value) return scriptsStore.tree
  const prune = (nodes: ScriptFileNode[]): ScriptFileNode[] =>
    nodes
      .map(n => {
        if (n.isDir) {
          const children = prune(n.children)
          return children.length > 0 ? { ...n, children } : null
        }
        return scriptsStore.getBinding(n.path)?.configId === filterConfigId.value ? n : null
      })
      .filter((n): n is ScriptFileNode => n !== null)
  return prune(scriptsStore.tree)
})

// ==================== 徽章 ====================
function badgeText(filePath: string): string {
  const binding = scriptsStore.getBinding(filePath)
  if (!binding?.configId) return ''
  const conn = connectionStore.connections.find(c => c.id === binding.configId)
  return conn ? conn.name : '连接不可用'
}

function isBindingUnavailable(filePath: string): boolean {
  const binding = scriptsStore.getBinding(filePath)
  if (!binding?.configId) return false
  return !connectionStore.connections.some(c => c.id === binding.configId)
}

// ==================== 打开文件 ====================
let lastClick = { path: '', time: 0 }

function onNodeClick(data: ScriptFileNode) {
  const now = Date.now()
  // 双击打开文件
  if (!data.isDir && lastClick.path === data.path && now - lastClick.time < 400) {
    openFile(data)
  }
  lastClick = { path: data.path, time: now }
}

async function openFile(node: ScriptFileNode | null) {
  if (!node || node.isDir) return
  try {
    const res = await tauri_request<ResponseData<string>>('read_note_file', { path: node.path })
    if (res?.code !== 200) {
      ElMessage.error(res?.msg || '读取文件失败')
      return
    }
    const binding = scriptsStore.getBinding(node.path)
    editorStore.openTab({
      title: node.name,
      type: 'sql',
      configId: binding?.configId,
      database: binding?.database,
      content: res.data || '',
      filePath: node.path,
    })
  } catch (e: any) {
    ElMessage.error('打开文件失败: ' + (e.message || e))
  }
}

async function openScriptsDir() {
  if (!scriptsStore.scriptsDir) return
  await tauri_request<ResponseData<string>>('file_manager_open_path', { path: scriptsStore.scriptsDir })
}

async function revealInFileManager(node: ScriptFileNode | null) {
  if (!node) return
  await tauri_request<ResponseData<string>>('file_manager_open_path', { path: node.path })
}

// ==================== 右键菜单 ====================
const ctxVisible = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxNode = ref<ScriptFileNode | null>(null)

function onNodeContextmenu(event: MouseEvent, data: ScriptFileNode) {
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

// ==================== 新建 ====================
const newFileDialogVisible = ref(false)
const newFolderDialogVisible = ref(false)
const newTargetDir = ref('')

function validateName(value: string): string | true {
  return value.trim() ? true : '名称不能为空'
}

function askNewFile(dirPath: string) {
  closeContextMenu()
  if (!dirPath) return
  newTargetDir.value = dirPath
  newFileDialogVisible.value = true
}

async function confirmNewFile(name: string) {
  try {
    const filePath = await scriptsStore.createFile(newTargetDir.value, name.trim())
    // 新建后立即打开
    await openFile({ name: filePath.split('/').pop() || name, path: filePath, isDir: false, children: [] })
  } catch (e: any) {
    ElMessage.error(e.message || '新建失败')
  }
}

function askNewFolder(dirPath: string) {
  closeContextMenu()
  if (!dirPath) return
  newTargetDir.value = dirPath
  newFolderDialogVisible.value = true
}

async function confirmNewFolder(name: string) {
  try {
    await scriptsStore.createFolder(newTargetDir.value, name.trim())
  } catch (e: any) {
    ElMessage.error(e.message || '新建文件夹失败')
  }
}

// ==================== 重命名 ====================
const renameDialogVisible = ref(false)
const renameInitial = ref('')
const renameTarget = ref<ScriptFileNode | null>(null)

function askRename(node: ScriptFileNode | null) {
  closeContextMenu()
  if (!node) return
  renameTarget.value = node
  renameInitial.value = node.name
  renameDialogVisible.value = true
}

async function confirmRename(newName: string) {
  const node = renameTarget.value
  if (!node) return
  try {
    if (node.isDir) {
      await scriptsStore.renameFolder(node.path, newName.trim())
    } else {
      await scriptsStore.renameFile(node.path, newName.trim())
    }
  } catch (e: any) {
    ElMessage.error(e.message || '重命名失败')
  }
}

// ==================== 删除 ====================
const deleteDialogVisible = ref(false)
const deleteMessage = ref('')
let pendingDeleteAction: (() => Promise<void>) | null = null

function askDelete(node: ScriptFileNode | null) {
  closeContextMenu()
  if (!node) return
  deleteMessage.value = node.isDir
    ? `确定删除文件夹「${node.name}」及其全部内容吗？`
    : `确定删除查询文件「${node.name}」吗？`
  pendingDeleteAction = async () => {
    if (node.isDir) await scriptsStore.deleteFolder(node.path)
    else await scriptsStore.deleteFile(node.path)
  }
  deleteDialogVisible.value = true
}

async function confirmDelete() {
  try {
    await pendingDeleteAction?.()
  } catch (e: any) {
    ElMessage.error(e.message || '删除失败')
  } finally {
    pendingDeleteAction = null
    deleteDialogVisible.value = false
  }
}

// ==================== 修改关联连接 ====================
const bindingDialogVisible = ref(false)
const bindingTarget = ref<ScriptFileNode | null>(null)
const bindingConfigId = ref('')
const bindingDatabase = ref('')
const bindingDatabases = ref<DatabaseInfo[]>([])

function askChangeBinding(node: ScriptFileNode | null) {
  closeContextMenu()
  if (!node) return
  bindingTarget.value = node
  const binding = scriptsStore.getBinding(node.path)
  bindingConfigId.value = binding?.configId || ''
  bindingDatabase.value = binding?.database || ''
  bindingDatabases.value = []
  bindingDialogVisible.value = true
  if (bindingConfigId.value) loadBindingDatabases(bindingConfigId.value)
}

async function loadBindingDatabases(configId: string) {
  try {
    if (!connectionStore.isConnected(configId)) {
      const connectRes = await connectionStore.connect(configId)
      if (connectRes.code !== 200) return
    }
    const res = await dbListDatabases(configId)
    if (res.code === 200) bindingDatabases.value = res.data
  } catch {
    // 加载失败保持空列表，用户仍可只关联连接
  }
}

async function onBindingConnectionChange(configId: string) {
  bindingDatabase.value = ''
  bindingDatabases.value = []
  if (configId) await loadBindingDatabases(configId)
}

async function confirmChangeBinding() {
  const node = bindingTarget.value
  if (!node) return
  await scriptsStore.bindFile(node.path, bindingConfigId.value || undefined, bindingDatabase.value || undefined)
  bindingDialogVisible.value = false
}

// ==================== 生命周期 ====================
onMounted(() => {
  scriptsStore.refresh()
  document.addEventListener('click', onDocumentClick)
})

onUnmounted(() => {
  document.removeEventListener('click', onDocumentClick)
})
</script>

<style scoped>
.script-explorer {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.explorer-header {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}
.explorer-header .title {
  font-size: var(--font-size-md);
  font-weight: 600;
  flex: 1;
}
.filter-select {
  padding: 6px 8px 0;
  flex-shrink: 0;
}
.explorer-body {
  flex: 1;
  overflow: auto;
  padding: 4px 0;
}
.empty-tip {
  padding: 16px;
  font-size: var(--font-size-base);
  color: var(--el-text-color-secondary);
  text-align: center;
}
.node-row {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: 1;
  overflow: hidden;
  font-size: var(--font-size-md);
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
.bind-badge {
  flex-shrink: 0;
  margin-left: auto;
  padding: 0 6px;
  font-size: var(--font-size-sm);
  line-height: 16px;
  border-radius: var(--radius-xl);
  color: var(--el-color-primary);
  background: var(--el-color-primary-light-9);
  max-width: 90px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bind-badge.unavailable {
  color: var(--el-text-color-placeholder);
  background: var(--el-fill-color);
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
  padding: 6px 16px;
  font-size: var(--font-size-md);
  cursor: pointer;
  color: var(--el-text-color-regular);
}
.ctx-item:hover {
  background: var(--el-fill-color-light);
}
.ctx-item.danger {
  color: var(--el-color-danger);
}
.delete-message {
  font-size: var(--font-size-md);
  color: var(--el-text-color-regular);
  line-height: 1.6;
}
.delete-sub {
  margin-top: 4px;
  font-size: var(--font-size-base);
  color: var(--el-text-color-secondary);
}
.binding-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.binding-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.binding-label {
  width: 48px;
  flex-shrink: 0;
  font-size: var(--font-size-md);
  color: var(--el-text-color-regular);
}
</style>
