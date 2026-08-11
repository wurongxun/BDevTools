<template>
  <div class="ssh-sidebar-panel">
    <!-- VSCode 风格头部 -->
    <div class="panel-header">
      <span class="panel-title">连接管理器</span>
      <div class="header-actions">
        <el-dropdown trigger="click" @command="handleHeaderCommand">
          <el-button size="small" link class="header-more-btn" title="更多操作">
            <el-icon><MoreFilled /></el-icon>
          </el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="addGroup">
                <el-icon><FolderAdd /></el-icon> 新建分组
              </el-dropdown-item>
              <el-dropdown-item command="addConnection" divided>
                <el-icon><Plus /></el-icon> 新建连接
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>
    </div>

    <!-- 分组列表 -->
    <div class="panel-body">
      <template v-if="orderedGroups.length > 0">
        <div
          v-for="group in orderedGroups"
          :key="group.name"
          class="group-section"
        >
          <!-- 分组头部 -->
          <div
            class="group-header"
            @click="toggleGroup(group.name)"
            @contextmenu.prevent="handleGroupContext(group.name, $event)"
          >
            <el-icon
              class="group-chevron"
              :class="{ expanded: expandedGroups.includes(group.name) }"
            >
              <ArrowRight />
            </el-icon>
            <span class="group-label">{{ group.name }}</span>
            <span class="group-count">{{ group.connections.length }}</span>
          </div>

          <!-- 分组内容 -->
          <div class="group-content" v-show="expandedGroups.includes(group.name)">
            <div
              v-for="conn in group.connections"
              :key="conn.id"
              class="conn-item"
              :class="{ 'conn-item-active': activeConnId === conn.id }"
              @click="handleConnectionClick(conn)"
              @contextmenu.prevent="handleConnContext(conn, $event)"
            >
              <!-- 连接图标 -->
              <span class="conn-icon">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                  <rect x="1" y="2" width="14" height="10" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.2" />
                  <line x1="1" y1="5" x2="15" y2="5" stroke="currentColor" stroke-width="1.2" />
                  <circle cx="3.5" cy="3.5" r="0.8" />
                  <circle cx="6" cy="3.5" r="0.8" />
                </svg>
              </span>
              <span class="conn-name">{{ conn.name }}</span>
              <span class="conn-addr">{{ conn.address }}:{{ conn.port }}</span>
              <!-- hover 操作按钮 -->
              <div class="conn-hover-actions">
                <span class="action-btn" @click.stop="openTerminal(conn)" title="终端">
                  <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M2 3l5 5-5 5" stroke="currentColor" stroke-width="1.5" fill="none" />
                    <line x1="8" y1="13" x2="14" y2="13" stroke="currentColor" stroke-width="1.5" />
                  </svg>
                </span>
                <span class="action-btn" @click.stop="openFtp(conn)" title="文件管理">
                  <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M2 3h12v10H2V3z" fill="none" stroke="currentColor" stroke-width="1" />
                    <line x1="2" y1="6" x2="14" y2="6" stroke="currentColor" stroke-width="1" />
                  </svg>
                </span>
              </div>
            </div>
            <!-- 空分组提示 -->
            <div v-if="group.connections.length === 0" class="group-empty">
              暂无连接，拖拽或右键添加
            </div>
          </div>
        </div>
      </template>

      <div v-else class="connection-empty">
        <p>暂无连接</p>
        <el-button size="small" @click="showConnectionDialog = true">新建连接</el-button>
      </div>
    </div>

    <!-- 分组右键菜单 -->
    <teleport to="body">
      <div
        v-if="groupContextMenu.visible"
        class="context-menu"
        :style="{ left: groupContextMenu.x + 'px', top: groupContextMenu.y + 'px' }"
        @click.stop
      >
        <div class="context-menu-item" @click="handleGroupRename">
          <el-icon><EditPen /></el-icon> 重命名分组
        </div>
        <div class="context-menu-item" @click="handleAddConnToGroup">
          <el-icon><Plus /></el-icon> 添加连接到本分组
        </div>
        <div class="context-menu-divider" />
        <div class="context-menu-item context-menu-danger" @click="handleGroupDelete">
          <el-icon><Delete /></el-icon> 删除分组
        </div>
      </div>
    </teleport>

    <!-- 连接右键菜单 -->
    <teleport to="body">
      <div
        v-if="connContextMenu.visible"
        class="context-menu"
        :style="{ left: connContextMenu.x + 'px', top: connContextMenu.y + 'px' }"
        @click.stop
      >
        <div class="context-menu-item" @click="openTerminal(connContextMenu.conn!)">
          <el-icon><Monitor /></el-icon> 打开终端
        </div>
        <div class="context-menu-item" @click="openFtp(connContextMenu.conn!)">
          <el-icon><FolderOpened /></el-icon> 文件管理
        </div>
        <div class="context-menu-divider" />
        <!-- 移动到分组 -->
        <div class="context-menu-submenu">
          <span>移动到分组</span>
          <el-icon class="submenu-arrow"><ArrowRight /></el-icon>
          <div class="context-menu-submenu-list">
            <div
              v-for="g in orderedGroups"
              :key="g.name"
              class="context-menu-item"
              :class="{ 'context-menu-active': connContextMenu.conn?.favorites_name === g.name }"
              @click="handleMoveToGroup(g.name)"
            >
              {{ g.name }}
            </div>
          </div>
        </div>
        <div class="context-menu-divider" />
        <div class="context-menu-item" @click="handleConnEdit">
          <el-icon><EditPen /></el-icon> 编辑连接
        </div>
        <div class="context-menu-item context-menu-danger" @click="handleConnDelete">
          <el-icon><Delete /></el-icon> 删除连接
        </div>
      </div>
    </teleport>

    <!-- 连接管理对话框 -->
    <ConnectionDialog
      v-model="showConnectionDialog"
      :edit-data="editingConnection"
      :default-group="defaultGroupForNew"
    />

    <!-- 全局点击关闭菜单 -->
    <teleport to="body">
      <div
        v-if="groupContextMenu.visible || connContextMenu.visible"
        class="context-menu-overlay"
        @click="closeAllContextMenus"
      />
    </teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { Plus, ArrowRight, MoreFilled, FolderAdd, EditPen, Delete, Monitor, FolderOpened } from '@element-plus/icons-vue'
import { ElMessageBox } from 'element-plus'
import ConnectionDialog from './components/RemoteConnection/ConnectionDialog.vue'
import { useConnectionStore } from './store'
import type { RemoteConnection } from './types'

const connectionStore = useConnectionStore()

const showConnectionDialog = ref(false)
const editingConnection = ref<RemoteConnection | null>(null)
const defaultGroupForNew = ref<string>('')
const expandedGroups = ref<string[]>([])
const activeConnId = ref<number | null>(null)

const connections = computed(() => connectionStore.connections)
const allGroupNames = computed(() => connectionStore.connectionGroups)

// 按分组顺序组织连接
const orderedGroups = computed(() => {
  const groups: { name: string; connections: RemoteConnection[] }[] = []
  for (const groupName of allGroupNames.value) {
    const conns = connections.value.filter(c => (c.favorites_name || '默认分组') === groupName)
    groups.push({ name: groupName, connections: conns })
  }
  // 添加未出现在分组列表中的连接（兜底）
  const knownGroups = new Set(allGroupNames.value)
  const orphanConns = connections.value.filter(c => !knownGroups.has(c.favorites_name || '默认分组'))
  if (orphanConns.length > 0) {
    groups.push({ name: '其他', connections: orphanConns })
  }
  return groups
})

// 分组右键菜单
const groupContextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  groupName: ''
})

// 连接右键菜单
const connContextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  conn: null as RemoteConnection | null
})

function toggleGroup(name: string) {
  const idx = expandedGroups.value.indexOf(name)
  if (idx >= 0) {
    expandedGroups.value.splice(idx, 1)
  } else {
    expandedGroups.value.push(name)
  }
}

function handleConnectionClick(conn: RemoteConnection) {
  activeConnId.value = conn.id
  openTerminal(conn)
}

function openTerminal(conn: RemoteConnection) {
  activeConnId.value = conn.id
  connectionStore.openTab(conn, 'terminal')
  closeAllContextMenus()
}

function openFtp(conn: RemoteConnection) {
  activeConnId.value = conn.id
  connectionStore.openTab(conn, 'ftp')
  closeAllContextMenus()
}

// 头部菜单命令
function handleHeaderCommand(cmd: string) {
  if (cmd === 'addGroup') {
    promptAddGroup()
  } else if (cmd === 'addConnection') {
    defaultGroupForNew.value = ''
    editingConnection.value = null
    showConnectionDialog.value = true
  }
}

// 添加分组
function promptAddGroup() {
  ElMessageBox.prompt('请输入分组名称', '新建分组', {
    confirmButtonText: '确定',
    cancelButtonText: '取消',
    inputPattern: /.+/,
    inputErrorMessage: '分组名称不能为空'
  }).then(({ value }) => {
    if (connectionStore.addGroup(value.trim())) {
      expandedGroups.value.push(value.trim())
    } else {
      ElMessageBox.alert('分组名称已存在', '提示')
    }
  }).catch(() => {})
}

// 分组右键菜单
function handleGroupContext(groupName: string, event: MouseEvent) {
  groupContextMenu.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    groupName
  }
}

function handleGroupRename() {
  const oldName = groupContextMenu.value.groupName
  closeAllContextMenus()
  ElMessageBox.prompt('请输入新名称', '重命名分组', {
    confirmButtonText: '确定',
    cancelButtonText: '取消',
    inputValue: oldName,
    inputPattern: /.+/,
    inputErrorMessage: '分组名称不能为空'
  }).then(({ value }) => {
    connectionStore.renameGroup(oldName, value.trim())
  }).catch(() => {})
}

function handleGroupDelete() {
  const name = groupContextMenu.value.groupName
  closeAllContextMenus()
  ElMessageBox.confirm(`确定删除分组 "${name}" 吗？\n该分组下的连接将移到"默认分组"。`, '确认删除', {
    type: 'warning'
  }).then(() => {
    connectionStore.deleteGroup(name)
  }).catch(() => {})
}

function handleAddConnToGroup() {
  const groupName = groupContextMenu.value.groupName
  closeAllContextMenus()
  defaultGroupForNew.value = groupName
  editingConnection.value = null
  showConnectionDialog.value = true
}

// 连接右键菜单
function handleConnContext(conn: RemoteConnection, event: MouseEvent) {
  connContextMenu.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    conn
  }
}

function handleConnEdit() {
  editingConnection.value = connContextMenu.value.conn
  closeAllContextMenus()
  showConnectionDialog.value = true
}

function handleConnDelete() {
  const conn = connContextMenu.value.conn
  closeAllContextMenus()
  if (!conn) return
  ElMessageBox.confirm(`确定删除连接 "${conn.name}" 吗？`, '确认删除', {
    type: 'warning'
  }).then(() => {
    connectionStore.deleteConnection(conn.id)
  }).catch(() => {})
}

function handleMoveToGroup(groupName: string) {
  const conn = connContextMenu.value.conn
  closeAllContextMenus()
  if (!conn) return
  connectionStore.moveConnectionToGroup(conn.id, groupName)
}

function closeAllContextMenus() {
  groupContextMenu.value.visible = false
  connContextMenu.value.visible = false
}

onMounted(() => {
  connectionStore.loadConnections()
  connectionStore.loadGroups()
  expandedGroups.value = allGroupNames.value.length > 0
    ? [...allGroupNames.value]
    : ['默认分组']
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.ssh-sidebar-panel {
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
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 12px;
  height: 35px;
  flex-shrink: 0;
  // border-bottom: 1px solid #2d2d2d;

  // @include theme.useTheme {
  //   border-bottom-color: theme.getVar('borderColor');
  // }
}

.panel-title {
  font-size: 11px;
  font-weight: 600;
  color: #bbb;
  text-transform: uppercase;
  letter-spacing: 1px;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 4px;
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
.panel-body {
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
.group-section {
  margin-bottom: 0;
}

.group-header {
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

.group-chevron {
  font-size: 10px;
  color: #888;
  transition: transform 0.15s ease;
  flex-shrink: 0;

  &.expanded {
    transform: rotate(90deg);
  }
}

.group-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-count {
  font-size: 10px;
  font-weight: 400;
  color: #666;
  background-color: rgba(255, 255, 255, 0.06);
  padding: 0 5px;
  border-radius: 8px;
  flex-shrink: 0;
}

.group-content {
  padding-left: 0;
}

.group-empty {
  padding: 4px 12px 4px 36px;
  font-size: 12px;
  color: #555;
  font-style: italic;
}

// ===== 连接项 =====
.conn-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 8px 3px 20px;
  cursor: pointer;
  transition: background-color 0.1s;
  position: relative;
  font-size: 13px;

  &:hover {
    background-color: rgba(255, 255, 255, 0.06);

    .conn-hover-actions {
      opacity: 1;
    }
  }

  &.conn-item-active {
    background-color: rgba(0, 122, 204, 0.25);
  }
}

.conn-icon {
  flex-shrink: 0;
  color: #888;
  display: flex;
  align-items: center;
}

.conn-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #ccc;
  font-size: 13px;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.conn-addr {
  font-size: 11px;
  color: #555;
  flex-shrink: 0;
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.conn-hover-actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.1s;
  flex-shrink: 0;
}

.action-btn {
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

// ===== 空状态 =====
.connection-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  color: #555;
  gap: 12px;

  p { font-size: 13px; margin: 0; }
}

// ===== 右键菜单 =====
.context-menu-overlay {
  position: fixed;
  inset: 0;
  z-index: 9998;
}

.context-menu {
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

.context-menu-item {
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

  .el-icon {
    font-size: 14px;
  }
}

.context-menu-danger {
  color: #e06c75;

  &:hover {
    background-color: rgba(224, 108, 117, 0.2);
    color: #e06c75;
  }
}

.context-menu-active {
  color: #61afef;
}

.context-menu-divider {
  height: 1px;
  background-color: #454545;
  margin: 4px 0;

  @include theme.useTheme {
    background-color: theme.getVar('borderColor');
  }
}

// 子菜单
.context-menu-submenu {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 16px;
  font-size: 13px;
  color: #ccc;
  cursor: pointer;
  transition: background-color 0.1s;

  &:hover {
    background-color: rgba(0, 122, 204, 0.3);
    color: #fff;
  }

  .submenu-arrow {
    font-size: 10px;
    color: #666;
  }
}

.context-menu-submenu-list {
  display: none;
  position: absolute;
  left: 100%;
  top: 0;
  background-color: #2d2d2d;
  border: 1px solid #454545;
  border-radius: 6px;
  padding: 4px 0;
  min-width: 140px;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.5);
  z-index: 10000;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-color: theme.getVar('borderColor');
  }
}

.context-menu-submenu:hover .context-menu-submenu-list {
  display: block;
}
</style>
