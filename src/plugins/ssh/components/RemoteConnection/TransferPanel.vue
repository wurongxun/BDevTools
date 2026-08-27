<template>
  <div class="transfer-panel" v-if="tasks.length > 0">
    <div class="transfer-header" @click="isExpanded = !isExpanded">
      <span class="transfer-title">
        <el-icon><ArrowDown v-if="!isExpanded" /><ArrowUp v-else /></el-icon>
        传输队列 ({{ activeCount }} 进行中 / {{ completedCount }} 已完成)
      </span>
      <el-button
        size="small"
        @click.stop="handleClearCompleted"
        :disabled="completedCount === 0"
      >
        清除已完成
      </el-button>
    </div>

    <div class="transfer-list" v-show="isExpanded">
      <div
        v-for="task in tasks"
        :key="task.id"
        class="transfer-item"
        :class="`transfer-${task.status}`"
      >
        <div class="transfer-info">
          <span class="transfer-icon">
            {{ task.type === 'upload' ? '⬆️' : '⬇️' }}
          </span>
          <span class="transfer-name">{{ task.fileName }}</span>
          <span class="transfer-direction">
            {{ task.type === 'upload' ? '→ 远程' : '← 本地' }}
          </span>
        </div>

        <div class="transfer-progress">
          <el-progress
            :percentage="task.progress"
            :status="progressStatus(task.status)"
            :stroke-width="6"
            :show-text="false"
          />
          <div class="transfer-meta">
            <span class="transfer-speed">{{ task.speed || '--' }}</span>
            <span class="transfer-status-text">{{ statusText(task.status) }}</span>
          </div>
        </div>

        <div class="transfer-error" v-if="task.error">
          {{ task.error }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { ArrowDown, ArrowUp } from '@element-plus/icons-vue'
import { useFtpStore } from '../../store'
import type { TransferTask, TransferStatus } from '../../types'

const props = defineProps<{
  tabId: string
}>()

const ftpStore = useFtpStore()

const isExpanded = ref(false)

const panel = computed(() => ftpStore.getPanel(props.tabId))
const tasks = computed(() => panel.value?.transferTasks || [])

const activeCount = computed(() =>
  tasks.value.filter(t => t.status === 'transferring' || t.status === 'pending').length
)

const completedCount = computed(() =>
  tasks.value.filter(t => t.status === 'completed' || t.status === 'failed').length
)

function progressStatus(status: TransferStatus): '' | 'success' | 'exception' | 'warning' {
  switch (status) {
    case 'completed': return 'success'
    case 'failed': return 'exception'
    case 'cancelled': return 'warning'
    default: return ''
  }
}

function statusText(status: TransferStatus): string {
  const map: Record<TransferStatus, string> = {
    pending: '等待中',
    transferring: '传输中',
    completed: '已完成',
    failed: '失败',
    cancelled: '已取消'
  }
  return map[status] || ''
}

function handleClearCompleted() {
  ftpStore.clearCompletedTasks(props.tabId)
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.transfer-panel {
  flex-shrink: 0;
  max-height: 200px;
  background-color: #252526;
  border-top: 1px solid #3e3e3e;
  display: flex;
  flex-direction: column;
  overflow: hidden;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-top-color: theme.getVar('borderColor');
  }
}

.transfer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 12px;
  cursor: pointer;
  user-select: none;
  background-color: #2d2d2d;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: rgba(255, 255, 255, 0.03);
  }

  &:hover {
    background-color: rgba(255, 255, 255, 0.05);
  }
}

.transfer-title {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: theme.getVar('fontSizeSmall');
  color: #ccc;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.transfer-list {
  flex: 1;
  overflow-y: auto;
  scrollbar-width: thin;

  &::-webkit-scrollbar {
    width: 4px;
  }
  &::-webkit-scrollbar-thumb {
    background-color: rgba(155, 155, 155, 0.3);
    border-radius: var(--radius-xs);
  }
}

.transfer-item {
  padding: 6px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.transfer-info {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.transfer-icon {
  font-size: theme.getVar('fontSizeSmall');
}

.transfer-name {
  flex: 1;
  font-size: theme.getVar('fontSizeSmall');
  color: #ccc;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.transfer-direction {
  font-size: theme.getVar('fontSizeSm');
  color: #888;
}

.transfer-progress {
  margin-bottom: 2px;
}

.transfer-meta {
  display: flex;
  justify-content: space-between;
  font-size: theme.getVar('fontSizeSm');
  color: #888;
  margin-top: 2px;
}

.transfer-status-text {
  &.transfer-completed {
    color: #4ec9b0;
  }
  &.transfer-failed {
    color: #f44747;
  }
}

.transfer-error {
  font-size: theme.getVar('fontSizeSm');
  color: #f44747;
  margin-top: 2px;
}
</style>
