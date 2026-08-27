<!-- src/components/WorkspaceManageDialog.vue -->
<!-- 共享的工作区管理弹窗：基于 GlobalDialog 统一风格，
     整合 MenuBar / AppStatusBar 中重复的管理界面（切换/重命名/删除/新建） -->
<template>
  <GlobalDialog v-model="visible" title="管理工作区" width="420px">
    <div v-if="workspaceList.length === 0" class="ws-empty">暂无工作区</div>
    <div
      v-for="ws in workspaceList"
      :key="ws.id"
      class="ws-item"
      :class="{ 'ws-item-active': currentWsId === ws.id }"
    >
      <!-- 编辑模式 -->
      <template v-if="editingId === ws.id">
        <div class="ws-edit-row">
          <input
            class="ws-edit-input"
            v-model="editingName"
            @keyup.enter="confirmRename(ws)"
            @keyup.escape="cancelRename"
            ref="editInputRef"
          />
          <span class="ws-btn ws-btn-confirm" @click="confirmRename(ws)">✓</span>
          <span class="ws-btn" @click="cancelRename">✗</span>
        </div>
      </template>
      <!-- 普通模式 -->
      <template v-else>
        <div class="ws-item-main" @click="handleSelectWorkspace(ws)">
          <div class="ws-item-name">
            {{ ws.name }}
            <span v-if="currentWsId === ws.id" class="ws-badge">当前</span>
          </div>
          <div class="ws-item-folders">{{ ws.folders.join(', ') || '无关联文件夹' }}</div>
        </div>
        <div class="ws-item-actions">
          <span class="ws-btn" title="重命名" @click.stop="startRename(ws)">✏</span>
          <span class="ws-btn ws-btn-danger" title="删除" @click.stop="requestDelete(ws)">🗑</span>
        </div>
      </template>
    </div>
    <template #footer>
      <span class="ws-create-link" @click="handleCreateWorkspace">+ 新建工作区</span>
    </template>
  </GlobalDialog>

  <!-- 删除确认 -->
  <GlobalDialog v-model="showDeleteConfirm" title="确认删除" width="400px">
    <div class="ws-delete-message">
      确定删除工作区「{{ deletingWs?.name }}」吗？
      <div class="ws-delete-sub">该操作不可恢复。</div>
    </div>
    <template #footer>
      <el-button size="small" @click="showDeleteConfirm = false">取消</el-button>
      <el-button size="small" type="danger" plain @click="confirmDelete">删除</el-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { open } from '@/core/runtime'
import GlobalDialog from './GlobalDialog.vue'
import { useWorkspaceStore } from '@/store/modules/workspace'
import { workspaceService } from '@/services/workspaceService'
import NotificationUtil from '@/utils/NotificationUtil'
import type { WorkspaceSummary } from '@/Types/workspace'

interface Props {
  modelValue: boolean
}

const props = defineProps<Props>()

interface Emits {
  (e: 'update:modelValue', value: boolean): void
  /** 工作区列表发生变化（新建/删除/重命名后），供父组件刷新自身列表 */
  (e: 'list-changed'): void
}

const emit = defineEmits<Emits>()

const workspaceStore = useWorkspaceStore()

const visible = ref<boolean>(props.modelValue)
const workspaceList = ref<WorkspaceSummary[]>([])

// 重命名状态
const editingId = ref<string | null>(null)
const editingName = ref('')
const editInputRef = ref<HTMLInputElement | null>(null)

// 删除确认状态
const showDeleteConfirm = ref(false)
const deletingWs = ref<WorkspaceSummary | null>(null)

const currentWsId = computed(() => workspaceStore.currentWorkspace?.id ?? '')

watch(() => props.modelValue, (val: boolean) => {
  visible.value = val
})

watch(visible, async (val: boolean) => {
  emit('update:modelValue', val)
  if (val) {
    await loadWorkspaceList()
  } else {
    cancelRename()
  }
})

async function loadWorkspaceList() {
  workspaceList.value = await workspaceService.list()
}

async function handleSelectWorkspace(ws: WorkspaceSummary) {
  const result = await workspaceStore.switchWorkspace(ws.id)
  visible.value = false
  if (result.success) {
    NotificationUtil.success(`已切换到工作区: ${ws.name}`)
  }
}

async function handleCreateWorkspace() {
  visible.value = false
  // 打开工作目录选择
  const folder = await open({ directory: true, multiple: false })
  if (!folder || typeof folder !== 'string') return
  const parts = folder.replace(/\\/g, '/').split('/')
  const name = parts[parts.length - 1] || folder
  const config = await workspaceStore.createWorkspace(name, [folder])
  if (config) {
    const result = await workspaceStore.switchWorkspace(config.id)
    await loadWorkspaceList()
    emit('list-changed')
    if (result.success) {
      NotificationUtil.success(`工作区已创建并切换: ${config.name}`)
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
    emit('list-changed')
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
function requestDelete(ws: WorkspaceSummary) {
  if (ws.id === workspaceStore.currentWorkspace?.id) {
    NotificationUtil.warn('无法删除当前正在使用的工作区')
    return
  }
  deletingWs.value = ws
  showDeleteConfirm.value = true
}

async function confirmDelete() {
  const ws = deletingWs.value
  showDeleteConfirm.value = false
  deletingWs.value = null
  if (!ws) return
  await workspaceStore.deleteWorkspace(ws.id)
  NotificationUtil.success(`工作区已删除: ${ws.name}`)
  await loadWorkspaceList()
  emit('list-changed')
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.ws-empty {
  text-align: center;
  padding: 32px 0;
  opacity: 0.5;
  font-size: var(--font-size-md);
}

.ws-item {
  padding: 10px 12px;
  border-radius: var(--radius-lg);
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

  &.ws-item-active {
    @include theme.useTheme {
      background: theme.getVar('hoverColor');
      // 浅色主题的 hoverColor 过浅，用文字色做高对比激活指示条
      border-left: 3px solid theme.getVar('textColor');
    }
  }
}

.ws-item-main {
  flex: 1;
  min-width: 0;
}

.ws-item-actions {
  display: flex;
  gap: 4px;
  margin-left: 8px;
  flex-shrink: 0;
}

.ws-item-name {
  font-size: var(--font-size-md);
  font-weight: 500;
  margin-bottom: 4px;
  display: flex;
  align-items: center;
  gap: 6px;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.ws-badge {
  font-size: var(--font-size-xs);
  padding: 1px 6px;
  border-radius: var(--radius-sm);
  font-weight: 400;

  // 反色方案：两种主题下均保证高对比（浅色 hoverColor 太浅不适合做底色）
  @include theme.useTheme {
    background: theme.getVar('textColor');
    color: theme.getVar('bgColor');
  }
}

.ws-item-folders {
  font-size: var(--font-size-sm);
  opacity: 0.6;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

// 编辑模式
.ws-edit-row {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
}

.ws-edit-input {
  flex: 1;
  padding: 4px 8px;
  font-size: var(--font-size-md);
  border-radius: var(--radius-md);
  outline: none;

  @include theme.useTheme {
    border: 1px solid theme.getVar('textSecondary');
    background: theme.getVar('bgColor');
    color: theme.getVar('textColor');
  }
}

// 操作按钮
.ws-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: var(--font-size-md);
  opacity: 0.6;
  transition: opacity 0.15s, background 0.15s;

  &:hover {
    opacity: 1;

    @include theme.useTheme {
      background: theme.getVar('activityBarHover');
    }
  }
}

.ws-btn-confirm {
  color: #52c41a;
}

.ws-btn-danger {
  &:hover { color: #ff4d4f; }
}

.ws-create-link {
  font-size: var(--font-size-base);
  cursor: pointer;
  opacity: 0.7;
  padding: 0 9px;

  &:hover {
    opacity: 1;

    @include theme.useTheme {
      color: theme.getVar('textColor');
    }
  }
}

// 删除确认文案
.ws-delete-message {
  line-height: 1.6;
}

.ws-delete-sub {
  margin-top: 4px;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
    font-size: theme.getVar('fontSizeSmall');
  }
}
</style>

