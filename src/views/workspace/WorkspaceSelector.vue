<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@/api/rpc'
import { open } from '@/core/runtime'
import type { WorkspaceConfig, WorkspaceSummary } from '@/Types/workspace'
import type { ResponseData } from '@/entity/entitys'
import { workspaceService } from '@/services/workspaceService'

const emit = defineEmits<{
  (e: 'select', workspace: WorkspaceConfig): void
  (e: 'skip'): void
}>()

const workspaces = ref<WorkspaceSummary[]>([])
const loading = ref(false)
const showCreateForm = ref(false)
const newName = ref('')
const newFolders = ref<string[]>([])

const hasWorkspaces = computed(() => workspaces.value.length > 0)

async function loadWorkspaces() {
  loading.value = true
  try {
    workspaces.value = await workspaceService.list()
  } catch (e) {
    console.error('加载工作区列表失败:', e)
  } finally {
    loading.value = false
  }
}

async function selectWorkspace(ws: WorkspaceSummary) {
  const config = await workspaceService.get(ws.id)
  if (config) {
    emit('select', config)
  }
}

async function chooseFolder() {
  const selected = await open({ directory: true, multiple: false })
  if (selected && typeof selected === 'string') {
    newFolders.value = [selected]
  }
}

async function createAndSelect() {
  if (!newName.value.trim()) return
  const config = await workspaceService.create(newName.value.trim(), newFolders.value)
  if (config) {
    emit('select', config)
  }
}

function skip() {
  emit('skip')
}

function formatTime(timestamp: number): string {
  const date = new Date(timestamp)
  return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

onMounted(loadWorkspaces)
</script>

<template>
  <div class="workspace-selector">
    <div class="workspace-header">
      <h2>选择工作区</h2>
      <p class="workspace-desc">选择一个已有工作区打开，或创建新的工作区</p>
    </div>

    <!-- 工作区列表 -->
    <div v-if="hasWorkspaces && !showCreateForm" class="workspace-list">
      <div
        v-for="ws in workspaces"
        :key="ws.id"
        class="workspace-item"
        @click="selectWorkspace(ws)"
      >
        <div class="workspace-item-info">
          <span class="workspace-name">{{ ws.name }}</span>
          <span class="workspace-time">{{ formatTime(ws.updated_at) }}</span>
        </div>
        <div class="workspace-folders">
          <span v-for="folder in ws.folders" :key="folder" class="folder-tag">{{ folder }}</span>
        </div>
      </div>
    </div>

    <!-- 空状态 -->
    <div v-if="!hasWorkspaces && !showCreateForm" class="workspace-empty">
      <p>暂无工作区，请创建一个新工作区</p>
    </div>

    <!-- 创建表单 -->
    <div v-if="showCreateForm" class="workspace-create">
      <div class="form-group">
        <label>工作区名称</label>
        <input v-model="newName" placeholder="输入工作区名称" class="form-input" />
      </div>
      <div class="form-group">
        <label>关联文件夹</label>
        <div class="folder-picker">
          <span v-if="newFolders.length > 0" class="folder-tag">{{ newFolders[0] }}</span>
          <button class="btn-pick" @click="chooseFolder">选择文件夹</button>
        </div>
      </div>
      <div class="form-actions">
        <button class="btn btn-primary" :disabled="!newName.trim()" @click="createAndSelect">创建并打开</button>
        <button class="btn btn-secondary" @click="showCreateForm = false">取消</button>
      </div>
    </div>

    <!-- 底部操作 -->
    <div class="workspace-footer">
      <button v-if="!showCreateForm" class="btn btn-primary" @click="showCreateForm = true">
        新建工作区
      </button>
      <button class="btn btn-link" @click="skip">跳过，使用默认配置</button>
    </div>
  </div>
</template>

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss';

.workspace-selector {
  max-width: 560px;
  margin: 0 auto;
  padding: 32px 24px;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.workspace-header {
  margin-bottom: 24px;

  h2 {
    font-size: var(--font-size-2xl);
    font-weight: 600;
    margin: 0 0 8px;
  }

  .workspace-desc {
    font-size: var(--font-size-md);
    opacity: 0.6;
    margin: 0;
  }
}

.workspace-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 24px;
}

.workspace-item {
  padding: 12px 16px;
  border-radius: var(--radius-lg);
  cursor: pointer;
  transition: background-color 0.15s;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderColor');
  }

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('hoverBgColor', 'rgba(0,0,0,0.04)');
    }
  }
}

.workspace-item-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.workspace-name {
  font-size: var(--font-size-lg);
  font-weight: 500;
}

.workspace-time {
  font-size: var(--font-size-base);
  opacity: 0.5;
}

.workspace-folders {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.folder-tag {
  font-size: var(--font-size-sm);
  padding: 2px 8px;
  border-radius: var(--radius-sm);

  @include theme.useTheme {
    background-color: theme.getVar('hoverBgColor', 'rgba(0,0,0,0.04)');
    color: theme.getVar('textColor');
  }
}

.workspace-empty {
  text-align: center;
  padding: 40px 0;
  opacity: 0.5;
  font-size: var(--font-size-lg);
}

.workspace-create {
  margin-bottom: 24px;

  .form-group {
    margin-bottom: 16px;

    label {
      display: block;
      font-size: var(--font-size-md);
      font-weight: 500;
      margin-bottom: 6px;
    }
  }

  .form-input {
    width: 100%;
    padding: 8px 12px;
    border-radius: var(--radius-md);
    font-size: var(--font-size-md);
    box-sizing: border-box;

    @include theme.useTheme {
      border: 1px solid theme.getVar('borderColor');
      background-color: theme.getVar('bgColor');
      color: theme.getVar('textColor');
    }
  }

  .folder-picker {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn-pick {
    font-size: var(--font-size-base);
    padding: 4px 12px;
    border-radius: var(--radius-md);
    cursor: pointer;

    @include theme.useTheme {
      border: 1px solid theme.getVar('borderColor');
      background-color: theme.getVar('bgColor');
      color: theme.getVar('textColor');
    }
  }

  .form-actions {
    display: flex;
    gap: 8px;
  }
}

.workspace-footer {
  display: flex;
  align-items: center;
  gap: 16px;
}

.btn {
  padding: 8px 20px;
  border-radius: var(--radius-md);
  font-size: var(--font-size-md);
  cursor: pointer;
  border: none;

  &.btn-primary {
    background-color: $theme-color;
    color: #fff;

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }

  &.btn-secondary {
    @include theme.useTheme {
      background-color: theme.getVar('hoverBgColor', 'rgba(0,0,0,0.04)');
      color: theme.getVar('textColor');
    }
  }

  &.btn-link {
    background: none;
    opacity: 0.6;
    text-decoration: underline;

    @include theme.useTheme {
      color: theme.getVar('textColor');
    }
  }
}
</style>
