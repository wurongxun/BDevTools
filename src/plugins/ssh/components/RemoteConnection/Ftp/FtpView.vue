<template>
  <div class="ftp-view">
    <!-- FTP 工具栏 -->
    <div class="ftp-toolbar">
      <el-button size="small" @click="handleConnect" :loading="isConnecting">
        {{ isConnected ? '已连接' : '连接' }}
      </el-button>
      <el-button size="small" @click="handleUpload" :disabled="!isConnected || selectedLocalFiles.length === 0">
        <el-icon><Upload /></el-icon> 上传
      </el-button>
      <el-button size="small" @click="handleDownload" :disabled="!isConnected || selectedRemoteFiles.length === 0">
        <el-icon><Download /></el-icon> 下载
      </el-button>
      <el-button size="small" @click="handleNewFolder" :disabled="!isConnected">
        <el-icon><FolderAdd /></el-icon> 新建目录
      </el-button>
      <el-button size="small" @click="handleDeleteSelected" :disabled="!isConnected || selectedRemoteFiles.length === 0">
        <el-icon><Delete /></el-icon> 删除
      </el-button>
      <el-button size="small" @click="handleRename" :disabled="!isConnected || selectedRemoteFiles.length !== 1">
        <el-icon><EditPen /></el-icon> 重命名
      </el-button>
      <el-button size="small" @click="handleRefresh" :disabled="!isConnected">
        <el-icon><Refresh /></el-icon> 刷新
      </el-button>
      <!-- 书签下拉 -->
      <el-dropdown trigger="click" :disabled="!isConnected">
        <el-button size="small">
          <el-icon><Star /></el-icon> 书签<el-icon class="el-icon--right"><ArrowDown /></el-icon>
        </el-button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item
              v-for="(bm, idx) in bookmarks"
              :key="idx"
              @click="navigateToBookmark(bm.path)"
            >
              <span style="flex:1">{{ bm.name }}</span>
              <el-icon
                style="margin-left:8px;color:#888"
                @click.stop="handleRemoveBookmark(bm.path)"
                title="删除书签"
              ><Close /></el-icon>
            </el-dropdown-item>
            <el-dropdown-item v-if="bookmarks.length === 0" disabled>
              暂无书签，右键目录可保存
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>

    <!-- 双栏文件管理器 -->
    <div class="ftp-panels">
      <!-- 本地文件面板 -->
      <div class="ftp-panel local-panel">
        <div class="panel-header">
          <span class="panel-title">本地文件</span>
          <div class="panel-path">
            <el-input
              v-model="localPath"
              size="small"
              @keyup.enter="navigateLocalDir"
              placeholder="本地路径"
            />
            <el-button size="small" @click="browseLocalPath">浏览</el-button>
            <el-button size="small" @click="goUpLocalDir" title="上一级">
              <el-icon><ArrowUp /></el-icon>
            </el-button>
          </div>
        </div>
        <div class="file-list">
          <div
            v-for="file in localFiles"
            :key="file.path"
            class="file-item"
            :class="{
              'file-item-selected': selectedLocalFiles.includes(file.path),
              'file-item-dir': file.isDirectory
            }"
            @click="handleLocalFileClick(file, $event)"
            @dblclick="handleLocalFileDblClick(file)"
          >
            <span class="file-icon">
              {{ file.isDirectory ? '📁' : getFileIcon(file.name) }}
            </span>
            <span class="file-name">{{ file.name }}</span>
            <span class="file-size">{{ file.isDirectory ? '' : formatSize(file.size) }}</span>
            <span class="file-time">{{ file.modifiedTime }}</span>
          </div>
          <div v-if="localFiles.length === 0" class="empty-tip">
            请选择本地目录
          </div>
        </div>
      </div>

      <!-- 分隔线 -->
      <div class="panel-divider" />

      <!-- 远程文件面板 -->
      <div class="ftp-panel remote-panel">
        <div class="panel-header">
          <span class="panel-title">远程文件</span>
          <!-- 面包屑导航 -->
          <div class="breadcrumb" v-if="isConnected">
            <span
              v-for="(part, idx) in breadcrumbParts"
              :key="idx"
              class="breadcrumb-item"
              @click="navigateToBreadcrumb(idx)"
            >
              {{ part }}
              <span v-if="idx < breadcrumbParts.length - 1" class="breadcrumb-sep">/</span>
            </span>
          </div>
          <div class="panel-path">
            <el-input
              v-model="remotePath"
              size="small"
              @keyup.enter="navigateRemoteDir"
              placeholder="远程路径"
              :disabled="!isConnected"
            />
            <el-button size="small" @click="goUpDir" :disabled="!isConnected || remotePath === '/'" title="上一级">
              <el-icon><ArrowUp /></el-icon>
            </el-button>
          </div>
        </div>
        <div class="file-list">
          <!-- 上级目录 -->
          <div
            v-if="remotePath !== '/' && remotePath !== ''"
            class="file-item file-item-dir"
            @dblclick="goUpDir"
          >
            <span class="file-icon">📂</span>
            <span class="file-name">..</span>
            <span class="file-size"></span>
            <span class="file-time"></span>
          </div>
          <div
            v-for="file in remoteFiles"
            :key="file.path"
            class="file-item"
            :class="{
              'file-item-selected': selectedRemoteFiles.includes(file.path),
              'file-item-dir': file.isDirectory
            }"
            @click="handleRemoteFileClick(file, $event)"
            @dblclick="handleRemoteFileDblClick(file)"
            @contextmenu="handleRemoteFileContext(file, $event)"
          >
            <span class="file-icon">
              {{ file.isDirectory ? '📁' : getFileIcon(file.name) }}
            </span>
            <span class="file-name">{{ file.name }}</span>
            <span class="file-size">{{ file.isDirectory ? '' : formatSize(file.size) }}</span>
            <span class="file-time">{{ file.modifiedTime }}</span>
          </div>
          <div v-if="remoteFiles.length === 0 && isConnected" class="empty-tip">
            远程目录为空
          </div>
          <div v-if="!isConnected" class="empty-tip">
            请先连接远程服务器
          </div>
        </div>
      </div>
    </div>

    <!-- 传输面板 -->
    <TransferPanel :tab-id="tab.id" />

    <!-- 右键上下文菜单 -->
    <teleport to="body">
      <div
        v-if="contextMenuVisible"
        class="ftp-context-menu"
        :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }"
        @click.stop
      >
        <div class="context-menu-item" @click="handleSaveBookmark">
          <el-icon><Star /></el-icon> 保存为书签
        </div>
      </div>
    </teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { desktopDir, dirname } from '@tauri-apps/api/path'
import { open } from '@tauri-apps/plugin-dialog'
import { Upload, Download, FolderAdd, Delete, Refresh, EditPen, ArrowUp, ArrowDown, Star, Close } from '@element-plus/icons-vue'
import { ElMessageBox, ElInput } from 'element-plus'
import useStore from '@/store'
import { ConnectionStatus } from '@/entity/entitys'
import type { ConnectionTab, RemoteConnection, FtpFileItem } from '@/entity/entitys'
import { useDocumentManagerStore } from '@/store/modules/documents'
import NotificationUtil from '@/utils/NotificationUtil'
import TransferPanel from '../TransferPanel.vue'

const props = defineProps<{
  tab: ConnectionTab
  connection: RemoteConnection
}>()

const { useConnectionStore, useFtpStore } = useStore()

const isConnecting = ref(false)

const panel = computed(() => useFtpStore.getPanel(props.tab.id))
const isConnected = computed(() => props.tab.status === ConnectionStatus.CONNECTED)

// remotePath / localPath 直接读写 store panel，消除双源不同步
const remotePath = computed({
  get: () => panel.value?.remotePath || '/',
  set: (v: string) => { if (panel.value) panel.value.remotePath = v }
})
const localPath = computed({
  get: () => panel.value?.localPath || '',
  set: (v: string) => { if (panel.value) panel.value.localPath = v }
})

const selectedLocalFiles = ref<string[]>([])
const selectedRemoteFiles = ref<string[]>([])

const localFiles = computed(() => panel.value?.localFiles || [])
const remoteFiles = computed(() => panel.value?.remoteFiles || [])

// 面包屑导航
const breadcrumbParts = computed(() => {
  const path = remotePath.value
  if (!path || path === '/') return ['根目录']
  return path.split('/').filter(Boolean)
})

let sftpSessionId: string | null = null
let connectionCheckTimer: ReturnType<typeof setInterval> | null = null

// 书签
interface Bookmark { name: string; path: string }
const bookmarks = ref<Bookmark[]>([])
const contextMenuVisible = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextMenuTarget = ref<FtpFileItem | null>(null)

// 加载书签
function loadBookmarks() {
  try {
    const key = `ftp-bookmarks-${props.connection.address}:${props.connection.username}`
    const saved = localStorage.getItem(key)
    if (saved) bookmarks.value = JSON.parse(saved)
  } catch { /* ignore */ }
}
function saveBookmarks() {
  try {
    const key = `ftp-bookmarks-${props.connection.address}:${props.connection.username}`
    localStorage.setItem(key, JSON.stringify(bookmarks.value))
  } catch { /* ignore */ }
}

onMounted(async () => {
  useFtpStore.initPanel(props.tab.id)
  loadBookmarks()
  window.addEventListener('click', handleGlobalClick)
  // 本地目录默认读取桌面目录
  try {
    const desktop = await desktopDir()
    localPath.value = desktop
    await useFtpStore.listLocalFiles(props.tab.id, desktop)
  } catch {
    // fallback: 不加载
  }
})

onUnmounted(() => {
  stopConnectionCheck()
  window.removeEventListener('click', handleGlobalClick)
  if (sftpSessionId) {
    invoke('sftp_disconnect', { sessionId: sftpSessionId }).catch(() => {})
  }
  useFtpStore.destroyPanel(props.tab.id)
})

async function handleConnect() {
  if (isConnected.value) return

  isConnecting.value = true
  useConnectionStore.updateTabStatus(props.tab.id, ConnectionStatus.CONNECTING)

  try {
    const res = await invoke<any>('sftp_connect', {
      tabId: props.tab.id,
      host: props.connection.address,
      port: props.connection.port,
      username: props.connection.username,
      password: props.connection.password,
      authType: props.connection.auth_type || 'password',
      privateKeyPath: props.connection.private_key_path || ''
    })

    if (res.code === 200) {
      sftpSessionId = res.data
      useConnectionStore.updateTabStatus(props.tab.id, ConnectionStatus.CONNECTED)
      // 列出根目录
      if (sftpSessionId) {
        await useFtpStore.listRemoteFiles(props.tab.id, sftpSessionId, '/')
      }
      // 启动连接状态定时检测
      startConnectionCheck()
    } else {
      useConnectionStore.updateTabStatus(props.tab.id, ConnectionStatus.ERROR)
    }
  } catch (error) {
    useConnectionStore.updateTabStatus(props.tab.id, ConnectionStatus.ERROR)
    console.error('SFTP connect error:', error)
  } finally {
    isConnecting.value = false
  }
}

async function handleUpload() {
  if (!sftpSessionId || selectedLocalFiles.value.length === 0) return
  await useFtpStore.uploadFiles(
    props.tab.id,
    sftpSessionId,
    selectedLocalFiles.value,
    remotePath.value
  )
}

async function handleDownload() {
  if (!sftpSessionId || selectedRemoteFiles.value.length === 0) return
  const dir = await open({ directory: true, multiple: false })
  if (dir) {
    await useFtpStore.downloadFiles(
      props.tab.id,
      sftpSessionId,
      selectedRemoteFiles.value,
      dir
    )
  }
}

async function handleNewFolder() {
  if (!sftpSessionId) return
  try {
    const { value } = await ElMessageBox.prompt('请输入目录名称', '新建远程目录', {
      confirmButtonText: '创建',
      cancelButtonText: '取消'
    })
    if (value) {
      await useFtpStore.createRemoteDir(props.tab.id, sftpSessionId, remotePath.value, value)
    }
  } catch {
    // cancelled
  }
}

async function handleDeleteSelected() {
  if (!sftpSessionId || selectedRemoteFiles.value.length === 0) return
  try {
    await ElMessageBox.confirm(
      `确定要删除选中的 ${selectedRemoteFiles.value.length} 个文件/目录吗？`,
      '确认删除',
      { type: 'warning' }
    )
    for (const path of selectedRemoteFiles.value) {
      await useFtpStore.deleteRemoteFile(props.tab.id, sftpSessionId, path)
    }
  } catch {
    // cancelled
  }
}

async function handleRefresh() {
  if (!sftpSessionId) return
  await useFtpStore.listRemoteFiles(props.tab.id, sftpSessionId, remotePath.value)
  if (localPath.value) {
    await useFtpStore.listLocalFiles(props.tab.id, localPath.value)
  }
}

function handleLocalFileClick(file: FtpFileItem, event: MouseEvent) {
  if (event.ctrlKey || event.metaKey) {
    const idx = selectedLocalFiles.value.indexOf(file.path)
    if (idx >= 0) {
      selectedLocalFiles.value.splice(idx, 1)
    } else {
      selectedLocalFiles.value.push(file.path)
    }
  } else {
    selectedLocalFiles.value = [file.path]
  }
}

function handleLocalFileDblClick(file: FtpFileItem) {
  if (file.isDirectory) {
    localPath.value = file.path
    useFtpStore.listLocalFiles(props.tab.id, file.path)
  }
}

function handleRemoteFileClick(file: FtpFileItem, event: MouseEvent) {
  if (event.ctrlKey || event.metaKey) {
    const idx = selectedRemoteFiles.value.indexOf(file.path)
    if (idx >= 0) {
      selectedRemoteFiles.value.splice(idx, 1)
    } else {
      selectedRemoteFiles.value.push(file.path)
    }
  } else {
    selectedRemoteFiles.value = [file.path]
  }
}

function handleRemoteFileDblClick(file: FtpFileItem) {
  if (file.isDirectory && sftpSessionId) {
    remotePath.value = file.path
    useFtpStore.listRemoteFiles(props.tab.id, sftpSessionId, file.path)
  } else if (!file.isDirectory && sftpSessionId) {
    // 双击打开远程文件
    handleOpenRemoteFile(file)
  }
}

async function handleOpenRemoteFile(file: FtpFileItem) {
  if (!sftpSessionId) return
  try {
    // 获取应用数据目录
    const dirRes = await invoke<any>('get_app_dir')
    if (dirRes.code !== 200) return
    const appDir = dirRes.data
    // 构造本地临时路径
    const fileName = file.name
    const relativePath = file.path.startsWith('/') ? file.path.substring(1) : file.path
    const localDir = `${appDir}/tmp/${sftpSessionId}`
    const localFilePath = `${localDir}/${relativePath.replace(/\//g, '_')}`
    // 确保临时目录存在
    await invoke('ensure_dir', { path: localDir })
    // 下载文件到临时目录
    const taskId = `download-${Date.now()}`
    const dlRes = await invoke<any>('sftp_download', {
      sessionId: sftpSessionId,
      remotePath: file.path,
      localPath: localFilePath,
      taskId
    })
    if (dlRes.code === 200) {
      // 打开远程文档
      const docStore = useDocumentManagerStore()
      const remoteKey = `remote:${sftpSessionId}:${file.path}`
      await docStore.openRemoteDocument({
        key: remoteKey,
        localTempPath: localFilePath,
        remotePath: file.path,
        sessionId: sftpSessionId,
        fileName: fileName
      })
    } else {
      NotificationUtil.error('下载远程文件失败: ' + dlRes.msg)
    }
  } catch (error) {
    console.error('Open remote file error:', error)
    NotificationUtil.error('打开远程文件失败')
  }
}

async function handleRename() {
  if (!sftpSessionId || selectedRemoteFiles.value.length !== 1) return
  const oldPath = selectedRemoteFiles.value[0]
  const oldName = oldPath.split('/').pop() || ''
  try {
    const { value } = await ElMessageBox.prompt('请输入新名称', '重命名', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      inputValue: oldName
    })
    if (value && value !== oldName) {
      const parentPath = oldPath.substring(0, oldPath.lastIndexOf('/'))
      const newPath = `${parentPath}/${value}`
      await useFtpStore.renameRemoteFile(props.tab.id, sftpSessionId, oldPath, newPath)
      selectedRemoteFiles.value = []
    }
  } catch {
    // cancelled
  }
}

function goUpDir() {
  if (!sftpSessionId) return
  const currentPath = remotePath.value
  if (!currentPath || currentPath === '/' || currentPath === '') return
  const parts = currentPath.split('/').filter(Boolean)
  parts.pop()
  const parentPath = parts.length > 0 ? '/' + parts.join('/') : '/'
  remotePath.value = parentPath
  useFtpStore.listRemoteFiles(props.tab.id, sftpSessionId, parentPath)
}

async function goUpLocalDir() {
  const currentPath = localPath.value
  if (!currentPath) return
  try {
    const parent = await dirname(currentPath)
    if (parent && parent !== currentPath) {
      localPath.value = parent
      await useFtpStore.listLocalFiles(props.tab.id, parent)
    }
  } catch {
    // fallback: 手动截取
    const idx = Math.max(currentPath.lastIndexOf('/'), currentPath.lastIndexOf('\\'))
    if (idx > 0) {
      const parent = currentPath.substring(0, idx)
      localPath.value = parent
      await useFtpStore.listLocalFiles(props.tab.id, parent)
    }
  }
}

function navigateToBreadcrumb(idx: number) {
  if (!sftpSessionId) return
  const parts = breadcrumbParts.value
  // 点击“根目录”回到 /
  if (parts[idx] === '根目录') {
    remotePath.value = '/'
  } else {
    remotePath.value = '/' + parts.slice(0, idx + 1).join('/')
  }
  useFtpStore.listRemoteFiles(props.tab.id, sftpSessionId, remotePath.value)
}

function navigateToBookmark(path: string) {
  if (!sftpSessionId) return
  remotePath.value = path
  useFtpStore.listRemoteFiles(props.tab.id, sftpSessionId, path)
}

// 右键菜单
function handleRemoteFileContext(file: FtpFileItem, event: MouseEvent) {
  event.preventDefault()
  if (!file.isDirectory) return
  contextMenuTarget.value = file
  contextMenuX.value = event.clientX
  contextMenuY.value = event.clientY
  contextMenuVisible.value = true
}

function handleSaveBookmark() {
  const file = contextMenuTarget.value
  if (!file || !file.isDirectory) return
  const name = file.name
  // 检查是否已存在
  if (bookmarks.value.some(b => b.path === file.path)) {
    NotificationUtil.error('该目录已在书签中')
    contextMenuVisible.value = false
    return
  }
  bookmarks.value.push({ name, path: file.path })
  saveBookmarks()
  NotificationUtil.success(`已保存书签: ${name}`)
  contextMenuVisible.value = false
}

function handleRemoveBookmark(path: string) {
  bookmarks.value = bookmarks.value.filter(b => b.path !== path)
  saveBookmarks()
}

// 点击外部关闭右键菜单
function handleGlobalClick() {
  contextMenuVisible.value = false
}

// 定时检测连接状态
function startConnectionCheck() {
  stopConnectionCheck()
  connectionCheckTimer = setInterval(async () => {
    if (!sftpSessionId || !isConnected.value) return
    const alive = await useFtpStore.checkSftpConnection(sftpSessionId)
    if (!alive) {
      useConnectionStore.updateTabStatus(props.tab.id, ConnectionStatus.DISCONNECTED)
      NotificationUtil.error('SFTP 连接已断开')
      stopConnectionCheck()
    }
  }, 30000) // 每 30 秒检测一次
}

function stopConnectionCheck() {
  if (connectionCheckTimer) {
    clearInterval(connectionCheckTimer)
    connectionCheckTimer = null
  }
}

function navigateLocalDir() {
  if (localPath.value) {
    useFtpStore.listLocalFiles(props.tab.id, localPath.value)
  }
}

function navigateRemoteDir() {
  if (sftpSessionId && remotePath.value) {
    useFtpStore.listRemoteFiles(props.tab.id, sftpSessionId, remotePath.value)
  }
}

async function browseLocalPath() {
  const dir = await open({ directory: true, multiple: false })
  if (dir) {
    localPath.value = dir
    await useFtpStore.listLocalFiles(props.tab.id, dir)
  }
}

function getFileIcon(name: string): string {
  const ext = name.split('.').pop()?.toLowerCase()
  const iconMap: Record<string, string> = {
    md: '📝', txt: '📄', js: '📜', ts: '📜', vue: '📜',
    json: '📋', xml: '📋', yaml: '📋', yml: '📋',
    png: '🖼️', jpg: '🖼️', jpeg: '🖼️', gif: '🖼️', svg: '🖼️',
    zip: '📦', tar: '📦', gz: '📦', rar: '📦',
    py: '🐍', java: '☕', rs: '🦀', go: '🔵', c: '🔧', cpp: '🔧'
  }
  return iconMap[ext || ''] || '📄'
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i]
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.ftp-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: #1e1e1e;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
  }
}

.ftp-toolbar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background-color: #252526;
  border-bottom: 1px solid #2d2d2d;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-bottom-color: theme.getVar('borderColor');
  }
}

.ftp-panels {
  flex: 1;
  display: flex;
  min-height: 0;
  overflow: hidden;
}

.ftp-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.panel-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  background-color: #2d2d2d;
  border-bottom: 1px solid #3e3e3e;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: rgba(255, 255, 255, 0.03);
    border-bottom-color: theme.getVar('borderColor');
  }
}

.panel-title {
  font-size: 12px;
  font-weight: 500;
  color: #ccc;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: 2px;
  font-size: 12px;
  color: #888;
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.breadcrumb-item {
  cursor: pointer;
  white-space: nowrap;
  color: #6699cc;

  &:hover {
    color: #88bbff;
    text-decoration: underline;
  }
}

.breadcrumb-sep {
  color: #555;
  margin: 0 2px;
  cursor: default;

  &:hover {
    color: #555;
    text-decoration: none;
  }
}

.panel-path {
  flex: 1;
  display: flex;
  gap: 4px;
}

.panel-divider {
  width: 1px;
  background-color: #3e3e3e;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('borderColor');
  }
}

.file-list {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  scrollbar-width: thin;

  &::-webkit-scrollbar {
    width: 6px;
  }
  &::-webkit-scrollbar-thumb {
    background-color: rgba(155, 155, 155, 0.3);
    border-radius: 3px;
  }
}

.file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 12px;
  font-size: 13px;
  color: #ccc;
  cursor: pointer;
  transition: background-color 0.1s;
  user-select: none;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }

  &:hover {
    background-color: rgba(255, 255, 255, 0.05);
  }
}

.file-item-selected {
  background-color: rgba(0, 122, 204, 0.3) !important;
}

.file-icon {
  flex-shrink: 0;
  width: 18px;
  text-align: center;
  font-size: 14px;
}

.file-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-size {
  flex-shrink: 0;
  width: 70px;
  text-align: right;
  font-size: 11px;
  color: #888;
}

.file-time {
  flex-shrink: 0;
  width: 130px;
  text-align: right;
  font-size: 11px;
  color: #888;
}

.empty-tip {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100px;
  color: #666;
  font-size: 13px;
}

.ftp-context-menu {
  position: fixed;
  z-index: 9999;
  background-color: #2d2d2d;
  border: 1px solid #454545;
  border-radius: 6px;
  padding: 4px 0;
  min-width: 160px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-color: theme.getVar('borderColor');
  }
}

.context-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  font-size: 13px;
  color: #ccc;
  cursor: pointer;
  transition: background-color 0.15s;

  &:hover {
    background-color: rgba(0, 122, 204, 0.3);
    color: #fff;
  }
}
</style>
