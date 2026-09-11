// stores/documentManager.ts
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@/api/rpc'
import { tauri_request } from '@/utils/tauri_request'
import { ResponseData } from '@/entity/entitys'
import { md5 } from '@/utils/Md5Utils'
import { pdf_store_conten_key_prefix, pdf_store_tem_key_prefix } from '@/setting'
import NotificationUtil from '@/utils/NotificationUtil'
import { load, type Store } from '@/core/runtime'
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

  // 收藏文件列表（持久化到 KV：plugins/files.json）
  const favoriteFiles = ref<string[]>([])
  // 最近打开文件列表（持久化到 KV：plugins/files.json）
  const recentFiles = ref<string[]>([])

  // 最大缓存文档数（超出则销毁最旧的）
  const MAX_CACHED_DOCUMENTS = 20
  const MAX_RECENT_FILES = 30
  /** 代码/文本文件走 monaco 编辑器的最大字节数（>该值走 TextView 只读分块查看，避免整读卡顿/内存膨胀） */
  const EDITOR_MAX_SIZE_BYTES = 1024 * 1024 // 1MB

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

  // ===== 收藏 / 最近打开 的持久化层 =====
  // 走自研 KV 命令（约定：插件数据 plugins/<plugin_id>.json），不用 localStorage。
  // 关键修正：store 创建时**立即**发起读取。此前由 SidebarPanel.onMounted 触发 load*()，
  // 而 App 启动的会话恢复更早执行 openDocument → addRecentFile，会以空列表为基准写回磁盘，
  // 把已持久化的「最近打开」整体冲掉（表现为：清空后下次启动又回来了）。
  const FILES_STORE_FILE = 'plugins/files.json'
  const KEY_FAVORITES = 'favorites'
  const KEY_RECENT = 'recent'

  /** 首次读取任务（单例，避免并发重复读） */
  let listsReady: Promise<void> | null = null

  function readPathList(value: unknown): string[] {
    return Array.isArray(value) ? value.filter((v): v is string => typeof v === 'string') : []
  }

  /** 读取旧版 localStorage 数据（一次性迁移用） */
  function readLegacyList(key: string): string[] {
    try {
      const raw = localStorage.getItem(key)
      if (!raw) return []
      const parsed = JSON.parse(raw)
      const list = readPathList(parsed)
      // 迁移成功即清理旧键，避免两套存储长期并存
      localStorage.removeItem(key)
      return list
    } catch {
      return []
    }
  }

  function ensureListsLoaded(): Promise<void> {
    if (!listsReady) {
      listsReady = (async () => {
        try {
          const s = await load(FILES_STORE_FILE, { autoSave: false, defaults: {} })
          const [fav, rec] = await Promise.all([s.get(KEY_FAVORITES), s.get(KEY_RECENT)])
          let favList = readPathList(fav)
          let recList = readPathList(rec)

          // 一次性迁移：旧版本把数据存在 localStorage（项目约定已禁止 localStorage 存关键数据）。
          // 无论 KV 是否已有数据都取一次并删除旧键，避免两套存储长期并存。
          const legacyFav = readLegacyList('file-favorites')
          const legacyRec = readLegacyList('file-recent')
          if (favList.length === 0 && legacyFav.length > 0) {
            favList = legacyFav
            await s.set(KEY_FAVORITES, favList).catch(() => {})
          }
          if (recList.length === 0 && legacyRec.length > 0) {
            recList = legacyRec
            await s.set(KEY_RECENT, recList).catch(() => {})
          }

          favoriteFiles.value = favList
          recentFiles.value = recList
        } catch (e) {
          console.warn('加载收藏/最近打开失败:', e)
        }
      })()
    }
    return listsReady
  }
  // 立即启动（不 await）：后续任何写操作都会先等它完成，保证不会用空列表覆盖磁盘数据
  void ensureListsLoaded()

  /** 写盘：必须等首次加载完成后再写，否则会把磁盘上的历史列表覆盖掉 */
  async function persistList(key: string, value: string[]) {
    await ensureListsLoaded()
    try {
      const s = await load(FILES_STORE_FILE, { autoSave: false, defaults: {} })
      await s.set(key, value)
    } catch (e) {
      console.warn('保存收藏/最近打开失败:', e)
    }
  }

  /** 列表变更串行化：连续打开多个文件时避免并发读写互相覆盖 */
  let listChain: Promise<unknown> = Promise.resolve()
  function withListLock(task: () => Promise<void>) {
    const p = listChain.then(task, task)
    listChain = p.catch(() => {})
    return p
  }

  // ===== 收藏管理 =====
  async function loadFavorites() {
    await ensureListsLoaded()
  }
  function toggleFavorite(filePath: string) {
    return withListLock(async () => {
      await ensureListsLoaded()
      const idx = favoriteFiles.value.indexOf(filePath)
      if (idx >= 0) {
        favoriteFiles.value.splice(idx, 1)
      } else {
        favoriteFiles.value.push(filePath)
      }
      await persistList(KEY_FAVORITES, favoriteFiles.value)
    })
  }
  function isFavorite(filePath: string) {
    return favoriteFiles.value.includes(filePath)
  }

  // ===== 最近打开管理 =====
  // 语义：「最近打开」是**用户主动打开过的文件历史**，与「当前打开的 tab」是两回事。
  // 会话恢复（启动还原上次未关闭的 tab）不是用户主动打开动作，不写入该列表，
  // 否则清空后下次启动会被恢复的 tab 重新填满，且列表将永远等于当前 tab 集合。
  async function loadRecentFiles() {
    await ensureListsLoaded()
  }
  function addRecentFile(filePath: string) {
    return withListLock(async () => {
      await ensureListsLoaded()
      // 移除重复项，添加到最前
      const next = [filePath, ...recentFiles.value.filter(p => p !== filePath)]
      // 限制数量
      recentFiles.value = next.slice(0, MAX_RECENT_FILES)
      await persistList(KEY_RECENT, recentFiles.value)
    })
  }
  function removeRecentFile(filePath: string) {
    return withListLock(async () => {
      await ensureListsLoaded()
      recentFiles.value = recentFiles.value.filter(p => p !== filePath)
      await persistList(KEY_RECENT, recentFiles.value)
    })
  }
  function clearRecentFiles() {
    return withListLock(async () => {
      await ensureListsLoaded()
      recentFiles.value = []
      await persistList(KEY_RECENT, recentFiles.value)
    })
  }

  /**
   * 打开或创建文档
   * @param opts.recordRecent 是否记入「最近打开」（默认 true）。会话恢复路径必须传 false：
   *   还原上次未关闭的 tab 不是用户主动打开动作，写进历史会让「最近打开」永远等于当前 tab 集合，
   *   并导致用户清空历史后下次启动又被自动填满。
   */
  async function openDocument(
    filePath: string,
    fileType?: FileType,
    opts?: { recordRecent?: boolean },
  ) {
    addAppOpenFile(filePath);
    if (opts?.recordRecent !== false) void addRecentFile(filePath)
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
      } else if (type === FileType.CODE || type === FileType.TEXT) {
        // 代码/文本文件：大文件分诊 —— ≤1MB 整读供代码编辑器（monaco）；>1MB 不整读，由 TextView 分块只读加载
        const size = await getFileSizeBytes(filePath)
        if (size == null || size <= EDITOR_MAX_SIZE_BYTES) {
          const content = await loadFileContent(filePath, type)
          doc.content = content
          doc.originalContent = content
        }
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

    // 未命名文档没有真实磁盘路径：跳过落盘（否则会以相对路径写出 "untitled:xxx" 垃圾文件），
    // 未保存内容由会话草稿机制保护（captureSnapshot → draftSave）
    if (doc.id.startsWith('untitled:')) return true

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
   * 未命名文档（untitled:）无磁盘路径，跳过落盘（由会话草稿机制保护）
   */
  async function saveAllDocuments() {
    const unsavedDocs = Array.from(documents.value.values())
      .filter(doc => doc.isDirty && !doc.id.startsWith('untitled:'))

    const savePromises = unsavedDocs.map(doc => saveDocument(doc.id))
    await Promise.allSettled(savePromises)
  }

  /**
   * 关闭文档
   * @param opts.syncSession 是否同步剔除会话快照（默认 true）。
   *   用户主动关闭 tab / 全部关闭 → true：立即从 {workspace}.session.json 剔除该路径，
   *   防进程异常退出后旧快照下次启动把已关闭文件恢复出来（表现为「关闭无效」）。
   *   工作区切换清理 / LRU 缓存逐出 → false：这两种不是用户关闭意图，不应动快照
   *   （切换场景的快照在 closeAllDocuments 之前已整体保存，逐出场景保持旧语义）。
   */
  function closeDocument(filePath: string, opts?: { syncSession?: boolean }) {
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

    // 用户主动关闭：同步剔除持久化会话快照（避免异常退出后旧快照复活已关闭 tab）
    if (opts?.syncSession !== false) {
      syncClosedSessionFiles([filePath])
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
   * 启动时恢复「上次未关闭的 tab」（openfile.json）。
   *
   * 与「最近打开」严格区分：
   * - openfile.json = 当前仍打开、上次退出时未关闭的 tab（打开时写入、关闭时删除）
   * - 最近打开 = 用户主动打开过的历史记录，不参与启动恢复
   *
   * 恢复前先校验文件是否仍存在，已被移动/删除的记录顺带清理，
   * 避免失效路径每次启动都尝试打开并在标签栏留下空壳。
   */
  async function restoreOpenFiles() {
    let store: Store | undefined
    try {
      store = await load('openfile.json', { autoSave: false, defaults: {} })
    } catch (e) {
      console.warn('加载 openfile.json 失败:', e)
      return
    }
    if (!store) return

    let entries: [string, unknown][] = []
    try {
      entries = await store.entries()
    } catch (e) {
      console.warn('读取 openfile.json 失败:', e)
      return
    }

    for (const [key, value] of entries) {
      const path = String(value ?? '')
      if (!path) {
        await store.delete(key).catch(() => false)
        continue
      }
      // 文件已不存在：清理失效记录，不再恢复
      const size = await getFileSizeBytes(path)
      if (size == null) {
        await store.delete(key).catch(() => false)
        continue
      }
      try {
        await openDocument(path, undefined, { recordRecent: false })
      } catch (e) {
        console.warn(`恢复文件失败: ${path}`, e)
      }
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
   * 用户主动关闭文档后，同步剔除持久化会话快照（异步 fire-and-forget）。
   * 动态 import 避免与 session store 形成静态循环依赖（session.ts 顶层引用了本 store）。
   */
  function syncClosedSessionFiles(paths: string[]) {
    if (!paths.length) return
    void (async () => {
      try {
        const { useSessionStore } = await import('@/store/modules/session')
        await useSessionStore().syncSnapshotAfterClosed(paths)
      } catch (e) {
        console.warn('同步会话快照失败:', e)
      }
    })()
  }

  /**
   * 关闭所有文档
   * @param opts.syncSession 默认 true（用户「全部关闭」→ 剔除快照）；工作区切换清理传 false
   */
  function closeAllDocuments(opts?: { syncSession?: boolean }) {
    const allIds = Array.from(documents.value.keys())
    allIds.forEach(id => closeDocument(id, { syncSession: false }))
    // 批量关闭后统一同步一次快照（避免 N 次串行读写）
    if (opts?.syncSession !== false) {
      syncClosedSessionFiles(allIds)
    }
  }

  /**
   * 关闭其他文档（保留指定文档）
   * @param opts.syncSession 默认 true（用户「关闭其他」→ 剔除快照）
   */
  function closeOtherDocuments(filePath: string, opts?: { syncSession?: boolean }) {
    const otherIds = Array.from(documents.value.keys())
      .filter(id => id !== filePath)

    otherIds.forEach(id => closeDocument(id, { syncSession: false }))
    if (opts?.syncSession !== false) {
      syncClosedSessionFiles(otherIds)
    }
  }

  /**
   * 重新加载文档（从磁盘重新读取）
   */
  async function reloadDocument(filePath: string) {
    const doc = documents.value.get(filePath)
    if (!doc) return
    try {
      doc.isLoading = true
      // 代码/文本文件同样做大小分诊：>1MB 不整读（TextView 分块），避免 reload 卡顿
      if (doc.type === FileType.CODE || doc.type === FileType.TEXT) {
        const size = await getFileSizeBytes(filePath)
        if (size != null && size > EDITOR_MAX_SIZE_BYTES) {
          doc.content = null
          doc.originalContent = null
          doc.isDirty = false
          doc.lastModified = Date.now()
          doc.isLoading = false
          return
        }
      }
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
      // 缓存逐出不是用户关闭意图：不剔除会话快照（保持旧语义）
      closeDocument(oldestId, { syncSession: false })
    }
  }

  /**
   * 检测文件类型
   */
  function detectFileType(filePath: string): FileType {
    const ext = filePath.split('.').pop()?.toLowerCase()

    const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'svg', 'webp']
    const binaryExts = ['exe', 'dll', 'so', 'dylib', 'bin', 'dat']
    // 代码/文本类扩展（走 monaco 编辑或大文件分诊；与 detectLanguage 的 languageMap 对齐）
    const codeExts = [
      // 前端/脚本
      'js', 'ts', 'jsx', 'tsx', 'vue', 'html', 'htm', 'css', 'scss', 'sass', 'less',
      // 后端/通用编程语言
      'py', 'java', 'kt', 'kts', 'swift', 'cpp', 'cc', 'cxx', 'c', 'h', 'hpp', 'rs', 'go', 'rb', 'php', 'cs',
      // 数据/配置/标记
      'json', 'xml', 'yaml', 'yml', 'toml', 'ini', 'sql', 'sh', 'bash', 'bat', 'ps1'
    ]


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
      'kt': 'kotlin',
      'kts': 'kotlin',
      'swift': 'swift',
      'cpp': 'cpp',
      'cc': 'cpp',
      'cxx': 'cpp',
      'c': 'c',
      'h': 'c',
      'hpp': 'cpp',
      'rs': 'rust',
      'go': 'go',
      'rb': 'ruby',
      'php': 'php',
      'cs': 'csharp',
      'html': 'html',
      'htm': 'html',
      'css': 'css',
      'scss': 'scss',
      'sass': 'scss',
      'less': 'less',
      'json': 'json',
      'xml': 'xml',
      'yaml': 'yaml',
      'yml': 'yaml',
      'toml': 'plaintext',
      'ini': 'ini',
      'md': 'markdown',
      'sql': 'sql',
      'sh': 'shell',
      'bash': 'shell',
      'bat': 'bat',
      'ps1': 'powershell'
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
   * 获取文件大小（字节）。失败（不存在/读不到）返回 null。
   * 供打开文档时做"大文件分诊"。
   */
  async function getFileSizeBytes(filePath: string): Promise<number | null> {
    try {
      const res = await tauri_request<ResponseData<number>>('get_file_size', { path: filePath })
      if (res && res.code === 200 && typeof res.data === 'number') return res.data
      return null
    } catch {
      return null
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
    restoreOpenFiles,
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
