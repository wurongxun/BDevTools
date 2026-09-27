<template>
  <div class="database-tree">
    <el-tree
      :ref="treeRef"
      :data="treeData"
      :props="treeProps"
      node-key="id"
      :load="loadNode"
      lazy
      highlight-current
      :key="treeKey"
      @node-click="handleNodeClick"
      @node-contextmenu="handleNodeContextMenu"
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
          <!-- 连接级 / 库级分组：折叠用 Folder，展开用 FolderOpened（node.expanded 由 el-tree 提供） -->
          <el-icon v-else-if="data.type === 'connection-group' || data.type === 'database-group'">
            <FolderOpened v-if="node.expanded" />
            <Folder v-else />
          </el-icon>
          <el-icon v-else-if="data.type === 'user'"><User /></el-icon>
          <el-icon v-else-if="data.type === 'admin'"><UserFilled /></el-icon>
          <el-icon v-else-if="data.type === 'status-group' || data.type === 'variable-group'"><Monitor /></el-icon>
          <el-icon v-else-if="data.type === 'view'"><View /></el-icon>
          <el-icon v-else-if="data.type === 'index'"><Sort /></el-icon>
          <el-icon v-else-if="data.type === 'procedure'"><SetUp /></el-icon>
          <el-icon v-else-if="data.type === 'trigger'"><MagicStick /></el-icon>
          <el-icon v-else-if="data.type === 'event'"><Timer /></el-icon>
          <span
            class="node-label"
            :class="{ 'is-connected': data.type === 'table' && isTableActive(data) }"
          >{{ node.label }}</span>
          <span v-if="data.type === 'connection' && isConnected(data.data?.id)" class="connected-badge">已连接</span>
        </span>
      </template>
    </el-tree>

    <!-- 右键菜单 -->
<!--    <teleport to="body">-->
<!--      <div-->
<!--        v-if="contextMenuVisible"-->
<!--        class="context-menu"-->
<!--        :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }"-->
<!--        @click.stop-->
<!--      >-->
<!--        <div v-if="contextNodeType === 'connection'" class="context-menu-item" @click="handleRefresh">刷新</div>-->
<!--        <div v-if="contextNodeType === 'connection'" class="context-menu-item" @click="c">新建查询</div>-->
<!--        <div v-if="contextNodeType === 'connection'" class="context-menu-item" @click="handleDeleteConnection">编辑连接</div>-->
<!--        <div v-if="contextNodeType === 'connection'" class="context-menu-item" @click="handleDisconnect">断开</div>-->
<!--        <div v-if="contextNodeType === 'connection'" class="context-menu-item" @click="handleDisconnectAndReconnect">断开/重新连接</div>-->
<!--        <div v-if="contextNodeType === 'connection'" class="context-menu-item" @click="handleDeleteConnection">删除</div>-->
<!--        <div v-if="contextNodeType === 'table'" class="context-menu-item" @click="handleViewTable">查看表数据</div>-->
<!--        <div v-if="contextNodeType === 'table'" class="context-menu-item" @click="handleViewStructure">查看表结构</div>-->
<!--        <div v-if="contextNodeType === 'table'" class="context-menu-item" @click="handleExportTable">导出数据</div>-->
<!--      </div>-->
<!--    </teleport>-->

    <ContextMenu
      :visible="menuVisible"
      :menu-items="menuItems"
      :position="menuPosition"
      @close="menuVisible = false"
    />

    <!-- 编辑连接弹窗（复用 ConnectionDialog 的编辑模式） -->
    <ConnectionDialog
      v-model="showConnectionDialog"
      :edit-config="editingConfig"
      @submit="handleConnectionSubmit"
    />

    <!-- 删除连接确认 -->
    <GlobalDialog
      v-model="showDeleteConfirm"
      title="删除连接"
      width="380px"
    >
      <p class="delete-confirm-text">
        确定要删除连接「{{ deletingConn?.name }}」吗？该连接下的所有查询标签页也会一并关闭。
      </p>
      <template #footer>
        <app-button size="small" @click="showDeleteConfirm = false">取消</app-button>
        <app-button size="small" type="danger" @click="confirmDeleteConnection">删除</app-button>
      </template>
    </GlobalDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted,computed } from 'vue'
import {
  Connection, Coin, Grid, Document,
  Folder, FolderOpened, User, UserFilled, Monitor,
  View, Sort, SetUp, MagicStick, Timer,
} from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import type { TreeInstance } from 'element-plus'
import { useConnectionStore, useEditorStore, useDbSidebarStore } from '../store'
import {
  dbListDatabases, dbListTables, dbTableColumns,
  dbCatalogCapabilities, dbListUsers, dbListAdmins,
  dbListViews, dbListProcedures, dbListTriggers, dbListEvents, dbListDatabaseIndexes,
} from '../api/metadata'
import type { MenuItem } from '@/components/context-menu/ContextMenu.vue'
import type {
  TreeNode, DbConnectionConfig, CatalogCapabilities, ObjectListType,
} from '../types'
import GlobalDialog from '@/components/GlobalDialog.vue'
import ConnectionDialog from './ConnectionDialog.vue'

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



//----------

const treeRef = ref<TreeInstance>()
const menuVisible = ref(false)
const menuPosition = ref({ x: 0, y: 0 })
const currentNode = ref<{ data: any; node: any } | null>(null)

// 编辑连接弹窗
const showConnectionDialog = ref(false)
const editingConfig = ref<DbConnectionConfig | null>(null)
// 删除连接确认弹窗
const showDeleteConfirm = ref(false)
const deletingConn = ref<DbConnectionConfig | null>(null)

// 取当前右键连接节点的 payload（DbConnectionConfig，含 id）
function currentConnPayload(): DbConnectionConfig | null {
  const node = currentNode.value?.data
  if (!node || node.type !== 'connection') return null
  return (node.data as DbConnectionConfig) ?? null
}

// 根据右键节点动态生成菜单
const menuItems = computed<MenuItem[]>(() => {
  if (!currentNode.value) return []
  const { data, node } = currentNode.value
  const isTable = data.type === 'table'
  const isConnection = data.type === 'connection'
  const isDatabase = data.type === 'database'
  if (isTable){
    return [
      { label: '查看表数据', icon: '📊', onClick: handleViewTable },
      { label: '查看表结构', icon: '📝',  onClick: handleViewStructure },
      { label: '导出数据', icon: '💾', onClick: handleExportTable },
    ]
  }
  if (isConnection){
    return [
      { label: '刷新', icon: '🔄',  onClick:handleRefresh },
      { divider: true },
      { label: '新建查询', icon: '➕',  onClick: handleNewQuery },
      { label: '编辑连接', icon: '📝',  onClick: handleEditConnection },
      { divider: true },
      { label: '连接',  icon: '', shortcut: 'F1', onClick: handleConnection },
      { label: '断开',  icon: '', shortcut: 'F2', onClick: handleDisconnect },
      { label: '断开/重新连接', shortcut: 'F2', danger: true, onClick: handleDisconnectAndReconnect },
      { label: '删除', icon: '🗑️', shortcut: 'Delete', danger: true, onClick: handleDeleteConnection },
    ]
  }
  if (isDatabase){
    return [
      { label: '新建查询', icon: '➕',  onClick: handleNewQuery }
    ]
  }
  // 连接级分组（用户/管理员/系统信息）：仅刷新
  if (data.type === 'connection-group'){
    return [
      { label: '刷新', icon: '🔄', onClick: handleRefresh }
    ]
  }
  // 库级分组（表/视图/索引/存储过程/触发器/事件）：刷新 + 在该库新建查询
  if (data.type === 'database-group'){
    return [
      { label: '刷新', icon: '🔄', onClick: handleRefresh },
      { divider: true },
      { label: '新建查询', icon: '➕', onClick: handleNewQuery }
    ]
  }
  // 系统信息子项：刷新
  if (data.type === 'status-group' || data.type === 'variable-group'){
    return [
      { label: '刷新', icon: '🔄', onClick: handleRefresh }
    ]
  }
  return []

})

const handleNodeContextMenu = (event: MouseEvent, data: any, node: any) => {
  event.preventDefault()                  // ① 阻止浏览器默认菜单
  treeRef.value?.setCurrentKey(data.id)   // ② 高亮当前节点
  currentNode.value = { data, node }      // ③ 记录当前节点
  menuPosition.value = { x: event.clientX, y: event.clientY } // ④ 定位
  menuVisible.value = true                // ⑤ 显示菜单
}
//---------













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
  // 重建整棵树时清掉能力缓存，保证下次展开拿到的是当前连接的最新能力
  capabilitiesCache.clear()
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

// ==================== 目录能力缓存 ====================
// key = configId：连接成功后拉一次并缓存。能力决定树上渲染哪些分组 ——
// 不支持的分组根本不出现，而不是「出现了点开才报『暂不支持』」。
const capabilitiesCache = new Map<string, CatalogCapabilities>()

async function ensureCapabilities(configId: string): Promise<CatalogCapabilities> {
  if (!configId) return (await dbCatalogCapabilities(configId)) as CatalogCapabilities
  const cached = capabilitiesCache.get(configId)
  if (cached) return cached
  const caps = await dbCatalogCapabilities(configId)
  capabilitiesCache.set(configId, caps)
  return caps
}

// 缓存的失效点放在 refreshTree()：连接/断开/删除后都会重建整棵树，
// 此时清空缓存，下次展开分组会重新拉一次能力声明（换库类型等场景不会用到旧值）。

/** 连接级分组 */
type ConnGroupKey = 'databases' | 'users' | 'admins' | 'system'
/** 库级分组 */
type DbGroupKey = 'tables' | 'views' | 'indexes' | 'procedures' | 'triggers' | 'events'
/** 系统信息子项 */
type SysInfoKey = 'session-status' | 'global-status' | 'session-variables' | 'global-variables'

const CONN_GROUP_LABELS: Record<ConnGroupKey, string> = {
  databases: '数据库',
  users: '用户',
  admins: '管理员',
  system: '系统信息',
}
const DB_GROUP_LABELS: Record<DbGroupKey, string> = {
  tables: '表',
  views: '视图',
  indexes: '索引',
  procedures: '存储过程',
  triggers: '触发器',
  events: '事件',
}
const SYS_INFO_LABELS: Record<SysInfoKey, string> = {
  'session-status': '会话状态',
  'global-status': '全局状态',
  'session-variables': '会话变量',
  'global-variables': '全局变量',
}

/** 库级对象叶子节点的 type（供图标与双击路由使用） */
const DB_GROUP_LEAF_TYPE: Record<Exclude<DbGroupKey, 'tables'>, TreeNode['type']> = {
  views: 'view',
  indexes: 'index',
  procedures: 'procedure',
  triggers: 'trigger',
  events: 'event',
}

async function loadNode(node: any, resolve: (data: TreeNode[]) => void) {
  if (node.level === 0) {
    resolve(treeData.value)
    return
  }

  const nodeData = node.data as TreeNode
  const d: any = nodeData.data ?? {}

  // ---- 连接：连上 → 取能力 → 返回连接级分组 ----
  if (nodeData.type === 'connection') {
    const connId = d?.id
    if (!connId) { resolve([]); return }

    if (!connectionStore.isConnected(connId)) {
      try {
        const res = await connectionStore.connect(connId)
        if (res.code !== 200) { resolve([]); return }
      } catch { resolve([]); return }
    }

    const caps = await ensureCapabilities(connId)
    const groups: ConnGroupKey[] = ['databases']
    if (caps.users) groups.push('users')
    if (caps.admins) groups.push('admins')
    if (caps.session_status || caps.global_status || caps.session_variables || caps.global_variables) {
      groups.push('system')
    }
    resolve(groups.map(g => ({
      id: nodeId('conn-group', [connId, g]),
      label: CONN_GROUP_LABELS[g],
      type: 'connection-group' as const,
      isLeaf: false,
      data: { configId: connId, group: g, caps },
    })))
    return
  }

  // ---- 连接级分组：数据库 / 用户 / 管理员 / 系统信息 ----
  if (nodeData.type === 'connection-group') {
    const connId = d?.configId as string | undefined
    const group = d?.group as ConnGroupKey | undefined
    if (!connId || !group) { resolve([]); return }
    const caps: CatalogCapabilities = d?.caps ?? (await ensureCapabilities(connId))

    if (group === 'databases') {
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
          return
        }
        console.warn('[DatabaseTree] dbListDatabases failed:', res.msg)
      } catch (e) { console.error('[DatabaseTree] load databases error:', e) }
      resolve([])
      return
    }

    if (group === 'users') {
      try {
        const res = await dbListUsers(connId)
        if (res.code === 200) {
          resolve(res.data.map(u => ({
            id: nodeId('user', [connId, `${u.user}@${u.host}`]),
            label: u.host ? `${u.user}@${u.host}` : u.user,
            type: 'user' as const,
            isLeaf: true,
            data: { configId: connId, name: u.user, host: u.host },
          })))
          return
        }
        console.warn('[DatabaseTree] dbListUsers failed:', res.msg)
      } catch (e) { console.error('[DatabaseTree] load users error:', e) }
      resolve([])
      return
    }

    if (group === 'admins') {
      try {
        const res = await dbListAdmins(connId)
        if (res.code === 200) {
          resolve(res.data.map(u => ({
            id: nodeId('admin', [connId, `${u.user}@${u.host}`]),
            label: u.host ? `${u.user}@${u.host}` : u.user,
            type: 'admin' as const,
            isLeaf: true,
            data: { configId: connId, name: u.user, host: u.host, reason: u.reason },
          })))
          return
        }
        console.warn('[DatabaseTree] dbListAdmins failed:', res.msg)
      } catch (e) { console.error('[DatabaseTree] load admins error:', e) }
      resolve([])
      return
    }

    if (group === 'system') {
      const items: SysInfoKey[] = []
      if (caps.session_status) items.push('session-status')
      if (caps.global_status) items.push('global-status')
      if (caps.session_variables) items.push('session-variables')
      if (caps.global_variables) items.push('global-variables')
      resolve(items.map(k => ({
        id: nodeId('sysinfo', [connId, k]),
        label: SYS_INFO_LABELS[k],
        type: (k.endsWith('-status') ? 'status-group' : 'variable-group') as TreeNode['type'],
        isLeaf: true,
        data: { configId: connId, subType: k },
      })))
      return
    }

    resolve([])
    return
  }

  // ---- 数据库：返回库级分组 ----
  if (nodeData.type === 'database') {
    const connId = d?.configId as string | undefined
    const dbName = d?.name as string | undefined
    if (!connId || !dbName) { resolve([]); return }

    const caps = await ensureCapabilities(connId)
    const groups: DbGroupKey[] = ['tables']
    if (caps.views) groups.push('views')
    if (caps.database_indexes) groups.push('indexes')
    if (caps.procedures) groups.push('procedures')
    if (caps.triggers) groups.push('triggers')
    if (caps.events) groups.push('events')
    resolve(groups.map(g => ({
      id: nodeId('db-group', [connId, dbName, g]),
      label: DB_GROUP_LABELS[g],
      type: 'database-group' as const,
      isLeaf: false,
      data: { configId: connId, database: dbName, group: g },
    })))
    return
  }

  // ---- 库级分组：表 / 视图 / 索引 / 存储过程 / 触发器 / 事件 ----
  if (nodeData.type === 'database-group') {
    const connId = d?.configId as string | undefined
    const dbName = d?.database as string | undefined
    const group = d?.group as DbGroupKey | undefined
    if (!connId || !dbName || !group) { resolve([]); return }

    if (group === 'tables') {
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
          return
        }
        console.warn('[DatabaseTree] dbListTables failed:', res.msg)
      } catch (e) { console.error('[DatabaseTree] load tables error:', e) }
      resolve([])
      return
    }

    // 其余分组：拉对象列表，渲染为叶子节点
    const fetcher = group === 'views' ? dbListViews
      : group === 'procedures' ? dbListProcedures
      : group === 'triggers' ? dbListTriggers
      : group === 'events' ? dbListEvents
      : null
    try {
      if (fetcher) {
        const res = await fetcher(connId, dbName)
        if (res.code === 200) {
          resolve(res.data.map(o => ({
            id: nodeId(group, [connId, dbName, o.name]),
            label: o.name,
            type: DB_GROUP_LEAF_TYPE[group as Exclude<DbGroupKey, 'tables'>],
            isLeaf: true,
            data: { configId: connId, database: dbName, name: o.name, group },
          })))
          return
        }
        console.warn(`[DatabaseTree] ${group} failed:`, res.msg)
      } else {
        // 索引：库级聚合，一行一个「表.索引」
        const res = await dbListDatabaseIndexes(connId, dbName)
        if (res.code === 200) {
          resolve(res.data.map(ix => ({
            id: nodeId('index', [connId, dbName, ix.table_name, ix.name, ix.column_name]),
            label: `${ix.name} · ${ix.table_name}`,
            type: 'index' as const,
            isLeaf: true,
            data: { configId: connId, database: dbName, name: ix.name, group: 'indexes' },
          })))
          return
        }
        console.warn('[DatabaseTree] dbListDatabaseIndexes failed:', res.msg)
      }
    } catch (e) { console.error(`[DatabaseTree] load ${group} error:`, e) }
    resolve([])
    return
  }

  // ---- 表：列列表 ----
  if (nodeData.type === 'table') {
    const connId = d?.configId
    const dbName = d?.database
    const tableName = d?.name
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
    return
  }

  resolve([])
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

/** 库级分组 key → 对象列表类型（object-list 标签页用） */
const DB_GROUP_OBJECT_TYPE: Record<Exclude<DbGroupKey, 'tables'>, ObjectListType> = {
  views: 'view',
  indexes: 'index',
  procedures: 'procedure',
  triggers: 'trigger',
  events: 'event',
}

const OBJECT_LIST_TITLE: Record<ObjectListType, string> = {
  user: '用户',
  admin: '管理员',
  view: '视图',
  index: '索引',
  procedure: '存储过程',
  trigger: '触发器',
  event: '事件',
}

/**
 * 打开「对象列表」标签页。
 * user / admin 是连接级（不需要 database），其余是库级（需要 database）。
 */
function openObjectList(configId: string, objectType: ObjectListType, database?: string) {
  if (!configId) return
  const title = database
    ? `${OBJECT_LIST_TITLE[objectType]} - ${database}`
    : OBJECT_LIST_TITLE[objectType]
  editorStore.openTab({
    title,
    type: 'object-list',
    configId,
    database,
    objectType,
  })
}

/** 打开「系统信息」标签页（会话/全局 状态与变量） */
function openSystemInfo(configId: string, subType: SysInfoKey) {
  if (!configId) return
  editorStore.openTab({
    title: SYS_INFO_LABELS[subType],
    type: 'system-info',
    configId,
    subType,
  })
}

function handleDblClick(data: TreeNode) {
  const d: any = data.data ?? {}

  // 系统信息子项（会话状态 / 全局状态 / 会话变量 / 全局变量）
  if (data.type === 'status-group' || data.type === 'variable-group') {
    if (d?.subType) openSystemInfo(d.configId, d.subType)
    return
  }

  // 用户 / 管理员（连接级）
  if (data.type === 'user') { openObjectList(d?.configId, 'user'); return }
  if (data.type === 'admin') { openObjectList(d?.configId, 'admin'); return }

  // 视图 / 索引 / 存储过程 / 触发器 / 事件 叶子：打开所属分组的完整列表
  if (
    data.type === 'view' || data.type === 'index' ||
    data.type === 'procedure' || data.type === 'trigger' || data.type === 'event'
  ) {
    const objectType = DB_GROUP_OBJECT_TYPE[d?.group as Exclude<DbGroupKey, 'tables'>]
    if (objectType) openObjectList(d?.configId, objectType, d?.database)
    return
  }

  // 连接级分组：直接打开对应列表（用户 / 管理员）；数据库/系统信息分组仅展开
  if (data.type === 'connection-group') {
    if (d?.group === 'users') openObjectList(d?.configId, 'user')
    else if (d?.group === 'admins') openObjectList(d?.configId, 'admin')
    return
  }

  // 库级分组：直接打开对应对象列表；「表」分组仅展开
  if (data.type === 'database-group') {
    const objectType = DB_GROUP_OBJECT_TYPE[d?.group as Exclude<DbGroupKey, 'tables'>]
    if (objectType) openObjectList(d?.configId, objectType, d?.database)
    return
  }

  if (data.type === 'table' && d) {
    editorStore.openTab({
      title: d.name,
      type: 'table',
      configId: d.configId,
      database: d.database,
      table: d.name,
    })
  } else if (data.type === 'database' && d) {
    editorStore.openTab({
      title: '新查询',
      type: 'sql',
      configId: d.configId,
      database: d.name,
      content: '',
      forceNew: true,
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
  const node = currentNode.value?.data as TreeNode | undefined
  if (!node) return
  const payload = node.data
  // 在「数据库连接」节点新建查询：不默认选数据库
  if (node.type === 'connection') {
    const connId = payload?.id
    if (!connId) return
    // 脚本区默认折叠；新建查询时自动展开，与左侧脚本区(可另存/关联)联动可见
    dbSidebar.setScriptsExpanded(true)
    editorStore.openTab({
      title: '新查询',
      type: 'sql',
      configId: connId,
      content: '',
      forceNew: true,
    })
  // 在「数据库」或库级分组（表/视图/…）节点新建查询：默认选当前节点所在的数据库
  } else if (node.type === 'database' || node.type === 'database-group') {
    const connId = payload?.configId
    const dbName = payload?.name ?? payload?.database
    if (!connId || !dbName) return
    dbSidebar.setScriptsExpanded(true)
    editorStore.openTab({
      title: '新查询',
      type: 'sql',
      configId: connId,
      database: dbName,
      content: '',
      forceNew: true,
    })
  }
}

//编辑连接：打开编辑弹窗（复用 ConnectionDialog 的编辑模式）
function handleEditConnection() {
  const conn = currentConnPayload()
  if (!conn) return
  editingConfig.value = conn
  showConnectionDialog.value = true
}

// 编辑弹窗提交：走 updateConnection 保存
async function handleConnectionSubmit(config: DbConnectionConfig) {
  try {
    const res = await connectionStore.updateConnection(config)
    if (res.code === 200) {
      ElMessage.success('连接配置已更新')
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e: any) {
    ElMessage.error('操作失败: ' + (e.message || e))
  } finally {
    showConnectionDialog.value = false
    editingConfig.value = null
    refreshTree()
  }
}

//连接数据库
async function handleConnection() {
  const conn = currentConnPayload()
  if (!conn?.id) return
  const res = await connectionStore.connect(conn.id)
  if (res.code === 200) {
    ElMessage.success(`已连接「${conn.name}」`)
    refreshTree()
  } else {
    ElMessage.error(res.msg)
  }
}

//删除连接：先弹确认框
function handleDeleteConnection() {
  const conn = currentConnPayload()
  if (!conn?.id) return
  deletingConn.value = conn
  showDeleteConfirm.value = true
}

// 确认删除：删除配置 + 关闭该连接下所有 tab + 刷新树
async function confirmDeleteConnection() {
  const conn = deletingConn.value
  if (!conn?.id) return
  try {
    // 先关闭该连接下所有打开的 tab（避免残留失效 tab）
    const ids = editorStore.tabs.filter(t => t.configId === conn.id).map(t => t.id)
    ids.forEach(id => editorStore.closeTab(id))
    const res = await connectionStore.deleteConnection(conn.id)
    if (res.code === 200) {
      ElMessage.success(`已删除连接「${conn.name}」`)
      refreshTree()
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e: any) {
    ElMessage.error('删除失败: ' + (e.message || e))
  } finally {
    showDeleteConfirm.value = false
    deletingConn.value = null
  }
}

//断开连接
async function handleDisconnect() {
  const conn = currentConnPayload()
  if (!conn?.id) return
  const res = await connectionStore.disconnect(conn.id)
  if (res.code === 200) {
    ElMessage.success(`已断开「${conn.name}」`)
    refreshTree()
  } else {
    ElMessage.error(res.msg)
  }
}

//断开并重新连接
async function handleDisconnectAndReconnect() {
  const conn = currentConnPayload()
  if (!conn?.id) return
  await connectionStore.disconnect(conn.id)
  const res = await connectionStore.connect(conn.id)
  if (res.code === 200) {
    ElMessage.success(`已重新连接「${conn.name}」`)
    refreshTree()
  } else {
    ElMessage.error(res.msg)
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
@use '@/styles/theme.scss';
@use '@/styles/tokens.scss';

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
