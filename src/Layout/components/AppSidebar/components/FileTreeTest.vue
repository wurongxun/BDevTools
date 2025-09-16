<template>
    <file-info-tree :data="tree_node" @node-click="handleNodeClick" @refresh="handleRefresh" @add-file="handleAddFile"
        @add-folder="handleAddFolder" @delete-node="handleDeleteNode" @rename-node="handleRenameNode" />
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import FileInfoTree from '@/components/FileTree/FileInfoTree.vue'
import { ref } from 'vue'
import useStore from '@/store'
import { useRouter } from 'vue-router'

const router = useRouter()
const { routeStore, system, routerNoteStore } = useStore()

const childRouteNote: any = computed(() =>
    routerNoteStore.routes
)
const tree_node: any = computed(() =>
    routerNoteStore.tree_node
)
const treeData = ref([
    {
        id: '1',
        label: '根目录',
        isFolder: true,
        children: [
            {
                id: '2',
                label: '文档',
                isFolder: true,
                children: [
                    {
                        id: '3',
                        label: '介绍.md',
                        isFolder: false,
                        type: 'markdown'
                    },
                    {
                        id: '4',
                        label: '手册.pdf',
                        isFolder: false,
                        type: 'pdf'
                    }
                ]
            },
            {
                id: '5',
                label: '资源',
                isFolder: true,
                children: [
                    {
                        id: '6',
                        label: '测试',
                        isFolder: true,
                        children: []
                    }
                ]
            }
        ]
    },
    {
        id: '1',
        label: '根目录',
        isFolder: true,
        children: [
            {
                id: '2',
                label: '文档',
                isFolder: true,
                children: [
                    {
                        id: '3',
                        label: '介绍.md',
                        isFolder: false,
                        type: 'markdown'
                    },
                    {
                        id: '4',
                        label: '手册.pdf',
                        isFolder: false,
                        type: 'pdf'
                    }
                ]
            },
            {
                id: '5',
                label: '资源',
                isFolder: true,
                children: [
                    {
                        id: '6',
                        label: '测试',
                        isFolder: true,
                        children: []
                    }
                ]
            }
        ]
    }
])

const handleNodeClick = (node) => {
    if (!node.isFolder) {
        router.push({
            path: node.path
        })
    }
}

const handleRefresh = () => {
    console.log('刷新树数据')
    // 重新加载数据逻辑
    const routerNote = routerNoteStore.setRoutes();
}

const handleAddFile = (parent, name) => {
    console.log(`在 ${parent.label} 下创建文件: ${name}`)
    // 添加文件逻辑
}

const handleAddFolder = (parent, name) => {
    console.log(`在 ${parent.label} 下创建文件夹: ${name}`)
    // 添加文件夹逻辑
}

const handleDeleteNode = (node) => {
    console.log('删除节点:', node)
    // 删除节点逻辑
}

const handleRenameNode = (node, newName) => {
    console.log(`重命名节点 ${node.label} 为 ${newName}`)
    // 重命名逻辑
}
</script>