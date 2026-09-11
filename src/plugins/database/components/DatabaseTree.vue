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
    >
      <template #default="{ node, data }">
        <span class="tree-node" @contextmenu.prevent="onTreeNodeContextmenu($event, data)">
          <el-icon
            v-if="data.type === 'connection'"
            :class="{ 'is-connected': isConnected(data.data?.id), 'is-disconnected': !isConnected(data.data?.id) }"
          >
            <Connection />
          </el-icon>
          <el-icon v-else-if="data.type === 'database'"><Coin /></el-icon>
          <el-icon
            v-else-if="data.type === 'table'"
            :class="{ 'is-connected': isTableActive(data) }"
          >
            <Grid />
          </el-icon>
          <el-icon v-else-if="data.type === 'column'"><Document /></el-icon>
          <span
            class="node-label"
            :class="{ 'is-connected': data.type === 'table' && isTableActive(data) }"
          >{{ node.label }}</span>
          <span v-if="data.type === 'connection' && isConnected(data.data?.id)" class="connected-badge">已连接</span>
        </span>
      </template>
    </el-tree>

    <!-- 右键菜单 -->
    <teleport to="body">
      <div
        v-if="contextMenuVisible"
        class="context-menu"
        :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }"
        @click.stop
      >
        <div v-if="contextNodeType === 'connection'" class="context-menu-item" @click="handleRefresh">刷新</div>
        <div v-if="contextNodeType === 'connection'" class="context-menu-item" @click="handleNewQuery">新建查询</div>
        <div v-if="contextNodeType === 'table'" class="context-menu-item" @click="handleViewTable">查看表数据</div>
        <div v-if="contextNodeType === 'table'" class="context-menu-item" @click="handleViewStructure">查看表结构</div>
        <div v-if="contextNodeType === 'table'" class="context-menu-item" @click="handleExportTable">导出数据</div>
      </div>
    </teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { Connection, Coin, Grid, Document } from '@element-plus/icons-vue'
import { useConnectionStore, useEditorStore, useDbSidebarStore } from '../store'
import { dbListDatabases, dbListTables, dbTableColumns } from '../api/metadata'
import type { TreeNode } from '../types'

const connectionStore = useConnectionStore()
const editorStore = useEditorStore()
const dbSidebar = useDbSidebarStore()

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

/** 生成 el-tree node-key：按段编码（名称可含 `_`/`|` 等），避免拼接碰撞导致下划线表名/库名节点不渲染 */
function nodeId(prefix: string, parts: (string | undefined)[]): string {
  return [prefix, ...parts.map(p => encodeURIComponent(p ?? ''))].join('|')
}

onMounted(() => {
  refreshTree()
})

// 连接列表变化（含异步加载完成/工作区切换）时自动刷新树，
// 修复打开侧边栏时 connections 尚未加载完成导致列表为空的问题
watch(
  () => connectionStore.connections,
  () => refreshTree()
)

// 跟踪「当前有活跃 tab 的连接」；当某连接的所有相关 tab 全部关闭时，
// 主动断开后端连接池（activeConnections 中移除 + 后端 dbDisconnect），
// 让挂起的 loadNode（el-tree lazy loading）因连接断开而失败 → resolve([]) → loading 图标消失。
const watchedActiveConfigs = new Set<string>()
function syncConnectionPoolOnTabsChange() {
  for (const conn of connectionStore.connections) {
    if (!conn.id) continue
    const hasTabs = editorStore.hasTabsForConfig(conn.id)
    if (hasTabs) {
      watchedActiveConfigs.add(conn.id)
    } else if (watchedActiveConfigs.has(conn.id)) {
      // 之前有 tab 现在没了：断开后端连接
      connectionStore.disconnect(conn.id).catch(() => {})
      watchedActiveConfigs.delete(conn.id)
    }
  }
}
watch(
  () => editorStore.tabs.map(t => t.configId).filter(Boolean).sort().join('|'),
  () => syncConnectionPoolOnTabsChange(),
  { immediate: true }
)

function refreshTree() {
  treeKey.value++
  treeData.value = connectionStore.connections.map(conn => ({
    id: nodeId('conn', [conn.id]),
    label: conn.name,
    type: 'connection' as const,
    isLeaf: false,
    data: conn,
  }))
}

/** 连接节点图标状态：后端连接成功即绿（与 tab 无关）；断开后灰显 */
function isConnected(configId?: string): boolean {
  return configId ? connectionStore.isConnected(configId) : false
}

/** 表节点图标/表名状态：该表有打开的 Tab（表数据/表结构）时表名变绿，全部关闭后变灰 */
function isTableActive(node: TreeNode): boolean {
  try {
    const d = node?.data
    if (!node || node.type !== 'table' || !d) return false
    return !!editorStore.hasTabForTable(d.configId, d.database, d.name)
  } catch {
    // 渲染期异常兜底：不因状态判断影响树节点显示
    return false
  }
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
          id: nodeId('db', [connId, db.name]),
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
          id: nodeId('tbl', [connId, dbName, tbl.name]),
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
          id: nodeId('col', [connId, dbName, tableName, col.name]),
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

function onTreeNodeContextmenu(event: MouseEvent, data: TreeNode) {
  event.preventDefault()
  contextMenuX.value = event.clientX
  contextMenuY.value = event.clientY
  contextNodeType.value = data.type
  contextNodeData.value = data.data
  contextMenuVisible.value = true
}

function closeContextMenu() {
  contextMenuVisible.value = false
}

// 点击菜单外部时关闭
function onDocMouseDown(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (!target?.closest?.('.context-menu')) {
    closeContextMenu()
  }
}

onMounted(() => {
  document.addEventListener('mousedown', onDocMouseDown, true)
})

onUnmounted(() => {
  document.removeEventListener('mousedown', onDocMouseDown, true)
})

function handleRefresh() {
  closeContextMenu()
  refreshTree()
}

function handleNewQuery() {
  closeContextMenu()
  if (contextNodeData.value?.id) {
    // 脚本区默认折叠；新建查询时自动展开，与左侧脚本区(可另存/关联)联动可见
    dbSidebar.setScriptsExpanded(true)
    editorStore.openTab({
      title: '新查询',
      type: 'sql',
      configId: contextNodeData.value.id,
      content: '',
    })
  }
}

function handleViewTable() {
  closeContextMenu()
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
  closeContextMenu()
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
  closeContextMenu()
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
  font-size: var(--font-size-md);
  /* 库名/表名/列名是代码标识符，用等宽字体渲染；
     阿里巴巴普惠体在 13px 小字号下 _ 字形位于基线下方且笔画过细，Windows 下几乎不可见 */
  font-family: var(--font-family-mono);
}
.node-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.connected-badge {
  font-size: var(--font-size-xs);
  color: #67c23a;
  margin-left: 4px;
}
.is-connected {
  color: #67c23a;
}
.is-disconnected {
  color: var(--el-text-color-disabled);
}
</style>

<style>
/* 右键菜单（非 scoped，因为 teleport 到 body） */
.context-menu {
  position: fixed;
  z-index: 9999;
  border-radius: var(--radius-md);
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.2);
  padding: 4px 0;
  min-width: 120px;
  /* 用设计令牌跟随 light/dark：light=#fff / dark=#3c3f41 */
  background: var(--bgColor);
  border: 1px solid var(--borderColor);
}
.context-menu-item {
  padding: 6px 16px;
  font-size: var(--font-size-md);
  cursor: pointer;
  white-space: nowrap;
  color: var(--textColor);
}
.context-menu-item:hover {
  background-color: var(--hoverColor);
  color: var(--activityBarActive);
}
</style>
