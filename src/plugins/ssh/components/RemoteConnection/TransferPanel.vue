<template>
  <div class="transfer-panel" v-if="tasks.length > 0">
    <div class="transfer-header" @click="isExpanded = !isExpanded">
      <span class="transfer-title">
        <el-icon><ArrowDown v-if="!isExpanded" /><ArrowUp v-else /></el-icon>
        传输队列 ({{ activeCount }} 进行中 / {{ completedCount }} 已结束)
      </span>
      <div class="transfer-header-actions" @click.stop>
        <label class="concurrency-label" title="同时传输的文件数，改动立即生效">
          并发
          <el-select
            :model-value="ftpStore.concurrency"
            size="small"
            class="concurrency-select"
            @change="ftpStore.setConcurrency($event)"
          >
            <el-option v-for="n in 8" :key="n" :label="String(n)" :value="n" />
          </el-select>
        </label>
        <app-button size="small" @click="handleClearCompleted" :disabled="completedCount === 0">
          清除已结束
        </app-button>
      </div>
    </div>

    <div class="transfer-list" v-show="isExpanded">
      <div
        v-for="task in tasks"
        :key="task.id"
        class="transfer-item"
        :class="`transfer-${task.status}`"
      >
        <div class="transfer-info">
          <span class="transfer-icon">{{ task.type === 'upload' ? '⬆️' : '⬇️' }}</span>
          <span class="transfer-name" :title="task.localPath + ' → ' + task.remotePath">
            {{ task.fileName }}
          </span>
          <span class="transfer-size">{{ sizeText(task) }}</span>
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
            <span class="transfer-status-text" :class="`status-${task.status}`">
              {{ statusText(task.status) }}{{ task.resume && task.status === 'pending' ? '（续传）' : '' }}
            </span>
          </div>
        </div>

        <div class="transfer-error" v-if="task.error">{{ task.error }}</div>

        <div class="transfer-actions">
          <template v-if="task.status === 'transferring'">
            <app-button size="small" @click="ftpStore.pauseTask(props.tabId, task.id)">暂停</app-button>
            <app-button size="small" @click="ftpStore.cancelTask(props.tabId, task.id)">取消</app-button>
          </template>
          <template v-else-if="task.status === 'pending'">
            <app-button size="small" @click="ftpStore.cancelTask(props.tabId, task.id)">取消</app-button>
          </template>
          <template v-else-if="task.status === 'paused' || task.status === 'cancelled' || task.status === 'failed'">
            <app-button size="small" type="primary" plain @click="ftpStore.retryTask(props.tabId, task.id)">
              重试（续传）
            </app-button>
            <app-button size="small" @click="ftpStore.removeTask(props.tabId, task.id)">移除</app-button>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { ArrowDown, ArrowUp } from '@element-plus/icons-vue'
import { useFtpStore } from '../../store'
import type { TransferStatus, TransferTask } from '../../types'

const props = defineProps<{ tabId: string }>()

const ftpStore = useFtpStore()
const isExpanded = ref(false)

const panel = computed(() => ftpStore.getPanel(props.tabId))
const tasks = computed(() => panel.value?.transferTasks || [])

const activeCount = computed(() =>
  tasks.value.filter(t => t.status === 'transferring' || t.status === 'pending').length
)
const completedCount = computed(() =>
  tasks.value.filter(t => ['completed', 'failed', 'cancelled'].includes(t.status)).length
)

// 有传输任务进行中时自动展开队列，确保进度条与完成态可见
watch(activeCount, (count) => { if (count > 0) isExpanded.value = true })

function progressStatus(status: TransferStatus): '' | 'success' | 'exception' | 'warning' {
  switch (status) {
    case 'completed': return 'success'
    case 'failed': return 'exception'
    case 'cancelled':
    case 'paused': return 'warning'
    default: return ''
  }
}

function statusText(status: TransferStatus): string {
  const map: Record<TransferStatus, string> = {
    pending: '等待中',
    transferring: '传输中',
    paused: '已暂停',
    completed: '已完成',
    failed: '失败',
    cancelled: '已取消'
  }
  return map[status] || ''
}

function formatSize(bytes: number): string {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)))
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`
}

function sizeText(task: TransferTask): string {
  const done = formatSize(task.transferredSize)
  return task.totalSize > 0 ? `${done} / ${formatSize(task.totalSize)}` : done
}

function handleClearCompleted() {
  ftpStore.clearCompletedTasks(props.tabId)
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.transfer-panel {
  flex-shrink: 0;
  max-height: 240px;
  display: flex;
  flex-direction: column;
  overflow: hidden;

  @include theme.useTheme {
    background-color: theme.getVar('appMainBgColor');
    border-top: 1px solid theme.getVar('borderColor');
  }
}

.transfer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 12px;
  cursor: pointer;
  user-select: none;
  flex-shrink: 0;

  @include theme.useTheme { background-color: theme.getVar('sidebarBg'); }
  &:hover { @include theme.useTheme { background-color: theme.getVar('activityBarHover'); } }
}

.transfer-title {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: theme.getVar('fontSizeSmall');
  @include theme.useTheme { color: theme.getVar('textColor'); }
}

.transfer-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.concurrency-label {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: theme.getVar('fontSizeSmall');
  @include theme.useTheme { color: theme.getVar('textSecondary'); }
}

.concurrency-select { width: 62px; }

.transfer-list {
  flex: 1;
  overflow-y: auto;
  scrollbar-width: thin;
  &::-webkit-scrollbar { width: 4px; }
  &::-webkit-scrollbar-thumb {
    background-color: rgba(155, 155, 155, 0.3);
    border-radius: var(--radius-xs);
  }
}

.transfer-item {
  padding: 6px 12px;
  @include theme.useTheme { border-bottom: 1px solid theme.getVar('borderColor'); }
}

.transfer-info {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.transfer-icon { font-size: theme.getVar('fontSizeSmall'); }

.transfer-name {
  flex: 1;
  min-width: 0;
  font-size: theme.getVar('fontSizeSmall');
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  @include theme.useTheme { color: theme.getVar('textColor'); }
}

.transfer-size,
.transfer-direction {
  flex-shrink: 0;
  font-size: theme.getVar('fontSizeSm');
  @include theme.useTheme { color: theme.getVar('textSecondary'); }
}

.transfer-progress { margin-bottom: 2px; }

.transfer-meta {
  display: flex;
  justify-content: space-between;
  font-size: theme.getVar('fontSizeSm');
  margin-top: 2px;
  @include theme.useTheme { color: theme.getVar('textSecondary'); }
}

.transfer-status-text {
  &.status-completed { color: #4ec9b0; }
  &.status-failed { color: #f44747; }
  &.status-paused, &.status-cancelled { color: #e2b714; }
}

.transfer-error {
  font-size: theme.getVar('fontSizeSm');
  color: #f44747;
  margin-top: 2px;
}

.transfer-actions {
  display: flex;
  gap: 6px;
  margin-top: 4px;
}
</style>
