/**
 * 会话 Pinia Store
 * 管理当前窗口的会话快照采集、保存与恢复
 */
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getCurrentWebviewWindow } from '@/core/runtime'
import type { Window } from '@/core/runtime'
import type { SessionSnapshot, WindowState, OpenFileInfo, LayoutState } from '@/Types/workspace'
import { sessionService } from '@/services/sessionService'
import useStore from '@/store'
import { useDocumentManagerStore, FileType } from '@/plugins/files/store/documents'

/** 草稿冲突项：目标文件在关闭后被外部修改，需用户决策 */
export interface DraftConflict {
  path: string
  /** 草稿保存时间（毫秒） */
  draftSavedAt: number
  /** 目标文件当前 mtime（秒） */
  fileMtime: number
}

export const useSessionStore = defineStore('session', () => {
  /** 上次保存/加载的快照 */
  const lastSnapshot = ref<SessionSnapshot | null>(null)
  /** 是否启用自动保存 */
  const autoSaveEnabled = ref(true)
  /** 自动保存定时器 */
  let autoSaveTimer: ReturnType<typeof setInterval> | null = null
  /** 本窗口已尝试恢复的工作区（幂等保护，避免重复恢复） */
  const restoredWorkspaces = new Set<string>()

  /**
   * 采集当前窗口状态生成快照
   */
  async function captureSnapshot(): Promise<SessionSnapshot> {
    const { system } = useStore()
    const docStore = useDocumentManagerStore()

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

    // 采集打开文件信息（脏文档同步写入草稿，记录指纹）
    const openFiles: OpenFileInfo[] = []

    // 获取工作区 ID（草稿/快照均按工作区隔离）
    const { useWorkspaceStore } = await import('./workspace')
    const workspaceStore = useWorkspaceStore()
    const workspaceId = workspaceStore.currentWorkspace?.id ?? null

    for (const doc of docStore.documentList) {
      let hasDraft = false
      let fileMtime = 0
      let fileSize = 0
      // 脏文档且内容为文本（远程文档不走本地草稿）：写入草稿保护未保存内容
      if (doc.isDirty && typeof doc.content === 'string' && !doc.isRemote && workspaceId) {
        const meta = await sessionService.draftSave(workspaceId, doc.id, doc.content).catch(() => null)
        if (meta) {
          hasDraft = true
          fileMtime = meta.file_mtime
          fileSize = meta.file_size
        }
      }
      openFiles.push({
        path: doc.id,
        is_dirty: doc.isDirty,
        cursor_line: doc.viewState.cursor?.line ?? 0,
        cursor_col: doc.viewState.cursor?.column ?? 0,
        scroll_top: doc.viewState.scrollTop ?? 0,
        has_draft: hasDraft,
        file_mtime: fileMtime,
        file_size: fileSize
      })
    }

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
   * 从快照恢复窗口状态（仅文件/布局，不含草稿冲突处理）
   */
  async function restoreFromSnapshot(snapshot: SessionSnapshot) {
    const { system } = useStore()
    const docStore = useDocumentManagerStore()

    lastSnapshot.value = snapshot

    // 恢复打开的文件（跳过脏文档，由 tryRestoreSession 处理草稿）
    for (const fileInfo of snapshot.open_files) {
      if (fileInfo.is_dirty) continue
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
   * 按工作区尝试恢复会话（窗口加载工作区后调用）
   * - 干净文件：直接打开
   * - 脏文件有草稿且指纹未变：静默还原草稿并标脏（内容仅在内存缓冲区，不落盘）
   * - 脏文件有草稿但目标文件被外部修改：返回冲突项由 UI 弹层让用户决策
   * @returns 草稿冲突列表（空数组表示无需处理）
   */
  async function tryRestoreSession(workspaceId: string): Promise<DraftConflict[]> {
    if (!workspaceId || restoredWorkspaces.has(workspaceId)) return []
    restoredWorkspaces.add(workspaceId)

    const snapshot = await sessionService.loadByWorkspace(workspaceId).catch(() => null)
    if (!snapshot) return []

    const { system } = useStore()
    const docStore = useDocumentManagerStore()
    lastSnapshot.value = snapshot
    const conflicts: DraftConflict[] = []

    for (const fileInfo of snapshot.open_files) {
      try {
        // 未命名文档（内容完全依赖草稿）：重建并还原
        if (fileInfo.path.startsWith('untitled:')) {
          if (fileInfo.has_draft) {
            const content = await sessionService.draftLoad(workspaceId, fileInfo.path)
            if (content !== null) {
              const newId = docStore.createNewDocument(FileType.MARKDOWN)
              docStore.updateDocumentContent(newId, content)
            }
          }
          continue
        }

        await docStore.openDocument(fileInfo.path)
        docStore.updateViewState(fileInfo.path, {
          cursor: { line: fileInfo.cursor_line, column: fileInfo.cursor_col },
          scrollTop: fileInfo.scroll_top
        })

        // 脏文档：指纹校验后决定静默还原还是进入冲突流程
        if (fileInfo.is_dirty && fileInfo.has_draft) {
          const check = await sessionService.draftCheck(workspaceId, fileInfo.path).catch(() => null)
          if (!check || !check.has_draft) continue
          if (!check.file_changed) {
            const content = await sessionService.draftLoad(workspaceId, fileInfo.path)
            // 还原到缓冲区；updateDocumentContent 会与磁盘内容比对自动标脏，写盘仍需用户显式保存
            if (content !== null) {
              docStore.updateDocumentContent(fileInfo.path, content)
            }
          } else {
            conflicts.push({
              path: fileInfo.path,
              draftSavedAt: check.draft_saved_at,
              fileMtime: check.file_mtime
            })
          }
        }
      } catch (e) {
        console.warn(`恢复文件失败: ${fileInfo.path}`, e)
      }
    }

    // 恢复活跃文件与布局
    if (snapshot.active_file) {
      docStore.setActiveDocument(snapshot.active_file)
    }
    if (snapshot.layout) {
      if (snapshot.layout.sidebar_collapsed) {
        system.packCollapse()
      } else {
        system.expandCollapse()
      }
    }

    return conflicts
  }

  /**
   * 处理草稿冲突（用户在弹窗中选择后调用）
   * - restore：草稿覆盖到缓冲区并标脏（写盘仍需显式保存）
   * - saveAs：草稿另存为原文件旁的 .draft-时间戳.bak，缓冲区用磁盘版（最稳妥，两不丢）
   * - discard：放弃草稿，使用磁盘当前内容（后两种处理后删除草稿）
   */
  async function resolveDraftConflict(
    workspaceId: string,
    conflict: DraftConflict,
    action: 'restore' | 'saveAs' | 'discard'
  ) {
    const docStore = useDocumentManagerStore()
    try {
      if (action === 'restore') {
        const content = await sessionService.draftLoad(workspaceId, conflict.path)
        if (content !== null) {
          await docStore.openDocument(conflict.path)
          docStore.updateDocumentContent(conflict.path, content)
        }
        // 保留草稿：缓冲区仍为脏，若再次崩溃由下次快照/自动保存接续保护
        return
      }

      if (action === 'saveAs') {
        const content = await sessionService.draftLoad(workspaceId, conflict.path)
        if (content !== null) {
          const stamp = new Date(conflict.draftSavedAt).toISOString().replace(/[:.]/g, '-')
          const target = `${conflict.path}.draft-${stamp}.bak`
          try {
            const { invoke } = await import('@/api/rpc')
            await invoke('save_note', { path: target, mdText: content })
          } catch (e) {
            console.warn('草稿另存失败:', e)
          }
        }
      }

      // discard / saveAs 后清理草稿，缓冲区保留磁盘内容（若未打开则打开磁盘版）
      await docStore.openDocument(conflict.path)
      await sessionService.draftDelete(workspaceId, conflict.path)
    } catch (e) {
      console.warn('处理草稿冲突失败:', e)
    }
  }

  /**
   * 清理已落盘文档的草稿（关闭流程中 saveAllDocuments 之后调用）
   * 文档已保存则草稿失去保护意义，避免草稿堆积；未保存文档的草稿保留。
   */
  async function cleanupSavedDrafts(workspaceId: string) {
    if (!workspaceId) return
    const docStore = useDocumentManagerStore()
    for (const doc of docStore.documentList) {
      if (!doc.isDirty && !doc.isRemote && !doc.id.startsWith('untitled:')) {
        await sessionService.draftDelete(workspaceId, doc.id).catch(() => {})
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
      const docStore = useDocumentManagerStore()
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
    tryRestoreSession,
    resolveDraftConflict,
    cleanupSavedDrafts,
    loadCurrentWindowSnapshot,
    startAutoSave,
    stopAutoSave,
    deleteCurrentWindowSnapshot
  }
})
