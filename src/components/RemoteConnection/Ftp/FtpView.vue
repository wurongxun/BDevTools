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
      <el-button size="small" @click="handleRefresh" :disabled="!isConnected">
        <el-icon><Refresh /></el-icon> 刷新
      </el-button>
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
          <div class="panel-path">
            <el-input
              v-model="remotePath"
              size="small"
              @keyup.enter="navigateRemoteDir"
              placeholder="远程路径"
              :disabled="!isConnected"
            />
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
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { Upload, Download, FolderAdd, Delete, Refresh } from '@element-plus/icons-vue'
import { ElMessageBox, ElInput } from 'element-plus'
import useStore from '@/store'
import { ConnectionStatus } from '@/entity/entitys'
import type { ConnectionTab, RemoteConnection, FtpFileItem } from '@/entity/entitys'
import TransferPanel from '../TransferPanel.vue'

const props = defineProps<{
  tab: ConnectionTab
  connection: RemoteConnection
}>()

const { useConnectionStore, useFtpStore } = useStore()

const isConnecting = ref(false)
const localPath = ref('')
const remotePath = ref('/')
const selectedLocalFiles = ref<string[]>([])
const selectedRemoteFiles = ref<string[]>([])

const panel = computed(() => useFtpStore.getPanel(props.tab.id))
const isConnected = computed(() => props.tab.status === ConnectionStatus.CONNECTED)
const localFiles = computed(() => panel.value?.localFiles || [])
const remoteFiles = computed(() => panel.value?.remoteFiles || [])

let sftpSessionId: string | null = null

onMounted(() => {
  useFtpStore.initPanel(props.tab.id)
})

onUnmounted(() => {
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
      password: props.connection.password
    })

    if (res.code === 200) {
      sftpSessionId = res.data
      useConnectionStore.updateTabStatus(props.tab.id, ConnectionStatus.CONNECTED)
      // 列出根目录
      if (sftpSessionId) {
        await useFtpStore.listRemoteFiles(props.tab.id, sftpSessionId, '/')
      }
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
  }
}

function goUpDir() {
  if (!sftpSessionId) return
  const parts = remotePath.value.split('/').filter(Boolean)
  parts.pop()
  const parentPath = '/' + parts.join('/')
  remotePath.value = parentPath || '/'
  useFtpStore.listRemoteFiles(props.tab.id, sftpSessionId, remotePath.value)
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
</style>
