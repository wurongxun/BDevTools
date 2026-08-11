import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { FtpFileItem, TransferTask, FtpPanelState } from '../types'
import NotificationUtil from '@/utils/NotificationUtil'

let taskIdCounter = 0
function generateTaskId(): string {
  taskIdCounter++
  return `task-${Date.now()}-${taskIdCounter}`
}

export const useFtpStore = defineStore('ftp', () => {
  // 每个 tab 独立的 FTP 面板状态
  const panels = ref<Map<string, FtpPanelState>>(new Map())

  /**
   * 初始化 FTP 面板
   */
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

  /**
   * 获取面板状态
   */
  function getPanel(tabId: string): FtpPanelState | null {
    return panels.value.get(tabId) || null
  }

  /**
   * 销毁面板
   */
  function destroyPanel(tabId: string) {
    panels.value.delete(tabId)
  }

  /**
   * 列出远程目录文件
   */
  async function listRemoteFiles(tabId: string, sessionId: string, path: string) {
    const panel = panels.value.get(tabId)
    if (!panel) return

    try {
      const res = await invoke<any>('sftp_list_dir', { sessionId, path })
      console.log("listRemoteFiles ",path,res)
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

  /**
   * 列出本地目录文件
   */
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

  /**
   * 导航到远程目录
   */
  async function navigateRemote(tabId: string, sessionId: string, path: string) {
    await listRemoteFiles(tabId, sessionId, path)
  }

  /**
   * 导航到本地目录
   */
  async function navigateLocal(tabId: string, path: string) {
    await listLocalFiles(tabId, path)
  }

  /**
   * 上传文件
   */
  async function uploadFiles(tabId: string, sessionId: string, localFiles: string[], remotePath: string) {
    const panel = panels.value.get(tabId)
    if (!panel) return

    for (const localFile of localFiles) {
      const fileName = localFile.split(/[/\\]/).pop() || 'unknown'
      const task: TransferTask = {
        id: generateTaskId(),
        tabId,
        type: 'upload',
        fileName,
        remotePath: `${remotePath}/${fileName}`,
        localPath: localFile,
        totalSize: 0,
        transferredSize: 0,
        progress: 0,
        status: 'pending',
        speed: '',
        error: '',
        startTime: Date.now()
      }
      panel.transferTasks.unshift(task)

      try {
        task.status = 'transferring'
        await invoke('sftp_upload', {
          sessionId,
          localPath: localFile,
          remotePath: task.remotePath,
          taskId: task.id
        })
        task.status = 'completed'
        task.progress = 100
        NotificationUtil.success(`${fileName} 上传完成`)
      } catch (error) {
        task.status = 'failed'
        task.error = String(error)
        NotificationUtil.error(`${fileName} 上传失败: ${error}`)
      }
    }

    // 刷新远程文件列表
    await listRemoteFiles(tabId, sessionId, panel.remotePath)
  }

  /**
   * 下载文件
   */
  async function downloadFiles(tabId: string, sessionId: string, remoteFiles: string[], localPath: string) {
    const panel = panels.value.get(tabId)
    if (!panel) return

    for (const remoteFile of remoteFiles) {
      const fileName = remoteFile.split('/').pop() || 'unknown'
      const task: TransferTask = {
        id: generateTaskId(),
        tabId,
        type: 'download',
        fileName,
        remotePath: remoteFile,
        localPath: `${localPath}/${fileName}`,
        totalSize: 0,
        transferredSize: 0,
        progress: 0,
        status: 'pending',
        speed: '',
        error: '',
        startTime: Date.now()
      }
      panel.transferTasks.unshift(task)

      try {
        task.status = 'transferring'
        await invoke('sftp_download', {
          sessionId,
          remotePath: remoteFile,
          localPath: task.localPath,
          taskId: task.id
        })
        task.status = 'completed'
        task.progress = 100
        NotificationUtil.success(`${fileName} 下载完成`)
      } catch (error) {
        task.status = 'failed'
        task.error = String(error)
        NotificationUtil.error(`${fileName} 下载失败: ${error}`)
      }
    }

    // 刷新本地文件列表
    await listLocalFiles(tabId, panel.localPath)
  }

  /**
   * 创建远程目录
   */
  async function createRemoteDir(tabId: string, sessionId: string, path: string, name: string) {
    try {
      const res = await invoke<any>('sftp_mkdir', { sessionId, path: `${path}/${name}` })
      if (res.code === 200) {
        NotificationUtil.success('目录创建成功')
        const panel = panels.value.get(tabId)
        if (panel) {
          await listRemoteFiles(tabId, sessionId, panel.remotePath)
        }
      } else {
        NotificationUtil.error(res.msg || '创建目录失败')
      }
    } catch (error) {
      console.error('Failed to create remote dir:', error)
      NotificationUtil.error('创建目录失败')
    }
  }

  /**
   * 删除远程文件/目录
   */
  async function deleteRemoteFile(tabId: string, sessionId: string, path: string) {
    try {
      const res = await invoke<any>('sftp_remove', { sessionId, path })
      if (res.code === 200) {
        NotificationUtil.success('删除成功')
        const panel = panels.value.get(tabId)
        if (panel) {
          await listRemoteFiles(tabId, sessionId, panel.remotePath)
        }
      } else {
        NotificationUtil.error(res.msg || '删除失败')
      }
    } catch (error) {
      console.error('Failed to delete remote file:', error)
      NotificationUtil.error('删除失败')
    }
  }

  /**
   * 重命名远程文件/目录
   */
  async function renameRemoteFile(tabId: string, sessionId: string, oldPath: string, newPath: string) {
    try {
      const res = await invoke<any>('sftp_rename', { sessionId, oldPath, newPath })
      if (res.code === 200) {
        NotificationUtil.success('重命名成功')
        const panel = panels.value.get(tabId)
        if (panel) {
          await listRemoteFiles(tabId, sessionId, panel.remotePath)
        }
      } else {
        NotificationUtil.error(res.msg || '重命名失败')
      }
    } catch (error) {
      console.error('Failed to rename remote file:', error)
      NotificationUtil.error('重命名失败')
    }
  }

  /**
   * 检测 SFTP 连接状态
   */
  async function checkSftpConnection(sessionId: string): Promise<boolean> {
    try {
      const res = await invoke<any>('sftp_check', { sessionId })
      return res.code === 200 && res.data === true
    } catch {
      return false
    }
  }

  /**
   * 更新传输进度（由事件监听触发）
   */
  function updateTransferProgress(tabId: string, taskId: string, progress: number, transferredSize: number, speed: string) {
    const panel = panels.value.get(tabId)
    if (!panel) return

    const task = panel.transferTasks.find(t => t.id === taskId)
    if (task) {
      task.progress = progress
      task.transferredSize = transferredSize
      task.speed = speed
    }
  }

  /**
   * 选择远程文件
   */
  function selectRemoteFiles(tabId: string, files: string[]) {
    const panel = panels.value.get(tabId)
    if (panel) {
      panel.selectedRemoteFiles = files
    }
  }

  /**
   * 选择本地文件
   */
  function selectLocalFiles(tabId: string, files: string[]) {
    const panel = panels.value.get(tabId)
    if (panel) {
      panel.selectedLocalFiles = files
    }
  }

  /**
   * 清除已完成的任务
   */
  function clearCompletedTasks(tabId: string) {
    const panel = panels.value.get(tabId)
    if (panel) {
      panel.transferTasks = panel.transferTasks.filter(
        t => t.status !== 'completed' && t.status !== 'failed'
      )
    }
  }

  return {
    panels,
    initPanel,
    getPanel,
    destroyPanel,
    listRemoteFiles,
    listLocalFiles,
    navigateRemote,
    navigateLocal,
    uploadFiles,
    downloadFiles,
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
