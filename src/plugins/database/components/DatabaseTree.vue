<template>
  <div class="database-tree">
    <el-tree
      :data="treeData"
      :props="treeProps"
      node-key="id"
      :load="loadNode"
      lazy
      highlight-current
      :key="treeKey"
      @node-click="handleNodeClick"
      @node-contextmenu="handleContextMenu"
    >
      <template #default="{ node, data }">
        <span class="tree-node">
          <el-icon v-if="data.type === 'connection'" :class="{ 'is-connected': isConnected(data.data?.id) }">
            <Connection />
          </el-icon>
          <el-icon v-else-if="data.type === 'database'"><Coin /></el-icon>
          <el-icon v-else-if="data.type === 'table'"><Grid /></el-icon>
          <el-icon v-else-if="data.type === 'column'"><Document /></el-icon>
          <span class="node-label">{{ node.label }}</span>
          <span v-if="data.type === 'connection' && isConnected(data.data?.id)" class="connected-badge">已连接</span>
        </span>
      </template>
    </el-tree>

    <!-- 右键菜单 -->
    <el-dropdown
      v-if="contextMenuVisible"
      ref="dropdownRef"
      trigger="contextmenu"
      :style="{ position: 'fixed', left: contextMenuX + 'px', top: contextMenuY + 'px' }"
      @visible-change="onDropdownChange"
    >
      <el-dropdown-menu>
        <el-dropdown-item v-if="contextNodeType === 'connection'" @click="handleRefresh">刷新</el-dropdown-item>
        <el-dropdown-item v-if="contextNodeType === 'connection'" @click="handleNewQuery">新建查询</el-dropdown-item>
        <el-dropdown-item v-if="contextNodeType === 'table'" @click="handleViewTable">查看表数据</el-dropdown-item>
        <el-dropdown-item v-if="contextNodeType === 'table'" @click="handleViewStructure">查看表结构</el-dropdown-item>
        <el-dropdown-item v-if="contextNodeType === 'table'" @click="handleExportTable">导出数据</el-dropdown-item>
      </el-dropdown-menu>
    </el-dropdown>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Connection, Coin, Grid, Document } from '@element-plus/icons-vue'
import { useConnectionStore, useEditorStore } from '../store'
import { dbListDatabases, dbListTables, dbTableColumns } from '../api/metadata'
import type { TreeNode } from '../types'

const connectionStore = useConnectionStore()
const editorStore = useEditorStore()

const treeData = ref<TreeNode[]>([])
const treeKey = ref(0)
const contextMenuVisible = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextNodeType = ref<string>('')
const contextNodeData = ref<any>(null)
let lastClickTime = 0
let lastClickData: TreeNode | null = null

const treeProps = {
  children: 'children',
  label: 'label',
  isLeaf: 'isLeaf',
}

onMounted(() => {
  refreshTree()
})

function refreshTree() {
  treeKey.value++
  treeData.value = connectionStore.connections.map(conn => ({
    id: `conn_${conn.id}`,
    label: conn.name,
    type: 'connection' as const,
    isLeaf: false,
    data: conn,
  }))
}

function isConnected(configId?: string): boolean {
  return configId ? connectionStore.isConnected(configId) : false
}

async function loadNode(node: any, resolve: (data: TreeNode[]) => void) {
  if (node.level === 0) {
    resolve(treeData.value)
    return
  }

  const nodeData = node.data as TreeNode

  if (nodeData.type === 'connection') {
    const connId = nodeData.data?.id
    if (!connId) { resolve([]); return }

    if (!connectionStore.isConnected(connId)) {
      try {
        const res = await connectionStore.connect(connId)
        if (res.code !== 200) {
          resolve([])
          return
        }
      } catch { resolve([]); return }
    }

    try {
      const res = await dbListDatabases(connId)
      if (res.code === 200) {
        resolve(res.data.map(db => ({
          id: `db_${connId}_${db.name}`,
          label: db.name,
          type: 'database' as const,
          isLeaf: false,
          data: { ...db, configId: connId },
        })))
      } else {
        console.warn('[DatabaseTree] dbListDatabases failed:', res.msg)
        resolve([])
      }
    } catch (e) { console.error('[DatabaseTree] load databases error:', e); resolve([]) }
  } else if (nodeData.type === 'database') {
    const connId = nodeData.data?.configId
    const dbName = nodeData.data?.name
    if (!connId || !dbName) { resolve([]); return }

    try {
      const res = await dbListTables(connId, dbName)
      if (res.code === 200) {
        resolve(res.data.map(tbl => ({
          id: `tbl_${connId}_${dbName}_${tbl.name}`,
          label: tbl.name,
          type: 'table' as const,
          isLeaf: false,
          data: { ...tbl, configId: connId, database: dbName },
        })))
      } else {
        console.warn('[DatabaseTree] dbListTables failed:', res.msg)
        resolve([])
      }
    } catch (e) { console.error('[DatabaseTree] load tables error:', e); resolve([]) }
  } else if (nodeData.type === 'table') {
    const connId = nodeData.data?.configId
    const dbName = nodeData.data?.database
    const tableName = nodeData.data?.name
    if (!connId || !dbName || !tableName) { resolve([]); return }

    try {
      const res = await dbTableColumns(connId, dbName, tableName)
      if (res.code === 200) {
        resolve(res.data.map(col => ({
          id: `col_${connId}_${dbName}_${tableName}_${col.name}`,
          label: `${col.name} (${col.data_type})`,
          type: 'column' as const,
          isLeaf: true,
          data: { ...col, configId: connId, database: dbName, table: tableName },
        })))
      } else {
        console.warn('[DatabaseTree] dbTableColumns failed:', res.msg)
        resolve([])
      }
    } catch (e) { console.error('[DatabaseTree] load columns error:', e); resolve([]) }
  } else {
    resolve([])
  }
}

function handleNodeClick(data: TreeNode) {
  connectionStore.selectedConnectionId = data.data?.id || ''
  // 双击检测：300ms 内连续点击同一节点视为双击
  const now = Date.now()
  if (now - lastClickTime < 300 && lastClickData?.id === data.id) {
    handleDblClick(data)
    lastClickTime = 0 // 重置，防止三击触发
  } else {
    lastClickTime = now
    lastClickData = data
  }
}

function handleDblClick(data: TreeNode) {
  if (data.type === 'table' && data.data) {
    const d = data.data
    editorStore.openTab({
      title: d.name,
      type: 'table',
      configId: d.configId,
      database: d.database,
      table: d.name,
    })
  } else if (data.type === 'database' && data.data) {
    const d = data.data
    editorStore.openTab({
      title: '新查询',
      type: 'sql',
      configId: d.configId,
      content: '',
    })
  }
}

function handleContextMenu(event: MouseEvent, data: TreeNode) {
  event.preventDefault()
  contextMenuVisible.value = true
  contextMenuX.value = event.clientX
  contextMenuY.value = event.clientY
  contextNodeType.value = data.type
  contextNodeData.value = data.data
}

function onDropdownChange(visible: boolean) {
  if (!visible) contextMenuVisible.value = false
}

function handleRefresh() {
  refreshTree()
}

function handleNewQuery() {
  if (contextNodeData.value?.id) {
    editorStore.openTab({
      title: '新查询',
      type: 'sql',
      configId: contextNodeData.value.id,
      content: '',
    })
  }
}

function handleViewTable() {
  const d = contextNodeData.value
  if (d) {
    editorStore.openTab({
      title: d.name || d.table,
      type: 'table',
      configId: d.configId,
      database: d.database,
      table: d.name || d.table,
    })
  }
}

function handleViewStructure() {
  const d = contextNodeData.value
  if (d) {
    editorStore.openTab({
      title: `${d.name || d.table} - 结构`,
      type: 'metadata',
      configId: d.configId,
      database: d.database,
      table: d.name || d.table,
    })
  }
}

function handleExportTable() {
  // 通过事件通知 MainPanel 打开导出对话框
  const d = contextNodeData.value
  if (d) {
    editorStore.openTab({
      title: `导出 ${d.name || d.table}`,
      type: 'table',
      configId: d.configId,
      database: d.database,
      table: d.name || d.table,
    })
  }
}

defineExpose({ refreshTree })
</script>

<style scoped>
.database-tree {
  height: 100%;
  overflow: auto;
}
.tree-node {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 13px;
}
.node-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.connected-badge {
  font-size: 10px;
  color: #67c23a;
  margin-left: 4px;
}
.is-connected {
  color: #67c23a;
}
</style>
