<script setup lang="ts">
import { computed, h, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import FileInfoTree from './FileInfoTree.vue'
import useStore from '@/store'
import { useRouter, useRoute } from 'vue-router'
import { tauri_request } from '@/utils/tauri_request'
import NotificationUtil from '@/utils/NotificationUtil'
import { ResponseData, TreeNode } from '@/Types/entitys'
import { ElMessageBox } from 'element-plus'
import StringUtil from '@/utils/StringUtil'
import { load, Store } from '@tauri-apps/plugin-store'
import { pdf_store_tem_key_prefix, pdf_store_tem_key, pdf_store_conten_key_prefix } from '@/setting'

// 常量定义
const INVALID_CHARS_REGEX = /.*[/\\:*?|].*/
const PURE_NUMBER_REGEX = /^\d+$/
const MAX_FILENAME_LENGTH = 255

let store: Store | undefined = undefined
const router = useRouter()
const route = useRoute()
const { routerNoteStore, useDocumentManagerStore, appConfig } = useStore()

// 树数据排序
const tree_node = computed(() => sortTreeData(routerNoteStore.tree_node))

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

// ===== 文档/收藏/最近 =====
const favoriteFiles = computed(() => useDocumentManagerStore.favoriteFiles)
const recentFiles = computed(() => useDocumentManagerStore.recentFiles)

// 工作目录名称
const workspaceName = computed(() => {
    const dir = appConfig.getMdWorkingDirectory
    if (!dir) return '工作区'
    const parts = dir.replace(/\\/g, '/').split('/')
    return parts.filter(Boolean).pop() || '工作区'
})

// 分组展开状态
const expandedSections = ref<Set<string>>(new Set(['workspace']))

function toggleSection(key: string) {
    if (expandedSections.value.has(key)) {
        expandedSections.value.delete(key)
    } else {
        expandedSections.value.add(key)
    }
}

// 收藏文件名（取最后一级）
function favoriteName(path: string): string {
    return path.replace(/\\/g, '/').split('/').filter(Boolean).pop() || path
}

// 点击收藏文件
function handleFavoriteClick(path: string) {
    useDocumentManagerStore.openDocument(path)
}

// 移除收藏
function handleRemoveFavorite(path: string) {
    useDocumentManagerStore.toggleFavorite(path)
}

// 点击最近文件
function handleRecentClick(path: string) {
    useDocumentManagerStore.openDocument(path)
}

// 清除最近列表
function handleClearRecent() {
    useDocumentManagerStore.clearRecentFiles()
}

// ===== Autoscroll 联动 =====
const fileTreeRef = ref<InstanceType<typeof FileInfoTree> | null>(null)

watch(() => useDocumentManagerStore.activeDocumentPath, (newPath) => {
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
    // 加载收藏和最近打开
    useDocumentManagerStore.loadFavorites()
    useDocumentManagerStore.loadRecentFiles()
    // 确保文件树数据已加载
    if (routerNoteStore.tree_node.length === 0) {
        routerNoteStore.setRoutes()
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
        useDocumentManagerStore.openDocument(node.filePath)
    }
}

const handleRefresh = () => { routerNoteStore.setRoutes() }

const handleAddFile = async (parent: TreeNode, name: string) => {
    if (isLoading.value) return
    const validation = validateName(name, true)
    if (!validation.valid) { NotificationUtil.error(validation.message!); return }
    const exists = await checkNameExists(parent.folder, name)
    if (exists) { NotificationUtil.error('已有相同文件名，请检查！'); return }
    try {
        isLoading.value = true
        const result = await tauri_request<ResponseData<string>>('create_note_file', { fatherPath: parent.folder, newNoteMdFileName: name })
        if (result?.code === 200) { NotificationUtil.success('笔记创建成功'); routerNoteStore.setRoutes() }
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
        if (result?.code === 200) { NotificationUtil.success('文件夹创建成功'); routerNoteStore.setRoutes() }
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
        routerNoteStore.setRoutes()
        if (route.path === node.routerPath && node.isfile) await processNodeCache(node)
    } catch (error: any) {
        if (error !== 'cancel') {
            console.error('删除操作失败:', error)
            NotificationUtil.error(`操作失败：${error.message}`)
        }
    }
}

const deleteFile = async (node: TreeNode) => {
    await ElMessageBox({
        title: '删除文件',
        message: h('div', {}, [h('br'), h('br'), h('p', { style: 'color:red;' }, `确定删除文件 "${node.label}" 吗？`), h('br'), h('br')]),
        showCancelButton: true, confirmButtonText: '删除', cancelButtonText: '取消',
    })
    try {
        isLoading.value = true
        const result = await tauri_request<ResponseData<string>>('remove_file', { path: node.filePath })
        if (result?.code === 200) NotificationUtil.success('删除文件成功！')
        else throw new Error(result?.msg || '未知错误')
    } catch (error: any) { if (error !== 'cancel') throw error }
    finally { isLoading.value = false }
}

const deleteFolder = async (node: TreeNode) => {
    if (StringUtil.isNullOrEmpty(node.folder) && node.isFolder) { NotificationUtil.error('请选择文件夹！'); return }
    await ElMessageBox({
        title: '删除文件夹',
        message: h('div', {}, [h('br'), h('br'), h('p', { style: 'color:red;' }, `删除文件夹 "${node.folder}" 吗，文件夹下笔记文件也将一并删除！`), h('br'), h('br')]),
        showCancelButton: true, confirmButtonText: '删除', cancelButtonText: '取消',
    })
    try {
        isLoading.value = true
        const children: TreeNode[] = []
        getAllChildrenFile(node, children)
        const cacheResults = await Promise.all(children.map(item => processNodeCache(item)))
        if (cacheResults.some(result => !result)) { NotificationUtil.error('文件缓存处理失败！'); return }
        const result = await tauri_request<ResponseData<string>>('remove_folder', { path: node.folder })
        if (result?.code === 200) NotificationUtil.success('操作成功')
        else throw new Error(result?.msg || '未知错误')
    } catch (error: any) { if (error !== 'cancel') throw error }
    finally { isLoading.value = false }
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
        routerNoteStore.setRoutes()
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
function handleCloseAllEditors() { useDocumentManagerStore.closeAllDocuments() }

function handleHeaderCommand(cmd: string) {
    switch (cmd) {
        case 'closeAll': handleCloseAllEditors(); break
        case 'refresh': handleRefresh(); break
        case 'clearRecent': handleClearRecent(); break
    }
}

function handleAddRootFile() { fileTreeRef.value?.addRootFile() }
function handleAddRootFolder() { fileTreeRef.value?.addRootFolder() }
function handleCollapseAll() { fileTreeRef.value?.collapseAll() }
function handleToggleSearch() { fileTreeRef.value?.setSearchVisible() }
</script>

<template>
    <div class="explorer-sidebar">
        <!-- 头部 -->
        <div class="explorer-header">
            <span class="explorer-title">资源管理器</span>
            <el-dropdown trigger="click" @command="handleHeaderCommand">
                <el-button size="small" link class="header-more-btn" title="更多操作">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <circle cx="4" cy="8" r="1.2" />
                        <circle cx="8" cy="8" r="1.2" />
                        <circle cx="12" cy="8" r="1.2" />
                    </svg>
                </el-button>
                <template #dropdown>
                    <el-dropdown-menu>
                        <el-dropdown-item command="closeAll">关闭所有编辑器</el-dropdown-item>
                        <el-dropdown-item command="clearRecent">清除最近打开</el-dropdown-item>
                        <el-dropdown-item command="refresh" divided>刷新资源管理器</el-dropdown-item>
                    </el-dropdown-menu>
                </template>
            </el-dropdown>
        </div>

        <!-- 面板内容 -->
        <div class="explorer-body">
            <!-- 收藏 -->
            <div class="section" v-if="favoriteFiles.length > 0">
                <div class="section-header" @click="toggleSection('favorites')">
                    <svg class="section-chevron" :class="{ expanded: expandedSections.has('favorites') }" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <path d="M6 4l4 4-4 4" />
                    </svg>
                    <span class="section-label">收藏</span>
                    <span class="section-count">{{ favoriteFiles.length }}</span>
                </div>
                <div class="section-content" v-show="expandedSections.has('favorites')">
                    <div
                        v-for="path in favoriteFiles"
                        :key="path"
                        class="favorite-item"
                        @click="handleFavoriteClick(path)"
                    >
                        <span class="favorite-star">★</span>
                        <span class="favorite-name" :title="path">{{ favoriteName(path) }}</span>
                        <span class="favorite-remove" @click.stop="handleRemoveFavorite(path)" title="取消收藏">×</span>
                    </div>
                </div>
            </div>

            <!-- 最近打开 -->
            <div class="section" v-if="recentFiles.length > 0">
                <div class="section-header" @click="toggleSection('recent')">
                    <svg class="section-chevron" :class="{ expanded: expandedSections.has('recent') }" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <path d="M6 4l4 4-4 4" />
                    </svg>
                    <span class="section-label">最近打开</span>
                    <span class="section-count">{{ recentFiles.length }}</span>
                </div>
                <div class="section-content" v-show="expandedSections.has('recent')">
                    <div
                        v-for="path in recentFiles"
                        :key="path"
                        class="recent-item"
                        @click="handleRecentClick(path)"
                    >
                        <span class="recent-icon">
                            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
                                <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="1.5" />
                                <path d="M8 4v4l3 2" fill="none" stroke="currentColor" stroke-width="1.5" />
                            </svg>
                        </span>
                        <span class="recent-name" :title="path">{{ favoriteName(path) }}</span>
                    </div>
                </div>
            </div>

            <!-- 工作区文件夹 -->
            <div class="section">
                <div class="section-header workspace-header" @click="toggleSection('workspace')">
                    <svg class="section-chevron" :class="{ expanded: expandedSections.has('workspace') }" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <path d="M6 4l4 4-4 4" />
                    </svg>
                    <span class="section-label workspace-label">{{ workspaceName }}</span>
                    <div class="workspace-toolbar">
                        <span class="toolbar-btn" @click.stop="handleToggleSearch" title="搜索文件">
                            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                                <circle cx="7" cy="7" r="4" fill="none" stroke="currentColor" stroke-width="1.3" />
                                <line x1="10" y1="10" x2="13.5" y2="13.5" stroke="currentColor" stroke-width="1.3" />
                            </svg>
                        </span>
                        <span class="toolbar-btn" @click.stop="handleAddRootFile" title="新建文件">
                            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                                <path d="M2 2h8l4 4v8H2V2z" fill="none" stroke="currentColor" stroke-width="1.2" />
                                <path d="M10 2v4h4" fill="none" stroke="currentColor" stroke-width="1.2" />
                                <line x1="8" y1="7" x2="8" y2="13" stroke="currentColor" stroke-width="1.2" />
                                <line x1="5" y1="10" x2="11" y2="10" stroke="currentColor" stroke-width="1.2" />
                            </svg>
                        </span>
                        <span class="toolbar-btn" @click.stop="handleAddRootFolder" title="新建文件夹">
                            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                                <path d="M1 4l2-2h4l2 2h5v8H1V4z" fill="none" stroke="currentColor" stroke-width="1.2" />
                                <line x1="8" y1="6" x2="8" y2="12" stroke="currentColor" stroke-width="1.2" />
                                <line x1="5" y1="9" x2="11" y2="9" stroke="currentColor" stroke-width="1.2" />
                            </svg>
                        </span>
                        <span class="toolbar-btn" @click.stop="handleRefresh" title="刷新">
                            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                                <path d="M13 8a5 5 0 1 1-1.5-3.5" fill="none" stroke="currentColor" stroke-width="1.3" />
                                <path d="M11.5 2v3h3" fill="none" stroke="currentColor" stroke-width="1.3" />
                            </svg>
                        </span>
                        <span class="toolbar-btn" @click.stop="handleCollapseAll" title="折叠全部文件夹">
                            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                                <rect x="2" y="2" width="12" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.2" />
                                <line x1="2" y1="6" x2="14" y2="6" stroke="currentColor" stroke-width="1.2" />
                                <line x1="5" y1="9" x2="11" y2="9" stroke="currentColor" stroke-width="1" />
                                <line x1="5" y1="11.5" x2="9" y2="11.5" stroke="currentColor" stroke-width="1" />
                            </svg>
                        </span>
                    </div>
                </div>
                <div class="section-content" v-show="expandedSections.has('workspace')">
                    <file-info-tree
                        ref="fileTreeRef"
                        :data="tree_node"
                        :loading="isLoading"
                        @node-click="handleNodeClick"
                        @refresh="handleRefresh"
                        @add-file="handleAddFile"
                        @add-folder="handleAddFolder"
                        @delete-node="handleDeleteNode"
                        @rename-node="handleRenameNode"
                    />
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.explorer-sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background-color: #1e1e1e;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;

    @include theme.useTheme {
        background-color: theme.getVar('bgColor');
    }
}

// ===== 头部 =====
.explorer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 12px;
    height: 35px;
    flex-shrink: 0;
}

.explorer-title {
    font-size: 11px;
    font-weight: 600;
    color: #bbb;
    text-transform: uppercase;
    letter-spacing: 1px;

    @include theme.useTheme {
        color: theme.getVar('textColor');
    }
}

.header-more-btn {
    color: #888;
    padding: 2px 4px;
    border-radius: 4px;

    &:hover {
        color: #ccc;
        background-color: rgba(255, 255, 255, 0.08);
    }
}

// ===== 主体 =====
.explorer-body {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;

    &::-webkit-scrollbar { width: 6px; }
    &::-webkit-scrollbar-thumb {
        background-color: rgba(155, 155, 155, 0.2);
        border-radius: 3px;
        &:hover { background-color: rgba(155, 155, 155, 0.4); }
    }
    &::-webkit-scrollbar-track { background: transparent; }
}

// ===== 分组 =====
.section {
    margin-bottom: 0;
}

.section-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    cursor: pointer;
    user-select: none;
    font-size: 11px;
    font-weight: 600;
    color: #bbb;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: background-color 0.1s;

    &:hover {
        background-color: rgba(255, 255, 255, 0.05);
    }

    @include theme.useTheme {
        color: theme.getVar('textColor');
    }
}

.section-chevron {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
    color: #888;
    transition: transform 0.15s ease;

    &.expanded {
        transform: rotate(90deg);
    }
}

.section-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.workspace-label {
    text-transform: none;
    font-weight: 500;
    letter-spacing: 0;
    font-size: 12px;
}

// 工作区 header：hover 时显示工具栏
.workspace-header {
    position: relative;

    .workspace-toolbar {
        display: flex;
        align-items: center;
        gap: 2px;
        opacity: 0;
        transition: opacity 0.15s;
        flex-shrink: 0;
    }

    &:hover .workspace-toolbar {
        opacity: 1;
    }
}

.section-count {
    font-size: 10px;
    font-weight: 400;
    color: #666;
    background-color: rgba(255, 255, 255, 0.06);
    padding: 0 5px;
    border-radius: 8px;
    flex-shrink: 0;
}

.section-content {
    padding-left: 0;
}

.section-empty {
    padding: 6px 12px 6px 28px;
    font-size: 12px;
    color: #555;
    font-style: italic;
}



// ===== 收藏列表 =====
.favorite-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px 3px 20px;
    cursor: pointer;
    transition: background-color 0.1s;
    font-size: 13px;

    &:hover {
        background-color: rgba(255, 255, 255, 0.06);

        .favorite-remove {
            opacity: 1;
        }
    }
}

.favorite-star {
    color: #e5c07b;
    font-size: 12px;
    flex-shrink: 0;
}

.favorite-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #ccc;

    @include theme.useTheme {
        color: theme.getVar('textColor');
    }
}

.favorite-remove {
    opacity: 0;
    color: #888;
    font-size: 14px;
    flex-shrink: 0;
    transition: all 0.1s;
    padding: 0 2px;

    &:hover {
        color: #e06c75;
    }
}

// ===== 最近打开列表 =====
.recent-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px 3px 20px;
    cursor: pointer;
    transition: background-color 0.1s;
    font-size: 13px;

    &:hover {
        background-color: rgba(255, 255, 255, 0.06);
    }
}

.recent-icon {
    flex-shrink: 0;
    color: #666;
    display: flex;
    align-items: center;
}

.recent-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #999;

    @include theme.useTheme {
        color: theme.getVar('textColor');
    }
}

// ===== 工具栏按钮 =====
.toolbar-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 4px;
    color: #888;
    cursor: pointer;
    transition: all 0.1s;

    &:hover {
        color: #ccc;
        background-color: rgba(255, 255, 255, 0.1);
    }
}
</style>
