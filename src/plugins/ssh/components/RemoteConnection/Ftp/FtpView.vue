<template>
  <div class="ftp-view">
    <!-- FTP 工具栏 -->
    <div class="ftp-toolbar">
      <button
        class="tool-btn"
        :class="isConnected ? 'tool-btn-active' : ''"
        @click="handleConnect"
        :disabled="isConnecting"
        :title="isConnected ? '已连接' : '连接'"
      >
        <svg v-if="isConnecting" class="tool-icon spinning" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1a7 7 0 0 1 7 7h-1.5A5.5 5.5 0 0 0 8 2.5V1z"/></svg>
        <svg v-else-if="isConnected" class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M4.5 1a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 4.5 1zm7 0a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 11.5 1zM8 5a3 3 0 1 0 0 6 3 3 0 0 0 0-6z"/></svg>
        <svg v-else class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M4.5 1a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 4.5 1zm7 0a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 11.5 1zM8 5a3 3 0 1 0 0 6 3 3 0 0 0 0-6z"/></svg>
        <span class="tool-label">{{ isConnected ? '已连接' : '连接' }}</span>
      </button>
      <button class="tool-btn" @click="handleUpload" :disabled="!isConnected || selectedLocalFiles.length === 0" title="上传">
        <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M.5 9.9a.5.5 0 0 1 .5.5v2.5a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-2.5a.5.5 0 0 1 1 0v2.5a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2v-2.5a.5.5 0 0 1 .5-.5z"/><path d="M7.646 1.146a.5.5 0 0 1 .708 0l3 3a.5.5 0 0 1-.708.708L8.5 2.707V11.5a.5.5 0 0 1-1 0V2.707L5.354 4.854a.5.5 0 1 1-.708-.708l3-3z"/></svg>
        <span class="tool-label">上传</span>
      </button>
      <button class="tool-btn" @click="handleDownload" :disabled="!isConnected || selectedRemoteFiles.length === 0" title="下载">
        <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M.5 9.9a.5.5 0 0 1 .5.5v2.5a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-2.5a.5.5 0 0 1 1 0v2.5a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2v-2.5a.5.5 0 0 1 .5-.5z"/><path d="M7.646 11.854a.5.5 0 0 0 .708 0l3-3a.5.5 0 0 0-.708-.708L8.5 10.293V1.5a.5.5 0 0 0-1 0v8.793L5.354 8.146a.5.5 0 1 0-.708.708l3 3z"/></svg>
        <span class="tool-label">下载</span>
      </button>
      <button class="tool-btn" @click="handleNewFolder" :disabled="!isConnected" title="新建目录">
        <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M.54 3.87.5 3a2 2 0 0 1 2-2h3.672a2 2 0 0 1 1.414.586l.828.828A2 2 0 0 0 9.828 3h3.982a2 2 0 0 1 1.992 2.181l-.637 7A2 2 0 0 1 13.174 14H2.826a2 2 0 0 1-1.991-1.819l-.637-7a1.99 1.99 0 0 1 .342-1.31zM2.19 4a1 1 0 0 0-.996 1.09l.637 7a1 1 0 0 0 .995.91h10.348a1 1 0 0 0 .995-.91l.637-7A1 1 0 0 0 13.81 4H2.19zm4.69-1.707A1 1 0 0 0 6.172 2H2.5a1 1 0 0 0-1 .981l.006.139C1.72 3.042 1.95 3 2.19 3h5.396l-.707-.707z"/><path d="M13.5 10a.5.5 0 0 1 .5.5V12h1.5a.5.5 0 0 1 0 1H14v1.5a.5.5 0 0 1-1 0V13h-1.5a.5.5 0 0 1 0-1H13v-1.5a.5.5 0 0 1 .5-.5z"/></svg>
        <span class="tool-label">新建目录</span>
      </button>
      <button class="tool-btn" @click="handleDeleteSelected" :disabled="!isConnected || selectedRemoteFiles.length === 0" title="删除">
        <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M5.5 5.5A.5.5 0 0 1 6 6v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5zm2.5 0a.5.5 0 0 1 .5.5v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5zm3 .5a.5.5 0 0 0-1 0v6a.5.5 0 0 0 1 0V6z"/><path fill-rule="evenodd" d="M14.5 3a1 1 0 0 1-1 1H13v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4h-.5a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1H5.5l1-1h3l1 1H13a1 1 0 0 1 1 1v1zM4.118 4 4 4.059V13a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4.059L11.882 4H4.118zM2.5 3V2h11v1h-11z"/></svg>
        <span class="tool-label">删除</span>
      </button>
      <button class="tool-btn" @click="handleRename" :disabled="!isConnected || selectedRemoteFiles.length !== 1" title="重命名">
        <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M12.146.146a.5.5 0 0 1 .708 0l3 3a.5.5 0 0 1 0 .708l-10 10a.5.5 0 0 1-.168.11l-5 2a.5.5 0 0 1-.65-.65l2-5a.5.5 0 0 1 .11-.168l10-10zM11.207 2.5 13.5 4.793 14.793 3.5 12.5 1.207 11.207 2.5zm1.586 3L10.5 3.207 4 9.707V10h.5a.5.5 0 0 1 .5.5v.5h.5a.5.5 0 0 1 .5.5v.5h.293l6.5-6.5zm-9.761 5.175-.106.106-1.528 3.821 3.821-1.528.106-.106A.5.5 0 0 1 5 12.5V12h-.5a.5.5 0 0 1-.5-.5V11h-.5a.5.5 0 0 1-.468-.325z"/></svg>
        <span class="tool-label">重命名</span>
      </button>
      <button class="tool-btn" @click="handleRefresh" :disabled="!isConnected" title="刷新">
        <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M11.534 7h3.932a.25.25 0 0 1 .192.41l-1.966 2.36a.25.25 0 0 1-.384 0l-1.966-2.36a.25.25 0 0 1 .192-.41zm-11 2h3.932a.25.25 0 0 0 .192-.41L2.692 6.23a.25.25 0 0 0-.384 0L.342 8.59A.25.25 0 0 0 .534 9z"/><path fill-rule="evenodd" d="M8 3c-1.552 0-2.94.707-3.857 1.818a.5.5 0 1 1-.771-.636A6.002 6.002 0 0 1 13.917 7H12.9A5.002 5.002 0 0 0 8 3zM3.1 9a5.002 5.002 0 0 0 8.757 2.182.5.5 0 1 1 .771.636A6.002 6.002 0 0 1 2.083 9H3.1z"/></svg>
      </button>
      <!-- 书签下拉 -->
      <el-dropdown trigger="click" :disabled="!isConnected">
        <button class="tool-btn" :disabled="!isConnected" title="书签">
          <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M2 2v13.5a.5.5 0 0 0 .74.439L8 13.069l5.26 2.87A.5.5 0 0 0 14 15.5V2a2 2 0 0 0-2-2H4a2 2 0 0 0-2 2z"/></svg>
          <span class="tool-label">书签</span>
          <svg class="tool-icon tool-icon-arrow" viewBox="0 0 16 16" fill="currentColor"><path fill-rule="evenodd" d="M1.646 4.646a.5.5 0 0 1 .708 0L8 10.293l5.646-5.647a.5.5 0 0 1 .708.708l-6 6a.5.5 0 0 1-.708 0l-6-6a.5.5 0 0 1 0-.708z"/></svg>
        </button>
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
            <button class="tool-btn tool-btn-sm" @click="browseLocalPath" title="浏览">
              <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M.54 3.87.5 3a2 2 0 0 1 2-2h3.672a2 2 0 0 1 1.414.586l.828.828A2 2 0 0 0 9.828 3h3.982a2 2 0 0 1 1.992 2.181l-.637 7A2 2 0 0 1 13.174 14H2.826a2 2 0 0 1-1.991-1.819l-.637-7a1.99 1.99 0 0 1 .342-1.31z"/></svg>
            </button>
            <button class="tool-btn tool-btn-sm" @click="goUpLocalDir" title="上一级">
              <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path fill-rule="evenodd" d="M1.646 4.646a.5.5 0 0 1 .708 0L8 10.293l5.646-5.647a.5.5 0 0 1 .708.708l-6 6a.5.5 0 0 1-.708 0l-6-6a.5.5 0 0 1 0-.708z"/></svg>
            </button>
          </div>
        </div>
        <div class="file-list">
          <!-- 表头：名称 / 大小 / 类型 / 修改时间可点击排序 -->
          <div class="file-list-header">
            <span class="col-icon-spacer" aria-hidden="true" />
            <template v-for="col in localColumns" :key="col.key">
              <button
                v-if="col.sortable"
                type="button"
                class="col-btn"
                :class="[col.cls, { 'col-sorted': localSort.key === col.key }]"
                :aria-label="sortLabel(localSort, col)"
                @click="toggleSort('local', col.key)"
              >
                <span class="col-label">{{ col.label }}</span>
                <svg
                  class="sort-arrows"
                  :class="sortArrowClass(localSort, col.key)"
                  viewBox="0 0 10 14"
                  fill="currentColor"
                  aria-hidden="true"
                >
                  <path class="arrow-up" d="M5 0 9 5H1Z" />
                  <path class="arrow-down" d="M5 14 1 9h8Z" />
                </svg>
              </button>
              <span v-else class="col-static" :class="col.cls">{{ col.label }}</span>
            </template>
          </div>
          <div
            v-for="file in sortedLocalFiles"
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
            <span class="file-type" :title="fileTypeOf(file)">{{ fileTypeOf(file) }}</span>
            <span class="file-time">{{ file.modifiedTime }}</span>
          </div>
          <div v-if="localFiles.length === 0" class="empty-tip">
            请选择本地目录
          </div>
        </div>
      </div>

      <!-- 分隔线 -->
      <div class="panel-divider" />

      <!-- 远程文件面板（系统级拖放落点） -->
      <div class="ftp-panel remote-panel" :class="{ 'drop-active': dropActive }" ref="remotePanelEl">
        <div v-if="dropActive" class="drop-overlay">松开即上传到 {{ remotePath }}</div>
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
            <button class="tool-btn tool-btn-sm" @click="goUpDir" :disabled="!isConnected || remotePath === '/'" title="上一级">
              <svg class="tool-icon" viewBox="0 0 16 16" fill="currentColor"><path fill-rule="evenodd" d="M1.646 4.646a.5.5 0 0 1 .708 0L8 10.293l5.646-5.647a.5.5 0 0 1 .708.708l-6 6a.5.5 0 0 1-.708 0l-6-6a.5.5 0 0 1 0-.708z"/></svg>
            </button>
          </div>
        </div>
        <div class="file-list">
          <!-- 表头：名称 / 大小 / 类型 / 修改时间可点击排序，权限列仅展示 -->
          <div class="file-list-header">
            <span class="col-icon-spacer" aria-hidden="true" />
            <template v-for="col in remoteColumns" :key="col.key">
              <button
                v-if="col.sortable"
                type="button"
                class="col-btn"
                :class="[col.cls, { 'col-sorted': remoteSort.key === col.key }]"
                :aria-label="sortLabel(remoteSort, col)"
                @click="toggleSort('remote', col.key)"
              >
                <span class="col-label">{{ col.label }}</span>
                <svg
                  class="sort-arrows"
                  :class="sortArrowClass(remoteSort, col.key)"
                  viewBox="0 0 10 14"
                  fill="currentColor"
                  aria-hidden="true"
                >
                  <path class="arrow-up" d="M5 0 9 5H1Z" />
                  <path class="arrow-down" d="M5 14 1 9h8Z" />
                </svg>
              </button>
              <span v-else class="col-static" :class="col.cls">{{ col.label }}</span>
            </template>
          </div>
          <!-- 上级目录（补齐列占位，保持与数据行对齐；不参与排序） -->
          <div
            v-if="remotePath !== '/' && remotePath !== ''"
            class="file-item file-item-dir"
            @dblclick="goUpDir"
          >
            <span class="file-icon">📂</span>
            <span class="file-name">...</span>
            <span class="file-size"></span>
            <span class="file-type"></span>
            <span class="file-perm"></span>
            <span class="file-time"></span>
          </div>
          <div
            v-for="file in sortedRemoteFiles"
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
            <span class="file-type" :title="fileTypeOf(file)">{{ fileTypeOf(file) }}</span>
            <span class="file-perm" title="Unix 权限（右键可修改）">{{ file.permissions || '-' }}</span>
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

    <!-- 新建远程目录（统一使用 GlobalPromptDialog） -->
    <GlobalPromptDialog
      v-model="showNewFolderDialog"
      title="新建远程目录"
      message="请输入目录名称"
      confirm-button-text="创建"
      :validator="validateDirName"
      @confirm="confirmNewFolder"
    />

    <!-- 重命名（v-if 保证每次打开重建以取最新预填值） -->
    <GlobalPromptDialog
      v-if="showRenameDialog"
      v-model="showRenameDialog"
      title="重命名"
      message="请输入新名称"
      :initial-value="renameInitial"
      :validator="validateDirName"
      @confirm="confirmRenameFile"
    />

    <!-- 删除确认（统一使用 GlobalDialog） -->
    <GlobalDialog v-model="showDeleteConfirm" title="确认删除" width="400px">
      <div class="ftp-delete-message">确定要删除选中的 {{ selectedRemoteFiles.length }} 个文件/目录吗？</div>
      <template #footer>
        <app-button size="small" @click="showDeleteConfirm = false">取消</app-button>
        <app-button size="small" type="danger" plain @click="confirmDeleteSelected">删除</app-button>
      </template>
    </GlobalDialog>

    <!-- 411 = 二次验证（SFTP 会话独立，需要单独输入一次验证码） -->
    <GlobalDialog v-model="showMfaDialog" title="二次验证" width="420px">
      <div class="ftp-delete-message">{{ mfaMessage }}</div>
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

    <!-- 右键上下文菜单 -->
    <teleport to="body">
      <div
        v-if="contextMenuVisible"
        class="ftp-context-menu"
        :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }"
        @click.stop
      >
        <div v-if="contextMenuTarget?.isDirectory" class="context-menu-item" @click="handleSaveBookmark">
          <el-icon><Star /></el-icon> 保存为书签
        </div>
        <div class="context-menu-item" @click="handleChmod">
          <el-icon><Lock /></el-icon> 修改权限
        </div>
        <div class="context-menu-item" @click="handleDownloadSelected">
          <el-icon><Download /></el-icon> 下载所选
        </div>
      </div>
    </teleport>

    <!-- 修改 Unix 权限（八进制，如 755） -->
    <GlobalPromptDialog
      v-if="showChmodDialog"
      v-model="showChmodDialog"
      title="修改权限"
      message="输入三位或四位八进制权限值，例如 755 / 0644"
      :initial-value="chmodInitial"
      :validator="validateChmod"
      @confirm="confirmChmod"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@/api/rpc'
import { desktopDir, dirname, open, onFileDrop } from '@/core/runtime'
import { Star, Close, Lock, Download } from '@element-plus/icons-vue'
// 保留 Element Plus 图标导入（其他位置可能使用）
import { ElInput, ElMessage } from 'element-plus'
import GlobalPromptDialog from '@/components/GlobalPromptDialog.vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { ConnectionStatus } from '../../../types'
import type { ConnectionTab, RemoteConnection, FtpFileItem, TransferItem } from '../../../types'
import { useDocumentManagerStore } from '@/store/modules/documents'
import { useConnectionStore, useFtpStore } from '../../../store'
import NotificationUtil from '@/utils/NotificationUtil'
import { withConnectOverrides } from '../../../utils/target'
import TransferPanel from '../TransferPanel.vue'
import type { Unsubscribe } from '@/core/runtime'

const props = defineProps<{
  tab: ConnectionTab
  connection: RemoteConnection
}>()

const connectionStore = useConnectionStore()
const ftpStore = useFtpStore()

const isConnecting = ref(false)

const panel = computed(() => ftpStore.getPanel(props.tab.id))
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

// ==================== 列表排序 / 文件类型 ====================

type SortKey = 'name' | 'size' | 'type' | 'time'
type SortOrder = 'asc' | 'desc'
interface SortState { key: SortKey | null; order: SortOrder }
interface FileColumn {
  key: SortKey | 'perm'
  label: string
  cls: string
  sortable: boolean
}

/** 两侧面板各自排序；key 为 null 时保持后端顺序（目录在前、名称升序） */
const localSort = ref<SortState>({ key: null, order: 'asc' })
const remoteSort = ref<SortState>({ key: null, order: 'asc' })

const COL_NAME: FileColumn = { key: 'name', label: '名称', cls: 'col-name', sortable: true }
const COL_SIZE: FileColumn = { key: 'size', label: '大小', cls: 'col-size', sortable: true }
const COL_TYPE: FileColumn = { key: 'type', label: '类型', cls: 'col-type', sortable: true }
const COL_PERM: FileColumn = { key: 'perm', label: '权限', cls: 'col-perm', sortable: false }
const COL_TIME: FileColumn = { key: 'time', label: '修改时间', cls: 'col-time', sortable: true }
const localColumns: FileColumn[] = [COL_NAME, COL_SIZE, COL_TYPE, COL_TIME]
const remoteColumns: FileColumn[] = [COL_NAME, COL_SIZE, COL_TYPE, COL_PERM, COL_TIME]

/** 首次点击各列的默认方向：名称/类型升序，大小/时间降序 */
const DEFAULT_ORDER: Record<SortKey, SortOrder> = {
  name: 'asc', size: 'desc', type: 'asc', time: 'desc'
}

function toggleSort(side: 'local' | 'remote', key: SortKey | 'perm') {
  if (key === 'perm') return
  const state = side === 'local' ? localSort : remoteSort
  state.value = state.value.key === key
    ? { key, order: state.value.order === 'asc' ? 'desc' : 'asc' }
    : { key, order: DEFAULT_ORDER[key] }
}

function sortLabel(state: SortState, col: FileColumn): string {
  if (state.key !== col.key) return `按${col.label}排序`
  return `按${col.label}排序，当前${state.order === 'asc' ? '升序' : '降序'}，点击切换方向`
}

function sortArrowClass(state: SortState, key: SortKey | 'perm'): string {
  if (key === 'perm' || state.key !== key) return ''
  return state.order === 'asc' ? 'sort-asc' : 'sort-desc'
}

/** 常见扩展名归到可读类别；未登记的显示「XXX 文件」，无扩展名显示「文件」 */
const FILE_TYPE_LABELS: Record<string, string> = {
  txt: '文本文档', md: 'Markdown 文档', log: '日志文件', pdf: 'PDF 文档',
  doc: 'Word 文档', docx: 'Word 文档', xls: 'Excel 表格', xlsx: 'Excel 表格',
  csv: 'CSV 表格', ppt: 'PPT 演示', pptx: 'PPT 演示',
  json: 'JSON 文件', xml: 'XML 文件', yaml: 'YAML 文件', yml: 'YAML 文件',
  html: 'HTML 文档', css: 'CSS 样式', scss: 'SCSS 样式', sql: 'SQL 脚本',
  js: 'JavaScript 文件', mjs: 'JavaScript 文件', cjs: 'JavaScript 文件',
  ts: 'TypeScript 文件', vue: 'Vue 组件', jsx: 'JavaScript 文件', tsx: 'TypeScript 文件',
  py: 'Python 脚本', java: 'Java 源码', rs: 'Rust 源码', go: 'Go 源码',
  c: 'C 源码', cpp: 'C++ 源码', h: 'C 头文件', sh: 'Shell 脚本', bat: '批处理文件', ps1: 'PowerShell 脚本',
  png: '图像', jpg: '图像', jpeg: '图像', gif: '图像', svg: '图像',
  webp: '图像', bmp: '图像', ico: '图像',
  mp4: '视频', avi: '视频', mov: '视频', mkv: '视频',
  mp3: '音频', wav: '音频', flac: '音频',
  zip: '压缩文件', rar: '压缩文件', '7z': '压缩文件', tar: '压缩文件',
  gz: '压缩文件', bz2: '压缩文件', tgz: '压缩文件',
  exe: '应用程序', msi: '安装包', dll: '动态链接库', so: '共享库', jar: 'JAR 包'
}

function fileTypeOf(file: FtpFileItem): string {
  if (file.isDirectory) return '文件夹'
  const dot = file.name.lastIndexOf('.')
  const ext = dot > 0 ? file.name.slice(dot + 1).toLowerCase() : ''
  if (!ext) return '文件'
  return FILE_TYPE_LABELS[ext] || `${ext.toUpperCase()} 文件`
}

function compareFiles(a: FtpFileItem, b: FtpFileItem, key: SortKey): number {
  switch (key) {
    case 'name':
      return a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' })
    case 'size':
      return (Number(a.size) || 0) - (Number(b.size) || 0)
    case 'type':
      return fileTypeOf(a).localeCompare(fileTypeOf(b), 'zh-CN')
    case 'time':
      // 后端格式固定 "YYYY-MM-DD HH:mm:ss"（本地时区），字典序即时间序；空值视为最旧
      return (a.modifiedTime || '').localeCompare(b.modifiedTime || '')
    default:
      return 0
  }
}

/** 目录恒在前（与后端默认顺序一致），同组内按所选列排序，同值按名称兜底 */
function sortFiles(files: FtpFileItem[], state: SortState): FtpFileItem[] {
  const list = [...files]
  if (!state.key) return list
  const key = state.key
  const dir = state.order === 'asc' ? 1 : -1
  list.sort((a, b) => {
    if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1
    const cmp = compareFiles(a, b, key)
    return cmp !== 0
      ? cmp * dir
      : a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' })
  })
  return list
}

const sortedLocalFiles = computed(() => sortFiles(localFiles.value, localSort.value))
const sortedRemoteFiles = computed(() => sortFiles(remoteFiles.value, remoteSort.value))

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
  ftpStore.initPanel(props.tab.id)
  loadBookmarks()
  window.addEventListener('click', handleGlobalClick)
  void setupFileDrop()
  // 本地目录默认读取桌面目录
  try {
    const desktop = await desktopDir()
    localPath.value = desktop
    await ftpStore.listLocalFiles(props.tab.id, desktop)
  } catch {
    // fallback: 不加载
  }
})

onUnmounted(() => {
  stopConnectionCheck()
  window.removeEventListener('click', handleGlobalClick)
  unlistenDrop?.()
  unlistenDrop = null
  if (sftpSessionId) {
    invoke('sftp_disconnect', { sessionId: sftpSessionId }).catch(() => {})
  }
  ftpStore.destroyPanel(props.tab.id)
})

const showMfaDialog = ref(false)
const mfaMessage = ref('')
const mfaCode = ref('')

/** optionsOverride 为字符串时表示带一次性验证码重连；来自 @click 的事件对象一律忽略 */
async function handleConnect(optionsOverride?: string | Event) {
  if (isConnected.value) return
  const options = typeof optionsOverride === 'string'
    ? optionsOverride
    : (props.connection.advanced_options || '')

  isConnecting.value = true
  connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.CONNECTING)

  try {
    const res = await invoke<any>('sftp_connect', {
      tabId: props.tab.id,
      host: props.connection.address,
      port: props.connection.port,
      username: props.connection.username,
      password: props.connection.password,
      authType: props.connection.auth_type || 'password',
      privateKeyPath: props.connection.private_key_path || '',
      passphrase: props.connection.passphrase || '',
      options
    })

    if (res.code === 200) {
      sftpSessionId = res.data
      connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.CONNECTED)
      // 列出根目录
      if (sftpSessionId) {
        await ftpStore.listRemoteFiles(props.tab.id, sftpSessionId, '/')
      }
      // 启动连接状态定时检测
      startConnectionCheck()
    } else {
      connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.ERROR)
      // 409 = 未知主机密钥：SFTP 侧不重复做信任交互，引导用户先在终端连一次
      const msg = res.code === 409
        ? '该主机密钥尚未保存，请先打开此连接的终端并完成指纹确认'
        : (res.msg || 'SFTP 连接失败')
      if (res.code === 411) {
        mfaMessage.value = String(res.msg || '')
        mfaCode.value = ''
        showMfaDialog.value = true
      } else {
        ElMessage.error(msg)
      }
    }
  } catch (error) {
    connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.ERROR)
    console.error('SFTP connect error:', error)
    ElMessage.error('SFTP 连接异常')
  } finally {
    isConnecting.value = false
  }
}

function confirmMfa() {
  const code = mfaCode.value.trim()
  if (!code) return
  showMfaDialog.value = false
  void handleConnect(withConnectOverrides(props.connection.advanced_options, { mfaCode: code }))
}

/** 拼接远端路径（base 可能是 '/'） */
function joinRemote(base: string, name: string): string {
  return `${base.replace(/\/+$/, '')}/${name}`
}

/** 本地路径沿用原串的分隔符，避免 Windows 上混用 / 与 \ 触发路径解析差异 */
function joinLocal(base: string, name: string): string {
  const sep = base.includes('\\') ? '\\' : '/'
  return `${base.replace(/[\\/]+$/, '')}${sep}${name}`
}

/** 把选中的路径还原成文件项；拖放进来的路径不在列表里，用 local_stat 补齐 */
async function resolveLocalItems(paths: string[]): Promise<FtpFileItem[]> {
  const out: FtpFileItem[] = []
  for (const p of paths) {
    const known = localFiles.value.find(f => f.path === p)
    if (known) { out.push(known); continue }
    const res = await invoke<any>('local_stat', { path: p })
    const stat = res.code === 200 ? res.data : null
    out.push({
      name: p.split(/[/\\]/).pop() || p,
      path: p,
      isDirectory: stat?.isDirectory ?? false,
      size: stat?.size ?? 0,
      modifiedTime: '',
      permissions: ''
    })
  }
  return out
}

/**
 * 本地目录递归展开：每个子目录先在远端 mkdir（已存在会报错，忽略），
 * 再把文件摊平成完整目标路径的 TransferItem。目录深度用显式队列，避免递归爆栈。
 */
async function expandLocal(items: FtpFileItem[], remoteBase: string): Promise<TransferItem[]> {
  const out: TransferItem[] = []
  const queue = items.map(item => ({ item, remoteDir: remoteBase }))
  while (queue.length > 0 && sftpSessionId) {
    const { item, remoteDir } = queue.shift()!
    if (item.isDirectory) {
      const targetDir = joinRemote(remoteDir, item.name)
      await invoke('sftp_mkdir', { sessionId: sftpSessionId, path: targetDir }).catch(() => {})
      const res = await invoke<any>('local_list_dir', { path: item.path })
      if (res.code === 200) {
        for (const child of res.data as FtpFileItem[]) queue.push({ item: child, remoteDir: targetDir })
      }
    } else {
      out.push({
        fileName: item.name,
        sourcePath: item.path,
        targetPath: joinRemote(remoteDir, item.name),
        size: item.size
      })
    }
  }
  return out
}

/** 远端目录递归展开：本地先 ensure_dir，再逐层列目录 */
async function expandRemote(items: FtpFileItem[], localBase: string): Promise<TransferItem[]> {
  const out: TransferItem[] = []
  const queue = items.map(item => ({ item, localDir: localBase }))
  while (queue.length > 0 && sftpSessionId) {
    const { item, localDir } = queue.shift()!
    if (item.isDirectory) {
      const targetDir = joinLocal(localDir, item.name)
      await invoke('ensure_dir', { path: targetDir }).catch(() => {})
      const res = await invoke<any>('sftp_list_dir', { sessionId: sftpSessionId, path: item.path })
      if (res.code === 200) {
        for (const child of res.data as FtpFileItem[]) queue.push({ item: child, localDir: targetDir })
      }
    } else {
      out.push({
        fileName: item.name,
        sourcePath: item.path,
        targetPath: joinLocal(localDir, item.name),
        size: item.size
      })
    }
  }
  return out
}

async function uploadPaths(paths: string[]) {
  if (!sftpSessionId || paths.length === 0) return
  const items = await expandLocal(await resolveLocalItems(paths), remotePath.value)
  if (items.length === 0) { NotificationUtil.warn('所选目录中没有可上传的文件'); return }
  ftpStore.enqueueUploads(props.tab.id, sftpSessionId, items)
}

async function handleUpload() {
  await uploadPaths(selectedLocalFiles.value)
}

async function handleDownload() {
  if (!sftpSessionId || selectedRemoteFiles.value.length === 0) return
  const dir = await open({ directory: true, multiple: false })
  if (!dir) return
  const selected = selectedRemoteFiles.value
    .map(p => remoteFiles.value.find(f => f.path === p))
    .filter((f): f is FtpFileItem => !!f)
  const items = await expandRemote(selected, String(dir))
  if (items.length === 0) { NotificationUtil.warn('所选目录中没有可下载的文件'); return }
  ftpStore.enqueueDownloads(props.tab.id, sftpSessionId, items)
}

/** 右键「下载所选」：与工具栏一致，但复用当前选区 */
function handleDownloadSelected() {
  closeAllContextMenus()
  void handleDownload()
}

// ==================== 权限（chmod） ====================

const showChmodDialog = ref(false)
const chmodInitial = ref('')
const chmodPath = ref('')

function validateChmod(value: string): string | true {
  const v = value.trim()
  if (!/^[0-7]{3,4}$/.test(v)) return '请输入 3~4 位八进制数字（0-7）'
  return true
}

function handleChmod() {
  const file = contextMenuTarget.value
  if (!file) return
  chmodPath.value = file.path
  chmodInitial.value = (file.permissions || '').slice(-3) || '644'
  closeAllContextMenus()
  showChmodDialog.value = true
}

async function confirmChmod(value: string) {
  if (!sftpSessionId) return
  const mode = parseInt(value.trim(), 8)
  await ftpStore.chmodRemote(props.tab.id, sftpSessionId, chmodPath.value, mode)
}

// ==================== 系统级拖放上传 ====================

const remotePanelEl = ref<HTMLElement>()
const dropActive = ref(false)
let unlistenDrop: Unsubscribe | null = null

function isInsideRemotePanel(position?: { x: number; y: number }): boolean {
  if (!position || !remotePanelEl.value) return false
  const rect = remotePanelEl.value.getBoundingClientRect()
  const dpr = window.devicePixelRatio || 1
  const x = position.x / dpr
  const y = position.y / dpr
  return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
}

async function setupFileDrop() {
  unlistenDrop = await onFileDrop((payload) => {
    // 所有 FTP tab 都常驻（v-show），必须只让激活且已连接的那个响应，
    // 否则一次拖放会被 N 个面板重复入队。
    const isFocusedTab = connectionStore.activeTabId === props.tab.id
      || (connectionStore.splitEnabled && connectionStore.secondaryTabId === props.tab.id)
    if (!isConnected.value || !isFocusedTab) return
    if (payload.type === 'leave') { dropActive.value = false; return }
    const overPanel = isInsideRemotePanel(payload.position)
    dropActive.value = payload.type !== 'drop' && overPanel
    if (payload.type === 'drop' && overPanel && payload.paths.length > 0) {
      void uploadPaths(payload.paths)
    }
  })
}

// 弹窗状态（新建目录/重命名/删除确认，统一使用 Global 弹窗组件）
const showNewFolderDialog = ref(false)
const showRenameDialog = ref(false)
const renameInitial = ref('')
const renameOldPath = ref('')
const showDeleteConfirm = ref(false)

// 名称校验：非空即可，错误文案由弹窗展示
function validateDirName(value: string): string | true {
  return value.trim() ? true : '名称不能为空'
}

async function handleNewFolder() {
  if (!sftpSessionId) return
  showNewFolderDialog.value = true
}

async function confirmNewFolder(value: string) {
  if (!sftpSessionId) return
  await ftpStore.createRemoteDir(props.tab.id, sftpSessionId, remotePath.value, value.trim())
}

async function handleDeleteSelected() {
  if (!sftpSessionId || selectedRemoteFiles.value.length === 0) return
  showDeleteConfirm.value = true
}

async function confirmDeleteSelected() {
  showDeleteConfirm.value = false
  if (!sftpSessionId) return
  for (const path of selectedRemoteFiles.value) {
    await ftpStore.deleteRemoteFile(props.tab.id, sftpSessionId, path)
  }
}

async function handleRefresh() {
  if (!sftpSessionId) return
  await ftpStore.listRemoteFiles(props.tab.id, sftpSessionId, remotePath.value)
  if (localPath.value) {
    await ftpStore.listLocalFiles(props.tab.id, localPath.value)
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
    ftpStore.listLocalFiles(props.tab.id, file.path)
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
    ftpStore.listRemoteFiles(props.tab.id, sftpSessionId, file.path)
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
  renameOldPath.value = oldPath
  renameInitial.value = oldName
  showRenameDialog.value = true
}

async function confirmRenameFile(value: string) {
  if (!sftpSessionId) return
  const oldPath = renameOldPath.value
  const oldName = oldPath.split('/').pop() || ''
  if (value && value !== oldName) {
    const parentPath = oldPath.substring(0, oldPath.lastIndexOf('/'))
    const newPath = `${parentPath}/${value}`
    await ftpStore.renameRemoteFile(props.tab.id, sftpSessionId, oldPath, newPath)
    selectedRemoteFiles.value = []
  }
}
//上一个文件夹（远程）
function goUpDir() {
  if (!sftpSessionId) return
  const currentPath = remotePath.value
  console.log("goUpDir currentPath",currentPath)
  if (!currentPath || currentPath === '/' || currentPath === '') return
  const parts = currentPath.replace(/\\/g, '/').split('/').filter(Boolean)
  parts.pop()
  console.log("goUpDir parts",parts)
  const parentPath = parts.length > 0 ? '/' + parts.join('/') : '/'
  remotePath.value = parentPath
  console.log("goUpDir",props.tab.id, sftpSessionId, parentPath)
  ftpStore.listRemoteFiles(props.tab.id, sftpSessionId, parentPath)
}

async function goUpLocalDir() {
  const currentPath = localPath.value
  if (!currentPath) return
  try {
    const parent = await dirname(currentPath)
    if (parent && parent !== currentPath) {
      localPath.value = parent
      await ftpStore.listLocalFiles(props.tab.id, parent)
    }
  } catch {
    // fallback: 手动截取
    const idx = Math.max(currentPath.lastIndexOf('/'), currentPath.lastIndexOf('\\'))
    if (idx > 0) {
      const parent = currentPath.substring(0, idx)
      localPath.value = parent
      await ftpStore.listLocalFiles(props.tab.id, parent)
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
  ftpStore.listRemoteFiles(props.tab.id, sftpSessionId, remotePath.value)
}

function navigateToBookmark(path: string) {
  if (!sftpSessionId) return
  remotePath.value = path
  ftpStore.listRemoteFiles(props.tab.id, sftpSessionId, path)
}

// 右键菜单
function handleRemoteFileContext(file: FtpFileItem, event: MouseEvent) {
  event.preventDefault()
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

function closeAllContextMenus() {
  contextMenuVisible.value = false
}

// 定时检测连接状态
function startConnectionCheck() {
  stopConnectionCheck()
  connectionCheckTimer = setInterval(async () => {
    if (!sftpSessionId || !isConnected.value) return
    const alive = await ftpStore.checkSftpConnection(sftpSessionId)
    if (!alive) {
      connectionStore.updateTabStatus(props.tab.id, ConnectionStatus.DISCONNECTED)
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
    ftpStore.listLocalFiles(props.tab.id, localPath.value)
  }
}

function navigateRemoteDir() {
  if (sftpSessionId && remotePath.value) {
    ftpStore.listRemoteFiles(props.tab.id, sftpSessionId, remotePath.value)
  }
}

async function browseLocalPath() {
  const dir = await open({ directory: true, multiple: false })
  if (dir) {
    localPath.value = dir
    await ftpStore.listLocalFiles(props.tab.id, dir)
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

  @include theme.useTheme {
    background-color: theme.getVar('appMainBgColor');
  }
}

.ftp-toolbar {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 4px 8px;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.tool-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 4px 8px;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: transparent;
  cursor: pointer;
  font-size: theme.getVar('fontSizeSmall');
  transition: background-color 0.15s;
  user-select: none;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
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

  &.tool-btn-active {
    @include theme.useTheme {
      color: #4ec9b0;
    }
  }

  &.tool-btn-sm {
    padding: 3px 6px;
  }
}

.tool-icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.tool-icon-arrow {
  width: 10px;
  height: 10px;
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
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.panel-title {
  font-size: theme.getVar('fontSizeSmall');
  font-weight: 500;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: 2px;
  font-size: theme.getVar('fontSizeSmall');
  flex: 1;
  min-width: 0;
  overflow: hidden;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.breadcrumb-item {
  cursor: pointer;
  white-space: nowrap;
  color: #6699cc; // 面包屑链接语义色（保留）：SSH 风格

  &:hover {
    color: #88bbff;
    text-decoration: underline;
  }
}

.breadcrumb-sep {
  margin: 0 2px;
  cursor: default;

  @include theme.useTheme {
    color: theme.getVar('textDisabled');
  }

  &:hover {
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
    border-radius: var(--radius-sm);
  }
}

/* 表头固定在列表顶部，列宽与 .file-item 的列一一对应 */
.file-list-header {
  position: sticky;
  top: 0;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 12px;
  flex-shrink: 0;
  font-size: theme.getVar('fontSizeSm');

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    border-bottom: 1px solid theme.getVar('borderColor');
    color: theme.getVar('textSecondary');
  }
}

.col-icon-spacer {
  flex-shrink: 0;
  width: 18px;
}

.col-name, .file-name { flex: 1; min-width: 0; }
.col-size, .file-size { flex-shrink: 0; width: 70px; }
.col-type, .file-type { flex-shrink: 0; width: 84px; }
.col-perm, .file-perm { flex-shrink: 0; width: 54px; }
.col-time, .file-time { flex-shrink: 0; width: 130px; }

.col-btn,
.col-static {
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  padding: 0;
  border: none;
  background: transparent;
  font-family: inherit;
  font-size: inherit;
  white-space: nowrap;
  text-align: right;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &.col-name,
  &.col-type {
    justify-content: flex-start;
    text-align: left;
  }
}

.col-btn {
  cursor: pointer;

  &:hover {
    @include theme.useTheme {
      color: theme.getVar('textColor');
    }
  }

  &.col-sorted {
    font-weight: 500;

    @include theme.useTheme {
      color: theme.getVar('textColor');
    }
  }
}

.col-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sort-arrows {
  width: 8px;
  height: 12px;
  flex-shrink: 0;
  opacity: 0;
  transition: opacity 0.15s;

  .arrow-up,
  .arrow-down { opacity: 0.35; }

  &.sort-asc .arrow-up,
  &.sort-desc .arrow-down { opacity: 1; }
}

.col-btn:hover .sort-arrows,
.col-btn.col-sorted .sort-arrows {
  opacity: 1;
}

.file-type {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: theme.getVar('fontSizeSm');

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 12px;
  font-size: theme.getVar('fontSize');
  cursor: pointer;
  transition: background-color 0.1s;
  user-select: none;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('activityBarHover');
    }
  }
}

.file-item-selected {
  @include theme.useTheme {
    background-color: theme.getVar('hoverColor') !important;
  }
}

.file-icon {
  flex-shrink: 0;
  width: 18px;
  text-align: center;
  font-size: theme.getVar('fontSize');
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
  font-size: theme.getVar('fontSizeSm');

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.file-perm {
  flex-shrink: 0;
  width: 54px;
  text-align: right;
  font-family: var(--font-family-mono);
  font-size: theme.getVar('fontSizeSm');

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.remote-panel {
  position: relative;
}

.drop-active {
  outline: 2px dashed var(--el-color-primary);
  outline-offset: -4px;
}

.drop-overlay {
  position: absolute;
  inset: 0;
  z-index: 5;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  font-size: theme.getVar('fontSize');
  font-weight: 500;
  background-color: rgba(64, 128, 255, 0.12);
  color: var(--el-color-primary);
}

.file-time {
  flex-shrink: 0;
  width: 130px;
  text-align: right;
  font-size: theme.getVar('fontSizeSm');

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.empty-tip {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100px;
  font-size: theme.getVar('fontSize');

  @include theme.useTheme {
    color: theme.getVar('textDisabled');
  }
}

.ftp-context-menu {
  position: fixed;
  z-index: 9999;
  background-color: #2d2d2d;
  border: 1px solid #454545;
  border-radius: var(--radius-lg);
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
  font-size: theme.getVar('fontSize');
  cursor: pointer;
  transition: background-color 0.15s;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('hoverColor');
      color: theme.getVar('activityBarActive');
    }
  }
}
</style>
