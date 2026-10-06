/**
 * 窗口命令（最小化 / 最大化 / 还原 / 关闭）与「是否最大化」状态。
 *
 * 为什么要有这一层：
 * 本项目所有窗口都是无边框的（`window_manager/*.rs` 里 5 处 `.decorations(false)`），
 * 标题栏是自绘的 —— 窗口控制按钮、双击标题栏、右键系统菜单**三处**都要发同一批命令。
 * 而「是否最大化」如果各处拿本地 ref 乐观记账，迟早和真实状态分叉：典型症状是
 * 窗口被 window-state 插件恢复成最大化启动时，按钮仍显示「最大化」，点一下才对上。
 * 所以这里把状态收敛成**单一事实源：每次直接问窗口要**，命令也只有一个出口。
 */
import { onMounted, onUnmounted, ref, type Ref } from 'vue'
import { getCurrentWindow } from '@/core/runtime'

/** 可执行的窗口命令。`toggle` = 按当前状态取反（双击标题栏用） */
export type WindowCommand = 'minimize' | 'maximize' | 'unmaximize' | 'toggle' | 'close'

export interface UseWindowCommands {
  /** 窗口当前是否最大化（始终与真实状态同步，不做乐观记账） */
  isMaximized: Ref<boolean>
  /** 立刻从窗口取一次最大化状态 */
  syncMaximized: () => Promise<void>
  /** 命令统一出口 */
  runWindowCommand: (cmd: WindowCommand) => Promise<void>
  minimize: () => Promise<void>
  maximize: () => Promise<void>
  unmaximize: () => Promise<void>
  toggleMaximize: () => Promise<void>
  closeWindow: () => Promise<void>
}

export function useWindowCommands(): UseWindowCommands {
  const isMaximized = ref(false)

  // 拖拽缩放期间 resize 会连发，用「在途标记」挡掉堆积的 IPC（每个查询都是一次跨进程调用）
  let syncing = false

  const syncMaximized = async () => {
    if (syncing) return
    syncing = true
    try {
      isMaximized.value = await getCurrentWindow().isMaximized()
    } catch (e) {
      console.warn('[useWindowCommands] 读取最大化状态失败:', e)
    } finally {
      syncing = false
    }
  }

  /**
   * 命令统一出口：失败必须冒出来（不空吞），否则「点了没反应」最难查。
   * 执行完再取一次真实状态 —— 不在这里猜结果。
   */
  const runWindowCommand = async (cmd: WindowCommand) => {
    try {
      const win = getCurrentWindow()
      switch (cmd) {
        case 'minimize':
          await win.minimize()
          break
        case 'maximize':
          await win.maximize()
          break
        case 'unmaximize':
          await win.unmaximize()
          break
        case 'close':
          await win.close()
          break
        case 'toggle':
          // 刻意用「查状态 → 下绝对值」而不是 Tauri 的 toggleMaximize()：
          // 绝对值幂等，即使事件被重复派发也不会在最大化/还原之间来回跳。
          if (await win.isMaximized()) await win.unmaximize()
          else await win.maximize()
          break
      }
      await syncMaximized()
    } catch (e) {
      console.error('[useWindowCommands] 窗口命令失败:', cmd, e)
    }
  }

  const minimize = () => runWindowCommand('minimize')
  const maximize = () => runWindowCommand('maximize')
  const unmaximize = () => runWindowCommand('unmaximize')
  const toggleMaximize = () => runWindowCommand('toggle')
  const closeWindow = () => runWindowCommand('close')

  onMounted(() => {
    // 挂载时先对齐一次（覆盖「上次以最大化关闭、本次被 window-state 恢复」这种情况）
    void syncMaximized()
    // 最大化/还原必定改变窗口尺寸 → resize 必达；由此也覆盖 Win+↑ / 拖拽吸附等外部操作
    window.addEventListener('resize', syncMaximized)
  })

  onUnmounted(() => {
    window.removeEventListener('resize', syncMaximized)
  })

  return {
    isMaximized,
    syncMaximized,
    runWindowCommand,
    minimize,
    maximize,
    unmaximize,
    toggleMaximize,
    closeWindow,
  }
}
