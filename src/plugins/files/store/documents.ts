// stores/documentManager.ts
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@/api/rpc'
import { tauri_request } from '@/utils/tauri_request'
import { ResponseData } from '@/entity/entitys'
import { md5 } from '@/utils/Md5Utils'
import { pdf_store_conten_key_prefix, pdf_store_tem_key_prefix } from '@/setting'
import NotificationUtil from '@/utils/NotificationUtil'
import { load } from '@/core/runtime'
import type { OpenFileInfo } from '@/Types/workspace'
// 文件类型枚举
export enum FileType {
  CODE = 'code',
  TEXT = 'text',
  IMAGE = 'image',
  BINARY = 'binary',
  MARKDOWN = 'markdown',
  PDF = 'pdf'
}

// 视图状态接口（不同类型文件保存不同的状态）
export interface ViewState {
  // 代码/文本编辑器状态
  cursor?: { line: number; column: number }
  scrollTop?: number
  scrollLeft?: number
  selections?: Array<{ start: number; end: number }>

  // 图片编辑器状态
  zoom?: number
  panX?: number
  panY?: number
  rotation?: number

  // 二进制/Hex编辑器状态
  offset?: number
  bytesPerLine?: number
  selectedBytes?: [number, number]

  // 通用状态
  activeTab?: string
  splitView?: boolean
}

// 文档状态接口
export interface DocumentState {
  key:string
  id: string                    // 文件绝对路径作为唯一ID
  name: string                  // 文件名
  type: FileType               // 文件类型
  content: string | ArrayBuffer | null  // 文件内容
  originalContent: string | ArrayBuffer | null  // 原始内容（用于判断是否修改）
  isDirty: boolean             // 是否有未保存的修改
  language: string             // 编程语言（仅代码文件）
  encoding: string             // 文件编码
  viewState: ViewState         // 视图状态
  lastModified: number         // 最后修改时间戳
  isReadOnly: boolean          // 是否只读
  isLoading: boolean           // 是否正在加载
  // 远程文件元信息
  isRemote?: boolean           // 是否为远程文件
  remotePath?: string          // 远程路径
  sessionId?: string           // SFTP 会话 ID
  localTempPath?: string       // 本地临时文件路径
}

export const useDocumentManagerStore = defineStore('documentManager', () => {
  // 所有打开的文档，key 为文件路径
  const documents = ref<Map<string, DocumentState>>(new Map())

  // 当前激活的文档路径
  const activeDocumentPath = ref<string | null>(null)
  // 当前激活的文档key
  const activeDocumentKey = ref<string | null>(null)
  // 文档打开顺序（用于标签页排序）
  const documentOrder = ref<string[]>([])

  // 最近关闭的文档（用于撤销关闭）
  const recentlyClosed = ref<Array<{ document: DocumentState; index: number }>>([])

  // 收藏文件列表（持久化到 localStorage）
  const favoriteFiles = ref<string[]>([])
  // 最近打开文件列表（持久化到 localStorage）
  const recentFiles = ref<string[]>([])

  // 最大缓存文档数（超出则销毁最旧的）
  const MAX_CACHED_DOCUMENTS = 20
  const MAX_RECENT_FILES = 30

  // 计算属性：获取当前激活的文档
  const activeDocument = computed(() => {
    if (!activeDocumentPath.value) return null
    return documents.value.get(activeDocumentPath.value) || null
  })

  // 计算属性：获取所有文档列表（按打开顺序）
  const documentList = computed(() => {
    return documentOrder.value
      .map(id => documents.value.get(id))
      .filter(Boolean) as DocumentState[]
  })

  // 计算属性：是否有未保存的文档
  const hasUnsavedChanges = computed(() => {
    return Array.from(documents.value.values()).some(doc => doc.isDirty)
  })

  // ===== 收藏管理 =====
  function loadFavorites() {
    try {
      const saved = localStorage.getItem('file-favorites')
      if (saved) favoriteFiles.value = JSON.parse(saved)
    } catch { /* ignore */ }
  }
  function saveFavorites() {
    try {
      localStorage.setItem('file-favorites', JSON.stringify(favoriteFiles.value))
    } catch { /* ignore */ }
  }
  function toggleFavorite(filePath: string) {
    const idx = favoriteFiles.value.indexOf(filePath)
    if (idx >= 0) {
      favoriteFiles.value.splice(idx, 1)
    } else {
      favoriteFiles.value.push(filePath)
    }
    saveFavorites()
  }
  function isFavorite(filePath: string) {
    return favoriteFiles.value.includes(filePath)
  }

  // ===== 最近打开管理 =====
  function loadRecentFiles() {
    try {
      const saved = localStorage.getItem('file-recent')
      if (saved) recentFiles.value = JSON.parse(saved)
    } catch { /* ignore */ }
  }
  function saveRecentFiles() {
    try {
      localStorage.setItem('file-recent', JSON.stringify(recentFiles.value))
    } catch { /* ignore */ }
  }
  function addRecentFile(filePath: string) {
    // 移除重复项，添加到最前
    recentFiles.value = recentFiles.value.filter(p => p !== filePath)
    recentFiles.value.unshift(filePath)
    // 限制数量
    if (recentFiles.value.length > MAX_RECENT_FILES) {
      recentFiles.value = recentFiles.value.slice(0, MAX_RECENT_FILES)
    }
    saveRecentFiles()
  }
  function removeRecentFile(filePath: string) {
    recentFiles.value = recentFiles.value.filter(p => p !== filePath)
    saveRecentFiles()
  }
  function clearRecentFiles() {
    recentFiles.value = []
    saveRecentFiles()
  }

  /**
   * 打开或创建文档
   */
  async function openDocument(filePath: string, fileType?: FileType) {
    addAppOpenFile(filePath);
    addRecentFile(filePath)
    // 如果文档已打开，直接激活
    if (documents.value.has(filePath)) {
      setActiveDocument(filePath)
      return
    }

    // 创建文档状态对象
    const fileName = filePath.split(/[/\\]/).pop() || 'Untitled'
    const type = fileType || detectFileType(filePath)
    const hash = md5(filePath)
    const doc: DocumentState = {
      key: hash,
      id: filePath,
      name: fileName,
      type,
      content: null,
      originalContent: null,
      isDirty: false,
      language: detectLanguage(filePath),
      encoding: 'utf-8',
      viewState: {},
      lastModified: Date.now(),
      isReadOnly: false,
      isLoading: true
    }
    // 添加到文档集合
    documents.value.set(filePath, doc)
    documentOrder.value.push(filePath)
    // 从磁盘加载文件内容
    try {
      if(type === FileType.MARKDOWN){
        const content = await loadFileContent(filePath, type)
        doc.content = content
        doc.originalContent = content
        doc.isLoading = false
        doc.lastModified = Date.now()
      }
    } catch (error) {
      console.error(`Failed to load file: ${filePath}`, error)
      doc.isLoading = false
      // 可以设置错误状态
    }
    // 激活该文档
    setActiveDocument(filePath)
    // 检查是否需要清理缓存
    cleanupDocumentCache()
  }

  /**
   * 打开远程文档（通过 SFTP 下载到临时目录后打开）
   */
  async function openRemoteDocument(options: {
    key: string
    localTempPath: string
    remotePath: string
    sessionId: string
    fileName: string
  }) {
    const { key, localTempPath, remotePath, sessionId, fileName } = options

    // 如果已打开，直接激活
    if (documents.value.has(key)) {
      setActiveDocument(key)
      return
    }

    const type = detectFileType(fileName)
    const doc: DocumentState = {
      key,
      id: key,
      name: `[远程] ${fileName}`,
      type,
      content: null,
      originalContent: null,
      isDirty: false,
      language: detectLanguage(fileName),
      encoding: 'utf-8',
      viewState: {},
      lastModified: Date.now(),
      isReadOnly: false,
      isLoading: true,
      isRemote: true,
      remotePath,
      sessionId,
      localTempPath
    }

    documents.value.set(key, doc)
    documentOrder.value.push(key)

    // 从本地临时文件读取内容
    try {
      const content = await loadFileContent(localTempPath, type)
      doc.content = content
      doc.originalContent = content
      doc.isLoading = false
    } catch (error) {
      console.error(`Failed to load remote file: ${remotePath}`, error)
      doc.isLoading = false
    }

    setActiveDocument(key)
    cleanupDocumentCache()
  }

  /**
   * 保存远程文档（上传回远程服务器）
   */
  async function saveRemoteDocument(filePath: string): Promise<boolean> {
    const doc = documents.value.get(filePath)
    if (!doc || !doc.isRemote || !doc.localTempPath || !doc.remotePath || !doc.sessionId) return false
    if (doc.content === doc.originalContent) return true

    try {
      // 先写入本地临时文件
      await invoke('save_note', {
        path: doc.localTempPath,
        mdText: doc.content
      })
      // 上传到远程服务器
      const taskId = `autosave-${Date.now()}`
      const res = await invoke<any>('sftp_upload', {
        sessionId: doc.sessionId,
        localPath: doc.localTempPath,
        remotePath: doc.remotePath,
        taskId
      })
      if (res.code === 200) {
        doc.originalContent = doc.content
        doc.isDirty = false
        doc.lastModified = Date.now()
        return true
      } else {
        NotificationUtil.error('远程保存失败: ' + (res.msg || '未知错误'))
        return false
      }
    } catch (error) {
      console.error(`Failed to save remote file: ${doc.remotePath}`, error)
      NotificationUtil.error('远程保存失败')
      return false
    }
  }

  /**
   * 设置激活文档
   */
  function setActiveDocument(filePath: string | null) {
    // 保存当前文档的视图状态
    if (activeDocumentPath.value) {
      // 这里可以触发保存视图状态的逻辑
    }
    // NotificationUtil.success(`已打开文件：${filePath}`)
    activeDocumentPath.value = filePath

    // 更新文档顺序（将激活的文档移到最前面）
    // if (filePath) {
    //   const index = documentOrder.value.indexOf(filePath)
    //   if (index > -1) {
    //     documentOrder.value.splice(index, 1)
    //     documentOrder.value.unshift(filePath)
    //   }
    // }
  }
  /**
   * 设置激活文档Key
   */
  function setActiveDocumentKey(key: string | null) {
    // 保存当前文档的视图状态
    if (activeDocumentKey.value) {
      // 这里可以触发保存视图状态的逻辑
    }
    // NotificationUtil.success(`已打开文件：${filePath}`)
    activeDocumentKey.value = key


  }


  /**
   * 更新文档内容
   */
  function updateDocumentContent(filePath: string, content: string | ArrayBuffer) {
    const doc = documents.value.get(filePath)
    if (!doc) return

    doc.content = content
    doc.isDirty = content !== doc.originalContent
  }

  /**
   * 更新文档视图状态
   */
  function updateViewState(filePath: string, viewState: Partial<ViewState>) {
    const doc = documents.value.get(filePath)
    if (!doc) return

    doc.viewState = {
      ...doc.viewState,
      ...viewState
    }
  }

  /**
   * 保存文档到磁盘
   */
  async function saveDocument(filePath: string): Promise<boolean> {
    const doc = documents.value.get(filePath)
    if (!doc) return true

    // 远程文档走远程保存逻辑
    if (doc.isRemote) {
      return await saveRemoteDocument(filePath)
    }

    if (!doc.content||doc.content==doc.originalContent) return true;

    try {
      // await invoke('save_file', {
      //   path: filePath,
      //   content: doc.content,
      //   encoding: doc.encoding
      // })
      //   doc.originalContent = doc.content
      //   doc.isDirty = false
      //   doc.lastModified = Date.now()
      const res = await tauri_request<ResponseData<string[]>>("save_note", {
        path: filePath,
        mdText: doc.content,
      });
      if (res && res.code === 200) {
        doc.originalContent = doc.content
        doc.isDirty = false
        doc.lastModified = Date.now()
      } else {
        NotificationUtil.error("笔记保存失败：" + (res?.msg || "未知错误"));
        return false;
      }

    } catch (error) {
      console.error(`Failed to save file: ${filePath}`, error)
      throw error
    }
    return true;
  }

  /**
   * 保存所有未保存的文档
   */
  async function saveAllDocuments() {
    const unsavedDocs = Array.from(documents.value.values())
      .filter(doc => doc.isDirty)

    const savePromises = unsavedDocs.map(doc => saveDocument(doc.id))
    await Promise.allSettled(savePromises)
  }

  /**
   * 关闭文档
   */
  function closeDocument(filePath: string) {
    //删除打开方式 保存的文件
    deleteAppOpenFile(filePath)
    //删除打开记录
    const doc = documents.value.get(filePath)
    if (!doc) return

    //保存文档
     saveDocument(filePath).then(r =>  {
      if (!r){
        NotificationUtil.error("笔记保存失败：" + (r|| "未知错误"));
      }
     });

    // 保存到最近关闭列表
    const index = documentOrder.value.indexOf(filePath)
    recentlyClosed.value.push({ document: { ...doc }, index })

    // 限制最近关闭列表大小
    if (recentlyClosed.value.length > 10) {
      recentlyClosed.value.shift()
    }

    // 从集合中移除
    documents.value.delete(filePath)
    documentOrder.value = documentOrder.value.filter(id => id !== filePath)

    // 如果关闭的是当前激活文档，切换到其他文档
    if (activeDocumentPath.value === filePath) {
      const nextDoc = documentOrder.value[0] || null
      setActiveDocument(nextDoc)
    }
  }
//  重排序方法
  function reorderDocuments(newOrder: string[]) {
    documentOrder.value = newOrder
  }

  async function deleteAppOpenFile(filePath: string) {
    var key = md5(filePath)
    console.log("deleteAppOpenFile "+key)
    // 应用启动时获取全局用户信息
    let store = await load('openfile.json', {
      autoSave: false,
      defaults: {}
    })
    if (store) {
      store.delete(key)
    }
  }

  async function addAppOpenFile(filePath: string) {
    var key = md5(filePath)
    console.log("addAppOpenFile "+key)
    // 应用启动时获取全局用户信息
    let store = await load('openfile.json', {
      autoSave: false,
      defaults: {}
    })
    if (store) {
      store.set(key, filePath)
    }
  }
  /**
   * 撤销关闭文档
   */
  function undoClose() {
    const lastClosed = recentlyClosed.value.pop()
    if (!lastClosed) return

    const { document: doc, index } = lastClosed

    // 重新添加文档
    documents.value.set(doc.id, doc)
    documentOrder.value.splice(Math.min(index, documentOrder.value.length), 0, doc.id)
    // 激活该文档
    setActiveDocument(doc.id)
    //setActiveDocumentKey(doc.key)
  }

  /**
   * 关闭所有文档
   */
  function closeAllDocuments() {
    const allIds = Array.from(documents.value.keys())
    allIds.forEach(id => closeDocument(id))
  }

  /**
   * 关闭其他文档（保留指定文档）
   */
  function closeOtherDocuments(filePath: string) {
    const otherIds = Array.from(documents.value.keys())
      .filter(id => id !== filePath)

    otherIds.forEach(id => closeDocument(id))
  }

  /**
   * 重新加载文档（从磁盘重新读取）
   */
  async function reloadDocument(filePath: string) {
    const doc = documents.value.get(filePath)
    if (!doc) return
    try {
      doc.isLoading = true
      const content = await loadFileContent(filePath, doc.type)
      doc.content = content
      doc.originalContent = content
      doc.isDirty = false
      doc.lastModified = Date.now()
      doc.isLoading = false
    } catch (error) {
      console.error(`Failed to reload file: ${filePath}`, error)
      doc.isLoading = false
    }
  }

  /**
   * 创建新文档
   */
  function createNewDocument(type: FileType = FileType.TEXT) {
    const id = `untitled:${Date.now()}`
    const hash = md5(id)
    const doc: DocumentState = {
      key: hash,
      id,
      name: 'Untitled',
      type,
      content: '',
      originalContent: '',
      isDirty: false,
      language: 'plaintext',
      encoding: 'utf-8',
      viewState: {},
      lastModified: Date.now(),
      isReadOnly: false,
      isLoading: false
    }

    documents.value.set(id, doc)
    documentOrder.value.unshift(id)
    setActiveDocument(id)
    //setActiveDocumentKey(hash)
    return id
  }

  /**
   * 重命名文档
   */
  function renameDocument(oldPath: string, newPath: string) {
    const doc = documents.value.get(oldPath)
    if (!doc) return

    // 创建新文档状态
    const newDoc: DocumentState = {
      ...doc,
      id: newPath,
      name: newPath.split(/[/\\]/).pop() || 'Untitled',
      language: detectLanguage(newPath)
    }

    // 更新集合
    documents.value.delete(oldPath)
    documents.value.set(newPath, newDoc)

    // 更新顺序
    const index = documentOrder.value.indexOf(oldPath)
    if (index > -1) {
      documentOrder.value[index] = newPath
    }

    // 更新激活状态
    if (activeDocumentPath.value === oldPath) {
      activeDocumentPath.value = newPath
    }
  }

  /**
   * 清理文档缓存（LRU策略）
   */
  function cleanupDocumentCache() {
    if (documents.value.size <= MAX_CACHED_DOCUMENTS) return

    // 获取最旧的文档（最后一个）
    const oldestId = documentOrder.value[documentOrder.value.length - 1]
    if (oldestId && oldestId !== activeDocumentPath.value) {
      closeDocument(oldestId)
    }
  }

  /**
   * 检测文件类型
   */
  function detectFileType(filePath: string): FileType {
    const ext = filePath.split('.').pop()?.toLowerCase()

    const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'svg', 'webp']
    const binaryExts = ['exe', 'dll', 'so', 'dylib', 'bin', 'dat']
    const codeExts = ['js', 'ts', 'jsx', 'tsx', 'vue', 'py', 'java', 'cpp', 'c', 'h', 'rs', 'go', 'rb', 'php']


    if (imageExts.includes(ext || '')) return FileType.IMAGE
    if (binaryExts.includes(ext || '')) return FileType.BINARY
    if (codeExts.includes(ext || '')) return FileType.CODE
    if (ext === 'md') return FileType.MARKDOWN
    if (ext === 'pdf') return FileType.PDF

    return FileType.TEXT
  }

  /**
   * 检测编程语言
   */
  function detectLanguage(filePath: string): string {
    const ext = filePath.split('.').pop()?.toLowerCase()

    const languageMap: Record<string, string> = {
      'js': 'javascript',
      'ts': 'typescript',
      'jsx': 'javascriptreact',
      'tsx': 'typescriptreact',
      'vue': 'vue',
      'py': 'python',
      'java': 'java',
      'cpp': 'cpp',
      'c': 'c',
      'h': 'c',
      'rs': 'rust',
      'go': 'go',
      'rb': 'ruby',
      'php': 'php',
      'html': 'html',
      'css': 'css',
      'scss': 'scss',
      'json': 'json',
      'xml': 'xml',
      'yaml': 'yaml',
      'yml': 'yaml',
      'md': 'markdown',
      'sql': 'sql',
      'sh': 'shell',
      'bash': 'shell'
    }

    return languageMap[ext || ''] || 'plaintext'
  }

  /**
   * 从磁盘加载文件内容
   */
  async function loadFileContent(filePath: string, fileType: FileType): Promise<string | ArrayBuffer> {
    // 对于图片和二进制文件，返回 ArrayBuffer
    if (fileType === FileType.IMAGE || fileType === FileType.BINARY|| fileType === FileType.PDF) {
      return await invoke('read_file_bytes', { path: filePath })
    }
    var res = await tauri_request<ResponseData<string>>("read_note_file", {
      path: filePath
    });
    if (res && res.code == 200) {
      //重新赋值
      return res.data;
    } else {
      NotificationUtil.error("读取失败：" + res.msg);
      throw new Error('Failed to read file')
    }
  }

  /**
   * 导出文档快照数据（用于会话快照）
   */
  function exportOpenFiles(): OpenFileInfo[] {
    return documentList.value.map(doc => ({
      path: doc.id,
      is_dirty: doc.isDirty,
      cursor_line: doc.viewState.cursor?.line ?? 0,
      cursor_col: doc.viewState.cursor?.column ?? 0,
      scroll_top: doc.viewState.scrollTop ?? 0
    }))
  }

  /**
   * 获取文档的保存状态描述
   */
  function getDocumentStatus(filePath: string): string {
    const doc = documents.value.get(filePath)
    if (!doc) return ''

    if (doc.isLoading) return 'Loading...'
    if (doc.isDirty) return 'Modified'
    if (doc.isReadOnly) return 'Read Only'
    return 'Saved'
  }

  return {
    // 状态
    documents,
    activeDocumentPath,
    documentOrder,
    recentlyClosed,
    favoriteFiles,
    recentFiles,
    // 计算属性
    activeDocument,
    documentList,
    hasUnsavedChanges,

    // 方法
    openDocument,
    openRemoteDocument,
    setActiveDocument,
    updateDocumentContent,
    updateViewState,
    saveDocument,
    saveRemoteDocument,
    saveAllDocuments,
    closeDocument,
    undoClose,
    closeAllDocuments,
    closeOtherDocuments,
    reloadDocument,
    reorderDocuments,
    createNewDocument,
    renameDocument,
    getDocumentStatus,
    exportOpenFiles,
    // 收藏
    loadFavorites,
    toggleFavorite,
    isFavorite,
    // 最近打开
    loadRecentFiles,
    addRecentFile,
    removeRecentFile,
    clearRecentFiles
  }
})
