<template>
  <GlobalDialog v-model="visible" title="宏：录制与回放" width="520px">
    <div class="macro-body">
      <p class="macro-hint">
        录制编辑器命令与输入片段，命名保存后可一键回放。不记录光标移动、选区与纯删除；
        回放会作用在当前编辑器上，请先确认目标文件。
      </p>

      <div class="macro-record-row">
        <app-button
          size="small"
          :type="recording ? 'danger' : 'primary'"
          @click="toggleRecording"
        >
          {{ recording ? `停止录制（${recordedCount} 步）` : '开始录制' }}
        </app-button>
        <app-button v-if="playing" size="small" @click="stopPlayback">停止回放</app-button>
        <span v-if="recording" class="recording-badge">● 录制中</span>
      </div>

      <div v-if="pendingSteps.length" class="macro-pending">
        <span class="pending-label">待保存 {{ pendingSteps.length }} 步</span>
        <el-input
          v-model="newName"
          size="small"
          placeholder="给宏起个名字"
          class="macro-name-input"
          @keyup.enter="savePending"
        />
        <app-button size="small" type="primary" @click="savePending">保存</app-button>
        <app-button size="small" text @click="discardPendingMacro">丢弃</app-button>
      </div>

      <div class="macro-section-title">已保存的宏（{{ macros.length }}）</div>
      <div class="macro-list">
        <div v-for="m in macros" :key="m.id" class="macro-item">
          <div class="macro-meta">
            <span class="macro-name">{{ m.name }}</span>
            <span class="macro-steps">{{ m.steps.length }} 步 · {{ formatDate(m.createdAt) }}</span>
          </div>
          <div class="macro-actions">
            <app-button size="small" :disabled="playing" @click="run(m.id)">回放</app-button>
            <app-button size="small" text @click="remove(m.id)">删除</app-button>
          </div>
        </div>
        <div v-if="!macros.length" class="macro-empty">还没有保存的宏。点击「开始录制」，做完一串操作后回来命名保存。</div>
      </div>
    </div>
    <template #footer>
      <app-button size="small" @click="visible = false">关闭</app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import {
  macroDialogVisible,
  macros,
  recording,
  playing,
  recordedCount,
  pendingSteps,
  toggleRecording,
  stopPlayback,
  savePendingMacro,
  discardPendingMacro,
  deleteMacro,
  runMacro
} from './useMacros'

const visible = computed({
  get: () => macroDialogVisible.value,
  set: (v: boolean) => { macroDialogVisible.value = v }
})

const newName = ref('')

function savePending() {
  if (savePendingMacro(newName.value)) newName.value = ''
}

function run(id: string) {
  void runMacro(id)
}

function remove(id: string) {
  deleteMacro(id)
}

function formatDate(ts: number): string {
  try {
    return new Date(ts).toLocaleDateString()
  } catch {
    return ''
  }
}
</script>

<style scoped lang="scss">
.macro-body {
  max-height: 420px;
  overflow-y: auto;
  padding-right: var(--space-2);
}

.macro-hint {
  margin: 0 0 var(--space-3) 0;
  font-size: var(--fontSizeSmall);
  color: var(--el-text-color-secondary);
  line-height: 1.6;
}

.macro-record-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.recording-badge {
  font-size: var(--fontSizeSmall);
  color: #d03050;
}

.macro-pending {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-top: var(--space-3);
  padding: var(--space-2);
  border: 1px solid var(--borderColor);
  border-radius: var(--radius-sm, 4px);
}

.pending-label {
  flex: none;
  font-size: var(--fontSizeSmall);
  color: var(--el-text-color-primary);
}

.macro-name-input {
  flex: 1;
  min-width: 0;
}

.macro-section-title {
  margin: var(--space-4) 0 var(--space-1) 0;
  font-size: var(--fontSizeSmall);
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.macro-list {
  border-top: 1px solid var(--borderColor);
}

.macro-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-2) 0;
  border-bottom: 1px solid var(--borderColor);
}

.macro-meta {
  display: flex;
  flex-direction: column;
  min-width: 0;
  gap: 2px;
}

.macro-name {
  font-size: var(--fontSizeSmall);
  color: var(--el-text-color-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.macro-steps {
  font-size: var(--fontSizeSmall);
  color: var(--el-text-color-secondary);
  opacity: 0.8;
}

.macro-actions {
  display: flex;
  flex: none;
  gap: var(--space-1);
}

.macro-empty {
  padding: var(--space-4) 0;
  font-size: var(--fontSizeSmall);
  color: var(--el-text-color-secondary);
  line-height: 1.6;
}
</style>
