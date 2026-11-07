<template>
  <div class="compose-bar">
    <div class="compose-head">
      <span class="compose-title">撰写栏</span>
      <el-switch
        v-model="broadcast"
        size="small"
        active-text="广播到所有终端"
        :disabled="connectedTerminalCount === 0"
      />
      <el-select v-if="!broadcast" v-model="target" size="small" class="compose-target">
        <el-option label="发送到主窗格" value="primary" />
        <el-option label="发送到副窗格" value="secondary" :disabled="!store.splitEnabled" />
      </el-select>
      <span class="compose-hint">{{ connectedTerminalCount }} 个已连接终端</span>
      <button class="compose-close" title="关闭撰写栏" aria-label="关闭撰写栏" @click="store.composeVisible = false">×</button>
    </div>
    <div class="compose-body">
      <el-input
        v-model="text"
        type="textarea"
        :rows="2"
        resize="none"
        placeholder="输入命令，Enter 发送 / Shift+Enter 换行；广播会同时发到所有已连接终端"
        @keydown="handleKeydown"
      />
      <div class="compose-actions">
        <app-button size="small" @click="clear">清空</app-button>
        <app-button size="small" type="primary" :disabled="!text.trim()" @click="send">
          {{ broadcast ? '广播发送' : '发送' }}
        </app-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useConnectionStore } from '../../store'
import { ConnectionStatus } from '../../types'

const store = useConnectionStore()

const text = ref('')
const broadcast = ref(false)
const target = ref<'primary' | 'secondary'>('primary')

const connectedTerminalCount = computed(() =>
  store.tabs.filter(t => t.type === 'terminal' && t.status === ConnectionStatus.CONNECTED).length
)

function targetTabId(): string | null {
  if (broadcast.value) return null
  return target.value === 'secondary' ? store.secondaryTabId : store.activeTabId
}

function send() {
  const value = text.value.replace(/\r?\n$/, '')
  if (!value.trim()) return
  store.sendCompose(value, broadcast.value, targetTabId())
}

function clear() {
  text.value = ''
}

/** Enter = 发送并保留文本（Xshell 撰写栏行为），Shift+Enter = 换行 */
function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault()
    send()
  }
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.compose-bar {
  flex-shrink: 0;
  padding: 4px 10px 8px;
  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    border-top: 1px solid theme.getVar('borderColor');
  }
}

.compose-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 4px;
}

.compose-title {
  font-size: theme.getVar('fontSizeSmall');
  font-weight: 500;
  @include theme.useTheme { color: theme.getVar('textColor'); }
}

.compose-target { width: 150px; }

.compose-hint {
  margin-left: auto;
  font-size: theme.getVar('fontSizeSm');
  @include theme.useTheme { color: theme.getVar('textDisabled'); }
}

.compose-close {
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: 16px;
  line-height: 1;
  padding: 0 4px;
  @include theme.useTheme { color: theme.getVar('textSecondary'); }
  &:hover { @include theme.useTheme { color: theme.getVar('textColor'); } }
}

.compose-body {
  display: flex;
  align-items: stretch;
  gap: 8px;
}

.compose-actions {
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  gap: 4px;
}
</style>
