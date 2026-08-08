<script setup lang="ts">
import { ref, onMounted } from 'vue'
import type { SessionSnapshot, SessionSummary } from '@/Types/workspace'
import { sessionService } from '@/services/sessionService'

const emit = defineEmits<{
  (e: 'restore', snapshots: SessionSnapshot[]): void
  (e: 'skip'): void
}>()

const sessions = ref<SessionSummary[]>([])
const loading = ref(false)
const selectedLabels = ref<Set<string>>(new Set())

const hasSessions = ref(false)

async function loadSessions() {
  loading.value = true
  try {
    sessions.value = await sessionService.list()
    hasSessions.value = sessions.value.length > 0
  } catch (e) {
    console.error('加载会话列表失败:', e)
  } finally {
    loading.value = false
  }
}

function toggleSession(label: string) {
  if (selectedLabels.value.has(label)) {
    selectedLabels.value.delete(label)
  } else {
    selectedLabels.value.add(label)
  }
}

function selectAll() {
  for (const s of sessions.value) {
    selectedLabels.value.add(s.window_label)
  }
}

async function restoreSelected() {
  const labels = Array.from(selectedLabels.value)
  const snapshots: SessionSnapshot[] = []
  for (const label of labels) {
    const snap = await sessionService.load(label)
    if (snap) snapshots.push(snap)
  }
  emit('restore', snapshots)
}

function skip() {
  emit('skip')
}

async function clearAll() {
  await sessionService.clear()
  sessions.value = []
  hasSessions.value = false
  selectedLabels.value.clear()
}

function formatTime(timestamp: number): string {
  const date = new Date(timestamp)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const minutes = Math.floor(diff / 60000)
  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes} 分钟前`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours} 小时前`
  const days = Math.floor(hours / 24)
  return `${days} 天前`
}

function roleLabel(role: string): string {
  switch (role) {
    case 'main': return '主窗口'
    case 'editor': return '编辑器窗口'
    case 'auxiliary': return '辅助窗口'
    default: return role
  }
}

onMounted(loadSessions)
</script>

<template>
  <div class="session-restore">
    <div class="session-header">
      <h2>恢复上次会话</h2>
      <p class="session-desc">检测到上次未保存的编辑会话，是否恢复？</p>
    </div>

    <!-- 会话列表 -->
    <div v-if="hasSessions" class="session-list">
      <div
        v-for="s in sessions"
        :key="s.window_label"
        class="session-item"
        :class="{ selected: selectedLabels.has(s.window_label) }"
        @click="toggleSession(s.window_label)"
      >
        <div class="session-check">
          <input type="checkbox" :checked="selectedLabels.has(s.window_label)" @click.stop />
        </div>
        <div class="session-info">
          <div class="session-title">
            <span class="session-role">{{ roleLabel(s.window_role) }}</span>
            <span class="session-label">{{ s.window_label }}</span>
          </div>
          <div class="session-meta">
            <span>{{ s.open_file_count }} 个文件</span>
            <span>{{ formatTime(s.timestamp) }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 空状态 -->
    <div v-if="!hasSessions && !loading" class="session-empty">
      <p>没有找到可恢复的会话</p>
    </div>

    <!-- 操作按钮 -->
    <div class="session-footer">
      <div class="session-actions-left">
        <button v-if="hasSessions" class="btn btn-link" @click="selectAll">全选</button>
        <button v-if="hasSessions" class="btn btn-danger" @click="clearAll">清除所有</button>
      </div>
      <div class="session-actions-right">
        <button class="btn btn-secondary" @click="skip">跳过</button>
        <button
          class="btn btn-primary"
          :disabled="selectedLabels.size === 0"
          @click="restoreSelected"
        >
          恢复选中 ({{ selectedLabels.size }})
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss';

.session-restore {
  max-width: 560px;
  margin: 0 auto;
  padding: 32px 24px;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.session-header {
  margin-bottom: 24px;

  h2 {
    font-size: 20px;
    font-weight: 600;
    margin: 0 0 8px;
  }

  .session-desc {
    font-size: 13px;
    opacity: 0.6;
    margin: 0;
  }
}

.session-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 24px;
}

.session-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  border-radius: 6px;
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

  &.selected {
    border-color: $theme-color;
  }
}

.session-check {
  flex-shrink: 0;
}

.session-info {
  flex: 1;
  min-width: 0;
}

.session-title {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.session-role {
  font-size: 12px;
  padding: 1px 6px;
  border-radius: 3px;
  background-color: $theme-color;
  color: #fff;
}

.session-label {
  font-size: 13px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-meta {
  display: flex;
  gap: 12px;
  font-size: 12px;
  opacity: 0.5;
}

.session-empty {
  text-align: center;
  padding: 40px 0;
  opacity: 0.5;
  font-size: 14px;
}

.session-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.session-actions-left,
.session-actions-right {
  display: flex;
  gap: 8px;
}

.btn {
  padding: 8px 20px;
  border-radius: 4px;
  font-size: 13px;
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

  &.btn-danger {
    color: #e74c3c;
    background: none;
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
