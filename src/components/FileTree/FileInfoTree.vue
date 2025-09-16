<template>
  <div class="file-tree-container">
    <el-tree v-if="treeData.length > 0" ref="treeRef" :data="treeData" node-key="id" :props="defaultProps"
      :expand-on-click-node="false" highlight-current @node-contextmenu="handleNodeContextMenu"
      @node-click="handleNodeClick">
      <template #default="{ node, data }">
        <div class="custom-tree-node" @mouseenter="onNodeMouseEnter(data)" @mouseleave="onNodeMouseLeave">
          <span class="node-label">
            <el-icon class="node-icon">
              <component :is="getNodeIcon(data)" />
            </el-icon>
            <span class="node-text">{{ node.label }}</span>
          </span>

          <span v-show="hoveredNodeId === data.id" class="node-actions">
            <template v-if="data.isFolder">
              <el-icon class="action-icon add-file" @click.stop="handleAddFile(data)" title="新增文件">
                <DocumentAdd />
              </el-icon>
              <el-icon class="action-icon add-folder" @click.stop="handleAddFolder(data)" title="新增文件夹">
                <FolderAdd />
              </el-icon>
            </template>
          </span>
        </div>
      </template>
    </el-tree>

    <!-- 当 treeData 无数据时显示新增按钮 -->
    <div v-else class="empty-tree-actions">
      <el-card class="button-card" @click="handleAddRootFile">
        <el-icon class="svg-icon">
          <DocumentAdd />
        </el-icon>
        <!-- <span>新增文件</span> -->
      </el-card>
      <el-card class="button-card" @click="handleAddRootFolder">
        <el-icon class="svg-icon">
          <FolderAdd />
        </el-icon>
        <!-- <span>新增文件夹</span> -->
      </el-card>
    </div>

    <!-- 重命名对话框 -->
    <el-dialog v-model="renameDialogVisible" title="重命名" width="640px" :modal="false" append-to-body>
      <el-input v-model="renameInput" ref="renameInputRef" @keyup.enter="confirmRename" />
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="renameDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="confirmRename">确定</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 创建文件/文件夹对话框 -->
    <el-dialog v-model="createDialogVisible" :title="createDialogTitle" width="640px" :modal="false" append-to-body>
      <div class="settings-item">
        <div class="setting-info">
          <div class="setting-header">
            <span class="setting-description">工作目录</span>
          </div>
          <div class="directory-display">
            <el-icon class="folder-icon">
              <Folder />
            </el-icon>
            <span class="setting-working-directory">{{ md_working_directory }}</span>
          </div>
        </div>
      </div>
      <el-input v-model="createInput" ref="createInputRef" @keyup.enter="confirmCreate" placeholder="请输入名称" />
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="createDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="confirmCreate">确定</el-button>
        </span>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, nextTick, onMounted, onBeforeUnmount, watch } from 'vue'
import type { Ref } from 'vue'
import {
  Folder,
  Document,
  FolderAdd,
  DocumentAdd,
  Delete,
  Refresh,
  Edit
} from '@element-plus/icons-vue'
import MarkDownColorful from '@/components/Icon/MarkDownColorful.vue'
import PDF from '@/components/Icon/PDF.vue'
import FlodeColorful from '@/components/Icon/FlodeColorful.vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import NotificationUtil from '@/utils/NotificationUtil'
import { tauri_request } from '@/utils/tauri_request'
import { ResponseData } from '@/entity/entitys'
import useStore from '@/store/index'
let md_working_directory = ref<string>('')

const { appConfig } = useStore()
const userStore = appConfig;
// 定义节点数据结构
interface TreeNode {
  id: string
  label: string
  isFolder: boolean
  type?: 'folder' | 'markdown' | 'pdf'
  children?: TreeNode[]
  [key: string]: any
}

// Props
const props = defineProps<{
  data: TreeNode[]
}>()

// Emits
const emit = defineEmits<{
  (e: 'node-click', node: TreeNode): void
  (e: 'refresh'): void
  (e: 'add-file', parent: TreeNode, name: string): void
  (e: 'add-folder', parent: TreeNode, name: string): void
  (e: 'delete-node', node: TreeNode): void
  (e: 'rename-node', node: TreeNode, newName: string): void
}>()

// 响应式数据
const treeData = ref<TreeNode[]>(props.data)
const treeRef: Ref<any> = ref(null)
const hoveredNodeId = ref<string | null>(null)
const currentContextNode = ref<TreeNode | null>(null)

// 菜单位置
const menuPosition = reactive({
  x: 0,
  y: 0
})

// 菜单项
const contextMenuItems = ref<Array<{
  label: string
  command: string
  icon: any
}>>([])

// 重命名对话框
const renameDialogVisible = ref(false)
const renameInput = ref('')
const renameInputRef: Ref<any> = ref(null)

// 创建对话框
const createDialogVisible = ref(false)
const createInput = ref('')
const createInputRef: Ref<any> = ref(null)
const createDialogTitle = ref('')
const createMode = ref<'file' | 'folder'>('file')

// Tree配置
const defaultProps = {
  children: 'children',
  label: 'label'
}

// 监听props.data变化
watch(() => props.data, (newData) => {
  treeData.value = [...newData]
}, { deep: true })
onMounted(() => {
  md_working_directory.value = userStore.getMdWorkingDirectory;
})
// 节点图标
const getNodeIcon = (data: TreeNode) => {
  if (data.isFolder) {
    return FlodeColorful
  }
  if (data.type === 'pdf') {
    return PDF
  }
  return MarkDownColorful
}

// 鼠标进入节点
const onNodeMouseEnter = (data: TreeNode) => {
  hoveredNodeId.value = data.id
}

// 鼠标离开节点
const onNodeMouseLeave = () => {
  hoveredNodeId.value = null
}

// 节点点击事件
const handleNodeClick = (data: TreeNode) => {
  emit('node-click', data)
}

// 节点右键菜单
const handleNodeContextMenu = (
  event: MouseEvent,
  data: TreeNode
) => {
  event.preventDefault()

  // 设置当前上下文节点
  currentContextNode.value = data

  // 计算菜单位置，确保不会超出视窗边界
  const menuWidth = 180
  const menuHeight = contextMenuItems.value.length * 36 + 12 // 估算菜单高度

  let x = event.clientX
  let y = event.clientY

  // 检查右侧是否足够显示菜单
  if (x + menuWidth > window.innerWidth) {
    x = window.innerWidth - menuWidth - 5
  }

  // 检查底部是否足够显示菜单
  if (y + menuHeight > window.innerHeight) {
    y = window.innerHeight - menuHeight - 5
  }

  // 设置菜单位置
  menuPosition.x = x
  menuPosition.y = y

  // 设置菜单项
  if (data.isFolder) {
    contextMenuItems.value = [
      { label: '新增子文件夹', command: 'add-folder', icon: 'FolderAdd' },
      { label: '新增子文件', command: 'add-file', icon: 'DocumentAdd' },
      { label: '在文件管理器中显示', command: 'file-manager', icon: 'Edit' },
      { label: '重命名', command: 'rename', icon: 'Edit' },
      { label: '删除文件夹', command: 'delete', icon: 'Delete' },
      { label: '刷新', command: 'refresh', icon: 'Refresh' }
    ]
  } else {
    contextMenuItems.value = [
      { label: '在文件管理器中显示', command: 'file-manager', icon: 'Edit' },
      { label: '重命名', command: 'rename', icon: 'Edit' },
      { label: '删除文件', command: 'delete', icon: 'Delete' }
    ]
  }

  // 创建并显示菜单
  createContextMenu()
}

// 创建右键菜单
const createContextMenu = () => {
  // 先移除已存在的菜单
  removeContextMenu()

  // 创建菜单元素
  const menu = document.createElement('ul')
  menu.className = 'file-tree-context-menu'

  // 设置菜单位置
  menu.style.left = menuPosition.x + 'px'
  menu.style.top = menuPosition.y + 'px'

  // 添加菜单项
  contextMenuItems.value.forEach(item => {
    const li = document.createElement('li')
    li.className = 'context-menu-item'

    // 创建图标元素
    const iconWrapper = document.createElement('span')
    iconWrapper.className = 'context-menu-icon'

    // 创建 Element Plus 图标
    const iconEl = document.createElement('i')
    iconEl.className = 'el-icon'

    // 根据图标名称创建对应的图标
    const iconSvg = getIconSvg(item.icon)
    if (iconSvg) {
      iconEl.innerHTML = iconSvg
    }

    iconWrapper.appendChild(iconEl)

    // 创建文本元素
    const textEl = document.createElement('span')
    textEl.textContent = item.label

    // 组装菜单项
    li.appendChild(iconWrapper)
    li.appendChild(textEl)

    li.addEventListener('click', (e) => {
      e.stopPropagation();
      handleContextMenuCommand(item.command)
    })
    menu.appendChild(li)
  })

  // 添加到 body
  document.body.appendChild(menu);

  // 保存引用以便后续移除
  (window as any).__FILE_TREE_CONTEXT_MENU__ = menu

  // 添加全局点击事件监听器来关闭菜单
  const handleClickOutside = (event: MouseEvent) => {
    const menu = (window as any).__FILE_TREE_CONTEXT_MENU__
    if (menu && !menu.contains(event.target as Node)) {
      removeContextMenu()
      document.removeEventListener('click', handleClickOutside)
    }
  }

  // 延迟添加监听器，避免立即触发
  setTimeout(() => {
    document.addEventListener('click', handleClickOutside)
  }, 0)
}

// 获取图标 SVG
const getIconSvg = (iconName: string) => {
  switch (iconName) {
    case 'FolderAdd':
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1em" height="1em" fill="currentColor"><path d="M128 128v640h768V320H576V128H128z m64 64h320v192h256v320H192V192z m384 128h160v64H576v-64z"></path></svg>`
    case 'DocumentAdd':
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1em" height="1em" fill="currentColor"><path d="M832 384H576V128H192v768h640V384z m-448 64h320v384H384V448z m192 128v64h-64v64h-64v-64h-64v-64h64v-64h64v64z m-192-192h320V192l192 192H384z"></path></svg>`
    case 'Edit':
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1em" height="1em" fill="currentColor"><path d="M832 512a32 32 0 0 1-32 32H352a32 32 0 0 1 0-64h448a32 32 0 0 1 32 32z m-32 192a32 32 0 0 1-32 32H352a32 32 0 0 1 0-64h448a32 32 0 0 1 32 32z m-320 192a32 32 0 0 1-32 32H352a32 32 0 0 1 0-64h96a32 32 0 0 1 32 32z m352-512V160H256v480h224v64H192a32 32 0 0 1-32-32V128a32 32 0 0 1 32-32h512l192 192v320h-64V320z"></path></svg>`
    case 'Delete':
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1em" height="1em" fill="currentColor"><path d="M832 128H688v-32a160 160 0 0 0-160-160h-96a160 160 0 0 0-160 160v32H192a96 96 0 0 0-96 96v64a96 96 0 0 0 96 96v448a96 96 0 0 0 96 96h512a96 96 0 0 0 96-96V384a96 96 0 0 0 96-96v-64a96 96 0 0 0-96-96z m-544-32a96 96 0 0 1 96-96h96a96 96 0 0 1 96 96v32H288v-32z m448 544H288V192h512v448z m-352-256a32 32 0 0 1 64 0v192a32 32 0 0 1-64 0V384z m128 0a32 32 0 0 1 64 0v192a32 32 0 0 1-64 0V384z"></path></svg>`
    case 'Refresh':
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1em" height="1em" fill="currentColor"><path d="M832 512a320 320 0 1 1-640 0 320 320 0 0 1 640 0z m64 0a384 384 0 1 0-768 0 384 384 0 0 0 768 0z m-672 96h128v-64H224a32 32 0 0 0-32 32v256h64V608z m640 160h-64v64h-64v-64h-64v-64h192v64z"></path></svg>`
    default:
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1em" height="1em" fill="currentColor"><path d="M832 128H192a32 32 0 0 0-32 32v640a32 32 0 0 0 32 32h640a32 32 0 0 0 32-32V160a32 32 0 0 0-32-32z m-64 640H256V192h512v576z"></path></svg>`
  }
}

// 移除右键菜单
const removeContextMenu = () => {
  const menu = (window as any).__FILE_TREE_CONTEXT_MENU__
  if (menu && menu.parentNode) {
    menu.parentNode.removeChild(menu)
  }
  delete (window as any).__FILE_TREE_CONTEXT_MENU__
}

// 处理菜单命令
const handleContextMenuCommand = (command: string) => {
  removeContextMenu()

  if (!currentContextNode.value) return

  switch (command) {
    case 'add-folder':
      showCreateDialog('folder')
      break
    case 'file-manager':
      FileMmanagerOpen()
      break
    case 'add-file':
      showCreateDialog('file')
      break
    case 'rename':
      showRenameDialog()
      break
    case 'delete':
      handleDelete()
      break
    case 'refresh':
      emit('refresh')
      break
  }
}

// 显示重命名对话框
const showRenameDialog = () => {
  if (!currentContextNode.value) return

  renameInput.value = currentContextNode.value.label
  renameDialogVisible.value = true

  nextTick(() => {
    renameInputRef.value?.focus()
  })
}

// 确认重命名
const confirmRename = () => {
  if (!currentContextNode.value || !renameInput.value.trim()) {
    ElMessage.warning('名称不能为空')
    return
  }

  emit('rename-node', currentContextNode.value, renameInput.value.trim())
  renameDialogVisible.value = false
}

// 显示创建对话框
const showCreateDialog = (mode: 'file' | 'folder') => {
  if (!currentContextNode.value) return

  createMode.value = mode
  createInput.value = ''
  createDialogTitle.value = mode === 'folder' ? '新建文件夹' : '新建文件'
  createDialogVisible.value = true

  nextTick(() => {
    createInputRef.value?.focus()
  })
}
// 显示创建对话框
const FileMmanagerOpen = async () => {
  try {
    console.log("FileMmanagerOpen", currentContextNode.value);
    let res = await tauri_request<ResponseData<string>>("file_manager_open_path", { path: currentContextNode.value.meta.folder });
    if (res && res.code != 200) {
      NotificationUtil.error(res.msg);
    }
  } catch (error) {
    NotificationUtil.error(error);
  }

}

// 确认创建
const confirmCreate = () => {
  if (!currentContextNode.value || !createInput.value.trim()) {
    ElMessage.warning('名称不能为空')
    return
  }

  if (createMode.value === 'folder') {
    emit('add-folder', currentContextNode.value, createInput.value.trim())
  } else {
    emit('add-file', currentContextNode.value, createInput.value.trim())
  }

  createDialogVisible.value = false
}

// 处理删除
const handleDelete = () => {
  if (!currentContextNode.value) return

  const message = currentContextNode.value.isFolder
    ? '确定要删除这个文件夹及其所有内容吗？'
    : '确定要删除这个文件吗？'

  ElMessageBox.confirm(message, '提示', {
    confirmButtonText: '确定',
    cancelButtonText: '取消',
    type: 'warning'
  }).then(() => {
    emit('delete-node', currentContextNode.value!)
  }).catch(() => {
    // 用户取消删除
  })
}

// 处理新增文件
const handleAddFile = (data: TreeNode) => {
  console.log('handleAddFile', data)
  showCreateDialog('file')
}

// 处理新增文件夹
const handleAddFolder = (data: TreeNode) => {
  showCreateDialog('folder')
}
// 处理新增根目录文件
const handleAddRootFile = () => {
  // 创建一个虚拟的根节点用于添加文件
  const rootNode: TreeNode = {
    id: 'root',
    label: '根目录',
    isFolder: true,
    children: []
  }
  currentContextNode.value = rootNode
  showCreateDialog('file')
}

// 处理新增根目录文件夹
const handleAddRootFolder = () => {
  // 创建一个虚拟的根节点用于添加文件夹
  const rootNode: TreeNode = {
    id: 'root',
    label: '根目录',
    isFolder: true,
    children: []
  }
  currentContextNode.value = rootNode
  showCreateDialog('folder')
}
// 组件卸载前移除菜单
onBeforeUnmount(() => {
  removeContextMenu()
})

// 暴露方法给父组件
defineExpose({
  refresh: () => {
    emit('refresh')
  }
})
</script>

<style scoped lang="scss">
.file-tree-container {
  position: relative;
  height: 100%;
  user-select: none;
}

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
    gap: 8px;

    .svg-icon {
      font-size: 24px;
      color: var(--el-color-primary);
      transition: color 0.2s;

      &:hover {
        color: var(--el-color-primary-light-4);
      }

    }
  }
}


.custom-tree-node {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 4px 0;
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
}

.node-actions {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  padding-right: 8px;
}

.action-icon {
  margin-left: 8px;
  font-size: 20px;
  cursor: pointer;
  padding: 2px;
  border-radius: 4px;
  transition: background-color 0.2s;

  &:hover {
    background-color: var(--el-fill-color-light);
  }

  &.add-file {
    color: var(--el-color-primary);
  }

  &.add-folder {
    color: var(--el-color-warning);
  }
}

.dialog-footer {
  text-align: right;
}
</style>

<style>
/* 全局样式，确保右键菜单显示在最上层 */
.file-tree-context-menu {
  position: fixed;
  z-index: 999999999 !important;
  background: var(--el-bg-color-overlay);
  border: 1px solid var(--el-border-color-light);
  border-radius: var(--el-border-radius-base);
  box-shadow: var(--el-box-shadow-light);
  padding: 6px 0;
  min-width: 180px;
  list-style: none;
  margin: 0;
  color: var(--el-text-color-regular);
}

.file-tree-context-menu .context-menu-item {
  padding: 8px 16px;
  cursor: pointer;
  display: flex;
  align-items: center;
  font-size: var(--el-font-size-base);
}

.file-tree-context-menu .context-menu-item:hover {
  background-color: var(--el-fill-color-light);
}

.file-tree-context-menu .context-menu-item .context-menu-icon {
  margin-right: 8px;
  font-size: var(--el-font-size-base);
  width: 1em;
  height: 1em;
  display: flex;
  align-items: center;
}

.file-tree-context-menu .context-menu-item .el-icon {
  width: 1em;
  height: 1em;
  color: var(--el-text-color-regular);
}

.settings-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 15px 0;
  border-bottom: 1px solid var(--el-border-color-lighter);

  &:last-child {
    border-bottom: none;
  }
}

.setting-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.setting-header {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.setting-label {
  display: block;
  font-size: 1rem;
  color: var(--el-text-color-primary);
  font-weight: 500;
}

.setting-description {
  display: block;
  font-size: 0.85rem;
  color: var(--el-text-color-secondary);
}

.directory-display {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px;
  background-color: var(--el-fill-color-light);
  border-radius: 4px;
  font-family: monospace;
  font-size: 0.9rem;
}

.folder-icon {
  color: var(--el-color-primary);
}

.setting-working-directory {
  color: var(--el-text-color-primary);
  word-break: break-all;
}
</style>