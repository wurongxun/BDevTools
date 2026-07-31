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
        <el-button
          size="small"
          :type="status === 'connected' ? 'danger' : 'primary'"
          @click="handleConnectToggle"
          :loading="status === 'connecting'"
        >
          {{ status === 'connected' ? '断开' : status === 'connecting' ? '连接中...' : '连接' }}
        </el-button>
        <el-button
          size="small"
          @click="handleClear"
          :disabled="status !== 'connected'"
        >
          清屏
        </el-button>
        <el-button
          size="small"
          @click="handleReconnect"
          :disabled="status !== 'connected'"
        >
          重连
        </el-button>
      </div>
    </div>

    <!-- xterm 终端容器 -->
    <div class="terminal-wrapper" ref="xtermContainer" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import '@xterm/xterm/css/xterm.css'
import useStore from '@/store'
import { ConnectionStatus } from '@/entity/entitys'
import type { ConnectionTab, RemoteConnection } from '@/entity/entitys'

const props = defineProps<{
  tab: ConnectionTab
  connection: RemoteConnection
}>()

const { useConnectionStore } = useStore()

const terminalContainer = ref<HTMLElement>()
const xtermContainer = ref<HTMLElement>()
let terminal: Terminal | null = null
let fitAddon: FitAddon | null = null
let sshSessionId: string | null = null

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

function initTerminal() {
  if (!xtermContainer.value) return

  terminal = new Terminal({
    cursorBlink: true,
    cursorStyle: 'block',
    fontSize: 14,
    fontFamily: "'Consolas', 'Courier New', monospace",
    theme: {
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
    scrollback: 5000,
    convertEol: true
  })

  fitAddon = new FitAddon()
  terminal.loadAddon(fitAddon)
  terminal.loadAddon(new WebLinksAddon())

  terminal.open(xtermContainer.value)
  fitAddon.fit()

  // 监听终端输入
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

  // 监听后端 SSH 输出
  window.addEventListener('ssh-output', handleSshOutput as EventListener)

  // 监听窗口 resize
  window.addEventListener('resize', handleResize)

  // 显示欢迎信息
  terminal.writeln(`\x1b[33m欢迎使用远程终端连接\x1b[0m`)
  terminal.writeln(`\x1b[36m主机: ${props.connection.address}:${props.connection.port}\x1b[0m`)
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

  useConnectionStore.updateTabStatus(props.tab.id, ConnectionStatus.CONNECTING)
  terminal.writeln(`\x1b[33m正在连接 ${props.connection.address}...\x1b[0m`)

  try {
    const res = await invoke<any>('ssh_connect', {
      tabId: props.tab.id,
      host: props.connection.address,
      port: props.connection.port,
      username: props.connection.username,
      password: props.connection.password
    })

    if (res.code === 200) {
      sshSessionId = res.data
      useConnectionStore.updateTabStatus(props.tab.id, ConnectionStatus.CONNECTED)
      terminal.writeln(`\x1b[32m连接成功!\x1b[0m\r\n`)
      terminal.focus()

      // 初始适配终端大小
      nextTick(() => {
        if (fitAddon) {
          fitAddon.fit()
          const dims = { cols: terminal!.cols, rows: terminal!.rows }
          invoke('ssh_resize_pty', { sessionId: sshSessionId, ...dims }).catch(() => {})
        }
      })
    } else {
      useConnectionStore.updateTabStatus(props.tab.id, ConnectionStatus.ERROR)
      terminal.writeln(`\x1b[31m连接失败: ${res.msg}\x1b[0m`)
    }
  } catch (error) {
    useConnectionStore.updateTabStatus(props.tab.id, ConnectionStatus.ERROR)
    terminal.writeln(`\x1b[31m连接错误: ${error}\x1b[0m`)
  }
}

async function disconnect() {
  if (sshSessionId) {
    try {
      await invoke('ssh_disconnect', { sessionId: sshSessionId })
    } catch (error) {
      console.error('Disconnect error:', error)
    }
    sshSessionId = null
  }

  useConnectionStore.updateTabStatus(props.tab.id, ConnectionStatus.DISCONNECTED)
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

onMounted(() => {
  nextTick(() => {
    initTerminal()
  })
})

onUnmounted(() => {
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
  background-color: #1e1e1e;
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
  font-size: 13px;
  font-weight: 500;
  color: #ccc;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.terminal-status {
  font-size: 11px;
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
  gap: 6px;
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
}
</style>
