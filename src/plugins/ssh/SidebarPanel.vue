<template>
  <div class="ssh-sidebar-panel">
    <!-- VSCode 风格头部 -->
    <div class="panel-header">
      <span class="panel-title">连接管理器</span>
      <div class="header-actions">
        <el-dropdown trigger="click" @command="handleHeaderCommand">
          <app-button size="small" link class="header-more-btn" title="更多操作">
            <el-icon><MoreFilled /></el-icon>
          </app-button>
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
<!--              <span class="conn-addr">{{ conn.address }}:{{ conn.port }}</span>-->
              <span class="conn-name">{{ conn.name }}</span>
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
        <app-button size="small" @click="showConnectionDialog = true">新建连接</app-button>
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

    <!-- 新建分组弹窗 -->
    <GlobalPromptDialog
      v-model="showAddGroupDialog"
      title="新建分组"
      message="请输入分组名称"
      placeholder="分组名称"
      :validator="validateGroupName"
      @confirm="handleAddGroup"
    />

    <!-- 重命名分组弹窗 -->
    <GlobalPromptDialog
      v-model="showRenameGroupDialog"
      title="重命名分组"
      message="请输入新名称"
      :initial-value="renamingGroup"
      :validator="validateGroupName"
      @confirm="handleRenameGroup"
    />

    <!-- 删除确认弹窗（分组/连接共用，统一使用 GlobalDialog） -->
    <GlobalDialog v-model="showDeleteConfirm" title="确认删除" width="400px">
      <div class="delete-confirm-message">{{ deleteConfirmMessage }}</div>
      <template #footer>
        <app-button size="small" @click="showDeleteConfirm = false">取消</app-button>
        <app-button size="small" type="danger" plain @click="confirmDelete">删除</app-button>
      </template>
    </GlobalDialog>

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
import ConnectionDialog from './components/RemoteConnection/ConnectionDialog.vue'
import GlobalPromptDialog from '@/components/GlobalPromptDialog.vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { useConnectionStore } from './store'
import type { RemoteConnection } from './types'

const connectionStore = useConnectionStore()

const showConnectionDialog = ref(false)
const editingConnection = ref<RemoteConnection | null>(null)
const defaultGroupForNew = ref<string>('')
const expandedGroups = ref<string[]>([])
const activeConnId = ref<number | null>(null)

// 新建/重命名分组弹窗状态
const showAddGroupDialog = ref(false)
const showRenameGroupDialog = ref(false)
const renamingGroup = ref('')

// 删除确认弹窗状态（分组/连接共用）
const showDeleteConfirm = ref(false)
const deleteConfirmMessage = ref('')
let pendingDeleteAction: (() => void) | null = null

function askDeleteConfirm(message: string, action: () => void) {
  deleteConfirmMessage.value = message
  pendingDeleteAction = action
  showDeleteConfirm.value = true
}

function confirmDelete() {
  pendingDeleteAction?.()
  pendingDeleteAction = null
  showDeleteConfirm.value = false
}

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
  renamingGroup.value = ''
  showAddGroupDialog.value = true
}

// 分组名校验：非空且不重复（重命名时允许保持原名）
function validateGroupName(value: string): string | true {
  if (!value) return '分组名称不能为空'
  if (value !== renamingGroup.value && allGroupNames.value.includes(value)) {
    return '分组名称已存在'
  }
  return true
}

function handleAddGroup(value: string) {
  if (connectionStore.addGroup(value)) {
    expandedGroups.value.push(value)
  }
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
  renamingGroup.value = groupContextMenu.value.groupName
  closeAllContextMenus()
  showRenameGroupDialog.value = true
}

function handleRenameGroup(value: string) {
  connectionStore.renameGroup(renamingGroup.value, value)
}

function handleGroupDelete() {
  const name = groupContextMenu.value.groupName
  closeAllContextMenus()
  askDeleteConfirm(`确定删除分组 "${name}" 吗？\n该分组下的连接将移到"默认分组"。`, () => {
    connectionStore.deleteGroup(name)
  })
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
  askDeleteConfirm(`确定删除连接 "${conn.name}" 吗？`, () => {
    connectionStore.deleteConnection(conn.id)
  })
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
  font-family: var(--font-family-base);

  // 侧边栏根背景用 sidebarBg（与 Layout/.panel-area 同源），保证 side 色板协调
  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
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

.header-actions {
  display: flex;
  align-items: center;
  gap: 4px;
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

// ===== 主体 =====
.panel-body {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  scrollbar-width: thin;

  &::-webkit-scrollbar { width: 6px; }
  &::-webkit-scrollbar-thumb {
    background-color: rgba(155, 155, 155, 0.2);
    border-radius: var(--radius-sm);
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
  font-size: theme.getVar('fontSizeSm');
  font-weight: 600;
  color: #bbb;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  transition: background-color 0.1s;

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('activityBarHover');
    }
  }

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.group-chevron {
  font-size: theme.getVar('fontSizeXs');
  transition: transform 0.15s ease;
  flex-shrink: 0;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

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
  font-size: theme.getVar('fontSizeXs');
  font-weight: 400;
  padding: 0 5px;
  border-radius: var(--radius-xl);
  flex-shrink: 0;

  @include theme.useTheme {
    color: theme.getVar('textDisabled');
    background-color: theme.getVar('activityBarHover');
  }
}

.group-content {
  padding-left: 0;
}

.group-empty {
  padding: 4px 12px 4px 36px;
  font-size: theme.getVar('fontSizeSmall');
  font-style: italic;

  @include theme.useTheme {
    color: theme.getVar('textDisabled');
  }
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
  font-size: theme.getVar('fontSize');

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('activityBarHover');
    }
  }

  &.conn-item-active {
    @include theme.useTheme {
      background-color: theme.getVar('hoverColor');
    }
  }
}

.conn-icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  line-height: 0; // 消除 span 内联 SVG 的 baseline 空隙，确保图标真正垂直居中

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

// SVG 默认 display: inline 会引入 baseline 下空隙，统一 block 化，
// 让连接图标与 action-btn（终端/文件管理）里的 SVG 在同一水平中心线上
.conn-icon svg,
.action-btn svg {
  display: block;
}

.conn-name {
  flex: 1;
  min-width: 0; // 允许在 flex 中收缩（不写会被内容撑开，无法 ellipsis）
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: theme.getVar('fontSize');

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.conn-addr {
  font-size: theme.getVar('fontSizeSm');
  flex-shrink: 1;
  min-width: 0; // 允许收缩到 0（sidebar 极窄时先压缩地址而不是按钮）
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textDisabled');
  }
}

// 终端 / 文件管理按钮：永远可见，靠右悬浮，不被 sidebar 宽度挤压
// 视觉行为参考 panel-header 的「更多操作」：基础态低饱和、hover 时变清晰
.conn-hover-actions {
  display: flex;
  gap: 2px;
  flex-shrink: 0; // 关键：永远不被压缩，按钮优先级最高
  margin-left: auto; // 靠右悬浮（即使 conn-name / conn-addr 占满也保持靠右）
  opacity: 0.55;
  transition: opacity 0.1s;

  &:hover {
    opacity: 1;
  }
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.1s;
  flex-shrink: 0;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &:hover {
    @include theme.useTheme {
      color: theme.getVar('textColor');
      background-color: theme.getVar('activityBarHover');
      opacity: 1; // 抵消父级 0.55，确保单独 hover 按钮时完全清晰
    }
  }
}

// ===== 空状态 =====
.connection-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  gap: 12px;

  @include theme.useTheme {
    color: theme.getVar('textDisabled');
  }

  p { font-size: theme.getVar('fontSize'); margin: 0; }
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
  border-radius: var(--radius-lg);
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
  font-size: theme.getVar('fontSize');
  cursor: pointer;
  transition: background-color 0.1s;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('hoverColor');
      color: theme.getVar('activityBarActive');
    }
  }

  .el-icon {
    font-size: theme.getVar('fontSize');
  }
}

.context-menu-danger {
  color: var(--el-color-danger);

  &:hover {
    background-color: rgba(245, 108, 108, 0.15);
    color: var(--el-color-danger);
  }
}

.context-menu-active {
  color: var(--el-color-primary);
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
  font-size: theme.getVar('fontSize');
  cursor: pointer;
  transition: background-color 0.1s;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('hoverColor');
      color: theme.getVar('activityBarActive');
    }
  }

  .submenu-arrow {
    font-size: theme.getVar('fontSizeXs');

    @include theme.useTheme {
      color: theme.getVar('textDisabled');
    }
  }
}

.context-menu-submenu-list {
  display: none;
  position: absolute;
  left: 100%;
  top: 0;
  background-color: #2d2d2d;
  border: 1px solid #454545;
  border-radius: var(--radius-lg);
  padding: 4px 0;
  min-width: 140px;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.5);
  z-index: 10000;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-color: theme.getVar('borderColor');
  }
}

// ===== 设置区 =====
.sidebar-settings-collapse {
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  flex-shrink: 0;

  :deep(.el-collapse-item__header) {
    font-size: var(--font-size-base);
    height: 32px;
    line-height: 32px;
    padding: 0 12px;
    background: transparent;
    color: #bbb;
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

.context-menu-submenu:hover .context-menu-submenu-list {
  display: block;
}

// 删除确认文案：保留 \n 换行
.delete-confirm-message {
  white-space: pre-line;
  line-height: 1.6;
}
</style>
