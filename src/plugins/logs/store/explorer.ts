/**
 * 日志浏览器（LogExplorer）状态管理
 *
 * 职责：
 * - 解析浏览器根目录：优先使用「单独设置的目录」（按工作区持久化），否则回退到当前工作区设置的目录；
 * - 按「过滤文件类型」递归列举目录树并裁剪（仅保留匹配日志文件的分支）；
 * - 维护过滤文件类型（默认 .log/.txt 等日志扩展名 + 无后缀名文件）。
 *
 * 与 database 的 ScriptExplorer 区别：ScriptExplorer 固定指向插件自身的脚本存储目录；
 * LogExplorer 指向「工作区设置的目录」（除非被单独设置覆盖），更贴近日志工作台场景。
 */
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { call } from '@/api/rpc'
import { open, subscribe } from '@/core/runtime'
import { getCurrentWorkspaceId } from '@/composables/usePluginDir'
import { useWorkspaceStore } from '@/store/modules/workspace'
import {
  DEFAULT_FILE_TYPES,
  formatFileTypes,
  loadExplorerDir,
  loadFileTypes,
  matchFileType,
  parseFileTypes,
  saveExplorerDir,
  saveFileTypes,
} from '../core/storage'
import { useLogStore } from './logs'

export interface LogFileNode {
  name: string
  path: string
  isDir: boolean
  children: LogFileNode[]
}

/** Rust list_script_files 的 snake_case 原始节点 */
interface RawFileEntry {
  name: string
  path: string
  is_dir: boolean
  children: RawFileEntry[]
}

interface ListResp {
  code: number
  msg?: string
  data: RawFileEntry[]
}

export const useLogExplorerStore = defineStore('logExplorer', () => {
  /** 单独设置的浏览器根目录（按工作区持久化）；空串表示回退工作区目录 */
  const explorerDir = ref('')
  /** 过滤文件类型（规范化后：'.ext' 或 '' 表示无后缀） */
  const fileTypes = ref<string[]>([...DEFAULT_FILE_TYPES])
  /** 过滤后的目录树 */
  const tree = ref<LogFileNode[]>([])
  const loading = ref(false)
  const errorMsg = ref('')

  const wsId = () => getCurrentWorkspaceId() || '__global__'

  /** 有效根目录：explorerDir 优先，否则回退当前工作区 folders[0] */
  const rootDir = computed(() => {
    if (explorerDir.value) return explorerDir.value
    return useWorkspaceStore().currentWorkspace?.folders?.[0] ?? ''
  })

  /** 是否正在使用工作区目录（用于界面提示） */
  const usingWorkspaceDir = computed(() => !explorerDir.value)

  function rawToNode(e: RawFileEntry): LogFileNode {
    return {
      name: e.name,
      path: e.path,
      isDir: e.is_dir,
      children: (e.children || []).map(rawToNode),
    }
  }

  /** 按文件类型裁剪目录树：仅保留包含匹配文件的目录分支 */
  function filterTree(nodes: LogFileNode[]): LogFileNode[] {
    const types = fileTypes.value
    const out: LogFileNode[] = []
    for (const n of nodes) {
      if (n.isDir) {
        const children = filterTree(n.children)
        if (children.length > 0) out.push({ ...n, children })
      } else if (matchFileType(n.name, types)) {
        out.push(n)
      }
    }
    return out
  }

  /** 递归列举根目录并过滤 */
  async function refresh() {
    const root = rootDir.value
    if (!root) {
      tree.value = []
      errorMsg.value = '未设置工作区目录，请在设置中指定日志目录'
      return
    }
    loading.value = true
    errorMsg.value = ''
    try {
      const res = await call<ListResp>('list_script_files', { path: root })
      if (!res || res.code !== 200) {
        errorMsg.value = res?.msg || '读取目录失败'
        tree.value = []
        return
      }
      const all = (res.data || []).map(rawToNode)
      tree.value = filterTree(all)
    } catch (e) {
      errorMsg.value = (e as Error).message || '读取目录失败'
      tree.value = []
    } finally {
      loading.value = false
    }
  }

  /** 设置单独目录（覆盖工作区目录） */
  async function setExplorerDir(dir: string) {
    const norm = dir.replace(/\\/g, '/').replace(/\/+$/, '')
    if (!norm) return
    explorerDir.value = norm
    await saveExplorerDir(norm, wsId())
    await refresh()
  }

  /** 重置为工作区目录（清除单独设置） */
  async function resetToWorkspace() {
    explorerDir.value = ''
    await saveExplorerDir('', wsId())
    await refresh()
  }

  /** 设置过滤文件类型（解析用户输入并持久化） */
  async function setFileTypesText(text: string) {
    const types = parseFileTypes(text)
    fileTypes.value = types
    await saveFileTypes(types, wsId())
    await refresh()
  }

  /** 当前文件类型的展示文本 */
  const fileTypesText = computed(() => formatFileTypes(fileTypes.value))

  /** 双击文件 → 打开为日志源（已打开则复用对应 tab，不重复加载） */
  function openFile(path: string, name?: string) {
    useLogStore().openLocalFileOrFocus(path, name)
  }

  /** 选择目录对话框 */
  async function pickDirectory(): Promise<void> {
    const selected = await open({ directory: true, multiple: false })
    if (typeof selected === 'string' && selected) {
      await setExplorerDir(selected)
    }
  }

  /** 在文件管理器中打开根目录 */
  async function revealRoot() {
    if (!rootDir.value) return
    try {
      await call('file_manager_open_path', { path: rootDir.value })
    } catch {
      /* ignore */
    }
  }

  /** 初始化：加载当前工作区的目录与文件类型设置，并刷新树 */
  async function init() {
    explorerDir.value = await loadExplorerDir(wsId())
    fileTypes.value = await loadFileTypes(wsId())
    await refresh()
  }

  /** 工作区切换：重新加载设置（explorerDir 按工作区隔离）并刷新 */
  async function onWorkspaceChanged() {
    await init()
  }

  // 工作区切换事件监听（store 单例只装一次）
  void (async () => {
    await subscribe<void>('workspace-changed', () => {
      void onWorkspaceChanged()
    })
  })()

  return {
    explorerDir,
    fileTypes,
    tree,
    loading,
    errorMsg,
    rootDir,
    usingWorkspaceDir,
    fileTypesText,
    init,
    refresh,
    setExplorerDir,
    resetToWorkspace,
    setFileTypesText,
    openFile,
    pickDirectory,
    revealRoot,
    onWorkspaceChanged,
  }
})
