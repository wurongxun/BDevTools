<template>
  <div class="status-bar">
    <div class="status-bar-left">
      <div class="status-item workspace-item" @click="showWorkspaceDialog = true" title="点击切换工作区">
        <svg class="status-icon" viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
          <path d="M1.5 2h5l1 1.5h7v10h-13z" fill="none" stroke="currentColor" stroke-width="1.2"/>
        </svg>
        <span class="workspace-name">{{ workspaceName }}</span>
      </div>
    </div>
    <div class="status-bar-right">
      <div class="status-item" v-if="workingDir" :title="workingDir">
        <svg class="status-icon" viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
          <path d="M1 3h6l1.5 2H15v9H1z" fill="none" stroke="currentColor" stroke-width="1.2"/>
        </svg>
        <span class="dir-name">{{ dirName }}</span>
      </div>
    </div>

    <!-- 工作区管理对话框 -->
    <teleport to="body">
      <div v-if="showWorkspaceDialog" class="statusbar-ws-overlay" @click="showWorkspaceDialog = false">
        <div class="statusbar-ws-dialog" @click.stop>
          <div class="sws-header">
            <span class="sws-title">管理工作区</span>
            <span class="sws-close" @click="showWorkspaceDialog = false">&times;</span>
          </div>
          <div class="sws-body">
            <div v-if="workspaceList.length === 0" class="sws-empty">暂无工作区</div>
            <div
              v-for="ws in workspaceList"
              :key="ws.id"
              class="sws-item"
              :class="{ 'sws-active': currentWsId === ws.id }"
            >
              <!-- 编辑模式 -->
              <template v-if="editingId === ws.id">
                <div class="sws-edit-row">
                  <input
                    class="sws-edit-input"
                    v-model="editingName"
                    @keyup.enter="confirmRename(ws)"
                    @keyup.escape="cancelRename"
                    ref="editInputRef"
                  />
                  <span class="sws-btn sws-btn-confirm" @click="confirmRename(ws)">✓</span>
                  <span class="sws-btn" @click="cancelRename">✗</span>
                </div>
              </template>
              <!-- 普通模式 -->
              <template v-else>
                <div class="sws-item-main" @click="handleSelectWorkspace(ws)">
                  <div class="sws-item-name">
                    {{ ws.name }}
                    <span v-if="currentWsId === ws.id" class="sws-badge">当前</span>
                  </div>
                  <div class="sws-item-folders">{{ ws.folders.join(', ') || '无关联文件夹' }}</div>
                </div>
                <div class="sws-item-actions">
                  <span class="sws-btn" title="重命名" @click.stop="startRename(ws)">✏</span>
                  <span class="sws-btn sws-btn-danger" title="删除" @click.stop="handleDeleteWorkspace(ws)">🗑</span>
                </div>
              </template>
            </div>
          </div>
          <div class="sws-footer">
            <span class="sws-create" @click="handleCreateWorkspace">+ 新建工作区</span>
          </div>
        </div>
      </div>
    </teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue'
import { useWorkspaceStore } from '@/store/modules/workspace'
import { workspaceService } from '@/services/workspaceService'
import useStore from '@/store'
import { open } from '@tauri-apps/plugin-dialog'
import NotificationUtil from '@/utils/NotificationUtil'
import type { WorkspaceSummary } from '@/Types/workspace'

const workspaceStore = useWorkspaceStore()
const { fileTreeStore } = useStore()

const showWorkspaceDialog = ref(false)
const workspaceList = ref<WorkspaceSummary[]>([])

// 重命名状态
const editingId = ref<string | null>(null)
const editingName = ref('')
const editInputRef = ref<HTMLInputElement | null>(null)

const workspaceName = computed(() => workspaceStore.currentWorkspace?.name ?? '默认工作区')
const workingDir = computed(() => fileTreeStore.workingDirectory)
const dirName = computed(() => {
  if (!workingDir.value) return ''
  const parts = workingDir.value.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1] || workingDir.value
})
const currentWsId = computed(() => workspaceStore.currentWorkspace?.id ?? '')

async function loadWorkspaceList() {
  workspaceList.value = await workspaceService.list()
}

async function handleSelectWorkspace(ws: WorkspaceSummary) {
  const result = await workspaceStore.switchWorkspace(ws.id)
  showWorkspaceDialog.value = false
  if (result.success) {
    NotificationUtil.success(`已切换到工作区: ${ws.name}`)
  }
}

async function handleCreateWorkspace() {
  showWorkspaceDialog.value = false
  const folder = await open({ directory: true, multiple: false })
  if (!folder || typeof folder !== 'string') return
  const parts = folder.replace(/\\/g, '/').split('/')
  const name = parts[parts.length - 1] || folder
  const config = await workspaceStore.createWorkspace(name, [folder])
  if (config) {
    const result = await workspaceStore.switchWorkspace(config.id)
    await loadWorkspaceList()
    if (result.success) {
      NotificationUtil.success(`工作区已创建: ${config.name}`)
    }
  }
}

// ===== 重命名 =====
function startRename(ws: WorkspaceSummary) {
  editingId.value = ws.id
  editingName.value = ws.name
  nextTick(() => {
    editInputRef.value?.focus()
    editInputRef.value?.select()
  })
}

function cancelRename() {
  editingId.value = null
  editingName.value = ''
}

async function confirmRename(ws: WorkspaceSummary) {
  const newName = editingName.value.trim()
  if (!newName || newName === ws.name) {
    cancelRename()
    return
  }
  const updated = await workspaceService.update(ws.id, { name: newName })
  if (updated) {
    NotificationUtil.success(`工作区已重命名: ${newName}`)
    await loadWorkspaceList()
    // 如果重命名的是当前工作区，刷新 store
    if (ws.id === workspaceStore.currentWorkspace?.id) {
      await workspaceStore.loadCurrentWorkspace(ws.id)
    }
  } else {
    NotificationUtil.error('重命名失败')
  }
  cancelRename()
}

// ===== 删除 =====
async function handleDeleteWorkspace(ws: WorkspaceSummary) {
  if (ws.id === workspaceStore.currentWorkspace?.id) {
    NotificationUtil.warn('无法删除当前正在使用的工作区')
    return
  }
  const confirmed = window.confirm(`确定删除工作区「${ws.name}」吗？\n该操作不可恢复。`)
  if (!confirmed) return

  await workspaceStore.deleteWorkspace(ws.id)
  NotificationUtil.success(`工作区已删除: ${ws.name}`)
  await loadWorkspaceList()
}

onMounted(async () => {
  await loadWorkspaceList()
  if (!workspaceStore.currentWorkspace) {
    await workspaceStore.loadCurrentWorkspace(null)
  }
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 22px;
  padding: 0 8px;
  font-size: 11px;
  user-select: none;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('activityBarBg');
    color: theme.getVar('textSecondary');
    border-top: 1px solid theme.getVar('borderColor');
  }
}

.status-bar-left,
.status-bar-right {
  display: flex;
  align-items: center;
  gap: 2px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0 6px;
  height: 22px;
  cursor: default;
  border-radius: 2px;
  transition: background-color 0.15s;

  &:hover {
    @include theme.useTheme {
      color: theme.getVar('textColor');
      background: theme.getVar('activityBarHover');
    }
  }
}

.workspace-item {
  cursor: pointer;
}

.status-icon {
  flex-shrink: 0;
  opacity: 0.85;
}

.workspace-name {
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dir-name {
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

// ===== 工作区对话框 =====
.statusbar-ws-overlay {
  position: fixed;
  inset: 0;
  z-index: 99999;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
}

.statusbar-ws-dialog {
  width: 400px;
  max-height: 440px;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderColor');
    color: theme.getVar('textColor');
  }
}

.sws-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.sws-title {
  font-size: 14px;
  font-weight: 600;
}

.sws-close {
  font-size: 20px;
  cursor: pointer;
  opacity: 0.6;
  line-height: 1;
  &:hover { opacity: 1; }
}

.sws-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.sws-empty {
  text-align: center;
  padding: 32px 0;
  opacity: 0.5;
  font-size: 13px;
}

.sws-item {
  padding: 10px 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: background-color 0.15s;
  margin-bottom: 4px;
  display: flex;
  align-items: center;
  justify-content: space-between;

  &:hover {
    @include theme.useTheme {
      background: theme.getVar('hoverColor');
    }
  }

  &.sws-active {
    @include theme.useTheme {
      background: theme.getVar('hoverColor');
      // 浅色主题的 hoverColor 过浅，用文字色做高对比激活指示条
      border-left: 3px solid theme.getVar('textColor');
    }
  }
}

.sws-item-main {
  flex: 1;
  min-width: 0;
}

.sws-item-actions {
  display: flex;
  gap: 4px;
  margin-left: 8px;
  flex-shrink: 0;
}

.sws-item-name {
  font-size: 13px;
  font-weight: 500;
  margin-bottom: 4px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.sws-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 3px;
  font-weight: 400;

  // 反色方案：两种主题下均保证高对比（浅色 hoverColor 太浅不适合做底色）
  @include theme.useTheme {
    background: theme.getVar('textColor');
    color: theme.getVar('bgColor');
  }
}

.sws-item-folders {
  font-size: 11px;
  opacity: 0.6;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

// 编辑模式
.sws-edit-row {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
}

.sws-edit-input {
  flex: 1;
  padding: 4px 8px;
  font-size: 13px;
  border-radius: 4px;
  outline: none;

  @include theme.useTheme {
    border: 1px solid theme.getVar('textSecondary');
    background: theme.getVar('bgColor');
    color: theme.getVar('textColor');
  }
}

// 操作按钮
.sws-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
  opacity: 0.6;
  transition: opacity 0.15s, background 0.15s;

  &:hover {
    opacity: 1;

    @include theme.useTheme {
      // 与导航栏图标按钮同一规格
      background: theme.getVar('activityBarHover');
    }
  }
}

.sws-btn-confirm {
  color: #52c41a;
}

.sws-btn-danger {
  &:hover { color: #ff4d4f; }
}

.sws-footer {
  padding: 10px 16px;

  @include theme.useTheme {
    border-top: 1px solid theme.getVar('borderColor');
  }
}

.sws-create {
  font-size: 12px;
  cursor: pointer;
  opacity: 0.7;

  &:hover {
    opacity: 1;

    @include theme.useTheme {
      color: theme.getVar('textColor');
    }
  }
}
</style>
