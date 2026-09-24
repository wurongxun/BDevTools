/**
 * TerminalSession —— 一个窗格 = 一个后端 PTY 会话 + 一个 xterm 实例
 *
 * 职责边界：
 * - **只做终端本身**（xterm 实例、后端桥、OSC 解析、键鼠交互），不碰布局/标签/持久化；
 * - 布局与生命周期由 `store.ts` 与 `components/*` 负责。
 *
 * ## 转义序列在**前端**解析
 * 后端只把原始字节 base64 透传，OSC 由 xterm.js 的 `parser.registerOscHandler` 解析：
 * - `OSC 7`  → cwd 跟踪
 * - `OSC 0/2`→ 标题
 * - `OSC 133`→ 命令边界（A 提示符开始 / B 可输入 / C 输出开始 / D;code 命令结束 + 退出码）
 *
 * 放在前端的原因：xterm 的解析器已经处理了「序列跨读块被切断」的状态机，
 * 后端再写一遍纯属重复；而 `exec()` 需要知道退出码，在前端 Promise 里等一个事件最自然。
 */
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { SearchAddon } from '@xterm/addon-search'
import { WebLinksAddon } from '@xterm/addon-web-links'
import { Unicode11Addon } from '@xterm/addon-unicode11'
import { openTerminal, writeTerminal, resizeTerminal, closeTerminal } from './api'
import { searchDecorations } from './theme'
import type {
  CommandMark,
  IntegrationLevel,
  TermExecResult,
  TermOpenOptions,
  TermOpenResult,
  TermPalette,
  TerminalAppearance,
} from './types'

const IS_MAC = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform || '')

/** base64 → 原始字节（后端把 PTY 输出按块 base64，避免分块边界切断多字节字符） */
function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64)
  const out = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i)
  return out
}

/** 解析 OSC 7 的 `file://host/path` → 本地路径 */
export function parseFileUri(uri: string): string {
  const m = /^file:\/\/([^/]*)(\/.*)?$/.exec(uri)
  if (!m) return uri
  let p = m[2] || '/'
  try {
    p = decodeURIComponent(p)
  } catch {
    /* 非法百分号编码：保留原文 */
  }
  // Windows：`/C:/Users/x` → `C:/Users/x`，再统一成反斜杠
  if (/^\/[A-Za-z]:/.test(p)) {
    p = p.slice(1).replace(/\//g, '\\')
  }
  return p
}

export interface SessionHooks {
  /** 收到输出（用于刷新「有活动」状态） */
  onActivity?: () => void
  /** 标题变化（OSC 0/2） */
  onTitle?: (title: string) => void
  /** cwd 变化（OSC 7） */
  onCwd?: (cwd: string) => void
  /** 命令边界（OSC 133） */
  onMark?: (mark: CommandMark, exitCode: number | null) => void
  /** 子进程退出 */
  onExit?: (code: number | null, signal: string | null) => void
  /** 会话级错误 */
  onError?: (message: string) => void
  /** 用户在终端里按了「查找」（Ctrl+Shift+F） */
  onRequestSearch?: () => void
  /** 搜索结果变化（`resultIndex` 从 0 开始；超出上限时为 -1） */
  onSearchResults?: (info: { resultIndex: number; resultCount: number }) => void
}

/** exec() 等待中的上下文 */
interface PendingExec {
  resolve: (r: TermExecResult) => void
  timer: ReturnType<typeof setTimeout>
  startedAt: number
  chunks: string[]
  decoder: TextDecoder
  sawOutputStart: boolean
}

export class TerminalSession {
  /** 前端窗格 id（与 store 的 paneId 一致） */
  readonly paneId: string
  /** 后端会话 id（打开成功后赋值） */
  sessionId = ''
  readonly term: Terminal
  readonly fitAddon = new FitAddon()
  readonly searchAddon = new SearchAddon()
  /** 会话建立结果（shell / 集成程度等） */
  info: TermOpenResult | null = null
  /** shell 集成程度：决定 exec() 能否拿到退出码 */
  integration: IntegrationLevel = 'none'
  exited = false
  exitCode: number | null = null
  cwd = ''

  private hooks: SessionHooks
  private appearance: TerminalAppearance
  private palette: TermPalette
  private disposed = false
  private pending: PendingExec | null = null
  private fitRaf = 0
  /**
   * 写入串行链。
   * xterm 每按一个键就 fire 一次 onData，而 invoke 是异步的——
   * 不做串行化的话，快速输入 / 大段粘贴时 IPC 回调可能乱序到达后端，表现为字符错位。
   */
  private writeChain: Promise<void> = Promise.resolve()
  /** 终端内「外部链接打通」的回调（默认交给宿主 opener） */
  private openLink: (uri: string) => void

  constructor(opts: {
    paneId: string
    container: HTMLElement
    appearance: TerminalAppearance
    palette: TermPalette
    hooks: SessionHooks
    openLink: (uri: string) => void
    /** Windows ConPTY 需要显式声明后端，xterm 才能正确处理滚动回看重排 */
    windowsPty?: boolean
  }) {
    this.paneId = opts.paneId
    this.hooks = opts.hooks
    this.appearance = opts.appearance
    this.palette = opts.palette
    this.openLink = opts.openLink

    this.term = new Terminal({
      // 搜索高亮用 decorations API；在桌面壳里开启 proposed API 无副作用
      allowProposedApi: true,
      fontFamily: opts.appearance.fontFamily,
      fontSize: opts.appearance.fontSize,
      lineHeight: opts.appearance.lineHeight,
      cursorStyle: opts.appearance.cursorStyle,
      cursorBlink: opts.appearance.cursorBlink,
      scrollback: opts.appearance.scrollback,
      theme: { ...this.palette },
      // ConPTY 声明：让 xterm 按 Windows 的行重排规则处理 resize
      windowsPty: opts.windowsPty ? { backend: 'conpty' } : undefined,
      // 终端里不做自动换行转换：交由 shell 的 PTY 行规程处理
      convertEol: false,
      drawBoldTextInBrightColors: false,
      // 允许用户用鼠标选中（右键保留给 shell/宿主菜单）
      rightClickSelectsWord: false,
      macOptionIsMeta: false,
      scrollOnUserInput: true,
    })

    this.term.loadAddon(this.fitAddon)
    this.term.loadAddon(this.searchAddon)
    // 链接识别：鼠标点 URL 交给宿主用系统浏览器打开
    this.term.loadAddon(new WebLinksAddon((_ev, uri) => this.openLink(uri)))
    const unicode11 = new Unicode11Addon()
    this.term.loadAddon(unicode11)
    this.term.unicode.activeVersion = '11'

    this.term.open(opts.container)
    this.registerOscHandlers()
    this.registerKeyHandler()
    this.registerInputPipes()
    this.term.onSelectionChange(() => {
      if (this.appearance.copyOnSelect) {
        const sel = this.term.getSelection()
        if (sel) void this.copyToClipboard(sel)
      }
    })
    this.searchAddon.onDidChangeResults((e) => this.hooks.onSearchResults?.(e))
  }

  // ==================== 后端桥 ====================

  /** 建立后端 PTY 会话（容器已挂载、尺寸可测） */
  async open(options: Omit<TermOpenOptions, 'cols' | 'rows'> = {}): Promise<TermOpenResult> {
    const dims = this.measure()
    const res = await openTerminal(
      { ...options, cols: dims.cols, rows: dims.rows },
      (ev) => this.onEvent(ev),
    )

    // 会话在「等后端建 PTY」这段时间里被关掉了（用户关标签 / 关窗格很快，
    // 而拉起一个 shell 要几百毫秒）。此时后端其实已经有一个活着的 shell，
    // 必须立刻回收：我们不再持有它，否则就是一个孤儿进程 + 一条常驻读线程。
    if (this.disposed) {
      if (res.sessionId) {
        void closeTerminal(res.sessionId).catch(() => {
          /* 尽力而为：后端可能已经自己退出了 */
        })
      }
      return res
    }

    this.info = res
    this.sessionId = res.sessionId
    this.integration = res.integration
    this.cwd = res.cwd
    return res
  }

  private onEvent(ev: import('./types').TermEvent) {
    if (this.disposed) return
    switch (ev.type) {
      case 'data': {
        const bytes = b64ToBytes(ev.data)
        this.term.write(bytes)
        if (this.pending) {
          const text = this.pending.decoder.decode(bytes, { stream: true })
          if (text) this.pending.chunks.push(text)
        }
        this.hooks.onActivity?.()
        break
      }
      case 'exit':
        this.exited = true
        this.exitCode = ev.code
        // 命令还在等待却收到退出：直接以「无退出码」结束，避免永久挂起
        if (this.pending) this.settlePending(null, false)
        this.hooks.onExit?.(ev.code, ev.signal)
        break
      case 'error':
        this.hooks.onError?.(ev.message)
        break
    }
  }

  /** 终端输入 → 后端 */
  private registerInputPipes() {
    this.term.onData((data) => {
      void this.send(data).catch((e) => {
        this.hooks.onError?.(e instanceof Error ? e.message : String(e))
      })
    })
    // 尺寸变化（窗口/分隔条/面板高度变化都会触发）→ 通知 PTY
    this.term.onResize(({ cols, rows }) => {
      if (!this.sessionId || this.exited) return
      void resizeTerminal(this.sessionId, cols, rows).catch(() => {
        /* 会话已关闭时后端返回 404，忽略即可 */
      })
    })
  }

  /**
   * 串行写入 PTY（保证与用户输入顺序一致）。
   * 返回的 Promise 会 reject（调用方自行处理），但内部链用 catch 兜住，
   * 保证一次写入失败不会卡死后续所有输入。
   */
  private send(data: string): Promise<void> {
    if (!this.sessionId || this.exited) return Promise.resolve()
    const sid = this.sessionId
    const next = this.writeChain.then(() => writeTerminal(sid, data))
    this.writeChain = next.catch(() => {
      /* 链本身不断：错误只回给当次调用方 */
    })
    return next
  }

  // ==================== OSC 解析（shell 集成） ====================

  private registerOscHandlers() {
    // OSC 7：cwd（`file://host/path`）
    this.term.parser.registerOscHandler(7, (data) => {
      const cwd = parseFileUri(data)
      if (cwd && cwd !== this.cwd) {
        this.cwd = cwd
        this.hooks.onCwd?.(cwd)
      }
      return true
    })
    // OSC 0 / 2：窗口/图标标题
    for (const ident of [0, 2]) {
      this.term.parser.registerOscHandler(ident, (data) => {
        if (data) this.hooks.onTitle?.(data)
        return true
      })
    }
    // OSC 133：命令边界（shell 集成注入，见后端 terminal.rs）
    // A 提示符开始 / B 命令开始输入 / C 输出开始 / D[;exit] 命令结束
    this.term.parser.registerOscHandler(133, (data) => {
      const [kind, codeRaw] = data.split(';')
      const exitCode = codeRaw !== undefined && codeRaw !== '' ? Number(codeRaw) : null
      const mark: CommandMark | null =
        kind === 'A' ? 'promptStart' : kind === 'B' ? 'commandStart' : kind === 'C' ? 'outputStart' : kind === 'D' ? 'commandEnd' : null
      if (!mark) return true
      if (mark === 'outputStart' && this.pending) {
        // shell 集成可用时以「输出开始」为界，丢掉提示符回显
        this.pending.sawOutputStart = true
        this.pending.chunks = []
      }
      if (mark === 'commandEnd' && this.pending) {
        this.settlePending(Number.isFinite(exitCode as number) ? (exitCode as number) : null, false)
      }
      this.hooks.onMark?.(mark, Number.isFinite(exitCode as number) ? (exitCode as number) : null)
      return true
    })
  }

  // ==================== 键盘 / 剪贴板 ====================

  private registerKeyHandler() {
    this.term.attachCustomKeyEventHandler((ev: KeyboardEvent) => {
      if (ev.type !== 'keydown') return true
      const mod = IS_MAC ? ev.metaKey : ev.ctrlKey

      // 复制：有选区时优先复制；无选区时 Ctrl+C 依然是 SIGINT（不拦截）
      if (mod && ev.code === 'KeyC') {
        const sel = this.term.getSelection()
        if (sel) {
          void this.copyToClipboard(sel)
          return false
        }
        return true
      }
      // 粘贴（含 Ctrl+Shift+V；Windows 下 Ctrl+V 也按粘贴处理，与 VS Code 终端一致）
      if (mod && ev.code === 'KeyV') {
        void this.pasteFromClipboard()
        return false
      }
      // 查找
      if (mod && ev.shiftKey && ev.code === 'KeyF') {
        this.hooks.onRequestSearch?.()
        return false
      }
      // 全选终端缓冲区
      if (mod && ev.code === 'KeyA') {
        this.term.selectAll()
        return false
      }
      // 清屏（本地清屏，shell 里也能用 Ctrl+L，这里只是更顺手）
      if (mod && ev.code === 'KeyK') {
        this.term.clear()
        return false
      }
      return true
    })
  }

  async copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text)
    } catch (e) {
      this.hooks.onError?.(`复制失败：${e instanceof Error ? e.message : String(e)}`)
    }
  }

  async pasteFromClipboard() {
    try {
      const text = await navigator.clipboard.readText()
      if (text) this.term.paste(text)
    } catch (e) {
      this.hooks.onError?.(`粘贴失败（剪贴板不可读）：${e instanceof Error ? e.message : String(e)}`)
    }
  }

  // ==================== 尺寸 ====================

  /** 重量容器尺寸 → cols/rows */
  private measure(): { cols: number; rows: number } {
    const dims = this.fitAddon.proposeDimensions()
    return {
      cols: Math.max(2, dims?.cols ?? 80),
      rows: Math.max(1, dims?.rows ?? 24),
    }
  }

  /**
   * 适配容器尺寸（下一帧执行，避免同一帧内多次 fit）。
   * 容器不可见（display:none / 0 尺寸）时跳过，否则会算出 0 列把 PTY 尺寸打歪。
   */
  fit() {
    if (this.disposed || this.fitRaf) return
    this.fitRaf = requestAnimationFrame(() => {
      this.fitRaf = 0
      if (this.disposed) return
      const el = this.term.element
      if (!el || !el.isConnected) return
      const rect = el.getBoundingClientRect()
      if (rect.width < 8 || rect.height < 8) return
      try {
        this.fitAddon.fit()
      } catch {
        /* 极端尺寸下 fit 可能抛错，忽略 */
      }
    })
  }

  /** 立即适配（不延帧）：面板刚显示 / 切标签时用 */
  fitNow() {
    if (this.disposed) return
    const el = this.term.element
    if (!el || !el.isConnected) return
    const rect = el.getBoundingClientRect()
    if (rect.width < 8 || rect.height < 8) return
    try {
      this.fitAddon.fit()
    } catch {
      /* ignore */
    }
  }

  // ==================== 外观 ====================

  applyAppearance(app: TerminalAppearance, palette: TermPalette) {
    this.appearance = app
    this.palette = palette
    this.term.options.fontFamily = app.fontFamily
    this.term.options.fontSize = app.fontSize
    this.term.options.lineHeight = app.lineHeight
    this.term.options.cursorStyle = app.cursorStyle
    this.term.options.cursorBlink = app.cursorBlink
    this.term.options.scrollback = app.scrollback
    this.term.options.theme = { ...palette }
    this.fit()
  }

  // ==================== 搜索 ====================

  findNext(term: string, opts?: { caseSensitive?: boolean; regex?: boolean; wholeWord?: boolean }) {
    return this.searchAddon.findNext(term, {
      caseSensitive: opts?.caseSensitive,
      regex: opts?.regex,
      wholeWord: opts?.wholeWord,
      incremental: true,
      decorations: this.decorations(),
    })
  }

  findPrevious(term: string, opts?: { caseSensitive?: boolean; regex?: boolean; wholeWord?: boolean }) {
    return this.searchAddon.findPrevious(term, {
      caseSensitive: opts?.caseSensitive,
      regex: opts?.regex,
      wholeWord: opts?.wholeWord,
      decorations: this.decorations(),
    })
  }

  clearSearch() {
    this.searchAddon.clearDecorations()
  }

  private decorations() {
    return searchDecorations(this.isDarkCache)
  }

  /** 由 store 在主题变化时写入，供搜索高亮取色 */
  isDarkCache = false

  // ==================== 异步执行 + 超时 ====================

  /**
   * 在会话里执行一条命令并等待结束。
   *
   * - 有 shell 集成（OSC 133）时：等到 `D;<退出码>` 得到真实退出码与输出；
   * - 无集成时：只能等到超时（`timedOut: true`，`exitCode` 为 null），
   *   此时命令仍在终端里正常执行，只是拿不到结果——这也是把集成程度暴露给 UI 的原因。
   */
  async exec(command: string, timeoutMs = 30_000): Promise<TermExecResult> {
    if (this.exited) throw new Error('会话已结束，无法执行命令')
    if (!this.sessionId) throw new Error('会话尚未就绪')
    if (this.pending) throw new Error('上一条命令仍在执行，请等待结束或增大超时')

    const startedAt = performance.now()
    const result = await new Promise<TermExecResult>((resolve) => {
      const timer = setTimeout(() => this.settlePending(null, true), timeoutMs)
      this.pending = {
        resolve,
        timer,
        startedAt,
        chunks: [],
        decoder: new TextDecoder('utf-8'),
        sawOutputStart: false,
      }
      // Enter 在终端里就是 CR
      void this.send(`${command}\r`).catch((e) => {
        clearTimeout(timer)
        this.pending = null
        resolve({
          exitCode: null,
          timedOut: false,
          output: '',
          elapsedMs: performance.now() - startedAt,
        })
        this.hooks.onError?.(e instanceof Error ? e.message : String(e))
      })
    })
    return result
  }

  private settlePending(exitCode: number | null, timedOut: boolean) {
    const p = this.pending
    if (!p) return
    this.pending = null
    clearTimeout(p.timer)
    const output = p.chunks.join('')
    p.resolve({
      exitCode,
      timedOut,
      output,
      elapsedMs: Math.round(performance.now() - p.startedAt),
    })
  }

  // ==================== 生命周期 ====================

  focus() {
    // 建会话是异步的：await 回来时窗格可能已经被关掉（xterm 已 dispose）
    if (this.disposed) return
    this.term.focus()
  }

  clear() {
    if (this.disposed) return
    this.term.clear()
  }

  /** 结束会话（kill shell + 释放后端资源）；不 dispose xterm */
  async kill() {
    if (this.sessionId) {
      const sid = this.sessionId
      this.sessionId = ''
      try {
        await closeTerminal(sid)
      } catch {
        /* 后端已回收时忽略 */
      }
    }
    this.exited = true
  }

  dispose() {
    if (this.disposed) return
    this.disposed = true
    if (this.fitRaf) cancelAnimationFrame(this.fitRaf)
    void this.kill()
    try {
      this.term.dispose()
    } catch {
      /* 已释放 */
    }
  }
}

// 搜索高亮取色由 theme.ts 的 searchDecorations(isDark) 提供（每次调用返回新对象，
// 避免 xterm 内部持有并被外部意外改写同一份常量）。
