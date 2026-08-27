<template>
  <div class="terminal-view" ref="terminalContainer">
    <!-- 终端工具栏 -->
    <div class="terminal-toolbar">
      <div class="toolbar-left">
        <span class="terminal-title">{{ connectionName }}</span>
        <span
          class="terminal-status"
          :class="`status-${status}`"
        >
          {{ statusText }}
        </span>
      </div>
      <div class="toolbar-right">
        <button
          class="tool-btn"
          :class="status === 'connected' ? 'tool-btn-danger' : 'tool-btn-primary'"
          @click="handleConnectToggle"
          :disabled="status === 'connecting'"
          :title="status === 'connected' ? '断开连接' : '连接'"
        >
          <svg v-if="status === 'connecting'" class="tool-icon spinning" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1a7 7 0 0 1 7 7h-1.5A5.5 5.5 0 0 0 8 2.5V1z"/></svg>
          <svg v-else-if="status === 'connected'" class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M4.5 1a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 4.5 1zm7 0a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 11.5 1zM8 5a3 3 0 1 0 0 6 3 3 0 0 0 0-6z"/></svg>
          <svg v-else class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M4.5 1a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 4.5 1zm7 0a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 11.5 1zM8 5a3 3 0 1 0 0 6 3 3 0 0 0 0-6z"/></svg>
          <span class="tool-label">{{ status === 'connected' ? '断开' : status === 'connecting' ? '连接中' : '连接' }}</span>
        </button>
        <button
          class="tool-btn"
          @click="handleClear"
          :disabled="status !== 'connected'"
          title="清屏"
        >
          <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M2.5 1a.5.5 0 0 0 0 1h1.11l.96 9.61A2 2 0 0 0 6.56 13h2.88a2 2 0 0 0 1.99-1.39L12.39 2H13.5a.5.5 0 0 0 0-1H2.5zM5.64 2h4.72l-.86 8.61a1 1 0 0 1-1 .89H6.56a1 1 0 0 1-1-.89L4.64 2zM6 5a.5.5 0 0 1 .5.5v4a.5.5 0 0 1-1 0v-4A.5.5 0 0 1 6 5zm4 0a.5.5 0 0 1 .5.5v4a.5.5 0 0 1-1 0v-4A.5.5 0 0 1 10 5z"/></svg>
        </button>
        <button
          class="tool-btn"
          @click="handleReconnect"
          :disabled="status !== 'connected'"
          title="重连"
        >
          <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M11.534 7h3.932a.25.25 0 0 1 .192.41l-1.966 2.36a.25.25 0 0 1-.384 0l-1.966-2.36a.25.25 0 0 1 .192-.41zm-11 2h3.932a.25.25 0 0 0 .192-.41L2.692 6.23a.25.25 0 0 0-.384 0L.342 8.59A.25.25 0 0 0 .534 9z"/><path fill-rule="evenodd" d="M8 3c-1.552 0-2.94.707-3.857 1.818a.5.5 0 1 1-.771-.636A6.002 6.002 0 0 1 13.917 7H12.9A5.002 5.002 0 0 0 8 3zM3.1 9a5.002 5.002 0 0 0 8.757 2.182.5.5 0 1 1 .771.636A6.002 6.002 0 0 1 2.083 9H3.1z"/></svg>
        </button>
      </div>
    </div>

    <!-- xterm 终端容器 -->
    <div class="terminal-wrapper" ref="xtermContainer" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { invoke } from '@/api/rpc'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import '@xterm/xterm/css/xterm.css'
import { ConnectionStatus } from '../../../types'
import type { ConnectionTab, RemoteConnection } from '../../../types'
import { useConnectionStore } from '../../../store'
import { isDark } from '@/composables/useDark'

const props = defineProps<{
  tab: ConnectionTab
  connection: RemoteConnection
}>()

const connectionStore = useConnectionStore()

const terminalContainer = ref<HTMLElement>()
const xtermContainer = ref<HTMLElement>()
let terminal: Terminal | null = null
let fitAddon: FitAddon | null = null
let sshSessionId: string | null = null
let connectionCheckTimer: ReturnType<typeof setInterval> | null = null

const status = computed(() => props.tab.status)
const connectionName = computed(() => props.connection.name)

const statusText = computed(() => {
  switch (status.value) {
    case ConnectionStatus.DISCONNECTED: return '未连接'
    case ConnectionStatus.CONNECTING: return '连接中...'
    case ConnectionStatus.CONNECTED: return '已连接'
    case ConnectionStatus.ERROR: return '连接错误'
    default: return ''
  }
})

// xterm 终端主题
const terminalThemes = {
  dark: {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
    cursor: '#d4d4d4',
    selectionBackground: 'rgba(255, 255, 255, 0.3)',
    black: '#000000',
    red: '#cd3131',
    green: '#0dbc79',
    yellow: '#e5e510',
    blue: '#2472c8',
    magenta: '#bc3fbc',
    cyan: '#11a8cd',
    white: '#e5e5e5'
  },
  light: {
    background: '#ffffff',
    foreground: '#383a42',
    cursor: '#383a42',
    selectionBackground: 'rgba(0, 0, 0, 0.2)',
    black: '#383a42',
    red: '#e45649',
    green: '#50a14f',
    yellow: '#c18401',
    blue: '#0184bc',
    magenta: '#a626a4',
    cyan: '#0997b3',
    white: '#fafafa'
  }
}

function getTerminalTheme() {
  return isDark.value ? terminalThemes.dark : terminalThemes.light
}

function applyTerminalTheme() {
  if (!terminal) return
  const theme = getTerminalTheme()
  // 通过新对象赋值确保 xterm.js 触发响应式更新
  terminal.options.theme = { ...theme }
  // 强制刷新终端渲染
  terminal.refresh(0, terminal.rows - 1)
}

function initTerminal() {
  if (!xtermContainer.value) return

  terminal = new Terminal({
    cursorBlink: true,
    cursorStyle: 'block',
    fontSize: 14,
    fontFamily: "'Consolas', 'Courier New', monospace",
    theme: getTerminalTheme(),
    scrollback: 5000,
    convertEol: true
  })

  fitAddon = new FitAddon()
  terminal.loadAddon(fitAddon)
  terminal.loadAddon(new WebLinksAddon())

  terminal.open(xtermContainer.value)
  fitAddon.fit()

  // 监听终端输入（ onData 处理所有按键包括特殊键的转义序列）
  terminal.onData((data) => {
    if (sshSessionId && status.value === ConnectionStatus.CONNECTED) {
      invoke('ssh_write', {
        sessionId: sshSessionId,
        data: data
      }).catch(err => {
        console.error('SSH write error:', err)
      })
    }
  })

  // 确保终端可以接收焦点
  terminal.textarea?.setAttribute('tabindex', '0')

  // 监听后端 SSH 输出
  window.addEventListener('ssh-output', handleSshOutput as EventListener)

  // 监听窗口 resize
  window.addEventListener('resize', handleResize)

  // 显示欢迎信息
  terminal.writeln(`\x1b[33m欢迎使用远程终端连接\x1b[0m`)
  terminal.writeln(`\x1b[36m主机: ${props.connection.address}:${props.connection.port}\x1b[0m`)
  terminal.writeln(`\x1b[36m用户: ${props.connection.username}\x1b[0m`)
  terminal.writeln(`\x1b[90m点击 "连接" 按钮开始连接...\x1b[0m`)
  terminal.writeln('')
}

function handleSshOutput(event: CustomEvent) {
  const { tabId, data } = event.detail
  if (tabId === props.tab.id && terminal) {
    terminal.write(data)
  }
}

async function handleConnectToggle() {
  if (status.value === ConnectionStatus.CONNECTED) {
    await disconnect()
  } else {
    await connect()
  }
}

async function connect() {
  if (!terminal) return

  connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.CONNECTING)
  terminal.writeln(`\x1b[33m正在连接 ${props.connection.address}...\x1b[0m`)

  try {
    const res = await invoke<any>('ssh_connect', {
      tabId: props.tab.id,
      host: props.connection.address,
      port: props.connection.port,
      username: props.connection.username,
      password: props.connection.password,
      authType: props.connection.auth_type || 'password',
      privateKeyPath: props.connection.private_key_path || ''
    })

    if (res.code === 200) {
      sshSessionId = res.data
      connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.CONNECTED)

      // 初始适配终端大小后再连接 shell，确保 PTY 尺寸正确
      nextTick(() => {
        if (fitAddon) {
          fitAddon.fit()
          const dims = { cols: terminal!.cols, rows: terminal!.rows }
          invoke('ssh_resize_pty', { sessionId: sshSessionId, ...dims }).catch(() => {})
        }
        // 确保终端获得焦点，可以接收键盘输入
        terminal!.focus()
      })

      terminal.writeln(`\x1b[32m✓ 已连接到 ${props.connection.username}@${props.connection.address}\x1b[0m`)
      terminal.writeln('')

      // 启动连接状态定时检测
      startConnectionCheck()
    } else {
      connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.ERROR)
      terminal.writeln(`\x1b[31m连接失败: ${res.msg}\x1b[0m`)
    }
  } catch (error) {
    connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.ERROR)
    terminal.writeln(`\x1b[31m连接错误: ${error}\x1b[0m`)
  }
}

async function disconnect() {
  stopConnectionCheck()
  if (sshSessionId) {
    try {
      await invoke('ssh_disconnect', { sessionId: sshSessionId })
    } catch (error) {
      console.error('Disconnect error:', error)
    }
    sshSessionId = null
  }

  connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.DISCONNECTED)
  if (terminal) {
    terminal.writeln('\r\n\x1b[33m已断开连接\x1b[0m')
  }
}

async function handleReconnect() {
  await disconnect()
  await connect()
}

function handleClear() {
  if (terminal) {
    terminal.clear()
  }
}

function handleResize() {
  if (fitAddon) {
    fitAddon.fit()
    if (sshSessionId && terminal) {
      invoke('ssh_resize_pty', {
        sessionId: sshSessionId,
        cols: terminal.cols,
        rows: terminal.rows
      }).catch(() => {})
    }
  }
}

// 定时检测 SSH 连接状态
function startConnectionCheck() {
  stopConnectionCheck()
  connectionCheckTimer = setInterval(async () => {
    if (!sshSessionId || status.value !== ConnectionStatus.CONNECTED) return
    try {
      const res = await invoke<any>('ssh_check', { sessionId: sshSessionId })
      if (res.code !== 200 || !res.data) {
        connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.DISCONNECTED)
        terminal?.writeln('\r\n\x1b[31m SSH 连接已断开\x1b[0m')
        stopConnectionCheck()
      }
    } catch {
      connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.DISCONNECTED)
      terminal?.writeln('\r\n\x1b[31m⚠ SSH 连接已断开\x1b[0m')
      stopConnectionCheck()
    }
  }, 15000) // 每 15 秒检测一次
}

function stopConnectionCheck() {
  if (connectionCheckTimer) {
    clearInterval(connectionCheckTimer)
    connectionCheckTimer = null
  }
}

onMounted(() => {
  nextTick(() => {
    initTerminal()
  })
})

// 监听主题切换，更新终端配色
watch(isDark, () => {
  applyTerminalTheme()
})

onUnmounted(() => {
  stopConnectionCheck()
  window.removeEventListener('ssh-output', handleSshOutput as EventListener)
  window.removeEventListener('resize', handleResize)

  if (sshSessionId) {
    invoke('ssh_disconnect', { sessionId: sshSessionId }).catch(() => {})
  }

  if (terminal) {
    terminal.dispose()
    terminal = null
  }
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.terminal-view {
  display: flex;
  flex-direction: column;
  height: 100%;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
  }
}

.terminal-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 12px;
  background-color: #252526;
  border-bottom: 1px solid #2d2d2d;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-bottom-color: theme.getVar('borderColor');
  }
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.terminal-title {
  font-size: theme.getVar('fontSize');
  font-weight: 500;
  color: #ccc;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.terminal-status {
  font-size: theme.getVar('fontSizeSm');
  padding: 1px 6px;
  border-radius: 3px;

  &.status-disconnected {
    color: #888;
    background-color: rgba(136, 136, 136, 0.15);
  }
  &.status-connecting {
    color: #e2b714;
    background-color: rgba(226, 183, 20, 0.15);
  }
  &.status-connected {
    color: #4ec9b0;
    background-color: rgba(78, 201, 176, 0.15);
  }
  &.status-error {
    color: #f44747;
    background-color: rgba(244, 71, 71, 0.15);
  }
}

.toolbar-right {
  display: flex;
  gap: 2px;
}

.tool-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 4px 8px;
  border: 1px solid transparent;
  border-radius: 4px;
  background: transparent;
  cursor: pointer;
  font-size: theme.getVar('fontSizeSmall');
  transition: background-color 0.15s;
  user-select: none;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &:hover:not(:disabled) {
    @include theme.useTheme {
      background-color: theme.getVar('activityBarHover');
      color: theme.getVar('textColor');
    }
  }

  &:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  &.tool-btn-primary {
    @include theme.useTheme {
      color: theme.getVar('textColor');
    }
  }

  &.tool-btn-danger {
    @include theme.useTheme {
      color: #f44747;
    }

    &:hover:not(:disabled) {
      background-color: rgba(244, 71, 71, 0.12);
    }
  }
}

.tool-icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.tool-label {
  line-height: 1;
}

.spinning {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.terminal-wrapper {
  flex: 1;
  padding: 4px;
  overflow: hidden;
  min-height: 0;

  :deep(.xterm) {
    height: 100%;
  }

  :deep(.xterm-viewport) {
    overflow-y: auto !important;
  }

  // 亮色模式下强制覆盖 xterm 内联背景
  html.light & :deep(.xterm-viewport) {
    background-color: #ffffff !important;
  }

  // 暗黑模式下强制覆盖 xterm 内联背景
  html.dark & :deep(.xterm-viewport) {
    background-color: #1e1e1e !important;
  }
}
</style>
