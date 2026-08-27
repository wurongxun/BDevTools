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

    <!-- 工作区管理对话框（基于 GlobalDialog 的共享组件） -->
    <WorkspaceManageDialog v-model="showWorkspaceDialog" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useWorkspaceStore } from '@/store/modules/workspace'
import WorkspaceManageDialog from '@/components/WorkspaceManageDialog.vue'
import useStore from '@/store'

const workspaceStore = useWorkspaceStore()
const { fileTreeStore } = useStore()

const showWorkspaceDialog = ref(false)

const workspaceName = computed(() => workspaceStore.currentWorkspace?.name ?? '默认工作区')
const workingDir = computed(() => fileTreeStore.workingDirectory)
const dirName = computed(() => {
  if (!workingDir.value) return ''
  const parts = workingDir.value.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1] || workingDir.value
})

onMounted(async () => {
  // 新窗口等待用户选择工作区期间，不自动分配兜底工作区
  if (!workspaceStore.currentWorkspace && !workspaceStore.pendingWorkspaceSelection) {
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
  font-size: var(--font-size-sm);
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
  border-radius: var(--radius-xs);
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
</style>
