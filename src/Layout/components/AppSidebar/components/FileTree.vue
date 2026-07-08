<script setup lang="ts">
import { computed, h, onMounted, onUnmounted, ref } from 'vue'
import FileInfoTree from '@/components/FileTree/FileInfoTree.vue'
import useStore from '@/store'
import { useRouter, useRoute } from 'vue-router'
import { tauri_request } from '@/utils/tauri_request'
import NotificationUtil from '@/utils/NotificationUtil'
import { ResponseData, TreeNode } from '@/Types/entitys'
import { ElMessageBox } from 'element-plus'
import StringUtil from '@/utils/StringUtil'
import { load, Store } from '@tauri-apps/plugin-store'
import { pdf_store_tem_key_prefix, pdf_store_tem_key, pdf_store_conten_key_prefix } from '@/setting'
import { ITagItem } from '@/Types/TagView'

// 常量定义
const INVALID_CHARS_REGEX = /.*[/\\:*?|].*/
const PURE_NUMBER_REGEX = /^\d+$/
const MAX_FILENAME_LENGTH = 255

let store: Store | undefined = undefined
const router = useRouter()
const route = useRoute()
const { routerNoteStore, useDocumentManagerStore } = useStore()

// 优化：缓存排序后的树数据，避免每次渲染都重新计算
const tree_node = computed(() => sortTreeData(routerNoteStore.tree_node))

// 加载状态
const isLoading = ref(false)

// 排序函数：文件夹在前，文件在后
const sortTreeData = (data: TreeNode[]): TreeNode[] => {
    if (!data || data.length === 0) return []

    return data.sort((a, b) => {
        // 文件夹排在前面
        if (a.isFolder !== b.isFolder) {
            return a.isFolder ? -1 : 1
        }
        // 同类型按字母顺序排序
        return a.label.localeCompare(b.label)
    }).map(node => {
        // 递归处理子节点
        if (node.children && node.children.length > 0) {
            node.children = sortTreeData(node.children)
        }
        return node
    })
}

// 初始化 store
onMounted(async () => {
    if (!store) {
        store = await load(pdf_store_tem_key, {
            autoSave: false,
            defaults: {}
        })
    }
})

// 组件卸载时清理资源
onUnmounted(() => {
    if (store) {
        store = undefined
    }
})

// 验证文件名/文件夹名
const validateName = (name: string, isFile = true): { valid: boolean; message?: string } => {
    if (!name || name.trim() === '') {
        return {
            valid: false,
            message: isFile ? '请填写文件名称！' : '请填写文件夹名称！'
        }
    }

    if (INVALID_CHARS_REGEX.test(name)) {
        return {
            valid: false,
            message: isFile
                ? '笔记名称命名不规范，不能包含\\ / : * ? " < > |请检查！'
                : '文件夹命名不规范，不能包含\\ / : * ? " < > |请检查！'
        }
    }

    if (PURE_NUMBER_REGEX.test(name)) {
        return {
            valid: false,
            message: isFile ? '笔记名称命名不规范，不能为纯数字！' : '文件夹命名不规范，不能为纯数字！'
        }
    }

    if (name.length > MAX_FILENAME_LENGTH) {
        return {
            valid: false,
            message: `名称长度不能超过 ${MAX_FILENAME_LENGTH} 个字符！`
        }
    }

    return { valid: true }
}

// 检查名称是否已存在
const checkNameExists = async (parentPath: string, name: string): Promise<boolean> => {
    try {
        const result = await tauri_request<ResponseData<string[]>>('get_path_childrenFoldersAndFiles', {
            path: parentPath
        })
        return result.data?.includes(name) || false
    } catch (error) {
        console.error('检查名称是否存在失败:', error)
        return false
    }
}

// 节点点击处理
const handleNodeClick = (node: TreeNode) => {
    if (!node.isFolder && node.filePath) {
        useDocumentManagerStore.openDocument(node.filePath)
    }
}

// 刷新树数据
const handleRefresh = () => {
    console.log('刷新树数据')
    routerNoteStore.setRoutes()
}

// 创建文件
const handleAddFile = async (parent: TreeNode, name: string) => {
    if (isLoading.value) return

    // 验证文件名
    const validation = validateName(name, true)
    if (!validation.valid) {
        NotificationUtil.error(validation.message!)
        return
    }

    // 检查名称是否已存在
    const exists = await checkNameExists(parent.folder, name)
    if (exists) {
        NotificationUtil.error('已有相同文件名，请检查！')
        return
    }

    try {
        isLoading.value = true

        const result = await tauri_request<ResponseData<string>>('create_note_file', {
            fatherPath: parent.folder,
            newNoteMdFileName: name
        })

        if (result?.code === 200) {
            NotificationUtil.success('笔记创建成功')
            routerNoteStore.setRoutes()
        } else {
            throw new Error(result?.msg || '未知错误')
        }
    } catch (error: any) {
        console.error('创建文件失败:', error)
        NotificationUtil.error(`笔记创建失败：${error.message}`)
    } finally {
        isLoading.value = false
    }
}

// 创建文件夹
const handleAddFolder = async (parent: TreeNode, name: string) => {
    if (isLoading.value) return

    // 验证文件夹名
    const validation = validateName(name, false)
    if (!validation.valid) {
        NotificationUtil.error(validation.message!)
        return
    }

    // 检查名称是否已存在
    const exists = await checkNameExists(parent.folder, name)
    if (exists) {
        NotificationUtil.error('已有相同文件夹名或者文件名，请检查！')
        return
    }

    try {
        isLoading.value = true

        const result = await tauri_request<ResponseData<string>>('create_folder', {
            fatherPath: parent.folder,
            newFolderName: name
        })

        if (result?.code === 200) {
            NotificationUtil.success('文件夹创建成功')
            routerNoteStore.setRoutes()
        } else {
            throw new Error(result?.msg || '未知错误')
        }
    } catch (error: any) {
        console.error('创建文件夹失败:', error)
        NotificationUtil.error(`笔记本创建失败：${error.message}`)
    } finally {
        isLoading.value = false
    }
}

// 删除节点
const handleDeleteNode = async (node: TreeNode) => {
    if (isLoading.value) return

    try {
        if (node.isfile) {
            await deleteFile(node)
        } else {
            await deleteFolder(node)
        }

        routerNoteStore.setRoutes()

        // 如果删除的是当前激活的文件，需要清理缓存
        if (route.path === node.routerPath && node.isfile) {
            await processNodeCache(node)
        }
    } catch (error: any) {
        console.error('删除操作失败:', error)
        NotificationUtil.error(`操作失败：${error.message}`)
    }
}

// 删除文件
const deleteFile = async (node: TreeNode) => {
    await ElMessageBox({
        title: '删除文件',
        message: h('div', {}, [
            h('br'),
            h('br'),
            h('p', { style: 'color:red;' }, `确定删除文件 "${node.label}" 吗？`),
            h('br'),
            h('br'),
        ]),
        showCancelButton: true,
        confirmButtonText: '删除',
        cancelButtonText: '取消',
    })

    try {
        isLoading.value = true

        const result = await tauri_request<ResponseData<string>>('remove_file', {
            path: node.filePath
        })

        if (result?.code === 200) {
            NotificationUtil.success('删除文件成功！')
        } else {
            throw new Error(result?.msg || '未知错误')
        }
    } catch (error: any) {
        if (error !== 'cancel') {
            throw error
        }
    } finally {
        isLoading.value = false
    }
}

// 删除文件夹
const deleteFolder = async (node: TreeNode) => {
    if (StringUtil.isNullOrEmpty(node.folder) && node.isFolder) {
        NotificationUtil.error('请选择文件夹！')
        return
    }

    await ElMessageBox({
        title: '删除文件夹',
        message: h('div', {}, [
            h('br'),
            h('br'),
            h(
                'p',
                { style: 'color:red;' },
                `删除文件夹 "${node.folder}" 吗，文件夹下笔记文件也将一并删除！`
            ),
            h('br'),
            h('br'),
        ]),
        showCancelButton: true,
        confirmButtonText: '删除',
        cancelButtonText: '取消',
    })

    try {
        isLoading.value = true

        // 查找文件夹下所有文件并清理缓存
        const children: TreeNode[] = []
        getAllChildrenFile(node, children)

        const cacheResults = await Promise.all(children.map(item => processNodeCache(item)))

        if (cacheResults.some(result => !result)) {
            NotificationUtil.error('文件缓存处理失败！')
            return
        }

        const result = await tauri_request<ResponseData<string>>('remove_folder', {
            path: node.folder
        })

        if (result?.code === 200) {
            NotificationUtil.success('操作成功')
        } else {
            throw new Error(result?.msg || '未知错误')
        }
    } catch (error: any) {
        if (error !== 'cancel') {
            throw error
        }
    } finally {
        isLoading.value = false
    }
}

// 重命名节点
const handleRenameNode = async (node: TreeNode, newName: string) => {
    if (isLoading.value) return

    // 确保 store 实例存在
    if (!store) {
        store = await load(pdf_store_tem_key, {
            autoSave: false,
            defaults: {}
        })
    }

    // 验证新名称
    const validation = validateName(newName, node.isfile)
    if (!validation.valid) {
        NotificationUtil.error(validation.message!)
        return
    }

    // 检查名称是否已存在（排除自身）
    const parentPath = node.isfile
        ? node.filePath.substring(0, node.filePath.lastIndexOf('/'))
        : node.folder.substring(0, node.folder.lastIndexOf('/'))

    const exists = await checkNameExists(parentPath, newName)
    if (exists) {
        NotificationUtil.error(`已有相同${node.isfile ? '文件' : '文件夹'}名，请检查！`)
        return
    }

    try {
        isLoading.value = true

        if (node.isfile) {
            await renameFile(node, newName)
        } else {
            await renameFolder(node, newName)
        }

        routerNoteStore.setRoutes()
    } catch (error: any) {
        console.error('重命名失败:', error)
        NotificationUtil.error(`操作失败：${error.message}`)
    } finally {
        isLoading.value = false
    }
}

// 重命名文件
const renameFile = async (node: TreeNode, newName: string) => {
    // 处理文件缓存
    const cacheProcessed = await processNodeCache(node)
    if (!cacheProcessed) {
        throw new Error('文件缓存处理失败')
    }

    const result = await tauri_request<ResponseData<string>>('up_file_name', {
        path: node.filePath,
        upFileName: newName
    })

    if (result?.code === 200) {
        NotificationUtil.success('修改文件名成功！')
    } else {
        throw new Error(result?.msg || '未知错误')
    }
}

// 重命名文件夹
const renameFolder = async (node: TreeNode, newName: string) => {
    // 查找文件夹下所有文件并清理缓存
    const children: TreeNode[] = []
    getAllChildrenFile(node, children)

    const cacheResults = await Promise.all(children.map(item => processNodeCache(item)))

    if (cacheResults.some(result => !result)) {
        throw new Error('文件缓存处理失败')
    }

    const result = await tauri_request<ResponseData<string>>('up_folder_name', {
        path: node.folder,
        upFolderName: newName
    })

    if (result?.code === 200) {
        NotificationUtil.success('修改文件夹名成功！')
    } else {
        throw new Error(result?.msg || '未知错误')
    }
}

// 获取所有子文件
const getAllChildrenFile = (node: TreeNode, list: TreeNode[]) => {
    if (node.isfile) {
        list.push(node)
    } else if (node.children) {
        node.children.forEach(child => {
            getAllChildrenFile(child, list)
        })
    }
}

// 处理节点缓存
const processNodeCache = async (item: TreeNode): Promise<boolean> => {
    try {
        if (!store) {
            console.warn('Store 实例不存在')
            return true
        }

        const keyTem = pdf_store_tem_key_prefix + item.id
        const keyConten = pdf_store_conten_key_prefix + item.id

        const temVal = await store.get<{ value: string }>(keyTem)

        // 如果没有缓存，无需处理
        if (!temVal || !temVal.value) {
            return true
        }

        const cachedContent = temVal.value
        const contentVal = await store.get<{ value: string }>(keyConten)
        const content = contentVal?.value || null

        // 如果有未保存的更改，先保存
        if (cachedContent && content !== cachedContent) {
            const saveResult = await tauri_request<ResponseData<string[]>>('save_note', {
                path: item.filePath,
                mdText: cachedContent
            })

            if (saveResult?.code !== 200) {
                throw new Error(saveResult?.msg || '文件保存失败')
            }
        }

        // 删除缓存
        const [deleteTem, deleteConten] = await Promise.all([
            store.delete(keyTem),
            store.delete(keyConten)
        ])

        if (!deleteTem || !deleteConten) {
            console.warn('缓存删除不完整:', keyTem, keyConten)
        }

        return true
    } catch (error: any) {
        console.error('处理节点缓存失败:', error)
        NotificationUtil.error(`操作失败：${error.message}`)
        return false
    }
}
</script>

<template>
    <file-info-tree
        :data="tree_node"
        :loading="isLoading"
        @node-click="handleNodeClick"
        @refresh="handleRefresh"
        @add-file="handleAddFile"
        @add-folder="handleAddFolder"
        @delete-node="handleDeleteNode"
        @rename-node="handleRenameNode"
    />
</template>
