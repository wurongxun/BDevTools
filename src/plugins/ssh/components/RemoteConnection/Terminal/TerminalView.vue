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
        <span v-if="recording" class="terminal-badge badge-record" title="日志正在写入磁盘">● 录制中</span>
        <span v-if="macroState === 'running'" class="terminal-badge badge-macro" :title="macroMessage">宏运行中</span>
      </div>
      <div class="toolbar-right">
        <!-- 显示组：编码 / 字号 / 查找。编码用与其他按钮同构的文本触发按钮，不再放一个宽大的 el-select -->
        <div class="toolbar-group">
          <el-dropdown trigger="click" @command="setEncoding">
            <button
              class="tool-btn tool-btn-text"
              :title="`终端字符编码：${encodingLabel}（点击切换）`"
              aria-label="终端字符编码"
            >
              <span class="tool-text">{{ encodingLabel }}</span>
              <svg class="tool-caret" viewBox="0 0 8 8" fill="currentColor" aria-hidden="true"><path d="M1 2.5h6L4 6z"/></svg>
            </button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item
                  v-for="opt in ENCODING_OPTIONS"
                  :key="opt.value"
                  :command="opt.value"
                >{{ opt.value === encoding ? '✓' : '·' }} {{ opt.label }}</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
          <button class="tool-btn" @click="adjustFontSize(-1)" title="缩小字号 (Ctrl+-)" aria-label="缩小字号">
            <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M3 8h10v1H3z"/></svg>
          </button>
          <button
            class="tool-btn tool-btn-metric"
            @click="resetFontSize"
            :title="`当前字号 ${fontSize}，点击恢复 ${DEFAULT_FONT_SIZE}`"
            aria-label="重置字号"
          >{{ fontSize }}</button>
          <button class="tool-btn" @click="adjustFontSize(1)" title="放大字号 (Ctrl++)" aria-label="放大字号">
            <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M7 3h2v4h4v2H9v4H7V9H3V7h4z"/></svg>
          </button>
          <button class="tool-btn" @click="toggleSearch" title="查找 (Ctrl+Shift+F)" aria-label="查找">
            <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85zm-.259 1.157a5 5 0 1 1 .707-.707l-.707.707z"/></svg>
          </button>
        </div>

        <span class="toolbar-sep" aria-hidden="true" />

        <!-- 会话组：连接是唯一带文字的主操作，用不同宽度做层级而不是不同高度 -->
        <div class="toolbar-group">
          <button
            class="tool-btn tool-btn-wide"
            :class="status === 'connected' ? 'tool-btn-danger' : 'tool-btn-primary'"
            @click="handleConnectToggle"
            :disabled="status === 'connecting'"
            :title="status === 'connected' ? '断开连接' : '连接'"
            :aria-label="status === 'connected' ? '断开连接' : '连接'"
          >
            <svg v-if="status === 'connecting'" class="tool-icon spinning" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1a7 7 0 0 1 7 7h-1.5A5.5 5.5 0 0 0 8 2.5V1z"/></svg>
            <svg v-else-if="status === 'connected'" class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M4.5 1a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 4.5 1zm7 0a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 11.5 1zM8 5a3 3 0 1 0 0 6 3 3 0 0 0 0-6z"/></svg>
            <svg v-else class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M4.5 1a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 4.5 1zm7 0a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 11.5 1zM8 5a3 3 0 1 0 0 6 3 3 0 0 0 0-6z"/></svg>
            <span class="tool-label">{{ status === 'connected' ? '断开' : status === 'connecting' ? '连接中' : '连接' }}</span>
          </button>
          <button class="tool-btn" @click="handleClear" :disabled="status !== 'connected'" title="清屏" aria-label="清屏">
            <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M2.5 1a.5.5 0 0 0 0 1h1.11l.96 9.61A2 2 0 0 0 6.56 13h2.88a2 2 0 0 0 1.99-1.39L12.39 2H13.5a.5.5 0 0 0 0-1H2.5zM5.64 2h4.72l-.86 8.61a1 1 0 0 1-1 .89H6.56a1 1 0 0 1-1-.89L4.64 2zM6 5a.5.5 0 0 1 .5.5v4a.5.5 0 0 1-1 0v-4A.5.5 0 0 1 6 5zm4 0a.5.5 0 0 1 .5.5v4a.5.5 0 0 1-1 0v-4A.5.5 0 0 1 10 5z"/></svg>
          </button>
          <button class="tool-btn" @click="handleReconnect" :disabled="status !== 'connected'" title="重连" aria-label="重连">
            <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M11.534 7h3.932a.25.25 0 0 1 .192.41l-1.966 2.36a.25.25 0 0 1-.384 0l-1.966-2.36a.25.25 0 0 1 .192-.41zm-11 2h3.932a.25.25 0 0 0 .192-.41L2.692 6.23a.25.25 0 0 0-.384 0L.342 8.59A.25.25 0 0 0 .534 9z"/><path fill-rule="evenodd" d="M8 3c-1.552 0-2.94.707-3.857 1.818a.5.5 0 1 1-.771-.636A6.002 6.002 0 0 1 13.917 7H12.9A5.002 5.002 0 0 0 8 3zM3.1 9a5.002 5.002 0 0 0 8.757 2.182.5.5 0 1 1 .771.636A6.002 6.002 0 0 1 2.083 9H3.1z"/></svg>
          </button>
        </div>

        <span class="toolbar-sep" aria-hidden="true" />

        <!-- 扩展组：高亮 / 录制 / 宏 / 设置 -->
        <div class="toolbar-group">
          <button
            class="tool-btn"
            :class="{ 'tool-btn-active': prefs.activeRules.length > 0 }"
            @click="openSettings('highlight')"
            title="高亮集：点击打开设置"
            aria-label="高亮集设置"
          >
            <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1l2.1 4.3 4.7.7-3.4 3.3.8 4.7L8 11.8l-4.2 2.2.8-4.7L1.2 6l4.7-.7z"/></svg>
          </button>
          <button
            class="tool-btn"
            :class="recording ? 'tool-btn-danger' : ''"
            @click="toggleRecording"
            :title="recording ? '停止日志录制' : '开始日志录制（带时间戳）'"
            aria-label="日志录制"
          >
            <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><circle cx="8" cy="8" r="5"/></svg>
          </button>
          <el-dropdown trigger="click" @command="runMacro">
            <button class="tool-btn" title="运行宏 / 登录脚本" aria-label="运行宏">
              <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M4 2l8 6-8 6z"/></svg>
            </button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item v-for="m in prefs.macros" :key="m.id" :command="m.id" :disabled="!m.enabled">
                  {{ m.name }}{{ m.autoRun ? '（自动）' : '' }}
                </el-dropdown-item>
                <el-dropdown-item divided command="__manage__">管理宏…</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
          <button class="tool-btn" @click="openSettings()" title="SSH 设置中心" aria-label="SSH 设置">
            <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M8 5.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5zm6.4 3.9l1.2-.9-1.2-2.1-1.4.5a5.8 5.8 0 0 0-1.3-.8L11.4 4h-2.4l-.3 1.6c-.5.1-.9.4-1.3.7l-1.4-.5L4.8 8l1.2.9a5.7 5.7 0 0 0 0 1.6l-1.2.9L6 13.5l1.4-.5c.4.3.8.6 1.3.8l.3 1.6h2.4l.3-1.6c.5-.2.9-.5 1.3-.8l1.4.5 1.2-2.1-1.2-.9a5.7 5.7 0 0 0 0-1.6z" transform="translate(0 -2)"/></svg>
          </button>
        </div>
      </div>
    </div>

    <!-- 查找栏 -->
    <div v-if="showSearch" class="terminal-search-bar">
      <el-input
        ref="searchInputRef"
        v-model="searchTerm"
        size="small"
        placeholder="查找终端输出（Enter 下一个 / Shift+Enter 上一个）"
        clearable
        aria-label="查找终端输出"
        @keydown.enter="handleSearchEnter"
        @keydown.esc="closeSearch"
      />
      <app-button size="small" @click="findPrevious" aria-label="上一个匹配">上一个</app-button>
      <app-button size="small" @click="findNext" aria-label="下一个匹配">下一个</app-button>
      <app-button size="small" @click="closeSearch" aria-label="关闭查找">关闭</app-button>
    </div>

    <!-- xterm 终端容器 -->
    <div
      class="terminal-wrapper"
      ref="xtermContainer"
      @contextmenu.prevent="handleContextMenu"
      @wheel="handleWheel"
    />

    <!-- 终端右键菜单（点击型，fixed 定位） -->
    <teleport to="body">
      <div
        v-if="contextMenu.visible"
        class="terminal-context-menu"
        :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
        @click.stop
      >
        <div class="context-menu-item" @click="copySelection">复制（Ctrl+Shift+C）</div>
        <div class="context-menu-item" @click="pasteFromClipboard">粘贴（Ctrl+Shift+V）</div>
        <div class="context-menu-item" @click="selectAll">全选</div>
        <div class="context-menu-divider" />
        <div class="context-menu-item" @click="toggleSearch">查找（Ctrl+Shift+F）</div>
        <div class="context-menu-item" @click="handleClear">清屏</div>
      </div>
      <div
        v-if="contextMenu.visible"
        class="terminal-context-overlay"
        @click="closeContextMenu"
      />
    </teleport>

    <!-- 未知主机密钥确认（对齐 Xshell 的「接受并保存主机密钥」） -->
    <GlobalDialog v-model="showHostKeyDialog" title="未知主机密钥" width="480px">
      <p class="host-key-message">{{ hostKeyMessage }}</p>
      <p class="host-key-warning">请通过可信渠道核对指纹后再信任；主机密钥一旦变更，连接会被直接拒绝。</p>
      <template #footer>
        <app-button size="small" @click="showHostKeyDialog = false">取消</app-button>
        <app-button size="small" type="primary" @click="confirmTrustHost">信任并连接</app-button>
      </template>
    </GlobalDialog>

    <!-- 411 = 服务器要求 keyboard-interactive 二次验证（TOTP / 短信码） -->
    <GlobalDialog v-model="showMfaDialog" title="二次验证" width="420px">
      <p class="host-key-message">{{ mfaMessage }}</p>
      <el-input
        v-model="mfaCode"
        placeholder="请输入动态验证码"
        autocomplete="off"
        @keyup.enter="confirmMfa"
      />
      <template #footer>
        <app-button size="small" @click="showMfaDialog = false">取消</app-button>
        <app-button size="small" type="primary" @click="confirmMfa">验证并连接</app-button>
      </template>
    </GlobalDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { invoke } from '@/api/rpc'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import { SearchAddon } from '@xterm/addon-search'
import '@xterm/xterm/css/xterm.css'
import { ConnectionStatus, ENCODING_OPTIONS } from '../../../types'
import type { ConnectionTab, RemoteConnection } from '../../../types'
import { useConnectionStore } from '../../../store'
import { isDark } from '@/composables/useDark'
import GlobalDialog from '@/components/GlobalDialog.vue'
import NotificationUtil from '@/utils/NotificationUtil'
import { withConnectOverrides } from '../../../utils/target'
import { useSshPreferencesStore } from '../../../store'
import { createHighlighter } from '../../../utils/highlight'
import { SessionRecorder } from '../../../utils/recorder'
import { ExpectRuleEngine, MacroRunner } from '../../../utils/automation'
import type { AutomationContext } from '../../../utils/automation'
import type { KeyBinding, MacroRunState } from '../../../types'

const props = defineProps<{
  tab: ConnectionTab
  connection: RemoteConnection
}>()

const connectionStore = useConnectionStore()
const prefs = useSshPreferencesStore()

// P3：高亮 / 录制 / 自动化。每个 TerminalView 各持一套实例（分屏双窗格互不干扰）。
const recording = ref(false)
const macroState = ref<MacroRunState>('idle')
const macroMessage = ref('')
const highlighter = createHighlighter(
  () => prefs.activeRules,
  text => terminal?.write(text)
)
const recorder = new SessionRecorder(
  () => prefs.logSettings,
  () => [props.connection.password, props.connection.passphrase].filter(Boolean)
)

const terminalContainer = ref<HTMLElement>()
const xtermContainer = ref<HTMLElement>()
const searchInputRef = ref<any>()
let terminal: Terminal | null = null
let fitAddon: FitAddon | null = null
let searchAddon: SearchAddon | null = null
let sshSessionId: string | null = null
let connectionCheckTimer: ReturnType<typeof setInterval> | null = null
let textDecoder: TextDecoder | null = null
// 分屏切换只改绝对定位，window 不会触发 resize；必须用 ResizeObserver 重新 fit
let resizeObserver: ResizeObserver | null = null
let lastContainerSize = ''

const DEFAULT_FONT_SIZE = 14
const fontSize = ref(DEFAULT_FONT_SIZE)
const encoding = ref(props.connection.encoding || 'utf-8')
// 工具栏只显示短标签，完整名放在下拉菜单里（GB18030 / Shift_JIS 全名会把工具条撑宽）
const ENCODING_SHORT: Record<string, string> = {
  'utf-8': 'UTF-8',
  gbk: 'GBK',
  gb18030: 'GB18030',
  big5: 'Big5',
  shift_jis: 'SJIS'
}
const encodingLabel = computed(
  () => ENCODING_SHORT[encoding.value] ?? (encoding.value || 'utf-8').toUpperCase()
)
const showSearch = ref(false)
const searchTerm = ref('')
const showHostKeyDialog = ref(false)
const hostKeyMessage = ref('')
const showMfaDialog = ref(false)
const mfaMessage = ref('')
const mfaCode = ref('')
// 重连时要保留上一次的信任标志与 options（409 与 411 可能先后出现）
let lastTrustHost = false
let lastConnectOptions = ''
const contextMenu = ref({ visible: false, x: 0, y: 0 })

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

// P3：主题真源是偏好中心的配色方案（内置 auto 跟随 IDE 明暗，导入方案见 utils/theme.ts）
function getTerminalTheme() {
  return { ...prefs.terminalTheme }
}

function applyTerminalTheme() {
  if (!terminal) return
  const theme = getTerminalTheme()
  // 通过新对象赋值确保 xterm.js 触发响应式更新
  terminal.options.theme = { ...theme }
  // 强制刷新终端渲染
  terminal.refresh(0, terminal.rows - 1)
}

/** 后端输出是原始字节，用同一个 Decoder 实例流式解码，跨块多字节字符才不会被切断 */
function resetDecoder() {
  try {
    textDecoder = new TextDecoder(encoding.value || 'utf-8')
  } catch {
    encoding.value = 'utf-8'
    textDecoder = new TextDecoder('utf-8')
    NotificationUtil.warn('不支持的字符编码，已回退 UTF-8')
  }
}

function handleEncodingChange() {
  resetDecoder()
  NotificationUtil.info(`终端编码已切换为 ${encoding.value}，后续输出按新编码解码`)
}

/** 下拉菜单选编码；与原来的 el-select v-model 走同一个解码器重建逻辑 */
function setEncoding(value: string) {
  if (!value || value === encoding.value) return
  encoding.value = value
  handleEncodingChange()
}

function initTerminal() {
  if (!xtermContainer.value) return

  resetDecoder()
  terminal = new Terminal({
    cursorBlink: true,
    cursorStyle: 'block',
    fontSize: fontSize.value,
    fontFamily: "'Consolas', 'Courier New', monospace",
    theme: getTerminalTheme(),
    scrollback: 5000,
    convertEol: true
  })

  fitAddon = new FitAddon()
  searchAddon = new SearchAddon()
  terminal.loadAddon(fitAddon)
  terminal.loadAddon(searchAddon)
  terminal.loadAddon(new WebLinksAddon())

  terminal.open(xtermContainer.value)
  fitAddon.fit()

  // 监听终端输入（onData 处理所有按键包括特殊键的转义序列）
  terminal.onData((data) => {
    if (sshSessionId && status.value === ConnectionStatus.CONNECTED) {
      // 只记录可打印输入：方向键 / 退格等控制序列不写进日志
      const printable = data
        .replace(/\x1b\[[0-9;?]*[A-Za-z]/g, '')
        .replace(/[\x00-\x1f\x7f]/g, '')
      recorder.append(printable, 'in')
      invoke('ssh_write', { sessionId: sshSessionId, data }).catch(err => {
        console.error('SSH write error:', err)
      })
    }
  })

  // Xshell 风格：选中即复制
  terminal.onSelectionChange(() => {
    const selection = terminal?.getSelection()
    if (selection) {
      void navigator.clipboard.writeText(selection).catch(() => {})
    }
  })

  // 自定义快捷键（返回 false 阻止 xterm 继续处理）
  terminal.attachCustomKeyEventHandler((event: KeyboardEvent) => {
    if (event.type !== 'keydown') return true
    const mod = event.ctrlKey || event.metaKey
    const key = event.key.toLowerCase()
    if (mod && event.shiftKey) {
      if (key === 'c') { copySelection(); return false }
      if (key === 'v') { void pasteFromClipboard(); return false }
      if (key === 'f') { toggleSearch(); return false }
      if (key === 'a') { selectAll(); return false }
    }
    if (mod && (event.key === '=' || event.key === '+')) { adjustFontSize(1); return false }
    if (mod && event.key === '-') { adjustFontSize(-1); return false }
    if (event.key === 'Escape' && showSearch.value) { closeSearch(); return false }
    // P3 自定义键盘映射放在内置快捷键之后，避免覆盖复制/查找等基础操作
    const binding = matchKeyBinding(event)
    if (binding) { executeKeyAction(binding); return false }
    return true
  })

  // 确保终端可以接收焦点
  terminal.textarea?.setAttribute('tabindex', '0')

  window.addEventListener('ssh-output', handleSshOutput as EventListener)
  window.addEventListener('ssh-closed', handleSshClosed as EventListener)
  window.addEventListener('ssh-compose-input', handleComposeInput as EventListener)
  window.addEventListener('resize', handleResize)
  document.addEventListener('click', closeContextMenu)

  // 容器尺寸变化（分屏开关、侧栏折叠、窗口缩放）都重新适配 PTY 行列数
  if (typeof ResizeObserver !== 'undefined' && xtermContainer.value) {
    resizeObserver = new ResizeObserver(() => {
      const el = xtermContainer.value
      if (!el) return
      const size = `${el.clientWidth}x${el.clientHeight}`
      if (size === lastContainerSize || el.clientWidth === 0 || el.clientHeight === 0) return
      lastContainerSize = size
      handleResize()
    })
    resizeObserver.observe(xtermContainer.value)
  }

  // 显示欢迎信息
  terminal.writeln(`\x1b[33m欢迎使用远程终端连接\x1b[0m`)
  terminal.writeln(`\x1b[36m主机: ${props.connection.address}:${props.connection.port}\x1b[0m`)
  terminal.writeln(`\x1b[36m用户: ${props.connection.username}\x1b[0m`)
  if (props.connection.description) {
    terminal.writeln(`\x1b[90m备注: ${props.connection.description}\x1b[0m`)
  }
  terminal.writeln(`\x1b[90m点击 "连接" 按钮开始连接...\x1b[0m`)
  terminal.writeln('')
}

function handleSshOutput(event: CustomEvent) {
  const { tabId, data, format } = event.detail || {}
  if (tabId !== props.tab.id || !terminal || data == null) return
  let text = ''
  if (format === 'base64') {
    try {
      const binary = atob(data)
      const bytes = Uint8Array.from(binary, ch => ch.charCodeAt(0))
      text = textDecoder
        ? textDecoder.decode(bytes, { stream: true })
        : new TextDecoder().decode(bytes)
    } catch (err) {
      console.error('SSH output decode error:', err)
      return
    }
  } else {
    // 兼容旧版后端（纯文本输出）
    text = String(data)
  }
  if (!text) return
  detectZmodem(text)
  // 自动化引擎吃未高亮的原始文本，避免注入的 SGR 干扰正则匹配
  expectEngine.feed(text)
  macroRunner.feedOutput(text)
  recorder.append(text, 'out')
  terminal.write(highlighter.feed(text))
}

/**
 * 撰写栏输入。sessionId 只存在本组件里，所以广播走 window 事件而不是 store 直接 invoke；
 * 非 terminal 类型的 tab 收不到（ConnectionPanel 不挂载本组件），无需额外过滤。
 */
function handleComposeInput(event: CustomEvent) {
  const { text, broadcast, targetTabId } = event.detail || {}
  if (!text || !sshSessionId || status.value !== ConnectionStatus.CONNECTED) return
  if (!broadcast && targetTabId !== props.tab.id) return
  // 远端 shell 需要回车才执行；撰写栏允许一次输入多行，逐行保留
  invoke('ssh_write', { sessionId: sshSessionId, data: `${String(text)}\r` }).catch(err => {
    console.error('SSH compose write error:', err)
  })
}

function handleSshClosed(event: CustomEvent) {
  const { tabId, reason } = event.detail || {}
  if (tabId !== props.tab.id || reason === 'manual') return
  stopConnectionCheck()
  sshSessionId = null
  terminal?.writeln('\r\n\x1b[31m SSH 连接已断开（服务端关闭或网络异常），可点击「连接」重连\x1b[0m')
  expectEngine.reset()
  macroRunner.stop()
  void recorder.stop().then(() => { recording.value = false })
}

async function handleConnectToggle() {
  if (status.value === ConnectionStatus.CONNECTED) {
    await disconnect()
  } else {
    await connect(false)
  }
}

async function connect(trustHost: boolean, optionsOverride?: string) {
  if (!terminal) return

  lastTrustHost = trustHost
  lastConnectOptions = optionsOverride ?? props.connection.advanced_options ?? ''
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
      privateKeyPath: props.connection.private_key_path || '',
      passphrase: props.connection.passphrase || '',
      trustHost,
      keepaliveInterval: props.connection.keepalive_interval ?? 30,
      termType: props.connection.term_type || 'xterm-256color',
      options: lastConnectOptions
    })

    if (res.code === 200) {
      sshSessionId = res.data
      connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.CONNECTED)

      // 初始适配终端大小后再连接 shell，确保 PTY 尺寸正确
      nextTick(() => {
        if (fitAddon) {
          fitAddon.fit()
          const dims = { cols: terminal!.cols, rows: terminal!.rows }
          invoke('ssh_resize_pty', { sessionId: sshSessionId, ...dims }).catch(err => {
            console.error('SSH resize error:', err)
          })
        }
        terminal!.focus()
      })

      terminal.writeln(`\x1b[32m✓ 已连接到 ${props.connection.username}@${props.connection.address}\x1b[0m`)
      terminal.writeln('')
      startConnectionCheck()
      onConnectedAutomation()
      return
    }

    connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.ERROR)
    terminal.writeln(`\x1b[31m连接失败: ${res.msg}\x1b[0m`)

    // 409 = 首次遇到未知主机密钥，交给用户核对指纹后决定是否信任；
    // 410 = 已知主机密钥不匹配（疑似 MITM），绝不提供「信任」按钮。
    if (res.code === 409) {
      hostKeyMessage.value = String(res.msg || '')
      showHostKeyDialog.value = true
    }
    // 411 = 需要一次性验证码，弹框收集后带码重连（不写入会话配置）
    if (res.code === 411) {
      mfaMessage.value = String(res.msg || '')
      mfaCode.value = ''
      showMfaDialog.value = true
    }
  } catch (error) {
    connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.ERROR)
    terminal.writeln(`\x1b[31m连接错误: ${error}\x1b[0m`)
  }
}

function confirmTrustHost() {
  showHostKeyDialog.value = false
  void connect(true, lastConnectOptions)
}

function confirmMfa() {
  const code = mfaCode.value.trim()
  if (!code) return
  showMfaDialog.value = false
  const options = withConnectOverrides(props.connection.advanced_options, { mfaCode: code })
  void connect(lastTrustHost, options)
}

async function disconnect() {
  stopConnectionCheck()
  await stopSessionSideEffects()
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
  await connect(false)
}

function handleClear() {
  closeContextMenu()
  terminal?.clear()
}

function handleResize() {
  if (fitAddon) {
    fitAddon.fit()
    if (sshSessionId && terminal) {
      invoke('ssh_resize_pty', {
        sessionId: sshSessionId,
        cols: terminal.cols,
        rows: terminal.rows
      }).catch(err => console.error('SSH resize error:', err))
    }
  }
}

// ==================== 复制 / 粘贴 / 查找 / 字号 ====================

async function copySelection() {
  closeContextMenu()
  const selection = terminal?.getSelection()
  if (!selection) return
  try {
    await navigator.clipboard.writeText(selection)
  } catch {
    NotificationUtil.warn('剪贴板不可用，请用 Ctrl+Shift+C 重试')
  }
}

async function pasteFromClipboard() {
  closeContextMenu()
  if (!sshSessionId || status.value !== ConnectionStatus.CONNECTED) return
  try {
    const text = await navigator.clipboard.readText()
    if (!text) return
    await invoke('ssh_write', { sessionId: sshSessionId, data: text })
  } catch (error) {
    console.error('SSH paste error:', error)
    NotificationUtil.warn('粘贴失败：剪贴板不可用或连接已断开')
  }
}

function selectAll() {
  closeContextMenu()
  terminal?.selectAll()
}

function handleContextMenu(event: MouseEvent) {
  contextMenu.value = { visible: true, x: event.clientX, y: event.clientY }
}

function closeContextMenu() {
  contextMenu.value.visible = false
}

function toggleSearch() {
  closeContextMenu()
  showSearch.value = !showSearch.value
  if (showSearch.value) {
    nextTick(() => searchInputRef.value?.focus())
  } else {
    searchAddon?.clearDecorations()
    terminal?.focus()
  }
}

function closeSearch() {
  showSearch.value = false
  searchAddon?.clearDecorations()
  terminal?.focus()
}

function findNext() {
  if (searchTerm.value) searchAddon?.findNext(searchTerm.value, { caseSensitive: false })
}

function findPrevious() {
  if (searchTerm.value) searchAddon?.findPrevious(searchTerm.value, { caseSensitive: false })
}

function handleSearchEnter(event: KeyboardEvent) {
  if (event.shiftKey) findPrevious()
  else findNext()
}

function adjustFontSize(delta: number) {
  fontSize.value = Math.max(9, Math.min(28, fontSize.value + delta))
  if (!terminal) return
  terminal.options.fontSize = fontSize.value
  fitAddon?.fit()
  if (sshSessionId) {
    invoke('ssh_resize_pty', {
      sessionId: sshSessionId,
      cols: terminal.cols,
      rows: terminal.rows
    }).catch(err => console.error('SSH resize error:', err))
  }
}

/** 工具栏上的字号数字既是状态也是「恢复默认」入口 */
function resetFontSize() {
  if (fontSize.value === DEFAULT_FONT_SIZE) return
  adjustFontSize(DEFAULT_FONT_SIZE - fontSize.value)
}

function handleWheel(event: WheelEvent) {
  if (!event.ctrlKey) return
  event.preventDefault()
  event.stopPropagation()
  adjustFontSize(event.deltaY < 0 ? 1 : -1)
}


// ==================== P3：高亮 / 录制 / 自动登录 / 宏 / 键位 ====================

/** 设置中心是全局单例（挂在 MainPanel），终端只负责发出打开意图 */
function openSettings(tab = 'highlight') {
  window.dispatchEvent(new CustomEvent('ssh-open-settings', { detail: { tab } }))
}

function writeRemote(data: string) {
  if (!sshSessionId || status.value !== ConnectionStatus.CONNECTED) return
  invoke('ssh_write', { sessionId: sshSessionId, data }).catch(err => {
    console.error('SSH automation write error:', err)
  })
}

const automationCtx: AutomationContext = {
  write: writeRemote,
  vars: () => ({
    password: props.connection.password || '',
    passphrase: props.connection.passphrase || '',
    username: props.connection.username || '',
    host: props.connection.address || ''
  }),
  onMacroState: (state, _index, message) => {
    macroState.value = state
    macroMessage.value = message
    if (state === 'failed') NotificationUtil.warn(message)
  },
  onNotice: message => { macroMessage.value = message }
}

const expectEngine = new ExpectRuleEngine(() => prefs.expectRules, automationCtx)
const macroRunner = new MacroRunner(automationCtx)
let zmodemWarned = false

/**
 * ZMODEM（rz/sz）是二进制帧协议，需要劫持整条 PTY 字节流并驱动外部 lrzsz，
 * 与当前「base64 字节 → TextDecoder → xterm」链路不兼容。这里只做检测并给替代路径，
 * 不假装支持，也不让远端卡在等待接收状态而不知情。
 */
function detectZmodem(text: string) {
  if (zmodemWarned) return
  if (!/\*\*\x18B|rz waiting to receive|\x18B0/i.test(text)) return
  zmodemWarned = true
  NotificationUtil.warn('检测到 ZMODEM（rz/sz）传输请求：本插件不支持该协议，请使用 SFTP 面板上传 / 下载文件')
}

function onConnectedAutomation() {
  expectEngine.reset()
  zmodemWarned = false
  const cfg = prefs.logSettings
  if (cfg.enabled && cfg.autoRecord) startRecording()
  // 只自动跑第一个启用的 autoRun 宏，避免多条登录脚本互相抢输入
  const auto = prefs.macros.find(m => m.enabled && m.autoRun)
  if (auto) void macroRunner.run(auto)
}

function startRecording() {
  if (!prefs.logSettings.enabled) {
    NotificationUtil.warn('日志录制已在 SSH 设置中关闭')
    return
  }
  const name = recorder.start(props.connection.name, props.tab.id)
  recording.value = true
  NotificationUtil.info(`开始录制日志：${name}`)
}

async function toggleRecording() {
  if (recording.value) {
    await recorder.stop()
    recording.value = false
    NotificationUtil.info('日志录制已停止，文件保存在 SSH 设置 → 会话日志')
  } else {
    startRecording()
  }
}

/** 断开 / 卸载时收尾：停宏、把日志缓冲刷盘、把高亮器尾部残字写回终端 */
async function stopSessionSideEffects() {
  macroRunner.stop()
  expectEngine.reset()
  await recorder.stop()
  recording.value = false
  const tail = highlighter.flush()
  if (tail) terminal?.write(tail)
}

function runMacro(command: string) {
  if (command === '__manage__') { openSettings('macro'); return }
  const macro = prefs.macros.find(m => m.id === command && m.enabled)
  if (!macro) { NotificationUtil.warn('宏不存在或已禁用'); return }
  if (macroRunner.running) { NotificationUtil.warn('已有宏正在运行'); return }
  void macroRunner.run(macro)
}

function matchKeyBinding(event: KeyboardEvent): KeyBinding | undefined {
  const key = event.key.toLowerCase()
  return prefs.keyBindings.find(b =>
    b.enabled && !!b.key &&
    b.key.toLowerCase() === key &&
    b.ctrl === event.ctrlKey &&
    b.alt === event.altKey &&
    b.shift === event.shiftKey &&
    b.meta === event.metaKey
  )
}

function executeKeyAction(binding: KeyBinding) {
  switch (binding.action) {
    case 'copy': copySelection(); break
    case 'paste': void pasteFromClipboard(); break
    case 'search': toggleSearch(); break
    case 'selectAll': selectAll(); break
    case 'clear': handleClear(); break
    case 'reconnect': void handleReconnect(); break
    case 'send': writeRemote(binding.payload); break
    case 'runMacro': runMacro(binding.payload); break
    case 'toggleRecord': void toggleRecording(); break
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
  }, 15000)
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

// 主题切换 / 导入新配色 / 改高亮规则都要即时生效
watch(isDark, () => applyTerminalTheme())
watch(() => prefs.terminalTheme, () => applyTerminalTheme(), { deep: true })
watch(() => prefs.activeRules, rules => highlighter.updateRules(rules), { deep: true })

onUnmounted(() => {
  stopConnectionCheck()
  macroRunner.stop()
  highlighter.dispose()
  void recorder.stop()
  window.removeEventListener('ssh-output', handleSshOutput as EventListener)
  window.removeEventListener('ssh-closed', handleSshClosed as EventListener)
  window.removeEventListener('ssh-compose-input', handleComposeInput as EventListener)
  window.removeEventListener('resize', handleResize)
  document.removeEventListener('click', closeContextMenu)
  resizeObserver?.disconnect()
  resizeObserver = null

  if (sshSessionId) {
    invoke('ssh_disconnect', { sessionId: sshSessionId }).catch(err => {
      console.error('SSH disconnect error:', err)
    })
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
    background-color: theme.getVar('appMainBgColor');
  }
}

.terminal-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 3px 8px;
  min-height: 30px;
  flex-shrink: 0;
  overflow: hidden;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  overflow: hidden;
}

.terminal-title {
  font-size: theme.getVar('fontSize');
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.terminal-status {
  font-size: theme.getVar('fontSizeSm');
  padding: 1px 6px;
  flex-shrink: 0;
  border-radius: var(--radius-sm);

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
  align-items: center;
  // 允许收缩 + 内部横向滚动：分屏双窗格时不会撑破工具条，也不会把左侧标题挤没
  flex: 0 1 auto;
  min-width: 0;
  // 分屏双窗格时让次要按钮横向滚出，不挤压左侧标题；下拉菜单 teleport 到 body 不受裁剪
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;

  &::-webkit-scrollbar { display: none; }
}

.toolbar-group {
  display: inline-flex;
  align-items: center;
  gap: 1px;
  flex-shrink: 0;
}

.toolbar-sep {
  width: 1px;
  height: 14px;
  margin: 0 5px;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('borderColor');
  }
}

.tool-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  width: 24px;
  height: 24px;
  padding: 0;
  flex-shrink: 0;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  background: transparent;
  cursor: pointer;
  font-size: theme.getVar('fontSizeSmall');
  transition: background-color 0.15s, color 0.15s;
  user-select: none;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  // 连接是唯一带文字的主操作：只放宽宽度，高度与图标按钮严格一致
  &.tool-btn-wide {
    width: auto;
    padding: 0 6px;
    gap: 3px;
  }

  // 编码触发器：无边框、无输入框背景，与其他图标按钮同一视觉重量
  &.tool-btn-text {
    width: auto;
    min-width: 46px;
    padding: 0 4px;
    gap: 2px;
  }

  &.tool-btn-metric {
    width: 20px;
    font-family: var(--font-family-mono);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    opacity: 0.8;
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

.tool-text {
  font-family: var(--font-family-mono);
  font-size: 11px;
  line-height: 1;
  letter-spacing: -0.2px;
}

.tool-caret {
  width: 7px;
  height: 7px;
  flex-shrink: 0;
  opacity: 0.6;
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

.terminal-search-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
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

  html.light & :deep(.xterm-viewport) {
    background-color: #ffffff !important;
  }

  html.dark & :deep(.xterm-viewport) {
    background-color: #1e1e1e !important;
  }
}

.host-key-message {
  line-height: 1.6;
  margin: 0 0 8px;
  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.host-key-warning {
  line-height: 1.6;
  margin: 0;
  @include theme.useTheme {
    color: theme.getVar('stateWarningFg');
  }
}

.terminal-badge {
  font-size: 11px;
  padding: 1px 7px;
  flex-shrink: 0;
  border-radius: 9px;
  color: #fff;
  white-space: nowrap;

  &.badge-record { background-color: #dc2626; }
  &.badge-macro { background-color: #7c3aed; }
}

.tool-btn-active {
  @include theme.useTheme {
    color: theme.getVar('primaryColor');
    background-color: theme.getVar('hoverBgColor');
  }
}
</style>

<style lang="scss">
@use '@/styles/theme.scss' as theme;

// Teleport 到 body 的节点不受 scoped 影响，必须放非 scoped 块
.terminal-context-menu {
  position: fixed;
  z-index: 3001;
  min-width: 190px;
  padding: 4px;
  border-radius: var(--radius-md);

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderStrong');
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.18);
    color: theme.getVar('textColor');
  }
}

.terminal-context-overlay {
  position: fixed;
  inset: 0;
  z-index: 3000;
}

.terminal-context-menu .context-menu-item {
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('activityBarHover');
    }
  }
}

.terminal-context-menu .context-menu-divider {
  height: 1px;
  margin: 4px 0;
  @include theme.useTheme {
    background-color: theme.getVar('borderColor');
  }
}
</style>
