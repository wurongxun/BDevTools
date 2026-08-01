<template>
  <div class="ssh-sidebar-panel">
    <div class="panel-header">
      <span class="panel-title">SSH 连接</span>
      <el-button size="small" @click="showConnectionDialog = true">
        <el-icon><Plus /></el-icon>
      </el-button>
    </div>
    <div class="panel-body">
      <div class="connection-list">
        <template v-if="groupedConnections.length > 0">
          <div
            v-for="group in groupedConnections"
            :key="group.name"
            class="connection-group"
          >
            <div class="group-header" @click="toggleGroup(group.name)">
              <el-icon
                class="group-arrow"
                :class="{ 'group-arrow-expanded': expandedGroups.includes(group.name) }"
              >
                <ArrowRight />
              </el-icon>
              <span class="group-name">{{ group.name || '默认分组' }}</span>
              <span class="group-count">{{ group.connections.length }}</span>
            </div>
            <div class="group-items" v-show="expandedGroups.includes(group.name)">
              <div
                v-for="conn in group.connections"
                :key="conn.id"
                class="connection-item"
                @click="handleConnectionClick(conn)"
              >
                <div class="conn-info">
                  <span class="conn-name">{{ conn.name }}</span>
                  <span class="conn-address">{{ conn.address }}:{{ conn.port }}</span>
                </div>
                <div class="conn-actions">
                  <el-button
                    size="small"
                    link
                    @click.stop="openTerminal(conn)"
                    title="打开终端"
                  >
                    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                      <path d="M2 3l5 5-5 5" stroke="currentColor" stroke-width="1.5" fill="none" />
                      <line x1="8" y1="13" x2="14" y2="13" stroke="currentColor" stroke-width="1.5" />
                    </svg>
                  </el-button>
                  <el-button
                    size="small"
                    link
                    @click.stop="openFtp(conn)"
                    title="文件管理"
                  >
                    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                      <path d="M2 3h12v10H2V3z" fill="none" stroke="currentColor" stroke-width="1" />
                      <line x1="2" y1="6" x2="14" y2="6" stroke="currentColor" stroke-width="1" />
                    </svg>
                  </el-button>
                  <el-dropdown
                    trigger="click"
                    @command="(cmd: string) => handleConnCommand(cmd, conn)"
                  >
                    <el-button size="small" link @click.stop>
                      <el-icon><MoreFilled /></el-icon>
                    </el-button>
                    <template #dropdown>
                      <el-dropdown-menu>
                        <el-dropdown-item command="edit">编辑</el-dropdown-item>
                        <el-dropdown-item command="delete" divided>删除</el-dropdown-item>
                      </el-dropdown-menu>
                    </template>
                  </el-dropdown>
                </div>
              </div>
            </div>
          </div>
        </template>
        <div v-else class="connection-empty">
          <p>暂无连接</p>
          <el-button size="small" @click="showConnectionDialog = true">新建连接</el-button>
        </div>
      </div>
    </div>

    <!-- 连接管理对话框 -->
    <ConnectionDialog
      v-model="showConnectionDialog"
      :edit-data="editingConnection"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { Plus, ArrowRight, MoreFilled } from '@element-plus/icons-vue'
import { ElMessageBox } from 'element-plus'
import ConnectionDialog from './components/RemoteConnection/ConnectionDialog.vue'
import useStore from '@/store'
import type { RemoteConnection } from '@/entity/entitys'

const { useConnectionStore } = useStore()

const showConnectionDialog = ref(false)
const editingConnection = ref<RemoteConnection | null>(null)
const expandedGroups = ref<string[]>([])

const connections = computed(() => useConnectionStore.connections)

const groupedConnections = computed(() => {
  const groups = new Map<string, RemoteConnection[]>()
  for (const conn of connections.value) {
    const groupName = conn.favorites_name || '默认分组'
    if (!groups.has(groupName)) {
      groups.set(groupName, [])
    }
    groups.get(groupName)!.push(conn)
  }
  return Array.from(groups.entries()).map(([name, conns]) => ({
    name,
    connections: conns,
  }))
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
  openTerminal(conn)
}

function openTerminal(conn: RemoteConnection) {
  useConnectionStore.openTab(conn, 'terminal')
}

function openFtp(conn: RemoteConnection) {
  useConnectionStore.openTab(conn, 'ftp')
}

function handleConnCommand(command: string, conn: RemoteConnection) {
  if (command === 'edit') {
    editingConnection.value = conn
    showConnectionDialog.value = true
  } else if (command === 'delete') {
    ElMessageBox.confirm(`确定删除连接 "${conn.name}" 吗？`, '确认删除', {
      type: 'warning',
    })
      .then(() => {
        useConnectionStore.deleteConnection(conn.id)
      })
      .catch(() => {})
  }
}

onMounted(() => {
  useConnectionStore.loadConnections()
  expandedGroups.value = groupedConnections.value.map((g) => g.name)
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.ssh-sidebar-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  flex-shrink: 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.panel-title {
  font-size: 12px;
  font-weight: 500;
  color: #ccc;
  text-transform: uppercase;
  letter-spacing: 0.5px;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.panel-body {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  scrollbar-width: thin;

  &::-webkit-scrollbar { width: 6px; }
  &::-webkit-scrollbar-thumb {
    background-color: rgba(155, 155, 155, 0.3);
    border-radius: 3px;
    &:hover { background-color: rgba(155, 155, 155, 0.5); }
  }
  &::-webkit-scrollbar-track { background: transparent; }
}

.connection-list { padding: 4px 0; }
.connection-group { margin-bottom: 2px; }

.group-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  cursor: pointer;
  user-select: none;
  font-size: 12px;
  color: #aaa;

  &:hover { background-color: rgba(255, 255, 255, 0.03); }

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.group-arrow { transition: transform 0.2s; font-size: 10px; }
.group-arrow-expanded { transform: rotate(90deg); }
.group-name { flex: 1; font-weight: 500; }

.group-count {
  font-size: 11px;
  color: #666;
  background-color: rgba(255, 255, 255, 0.05);
  padding: 0 6px;
  border-radius: 8px;
}

.connection-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px 6px 28px;
  cursor: pointer;
  transition: background-color 0.15s;

  &:hover {
    background-color: rgba(255, 255, 255, 0.05);
    .conn-actions { opacity: 1; }
  }
}

.conn-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.conn-name {
  font-size: 13px;
  color: #ccc;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.conn-address { font-size: 11px; color: #666; }

.conn-actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.15s;
  flex-shrink: 0;
}

.connection-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  color: #666;
  gap: 12px;

  p { font-size: 13px; margin: 0; }
}
</style>
