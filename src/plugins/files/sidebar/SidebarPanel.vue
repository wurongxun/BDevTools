<script setup lang="ts">
import { computed, nextTick, onActivated, onMounted, onUnmounted, ref, watch } from 'vue'
import FileInfoTree from './FileInfoTree.vue'
import useStore from '@/store'
import { tauri_request } from '@/utils/tauri_request'
import NotificationUtil from '@/utils/NotificationUtil'
import { ResponseData, TreeNode } from '@/Types/entitys'
import GlobalDialog from '@/components/GlobalDialog.vue'
import StringUtil from '@/utils/StringUtil'
import { load, Store } from '@/core/runtime'
import { pdf_store_tem_key_prefix, pdf_store_tem_key, pdf_store_conten_key_prefix } from '@/setting'
import { useDocumentManagerStore } from '../store/documents'

// 常量定义
const INVALID_CHARS_REGEX = /.*[/\\:*?|].*/
const PURE_NUMBER_REGEX = /^\d+$/
const MAX_FILENAME_LENGTH = 255
let store: Store | undefined = undefined
const { fileTreeStore } = useStore()
const docStore = useDocumentManagerStore()

// 树数据排序
const file_node = computed(() => sortTreeData(fileTreeStore.tree_node))

const isLoading = ref(false)

const sortTreeData = (data: TreeNode[]): TreeNode[] => {
    if (!data || data.length === 0) return []
    return data.sort((a, b) => {
        if (a.isFolder !== b.isFolder) return a.isFolder ? -1 : 1
        return a.label.localeCompare(b.label)
    }).map(node => {
        if (node.children && node.children.length > 0) {
            node.children = sortTreeData(node.children)
        }
        return node
    })
}

// ===== Autoscroll 联动 =====
const fileTreeRef = ref<InstanceType<typeof FileInfoTree> | null>(null)

watch(() => docStore.activeDocumentPath, (newPath) => {
    if (newPath && fileTreeRef.value) {
        nextTick(() => {
            fileTreeRef.value?.scrollToNode(newPath)
        })
    }
})

// ===== 初始化 =====
onMounted(async () => {
    if (!store) {
        store = await load(pdf_store_tem_key, {
            autoSave: false,
            defaults: {}
        })
    }
    // 加载收藏和最近打开（由 FileInfoTree 内部处理显示）
    await docStore.loadFavorites()
    await docStore.loadRecentFiles()
    // 确保文件树数据已加载（工作目录就绪后才请求；未就绪时由 onActivated/工作区加载流程补上）
    if (fileTreeStore.tree_node.length === 0 && fileTreeStore.workingDirectory) {
        fileTreeStore.setFileTree()
    }
})

// KeepAlive 缓存恢复（应用启动首挂载后，点击插件图标切换回来只触发 activated 而非 mounted）：
// 若首挂载时工作区尚未就绪导致树未加载，这里在工作目录就绪后补加载
onActivated(() => {
    if (fileTreeStore.tree_node.length === 0 && fileTreeStore.workingDirectory) {
        fileTreeStore.setFileTree()
    }
})

onUnmounted(() => {
    if (store) {
        store = undefined
    }
})

// ===== 验证/CRUD（保持原有逻辑）=====
const validateName = (name: string, isFile = true): { valid: boolean; message?: string } => {
    if (!name || name.trim() === '') return { valid: false, message: isFile ? '请填写文件名称！' : '请填写文件夹名称！' }
    if (INVALID_CHARS_REGEX.test(name)) return { valid: false, message: isFile ? '笔记名称命名不规范，不能包含\\ / : * ? " < > |请检查！' : '文件夹命名不规范，不能包含\\ / : * ? " < > |请检查！' }
    if (PURE_NUMBER_REGEX.test(name)) return { valid: false, message: isFile ? '笔记名称命名不规范，不能为纯数字！' : '文件夹命名不规范，不能为纯数字！' }
    if (name.length > MAX_FILENAME_LENGTH) return { valid: false, message: `名称长度不能超过 ${MAX_FILENAME_LENGTH} 个字符！` }
    return { valid: true }
}

const checkNameExists = async (parentPath: string, name: string): Promise<boolean> => {
    try {
        const result = await tauri_request<ResponseData<string[]>>('get_path_childrenFoldersAndFiles', { path: parentPath })
        return result.data?.includes(name) || false
    } catch { return false }
}

const handleNodeClick = (node: TreeNode) => {
    if (!node.isFolder && node.filePath) {
        docStore.openDocument(node.filePath)
    }
}

const handleRefresh = () => { fileTreeStore.setFileTree() }

const handleAddFile = async (parent: TreeNode, name: string) => {
    if (isLoading.value) return
    const validation = validateName(name, true)
    if (!validation.valid) { NotificationUtil.error(validation.message!); return }
    const exists = await checkNameExists(parent.folder, name)
    if (exists) { NotificationUtil.error('已有相同文件名，请检查！'); return }
    try {
        isLoading.value = true
        const result = await tauri_request<ResponseData<string>>('create_note_file', { fatherPath: parent.folder, newNoteMdFileName: name })
        if (result?.code === 200) { NotificationUtil.success('笔记创建成功'); fileTreeStore.setFileTree() }
        else throw new Error(result?.msg || '未知错误')
    } catch (error: any) {
        console.error('创建文件失败:', error)
        NotificationUtil.error(`笔记创建失败：${error.message}`)
    } finally { isLoading.value = false }
}

const handleAddFolder = async (parent: TreeNode, name: string) => {
    if (isLoading.value) return
    const validation = validateName(name, false)
    if (!validation.valid) { NotificationUtil.error(validation.message!); return }
    const exists = await checkNameExists(parent.folder, name)
    if (exists) { NotificationUtil.error('已有相同文件夹名或者文件名，请检查！'); return }
    try {
        isLoading.value = true
        const result = await tauri_request<ResponseData<string>>('create_folder', { fatherPath: parent.folder, newFolderName: name })
        if (result?.code === 200) { NotificationUtil.success('文件夹创建成功'); fileTreeStore.setFileTree() }
        else throw new Error(result?.msg || '未知错误')
    } catch (error: any) {
        console.error('创建文件夹失败:', error)
        NotificationUtil.error(`笔记本创建失败：${error.message}`)
    } finally { isLoading.value = false }
}

const handleDeleteNode = async (node: TreeNode) => {
    if (isLoading.value) return
    try {
        if (node.isfile) await deleteFile(node)
        else await deleteFolder(node)
        fileTreeStore.setFileTree()
        const activeDoc = docStore.activeDocument
        if (activeDoc?.id === node.routerPath && node.isfile) await processNodeCache(node)
    } catch (error: any) {
        if (error !== 'cancel') {
            console.error('删除操作失败:', error)
            NotificationUtil.error(`操作失败：${error.message}`)
        }
    }
}

// ===== 删除确认弹窗（统一使用 GlobalDialog）=====
const showDeleteConfirm = ref(false)
const deleteConfirmMessage = ref('')
let pendingDeleteAction: (() => Promise<void> | void) | null = null

function askDeleteConfirm(message: string, action: () => Promise<void> | void) {
    deleteConfirmMessage.value = message
    pendingDeleteAction = action
    showDeleteConfirm.value = true
}

async function confirmDelete() {
    showDeleteConfirm.value = false
    const action = pendingDeleteAction
    pendingDeleteAction = null
    if (!action) return
    try {
        await action()
    } catch (error: any) {
        console.error('删除操作失败:', error)
        NotificationUtil.error(`操作失败：${error.message}`)
    }
}

const deleteFile = async (node: TreeNode) => {
    askDeleteConfirm(`确定删除文件 "${node.label}" 吗？`, async () => {
        try {
            isLoading.value = true
            const result = await tauri_request<ResponseData<string>>('remove_file', { path: node.filePath })
            if (result?.code === 200) NotificationUtil.success('删除文件成功！')
            else throw new Error(result?.msg || '未知错误')
        } finally { isLoading.value = false }
    })
}

const deleteFolder = async (node: TreeNode) => {
    if (StringUtil.isNullOrEmpty(node.folder) && node.isFolder) { NotificationUtil.error('请选择文件夹！'); return }
    askDeleteConfirm(`删除文件夹 "${node.folder}" 吗，文件夹下笔记文件也将一并删除！`, async () => {
        try {
            isLoading.value = true
            const children: TreeNode[] = []
            getAllChildrenFile(node, children)
            const cacheResults = await Promise.all(children.map(item => processNodeCache(item)))
            if (cacheResults.some(result => !result)) { NotificationUtil.error('文件缓存处理失败！'); return }
            const result = await tauri_request<ResponseData<string>>('remove_folder', { path: node.folder })
            if (result?.code === 200) NotificationUtil.success('操作成功')
            else throw new Error(result?.msg || '未知错误')
        } finally { isLoading.value = false }
    })
}

const handleRenameNode = async (node: TreeNode, newName: string) => {
    if (isLoading.value) return
    if (!store) {
        store = await load(pdf_store_tem_key, { autoSave: false, defaults: {} })
    }
    const validation = validateName(newName, node.isfile)
    if (!validation.valid) { NotificationUtil.error(validation.message!); return }
    const parentPath = node.isfile
        ? node.filePath.substring(0, node.filePath.lastIndexOf('/'))
        : node.folder.substring(0, node.folder.lastIndexOf('/'))
    const exists = await checkNameExists(parentPath, newName)
    if (exists) { NotificationUtil.error(`已有相同${node.isfile ? '文件' : '文件夹'}名，请检查！`); return }
    try {
        isLoading.value = true
        if (node.isfile) await renameFile(node, newName)
        else await renameFolder(node, newName)
        fileTreeStore.setFileTree()
    } catch (error: any) {
        console.error('重命名失败:', error)
        NotificationUtil.error(`操作失败：${error.message}`)
    } finally { isLoading.value = false }
}

const renameFile = async (node: TreeNode, newName: string) => {
    const cacheProcessed = await processNodeCache(node)
    if (!cacheProcessed) throw new Error('文件缓存处理失败')
    const result = await tauri_request<ResponseData<string>>('up_file_name', { path: node.filePath, upFileName: newName })
    if (result?.code === 200) NotificationUtil.success('修改文件名成功！')
    else throw new Error(result?.msg || '未知错误')
}

const renameFolder = async (node: TreeNode, newName: string) => {
    const children: TreeNode[] = []
    getAllChildrenFile(node, children)
    const cacheResults = await Promise.all(children.map(item => processNodeCache(item)))
    if (cacheResults.some(result => !result)) throw new Error('文件缓存处理失败')
    const result = await tauri_request<ResponseData<string>>('up_folder_name', { path: node.folder, upFolderName: newName })
    if (result?.code === 200) NotificationUtil.success('修改文件夹名成功！')
    else throw new Error(result?.msg || '未知错误')
}

const getAllChildrenFile = (node: TreeNode, list: TreeNode[]) => {
    if (node.isfile) list.push(node)
    else if (node.children) node.children.forEach(child => getAllChildrenFile(child, list))
}

const processNodeCache = async (item: TreeNode): Promise<boolean> => {
    try {
        if (!store) { console.warn('Store 实例不存在'); return true }
        const keyTem = pdf_store_tem_key_prefix + item.id
        const keyConten = pdf_store_conten_key_prefix + item.id
        const temVal = await store.get<{ value: string }>(keyTem)
        if (!temVal || !temVal.value) return true
        const cachedContent = temVal.value
        const contentVal = await store.get<{ value: string }>(keyConten)
        const content = contentVal?.value || null
        if (cachedContent && content !== cachedContent) {
            const saveResult = await tauri_request<ResponseData<string[]>>('save_note', { path: item.filePath, mdText: cachedContent })
            if (saveResult?.code !== 200) throw new Error(saveResult?.msg || '文件保存失败')
        }
        const [deleteTem, deleteConten] = await Promise.all([store.delete(keyTem), store.delete(keyConten)])
        if (!deleteTem || !deleteConten) console.warn('缓存删除不完整:', keyTem, keyConten)
        return true
    } catch (error: any) {
        console.error('处理节点缓存失败:', error)
        NotificationUtil.error(`操作失败：${error.message}`)
        return false
    }
}

// ===== 头部操作 =====
function handleCloseAllEditors() { docStore.closeAllDocuments() }

function handleHeaderCommand(cmd: string) {
    switch (cmd) {
        case 'closeAll': handleCloseAllEditors(); break
        case 'refresh': handleRefresh(); break
        case 'clearRecent': docStore.clearRecentFiles(); break
    }
}


</script>

<template>
    <div class="explorer-sidebar">
        <!-- 头部 -->
        <div class="panel-header">
            <span class="panel-title">资源管理器</span>
            <el-dropdown trigger="click" @command="handleHeaderCommand">
                <app-button size="small" link class="header-more-btn" title="更多操作">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <circle cx="4" cy="8" r="1.2" />
                        <circle cx="8" cy="8" r="1.2" />
                        <circle cx="12" cy="8" r="1.2" />
                    </svg>
                </app-button>
                <template #dropdown>
                    <el-dropdown-menu>
                        <el-dropdown-item command="closeAll">关闭所有编辑器</el-dropdown-item>
                        <el-dropdown-item command="clearRecent">清除最近打开</el-dropdown-item>
                        <el-dropdown-item command="refresh" divided>刷新资源管理器</el-dropdown-item>
                    </el-dropdown-menu>
                </template>
            </el-dropdown>
        </div>

        <!-- 文件树 -->
        <div class="file-tree-section">
            <file-info-tree
                ref="fileTreeRef"
                :data="file_node"
                :loading="isLoading"
                @node-click="handleNodeClick"
                @refresh="handleRefresh"
                @add-file="handleAddFile"
                @add-folder="handleAddFolder"
                @delete-node="handleDeleteNode"
                @rename-node="handleRenameNode"
            />
        </div>

        <!-- 删除确认弹窗（统一使用 GlobalDialog） -->
        <GlobalDialog v-model="showDeleteConfirm" :title="'确认删除'" width="400px">
            <div class="delete-confirm-message">{{ deleteConfirmMessage }}</div>
            <template #footer>
                <app-button size="small" @click="showDeleteConfirm = false">取消</app-button>
                <app-button size="small" type="danger" plain @click="confirmDelete">删除</app-button>
            </template>
        </GlobalDialog>
    </div>
</template>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.explorer-sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    font-family: var(--font-family-base);

    @include theme.useTheme {
        background-color: theme.getVar('sidebarBg');
    }
}

// ===== 头部 =====
// ===== 头部 =====
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 12px;
  height: 35px;
  flex-shrink: 0;
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

.header-more-btn {
    padding: 2px 4px;
    border-radius: var(--radius-md);

    @include theme.useTheme {
        color: theme.getVar('textSecondary');
    }

    &:hover {
        @include theme.useTheme {
            color: theme.getVar('textColor');
            background-color: theme.getVar('activityBarHover');
        }
    }
}

// ===== 文件树区域 =====
.file-tree-section {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    overflow: hidden;
}

// ===== 设置区 =====
.sidebar-settings-collapse {
    border-top: 1px solid var(--el-border-color-lighter);
    flex-shrink: 0;

    :deep(.el-collapse-item__header) {
        font-size: var(--font-size-base);
        height: 32px;
        line-height: 32px;
        padding: 0 12px;
        background: transparent;
        border-bottom: none;
    }
    :deep(.el-collapse-item__wrap) {
        padding: 0 12px;
        border-bottom: none;
    }
    :deep(.el-collapse-item__content) {
        padding-bottom: 8px;
    }
}

// 删除确认文案
.delete-confirm-message {
    line-height: 1.6;
    word-break: break-all;
}
</style>
