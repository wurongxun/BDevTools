import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@/api/rpc'
import type { TransferTask, TransferItem, FtpPanelState } from '../types'
import NotificationUtil from '@/utils/NotificationUtil'

let taskIdCounter = 0
function generateTaskId(): string {
  taskIdCounter++
  return `task-${Date.now()}-${taskIdCounter}`
}

const CONCURRENCY_KEY = 'ssh-transfer-concurrency'
function readConcurrency(): number {
  const n = Number(localStorage.getItem(CONCURRENCY_KEY))
  return Number.isFinite(n) && n >= 1 && n <= 8 ? Math.floor(n) : 3
}

/** 全局正在跑的任务 id。跨所有 FTP 面板共享上限，避免多个 tab 同时打满同一条 SSH 连接 */
const RUNNING = new Set<string>()
/** 499 = 后端协作式取消（用户暂停 / 取消） */
const CANCELLED_CODE = 499

export const useFtpStore = defineStore('ftp', () => {
  // 每个 tab 独立的 FTP 面板状态
  const panels = ref<Map<string, FtpPanelState>>(new Map())
  const concurrency = ref(readConcurrency())

  function setConcurrency(n: number) {
    const v = Math.max(1, Math.min(8, Math.floor(Number(n) || 3)))
    concurrency.value = v
    try { localStorage.setItem(CONCURRENCY_KEY, String(v)) } catch { /* ignore */ }
    pumpAll()
  }

  function initPanel(tabId: string) {
    if (!panels.value.has(tabId)) {
      panels.value.set(tabId, {
        tabId,
        remotePath: '/',
        localPath: '',
        remoteFiles: [],
        localFiles: [],
        selectedRemoteFiles: [],
        selectedLocalFiles: [],
        transferTasks: []
      })
    }
    return panels.value.get(tabId)!
  }

  function getPanel(tabId: string): FtpPanelState | null {
    return panels.value.get(tabId) || null
  }

  function destroyPanel(tabId: string) {
    // 关掉面板时取消仍在跑的任务，防止 spawn_blocking 继续写已销毁的 UI 状态
    const panel = panels.value.get(tabId)
    if (panel) {
      for (const t of panel.transferTasks) {
        if (t.status === 'transferring' || t.status === 'pending') {
          RUNNING.delete(t.id)
          if (t.status === 'transferring') {
            void invoke('sftp_cancel_transfer', { taskId: t.id }).catch(() => {})
          }
          t.status = 'cancelled'
        }
      }
    }
    panels.value.delete(tabId)
  }

  async function listRemoteFiles(tabId: string, sessionId: string, path: string) {
    const panel = panels.value.get(tabId)
    if (!panel) return
    try {
      const res = await invoke<any>('sftp_list_dir', { sessionId, path })
      if (res.code === 200) {
        panel.remoteFiles = res.data
        panel.remotePath = path
        panel.selectedRemoteFiles = []
      } else {
        NotificationUtil.error(res.msg || '列出远程文件失败')
      }
    } catch (error) {
      console.error('Failed to list remote files:', error)
      NotificationUtil.error('列出远程文件失败')
    }
  }

  async function listLocalFiles(tabId: string, path: string) {
    const panel = panels.value.get(tabId)
    if (!panel) return
    try {
      const res = await invoke<any>('local_list_dir', { path })
      if (res.code === 200) {
        panel.localFiles = res.data
        panel.localPath = path
        panel.selectedLocalFiles = []
      } else {
        NotificationUtil.error(res.msg || '列出本地文件失败')
      }
    } catch (error) {
      console.error('Failed to list local files:', error)
      NotificationUtil.error('列出本地文件失败')
    }
  }

  async function navigateRemote(tabId: string, sessionId: string, path: string) {
    await listRemoteFiles(tabId, sessionId, path)
  }

  async function navigateLocal(tabId: string, path: string) {
    await listLocalFiles(tabId, path)
  }

  // ==================== 传输队列（P1-C） ====================

  function makeTask(
    tabId: string,
    sessionId: string,
    type: 'upload' | 'download',
    item: TransferItem
  ): TransferTask {
    return {
      id: generateTaskId(),
      tabId,
      sessionId,
      type,
      fileName: item.fileName,
      remotePath: type === 'upload' ? item.targetPath : item.sourcePath,
      localPath: type === 'upload' ? item.sourcePath : item.targetPath,
      totalSize: item.size,
      transferredSize: 0,
      progress: 0,
      status: 'pending',
      speed: '',
      error: '',
      startTime: Date.now(),
      resume: false,
      pauseRequested: false
    }
  }

  /** 入队一批上传任务；items 已由调用方递归展开并算好完整目标路径 */
  function enqueueUploads(tabId: string, sessionId: string, items: TransferItem[]) {
    const panel = initPanel(tabId)
    for (const item of items) {
      panel.transferTasks.unshift(makeTask(tabId, sessionId, 'upload', item))
    }
    pumpAll()
  }

  /** 入队一批下载任务 */
  function enqueueDownloads(tabId: string, sessionId: string, items: TransferItem[]) {
    const panel = initPanel(tabId)
    for (const item of items) {
      panel.transferTasks.unshift(makeTask(tabId, sessionId, 'download', item))
    }
    pumpAll()
  }

  /** 全局调度：并发上限以内按入队顺序启动 pending 任务 */
  function pumpAll() {
    for (const panel of panels.value.values()) {
      while (RUNNING.size < concurrency.value) {
        // unshift 入队 ⇒ 数组头部是最新任务；从尾部取才是 FIFO
        const task = [...panel.transferTasks].reverse().find(t => t.status === 'pending')
        if (!task) break
        task.status = 'transferring'
        RUNNING.add(task.id)
        void runTask(panel, task)
      }
    }
  }

  async function runTask(panel: FtpPanelState, task: TransferTask) {
    try {
      const res = task.type === 'upload'
        ? await invoke<any>('sftp_upload', {
            sessionId: task.sessionId,
            localPath: task.localPath,
            remotePath: task.remotePath,
            taskId: task.id,
            resume: task.resume
          })
        : await invoke<any>('sftp_download', {
            sessionId: task.sessionId,
            remotePath: task.remotePath,
            localPath: task.localPath,
            taskId: task.id,
            resume: task.resume
          })

      if (res.code === 200) {
        task.status = 'completed'
        task.progress = 100
        task.transferredSize = task.totalSize || task.transferredSize
      } else if (res.code === CANCELLED_CODE) {
        task.status = task.pauseRequested ? 'paused' : 'cancelled'
      } else {
        task.status = 'failed'
        task.error = String(res.msg || '传输失败')
      }
    } catch (error) {
      if (task.pauseRequested) {
        task.status = 'paused'
      } else {
        task.status = 'failed'
        task.error = String(error)
      }
    } finally {
      RUNNING.delete(task.id)
      // 队列排空后刷新两侧列表（旧实现每次传输完都刷，多文件时会刷 N 次）
      if (!panel.transferTasks.some(t => t.status === 'transferring' || t.status === 'pending')) {
        void listRemoteFiles(panel.tabId, task.sessionId, panel.remotePath)
        if (panel.localPath) void listLocalFiles(panel.tabId, panel.localPath)
      }
      pumpAll()
    }
  }

  /** 暂停 / 取消是同一个后端动作，区别只在回落到哪个状态 */
  async function stopTask(tabId: string, taskId: string, pause: boolean) {
    const panel = panels.value.get(tabId)
    const task = panel?.transferTasks.find(t => t.id === taskId)
    if (!task) return
    if (task.status === 'transferring') {
      task.pauseRequested = pause
      await invoke('sftp_cancel_transfer', { taskId }).catch(() => {})
    } else if (task.status === 'pending') {
      // 还没开跑：直接落状态，避免随后被 pumpAll 拉起来
      task.status = pause ? 'paused' : 'cancelled'
    }
  }

  function pauseTask(tabId: string, taskId: string) {
    return stopTask(tabId, taskId, true)
  }

  function cancelTask(tabId: string, taskId: string) {
    return stopTask(tabId, taskId, false)
  }

  /** 重试：复用同一 taskId，置 resume=true 让后端按已有字节续传 */
  function retryTask(tabId: string, taskId: string) {
    const panel = panels.value.get(tabId)
    const task = panel?.transferTasks.find(t => t.id === taskId)
    if (!task || !['paused', 'cancelled', 'failed'].includes(task.status)) return
    task.status = 'pending'
    task.resume = true
    task.pauseRequested = false
    task.error = ''
    task.speed = ''
    task.progress = task.totalSize > 0
      ? Math.min(99, Math.round(task.transferredSize / task.totalSize * 100))
      : 0
    pumpAll()
  }

  function removeTask(tabId: string, taskId: string) {
    const panel = panels.value.get(tabId)
    if (!panel) return
    const task = panel.transferTasks.find(t => t.id === taskId)
    if (!task || task.status === 'transferring') return
    panel.transferTasks = panel.transferTasks.filter(t => t.id !== taskId)
  }

  /** 大小预取：列目录结果里已有 size 时直接透传，拖放进来的路径单独 stat */
  async function statLocal(path: string): Promise<number> {
    try {
      const res = await invoke<any>('local_stat', { path })
      return res.code === 200 ? Number(res.data?.size) || 0 : 0
    } catch { return 0 }
  }

  /** 修改远端权限（mode 为八进制数值，如 0o755 = 493） */
  async function chmodRemote(tabId: string, sessionId: string, path: string, mode: number) {
    try {
      const res = await invoke<any>('sftp_chmod', { sessionId, path, mode })
      if (res.code === 200) {
        NotificationUtil.success('权限已修改')
        const panel = panels.value.get(tabId)
        if (panel) await listRemoteFiles(tabId, sessionId, panel.remotePath)
      } else {
        NotificationUtil.error(res.msg || '修改权限失败')
      }
    } catch (error) {
      NotificationUtil.error(`修改权限失败: ${error}`)
    }
  }

  /**
   * 更新传输进度。后端 transfer-progress 的 tabId 为空串（历史实现），
   * 因此优先按 taskId 跨面板查找；totalSize 由后端下发，作为权威值覆盖预取值。
   */
  function updateTransferProgress(
    tabId: string,
    taskId: string,
    progress: number,
    transferredSize: number,
    speed: string,
    totalSize?: number
  ) {
    let panel: FtpPanelState | undefined = tabId ? panels.value.get(tabId) : undefined
    if (!panel) {
      for (const p of panels.value.values()) {
        if (p.transferTasks.some(t => t.id === taskId)) { panel = p; break }
      }
    }
    if (!panel) return
    const task = panel.transferTasks.find(t => t.id === taskId)
    if (!task) return
    if (Number.isFinite(totalSize) && (totalSize as number) > 0) {
      task.totalSize = totalSize as number
    }
    if (task.status === 'pending' || task.status === 'transferring') {
      task.progress = progress
      task.transferredSize = transferredSize
      task.speed = speed
    }
  }

  function selectRemoteFiles(tabId: string, files: string[]) {
    const panel = panels.value.get(tabId)
    if (panel) panel.selectedRemoteFiles = files
  }

  function selectLocalFiles(tabId: string, files: string[]) {
    const panel = panels.value.get(tabId)
    if (panel) panel.selectedLocalFiles = files
  }

  function clearCompletedTasks(tabId: string) {
    const panel = panels.value.get(tabId)
    if (panel) {
      panel.transferTasks = panel.transferTasks.filter(
        t => !['completed', 'failed', 'cancelled'].includes(t.status)
      )
    }
  }

  async function createRemoteDir(tabId: string, sessionId: string, path: string, name: string) {
    try {
      const res = await invoke<any>('sftp_mkdir', { sessionId, path: `${path}/${name}` })
      if (res.code === 200) {
        NotificationUtil.success('目录创建成功')
        if (panels.value.has(tabId)) {
          const p = panels.value.get(tabId)!
          await listRemoteFiles(tabId, sessionId, p.remotePath)
        }
      } else {
        NotificationUtil.error(res.msg || '创建目录失败')
      }
    } catch (error) {
      NotificationUtil.error('创建目录失败')
    }
  }

  async function deleteRemoteFile(tabId: string, sessionId: string, path: string) {
    try {
      const res = await invoke<any>('sftp_remove', { sessionId, path })
      if (res.code === 200) {
        NotificationUtil.success('删除成功')
        if (panels.value.has(tabId)) {
          const p = panels.value.get(tabId)!
          await listRemoteFiles(tabId, sessionId, p.remotePath)
        }
      } else {
        NotificationUtil.error(res.msg || '删除失败')
      }
    } catch { NotificationUtil.error('删除失败') }
  }

  async function renameRemoteFile(tabId: string, sessionId: string, oldPath: string, newPath: string) {
    try {
      const res = await invoke<any>('sftp_rename', { sessionId, oldPath, newPath })
      if (res.code === 200) {
        NotificationUtil.success('重命名成功')
        if (panels.value.has(tabId)) {
          const p = panels.value.get(tabId)!
          await listRemoteFiles(tabId, sessionId, p.remotePath)
        }
      } else {
        NotificationUtil.error(res.msg || '重命名失败')
      }
    } catch { NotificationUtil.error('重命名失败') }
  }

  async function checkSftpConnection(sessionId: string): Promise<boolean> {
    try {
      const res = await invoke<any>('sftp_check', { sessionId })
      return res.code === 200 && res.data === true
    } catch { return false }
  }

  return {
    panels,
    concurrency,
    setConcurrency,
    initPanel,
    getPanel,
    destroyPanel,
    listRemoteFiles,
    listLocalFiles,
    navigateRemote,
    navigateLocal,
    enqueueUploads,
    enqueueDownloads,
    pauseTask,
    cancelTask,
    retryTask,
    removeTask,
    statLocal,
    chmodRemote,
    createRemoteDir,
    deleteRemoteFile,
    renameRemoteFile,
    checkSftpConnection,
    updateTransferProgress,
    selectRemoteFiles,
    selectLocalFiles,
    clearCompletedTasks
  }
})
