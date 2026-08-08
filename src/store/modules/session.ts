/**
 * 会话 Pinia Store
 * 管理当前窗口的会话快照采集、保存与恢复
 */
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Window } from '@tauri-apps/api/window'
import type { SessionSnapshot, WindowState, OpenFileInfo, LayoutState } from '@/Types/workspace'
import { sessionService } from '@/services/sessionService'
import useStore from '@/store'

export const useSessionStore = defineStore('session', () => {
  /** 上次保存/加载的快照 */
  const lastSnapshot = ref<SessionSnapshot | null>(null)
  /** 是否启用自动保存 */
  const autoSaveEnabled = ref(true)
  /** 自动保存定时器 */
  let autoSaveTimer: ReturnType<typeof setInterval> | null = null

  /**
   * 采集当前窗口状态生成快照
   */
  async function captureSnapshot(): Promise<SessionSnapshot> {
    const { useDocumentManagerStore, system } = useStore()
    const docStore = useDocumentManagerStore

    // 获取窗口状态
    let windowState: WindowState = {
      x: 0, y: 0, width: 960, height: 640, is_maximized: false
    }
    try {
      const win = getCurrentWebviewWindow() as unknown as Window
      const pos = await win.innerPosition()
      const size = await win.innerSize()
      const maximized = await win.isMaximized()
      const scaleFactor = await win.scaleFactor()
      windowState = {
        x: pos.x / scaleFactor,
        y: pos.y / scaleFactor,
        width: size.width / scaleFactor,
        height: size.height / scaleFactor,
        is_maximized: maximized
      }
    } catch (_) { /* 使用默认值 */ }

    // 获取窗口标签和角色
    let windowLabel = 'main'
    let windowRole: 'main' | 'editor' | 'auxiliary' = 'main'
    try {
      const win = getCurrentWebviewWindow()
      const label = await win.label
      windowLabel = label
      if (label.startsWith('editor-')) windowRole = 'editor'
      else if (label.startsWith('app-window-')) windowRole = 'auxiliary'
    } catch (_) { /* 默认 main */ }

    // 采集打开文件信息
    const openFiles: OpenFileInfo[] = []
    for (const doc of docStore.documentList) {
      openFiles.push({
        path: doc.id,
        is_dirty: doc.isDirty,
        cursor_line: doc.viewState.cursor?.line ?? 0,
        cursor_col: doc.viewState.cursor?.column ?? 0,
        scroll_top: doc.viewState.scrollTop ?? 0
      })
    }

    // 获取工作区 ID
    const { useWorkspaceStore } = await import('./workspace')
    const workspaceStore = useWorkspaceStore()
    const workspaceId = workspaceStore.currentWorkspace?.id ?? null

    // 采集布局状态
    const { pluginManager } = await import('@/core/pluginManager')
    const layout: LayoutState = {
      sidebar_width: 220,
      sidebar_collapsed: system.isCollapse,
      active_plugin_id: pluginManager.activeToolId.value
    }

    const snapshot: SessionSnapshot = {
      id: `session-${windowLabel}-${Date.now()}`,
      workspace_id: workspaceId,
      window_label: windowLabel,
      window_role: windowRole,
      window_state: windowState,
      open_files: openFiles,
      active_file: docStore.activeDocumentPath,
      layout: layout,
      timestamp: Date.now()
    }

    lastSnapshot.value = snapshot
    return snapshot
  }

  /**
   * 保存当前快照到 Rust 后端
   */
  async function saveSnapshot() {
    try {
      const snapshot = await captureSnapshot()
      await sessionService.save(snapshot)
    } catch (e) {
      console.warn('保存会话快照失败:', e)
    }
  }

  /**
   * 从快照恢复窗口状态
   */
  async function restoreFromSnapshot(snapshot: SessionSnapshot) {
    const { useDocumentManagerStore, system } = useStore()
    const docStore = useDocumentManagerStore

    lastSnapshot.value = snapshot

    // 恢复打开的文件
    for (const fileInfo of snapshot.open_files) {
      try {
        await docStore.openDocument(fileInfo.path)
        // 恢复视图状态
        docStore.updateViewState(fileInfo.path, {
          cursor: { line: fileInfo.cursor_line, column: fileInfo.cursor_col },
          scrollTop: fileInfo.scroll_top
        })
      } catch (e) {
        console.warn(`恢复文件失败: ${fileInfo.path}`, e)
      }
    }

    // 恢复活跃文件
    if (snapshot.active_file) {
      docStore.setActiveDocument(snapshot.active_file)
    }

    // 恢复布局状态
    if (snapshot.layout) {
      if (snapshot.layout.sidebar_collapsed) {
        system.packCollapse()
      } else {
        system.expandCollapse()
      }
    }
  }

  /**
   * 加载当前窗口的会话快照
   */
  async function loadCurrentWindowSnapshot(): Promise<SessionSnapshot | null> {
    try {
      const win = getCurrentWebviewWindow()
      const label = await win.label
      return await sessionService.load(label)
    } catch (e) {
      console.warn('加载会话快照失败:', e)
      return null
    }
  }

  /**
   * 启动自动保存定时器
   */
  function startAutoSave(intervalMs: number = 60000) {
    stopAutoSave()
    if (!autoSaveEnabled.value) return

    autoSaveTimer = setInterval(async () => {
      const { useDocumentManagerStore } = useStore()
      const docStore = useDocumentManagerStore
      // 仅在有脏文档时保存
      if (docStore.hasUnsavedChanges) {
        await saveSnapshot()
      }
    }, intervalMs)
  }

  /**
   * 停止自动保存
   */
  function stopAutoSave() {
    if (autoSaveTimer) {
      clearInterval(autoSaveTimer)
      autoSaveTimer = null
    }
  }

  /**
   * 删除当前窗口的会话快照
   */
  async function deleteCurrentWindowSnapshot() {
    try {
      const win = getCurrentWebviewWindow()
      const label = await win.label
      await sessionService.delete(label)
      lastSnapshot.value = null
    } catch (e) {
      console.warn('删除会话快照失败:', e)
    }
  }

  return {
    // 状态
    lastSnapshot,
    autoSaveEnabled,
    // 方法
    captureSnapshot,
    saveSnapshot,
    restoreFromSnapshot,
    loadCurrentWindowSnapshot,
    startAutoSave,
    stopAutoSave,
    deleteCurrentWindowSnapshot
  }
})
