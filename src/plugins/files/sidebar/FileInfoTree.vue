<template>
  <div class="file-tree-container">
    <!-- 搜索过滤 -->
    <div class="tree-search" v-if="showSearch">
      <el-input
        v-model="searchQuery"
        size="small"
        placeholder="过滤文件名..."
        clearable
        prefix-icon="Search"
        @input="handleSearch"
      />
    </div>

    <el-tree
      v-show="filteredTreeData.length > 0"
      ref="treeRef"
      :data="filteredTreeData"
      node-key="id"
      :props="defaultProps"
      :expand-on-click-node="false"
      :allow-drop="handleAllowDrop"
      @node-drop="handleNodeDrop"
      draggable
      highlight-current
      @node-contextmenu="handleNodeContextMenu"
      @node-click="handleNodeClick"
    >
      <template #default="{ node, data }">
        <div
          class="custom-tree-node"
          @mouseenter="onNodeMouseEnter(data)"
          @mouseleave="onNodeMouseLeave"
          :class="{ 'node-is-favorite': isFavoriteNode(data) }"
        >
          <span class="node-label">
            <el-icon class="node-icon">
              <component :is="getNodeIcon(data)" />
            </el-icon>
            <span class="node-text" :title="node.label">{{ node.label }}</span>
          </span>

          <span v-show="hoveredNodeId === data.id" class="node-actions">
            <template v-if="data.isFolder">
              <el-icon class="action-icon" @click.stop="handleAddFile(data)" title="新建文件">
                <DocumentAdd />
              </el-icon>
              <el-icon class="action-icon" @click.stop="handleAddFolder(data)" title="新建文件夹">
                <FolderAdd />
              </el-icon>
            </template>
          </span>
        </div>
      </template>
    </el-tree>

    <!-- 空状态 -->
    <div v-show="filteredTreeData.length === 0" class="empty-tree-actions">
      <div class="empty-hint" v-if="searchQuery">无匹配文件</div>
      <template v-else>
        <el-card class="button-card" @click="handleAddRootFile">
          <el-icon class="svg-icon"><DocumentAdd /></el-icon>
        </el-card>
        <el-card class="button-card" @click="handleAddRootFolder">
          <el-icon class="svg-icon"><FolderAdd /></el-icon>
        </el-card>
      </template>
    </div>

    <!-- 重命名对话框 -->
    <el-dialog v-model="renameDialogVisible" title="重命名" width="400px" :modal="false" append-to-body>
      <el-input v-model="renameInput" ref="renameInputRef" @keyup.enter="confirmRename" />
      <template #footer>
        <el-button @click="renameDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="confirmRename">确定</el-button>
      </template>
    </el-dialog>

    <!-- 创建文件/文件夹对话框 -->
    <el-dialog v-model="createDialogVisible" :title="createDialogTitle" width="400px" :modal="false" append-to-body>
      <el-input v-model="createInput" ref="createInputRef" @keyup.enter="confirmCreate" placeholder="请输入名称" />
      <template #footer>
        <el-button @click="createDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="confirmCreate">确定</el-button>
      </template>
    </el-dialog>

    <!-- 右键菜单 -->
    <teleport to="body">
      <div
        v-if="contextMenuVisible"
        class="tree-context-menu"
        :style="{ left: menuX + 'px', top: menuY + 'px' }"
        @click.stop
      >
        <template v-if="contextNode?.isFolder">
          <div class="ctx-item" @click="showCreateDialog('file')">
            <el-icon><DocumentAdd /></el-icon> 新建文件
          </div>
          <div class="ctx-item" @click="showCreateDialog('folder')">
            <el-icon><FolderAdd /></el-icon> 新建文件夹
          </div>
          <div class="ctx-divider" />
          <div class="ctx-item" @click="handleCopyPath">
            <el-icon><DocumentCopy /></el-icon> 复制路径
          </div>
          <div class="ctx-item" @click="handleOpenInExplorer">
            <el-icon><FolderOpened /></el-icon> 在文件管理器中显示
          </div>
          <div class="ctx-divider" />
          <div class="ctx-item" @click="showRenameDialog">
            <el-icon><Edit /></el-icon> 重命名
          </div>
          <div class="ctx-item ctx-danger" @click="handleDelete">
            <el-icon><Delete /></el-icon> 删除
          </div>
        </template>
        <template v-else>
          <div class="ctx-item" @click="handleCopyPath">
            <el-icon><DocumentCopy /></el-icon> 复制路径
          </div>
          <div class="ctx-item" @click="handleToggleFavorite">
            <el-icon><Star /></el-icon> {{ isFavoriteNode(contextNode!) ? '取消收藏' : '添加收藏' }}
          </div>
          <div class="ctx-divider" />
          <div class="ctx-item" @click="handleOpenInExplorer">
            <el-icon><FolderOpened /></el-icon> 在文件管理器中显示
          </div>
          <div class="ctx-divider" />
          <div class="ctx-item" @click="showRenameDialog">
            <el-icon><Edit /></el-icon> 重命名
          </div>
          <div class="ctx-item ctx-danger" @click="handleDelete">
            <el-icon><Delete /></el-icon> 删除
          </div>
        </template>
      </div>
    </teleport>

    <!-- 全局点击关闭菜单 -->
    <teleport to="body">
      <div v-if="contextMenuVisible" class="ctx-overlay" @click="closeContextMenu" />
    </teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onBeforeUnmount, watch } from 'vue'
import type { Ref } from 'vue'
import {
  Folder,
  Document,
  FolderAdd,
  DocumentAdd,
  Delete,
  Edit,
  DocumentCopy,
  FolderOpened,
  Star,
  Search,
  Picture,
  Notebook,
  Grid
} from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import NotificationUtil from '@/utils/NotificationUtil'
import { tauri_request } from '@/utils/tauri_request'
import { ResponseData } from '@/entity/entitys'
import useStore from '@/store/index'
import { TreeNode } from '@/Types/entitys'

const { appConfig, useDocumentManagerStore } = useStore()
let md_working_directory = ref<string>('')

// Props & Emits
const props = defineProps<{
  data: TreeNode[]
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'node-click', node: TreeNode): void
  (e: 'refresh'): void
  (e: 'add-file', parent: TreeNode, name: string): void
  (e: 'add-folder', parent: TreeNode, name: string): void
  (e: 'delete-node', node: TreeNode): void
  (e: 'rename-node', node: TreeNode, newName: string): void
}>()

// ===== 搜索过滤 =====
const showSearch = ref(false)
const searchQuery = ref('')
const searchLower = computed(() => searchQuery.value.toLowerCase())

function handleSearch() {
  // 搜索时自动展开所有匹配节点
  if (searchQuery.value && treeRef.value) {
    expandAllMatches(treeRef.value.store.root.childNodes, searchLower.value)
  }
}

function expandAllMatches(nodes: any[], query: string) {
  for (const node of nodes) {
    const label = (node.label || '').toLowerCase()
    const hasMatch = label.includes(query)
    if (hasMatch || node.childNodes?.some((c: any) => matchesQuery(c, query))) {
      node.expand()
    }
    if (node.childNodes?.length) {
      expandAllMatches(node.childNodes, query)
    }
  }
}

function matchesQuery(node: any, query: string): boolean {
  const label = (node.label || '').toLowerCase()
  if (label.includes(query)) return true
  if (node.childNodes?.length) {
    return node.childNodes.some((c: any) => matchesQuery(c, query))
  }
  return false
}

// ===== Compact Folders =====
// 将连续单子文件夹合并显示
function compactFolders(nodes: TreeNode[]): TreeNode[] {
  return nodes.map(node => {
    if (!node.isFolder || !node.children?.length) return node
    // 递归处理子节点
    const compactedChildren = compactFolders(node.children)
    // 如果只有一个子文件夹，合并显示
    if (compactedChildren.length === 1 && compactedChildren[0].isFolder) {
      const child = compactedChildren[0]
      const merged: TreeNode = {
        ...child,
        label: `${node.label}/${child.label}`,
        folder: child.folder,
        filePath: child.filePath,
        id: child.id,
        children: child.children ? compactFolders(child.children) : child.children
      }
      return merged
    }
    return { ...node, children: compactedChildren }
  })
}

const treeData = computed(() => compactFolders(props.data))

// 过滤后的树数据
const filteredTreeData = computed(() => {
  if (!searchQuery.value) return treeData.value
  return filterTree(treeData.value, searchLower.value)
})

function filterTree(nodes: TreeNode[], query: string): TreeNode[] {
  const result: TreeNode[] = []
  for (const node of nodes) {
    const label = node.label.toLowerCase()
    if (node.children?.length) {
      const filteredChildren = filterTree(node.children, query)
      if (filteredChildren.length > 0 || label.includes(query)) {
        result.push({ ...node, children: filteredChildren.length > 0 ? filteredChildren : node.children })
      }
    } else if (label.includes(query)) {
      result.push(node)
    }
  }
  return result
}

// ===== 树引用 =====
const treeRef: Ref<any> = ref(null)
const hoveredNodeId = ref<string | null>(null)

// ===== 对话框状态 =====
const renameDialogVisible = ref(false)
const renameInput = ref('')
const renameInputRef: Ref<any> = ref(null)
const createDialogVisible = ref(false)
const createInput = ref('')
const createInputRef: Ref<any> = ref(null)
const createDialogTitle = ref('')
const createMode = ref<'file' | 'folder'>('file')

// ===== 右键菜单 =====
const contextMenuVisible = ref(false)
const contextNode = ref<TreeNode | null>(null)
const menuX = ref(0)
const menuY = ref(0)

const defaultProps = { children: 'children', label: 'label' }

// ===== 收藏判断 =====
function isFavoriteNode(node: TreeNode): boolean {
  return useDocumentManagerStore.isFavorite(node.filePath || node.folder || '')
}

// ===== 文件图标 =====
function getNodeIcon(data: TreeNode) {
  if (data.isFolder) return Folder
  const name = data.label || data.filePath || ''
  const ext = name.split('.').pop()?.toLowerCase() || ''
  // Markdown
  if (['md', 'markdown'].includes(ext)) return Notebook
  // PDF
  if (ext === 'pdf') return Document
  // 图片类型
  if (['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'bmp'].includes(ext)) return Picture
  // 代码/配置类型
  if (['ts', 'js', 'vue', 'json', 'yaml', 'yml', 'xml', 'html', 'css', 'scss', 'java', 'py', 'rs', 'go', 'c', 'cpp', 'h', 'sh', 'bat', 'properties', 'toml', 'env'].includes(ext)) return Grid
  // 压缩文件
  if (['zip', 'tar', 'gz', 'rar', '7z'].includes(ext)) return Grid
  return Document
}

// ===== 节点交互 =====
function onNodeMouseEnter(data: TreeNode) {
  hoveredNodeId.value = data.id
}
function onNodeMouseLeave() {
  hoveredNodeId.value = null
}

function handleNodeClick(data: TreeNode) {
  emit('node-click', data)
}

// ===== 右键菜单 =====
function handleNodeContextMenu(event: MouseEvent, data: TreeNode) {
  event.preventDefault()
  event.stopPropagation()
  contextNode.value = data
  menuX.value = event.clientX
  menuY.value = event.clientY
  contextMenuVisible.value = true
}

function closeContextMenu() {
  contextMenuVisible.value = false
  contextNode.value = null
}

// ===== 菜单操作 =====
function handleCopyPath() {
  const node = contextNode.value
  if (!node) return
  const path = node.folder || node.filePath || ''
  navigator.clipboard.writeText(path).then(() => {
    ElMessage.success('路径已复制')
  }).catch(() => {
    ElMessage.error('复制失败')
  })
  closeContextMenu()
}

function handleToggleFavorite() {
  const node = contextNode.value
  if (!node) return
  const path = node.filePath || node.folder || ''
  useDocumentManagerStore.toggleFavorite(path)
  closeContextMenu()
}

async function handleOpenInExplorer() {
  const node = contextNode.value
  if (!node) return
  try {
    const path = node.folder || node.filePath?.substring(0, node.filePath.lastIndexOf('/')) || ''
    await tauri_request<ResponseData<string>>('file_manager_open_path', { path })
  } catch (error) {
    NotificationUtil.error(String(error))
  }
  closeContextMenu()
}

function showRenameDialog() {
  const node = contextNode.value
  if (!node) return
  let label = node.label
  if (!node.isFolder) {
    const lastDotIndex = label.lastIndexOf('.')
    if (lastDotIndex > 0) label = label.substring(0, lastDotIndex)
  }
  renameInput.value = label
  renameDialogVisible.value = true
  closeContextMenu()
  nextTick(() => renameInputRef.value?.focus())
}

function confirmRename() {
  if (!contextNode.value || !renameInput.value.trim()) {
    ElMessage.warning('名称不能为空')
    return
  }
  emit('rename-node', contextNode.value, renameInput.value.trim())
  renameDialogVisible.value = false
}

function handleDelete() {
  if (!contextNode.value) return
  emit('delete-node', contextNode.value)
  closeContextMenu()
}

// ===== 创建文件/文件夹 =====
function showCreateDialog(mode: 'file' | 'folder') {
  if (!contextNode.value) return
  createMode.value = mode
  createInput.value = ''
  createDialogTitle.value = mode === 'folder' ? '新建文件夹' : '新建文件'
  createDialogVisible.value = true
  closeContextMenu()
  nextTick(() => createInputRef.value?.focus())
}

function confirmCreate() {
  if (!contextNode.value || !createInput.value.trim()) {
    ElMessage.warning('名称不能为空')
    return
  }
  if (createMode.value === 'folder') {
    emit('add-folder', contextNode.value, createInput.value.trim())
  } else {
    emit('add-file', contextNode.value, createInput.value.trim())
  }
  createDialogVisible.value = false
}

function handleAddFile(data: TreeNode) {
  contextNode.value = data
  showCreateDialog('file')
}

function handleAddFolder(data: TreeNode) {
  contextNode.value = data
  showCreateDialog('folder')
}

function handleRefresh() {
  emit('refresh')
}

// ===== 根目录操作 =====
function handleAddRootFile() {
  const rootNode: TreeNode = {
    id: 'root',
    label: '根目录',
    isFolder: true,
    routerPath: '',
    folder: md_working_directory.value,
    filePath: '',
    isfile: false,
    children: [],
    value: md_working_directory.value,
  }
  contextNode.value = rootNode
  createMode.value = 'file'
  createInput.value = ''
  createDialogTitle.value = '新建文件'
  createDialogVisible.value = true
  nextTick(() => createInputRef.value?.focus())
}

function handleAddRootFolder() {
  const rootNode: TreeNode = {
    id: 'root',
    label: '根目录',
    isFolder: true,
    routerPath: '',
    folder: md_working_directory.value,
    filePath: '',
    isfile: false,
    value: md_working_directory.value,
    children: []
  }
  contextNode.value = rootNode
  createMode.value = 'folder'
  createInput.value = ''
  createDialogTitle.value = '新建文件夹'
  createDialogVisible.value = true
  nextTick(() => createInputRef.value?.focus())
}

// ===== 拖拽 =====
function handleAllowDrop(draggingNode: any, dropNode: any, type: string) {
  // 只允许拖到文件夹内
  if (type === 'inner') {
    return dropNode.data.isFolder
  }
  return false
}

async function handleNodeDrop(draggingNode: any, dropNode: any, type: string) {
  const dragged = draggingNode.data as TreeNode
  const target = dropNode.data as TreeNode
  if (!target.isFolder) return

  try {
    const oldPath = dragged.filePath || dragged.folder
    const name = oldPath.split(/[/\\]/).pop() || ''
    const newPath = `${target.folder}/${name}`

    if (dragged.isFolder) {
      const res = await tauri_request<ResponseData<string>>('up_folder_name', {
        path: oldPath,
        upFolderName: name,
        fatherPath: target.folder
      })
      // 后端可能需要不同的参数，这里尝试 move
      if (res?.code !== 200) {
        // 尝试用 rename 方式移动
        const res2 = await tauri_request<ResponseData<string>>('move_file', {
          oldPath,
          newPath
        })
        if (res2?.code !== 200 && res?.code !== 200) {
          NotificationUtil.error('移动失败：后端不支持拖拽移动')
          emit('refresh')
          return
        }
      }
    } else {
      const res = await tauri_request<ResponseData<string>>('move_file', {
        oldPath,
        newPath
      })
      if (res?.code !== 200) {
        NotificationUtil.error('移动失败')
        emit('refresh')
        return
      }
    }
    NotificationUtil.success(`已移动到 ${target.label}`)
    emit('refresh')
  } catch (error) {
    console.error('拖拽移动失败:', error)
    NotificationUtil.error('移动失败')
    emit('refresh')
  }
}

// ===== 折叠全部 =====
function collapseAll() {
  if (treeRef.value) {
    const flatten = (nodes: TreeNode[]): string[] => {
      const ids: string[] = []
      for (const n of nodes) {
        if (n.isFolder && n.children?.length) {
          ids.push(n.id)
          ids.push(...flatten(n.children))
        }
      }
      return ids
    }
    const allIds = flatten(treeData.value)
    allIds.forEach(id => treeRef.value.store.nodesMap[id]?.collapse())
  }
}

// 展开匹配搜索的节点
watch(() => searchQuery.value, () => {
  if (searchQuery.value && treeRef.value) {
    nextTick(() => {
      expandAllMatches(treeRef.value.store.root.childNodes, searchLower.value)
    })
  }
})

// 初始化
onMounted(() => {
  md_working_directory.value = appConfig.getMdWorkingDirectory
})

// 暴露方法
defineExpose({
  refresh: () => emit('refresh'),
  addRootFile: handleAddRootFile,
  addRootFolder: handleAddRootFolder,
  collapseAll,
  scrollToNode: (filePath: string) => {
    // Autoscroll: 根据文件路径定位并高亮树节点
    if (!treeRef.value) return
    const findNode = (nodes: TreeNode[], path: string): TreeNode | null => {
      for (const n of nodes) {
        if (n.filePath === path || n.folder === path) return n
        if (n.children?.length) {
          const found = findNode(n.children, path)
          if (found) return found
        }
      }
      return null
    }
    const node = findNode(treeData.value, filePath)
    if (node) {
      treeRef.value.setCurrentKey(node.id)
      // 展开父节点
      let parent = treeRef.value.store.nodesMap[node.id]?.parent
      while (parent && parent.id !== 'root') {
        parent.expand()
        parent = parent.parent
      }
    }
  },
  setSearchVisible: (v?: boolean) => { showSearch.value = v ?? !showSearch.value; if (!showSearch.value) { searchQuery.value = '' } }
})

onBeforeUnmount(() => {
  closeContextMenu()
})
</script>

<style lang="scss">
@use '@/styles/theme.scss' as theme;

.file-tree-container {
  position: relative;
  height: 100%;
  user-select: none;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    color: theme.getVar('textColor');
  }
}

// ===== 搜索框 =====
.tree-search {
  padding: 4px 8px;

  @include theme.useTheme {
    .el-input__wrapper {
      background-color: rgba(255, 255, 255, 0.05);
      box-shadow: none;
      border: 1px solid theme.getVar('borderColor');
    }
    .el-input__inner {
      color: theme.getVar('textColor');
    }
  }
}

// ===== 空状态 =====
.empty-tree-actions {
  display: grid;
  justify-content: center;
  gap: 12px;
  padding: 20px;

  .button-card {
    padding: 12px 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;

    @include theme.useTheme {
      background-color: theme.getVar('bgColor');
      border-color: theme.getVar('borderColor');
    }

    .svg-icon {
      font-size: 24px;
      color: var(--el-color-primary);
      transition: color 0.2s;
      &:hover { color: var(--el-color-primary-light-4); }
    }
  }
}

.empty-hint {
  text-align: center;
  padding: 20px;
  color: #555;
  font-size: 13px;
}

// ===== 树节点 =====
.custom-tree-node {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 2px 0;
  flex: 1;
}

.node-label {
  display: flex;
  align-items: center;
  flex: 1;
  overflow: hidden;
}

.node-icon {
  margin-right: 6px;
  font-size: 16px;
  width: 1em;
  height: 1em;
  flex-shrink: 0;
}

.node-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.node-is-favorite .node-text {
  color: #e5c07b;
}

.node-actions {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  padding-right: 4px;
}

.action-icon {
  margin-left: 4px;
  font-size: 16px;
  cursor: pointer;
  padding: 2px;
  border-radius: 4px;
  color: #888;
  transition: all 0.15s;

  &:hover {
    color: #ccc;
    @include theme.useTheme {
      background-color: theme.getVar('hoverColor');
    }
  }
}

// ===== 右键菜单 =====
.ctx-overlay {
  position: fixed;
  inset: 0;
  z-index: 9998;
}

.tree-context-menu {
  position: fixed;
  z-index: 9999;
  background-color: #2d2d2d;
  border: 1px solid #454545;
  border-radius: 6px;
  padding: 4px 0;
  min-width: 180px;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.5);

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-color: theme.getVar('borderColor');
  }
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 16px;
  font-size: 13px;
  color: #ccc;
  cursor: pointer;
  transition: background-color 0.1s;

  &:hover {
    background-color: rgba(0, 122, 204, 0.3);
    color: #fff;
  }

  .el-icon { font-size: 14px; }
}

.ctx-danger {
  color: #e06c75;
  &:hover {
    background-color: rgba(224, 108, 117, 0.2);
    color: #e06c75;
  }
}

.ctx-divider {
  height: 1px;
  background-color: #454545;
  margin: 4px 0;

  @include theme.useTheme {
    background-color: theme.getVar('borderColor');
  }
}

// ===== el-tree 样式 =====
.el-tree {
  background-color: transparent;
  color: inherit;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }

  .el-tree-node__content {
    transition: background-color 0.15s;

    @include theme.useTheme {
      background-color: theme.getVar('bgColor');
      color: theme.getVar('textColor');
    }

    &:hover {
      @include theme.useTheme {
        background-color: theme.getVar('hoverColor');
      }
    }
  }

  .el-tree-node.is-current > .el-tree-node__content {
    @include theme.useTheme {
      background-color: theme.getVar('hoverColor');
    }
  }

  .el-tree-node__expand-icon {
    @include theme.useTheme {
      color: theme.getVar('textColor');
    }
  }
}
</style>
