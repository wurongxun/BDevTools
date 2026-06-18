// stores/documentManager.ts
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from "@tauri-apps/api/core";
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
}

export const useDocumentManagerStore = defineStore('documentManager', () => {
  // 所有打开的文档，key 为文件路径
  const documents = ref<Map<string, DocumentState>>(new Map())

  // 当前激活的文档路径
  const activeDocumentId = ref<string | null>(null)

  // 文档打开顺序（用于标签页排序）
  const documentOrder = ref<string[]>([])

  // 最近关闭的文档（用于撤销关闭）
  const recentlyClosed = ref<Array<{ document: DocumentState; index: number }>>([])

  // 最大缓存文档数（超出则销毁最旧的）
  const MAX_CACHED_DOCUMENTS = 20

  // 计算属性：获取当前激活的文档
  const activeDocument = computed(() => {
    if (!activeDocumentId.value) return null
    return documents.value.get(activeDocumentId.value) || null
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

  /**
   * 打开或创建文档
   */
  async function openDocument(filePath: string, fileType?: FileType) {
    // 如果文档已打开，直接激活
    if (documents.value.has(filePath)) {
      setActiveDocument(filePath)
      return
    }

    // 创建文档状态对象
    const fileName = filePath.split(/[/\\]/).pop() || 'Untitled'
    const type = fileType || detectFileType(filePath)

    const doc: DocumentState = {
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
      const content = await loadFileContent(filePath, type)
      doc.content = content
      doc.originalContent = content
      doc.isLoading = false
      doc.lastModified = Date.now()
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
   * 设置激活文档
   */
  function setActiveDocument(filePath: string | null) {
    // 保存当前文档的视图状态
    if (activeDocumentId.value) {
      // 这里可以触发保存视图状态的逻辑
    }

    activeDocumentId.value = filePath

    // 更新文档顺序（将激活的文档移到最前面）
    if (filePath) {
      const index = documentOrder.value.indexOf(filePath)
      if (index > -1) {
        documentOrder.value.splice(index, 1)
        documentOrder.value.unshift(filePath)
      }
    }
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
  async function saveDocument(filePath: string) {
    const doc = documents.value.get(filePath)
    if (!doc || !doc.content) return

    try {
      await invoke('save_file', {
        path: filePath,
        content: doc.content,
        encoding: doc.encoding
      })

      doc.originalContent = doc.content
      doc.isDirty = false
      doc.lastModified = Date.now()
    } catch (error) {
      console.error(`Failed to save file: ${filePath}`, error)
      throw error
    }
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
    const doc = documents.value.get(filePath)
    if (!doc) return

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
    if (activeDocumentId.value === filePath) {
      const nextDoc = documentOrder.value[0] || null
      setActiveDocument(nextDoc)
    }
  }
//  重排序方法
  function reorderDocuments(newOrder: string[]) {
    documentOrder.value = newOrder
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
    const doc: DocumentState = {
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
    if (activeDocumentId.value === oldPath) {
      activeDocumentId.value = newPath
    }
  }

  /**
   * 清理文档缓存（LRU策略）
   */
  function cleanupDocumentCache() {
    if (documents.value.size <= MAX_CACHED_DOCUMENTS) return

    // 获取最旧的文档（最后一个）
    const oldestId = documentOrder.value[documentOrder.value.length - 1]
    if (oldestId && oldestId !== activeDocumentId.value) {
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
    if (fileType === FileType.IMAGE || fileType === FileType.BINARY) {
      return await invoke('read_file_bytes', { path: filePath })
    }

    // 对于文本和代码文件，返回字符串
    return await invoke('read_file_text', { path: filePath })
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
    activeDocumentId,
    documentOrder,
    recentlyClosed,

    // 计算属性
    activeDocument,
    documentList,
    hasUnsavedChanges,

    // 方法
    openDocument,
    setActiveDocument,
    updateDocumentContent,
    updateViewState,
    saveDocument,
    saveAllDocuments,
    closeDocument,
    undoClose,
    closeAllDocuments,
    closeOtherDocuments,
    reloadDocument,
    createNewDocument,
    renameDocument,
    getDocumentStatus
  }
})
