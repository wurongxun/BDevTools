/**
 * 查询文件管理器状态：
 * - 树数据来自 scripts 目录（usePluginDir 子目录，按工作区隔离）
 * - 文件与连接/数据库的绑定关系存于 scripts/index.json（中央索引，加载时自动对账）
 */
import { defineStore } from 'pinia'
import { ref, onMounted, onUnmounted } from 'vue'
import { tauri_request } from '@/utils/tauri_request'
import { usePluginDir } from '@/composables/usePluginDir'
import type { ResponseData } from '@/Types/entitys'
import type { ScriptFileBinding, ScriptFileNode } from '../types'

/** Rust 返回的原始节点（snake_case） */
interface ScriptFileEntryRaw {
  name: string
  path: string
  is_dir: boolean
  children: ScriptFileEntryRaw[]
}

const INDEX_FILE = 'index.json'

/** 路径归一化：统一为正斜杠，保证索引 key 与树节点路径可比 */
function norm(path: string): string {
  return path.replace(/\\/g, '/')
}

export const useScriptsStore = defineStore('dbScripts', () => {
  const pluginDir = usePluginDir('database')

  /** scripts 目录绝对路径 */
  const scriptsDir = ref('')
  /** 目录树（已排除 index.json） */
  const tree = ref<ScriptFileNode[]>([])
  /** 绑定索引：归一化绝对路径 -> 连接/数据库 */
  const bindings = ref<Record<string, ScriptFileBinding>>({})
  const loading = ref(false)

  /** 获取文件的绑定（路径自动归一化） */
  function getBinding(filePath: string): ScriptFileBinding | undefined {
    return bindings.value[norm(filePath)]
  }

  /** 收集树中所有文件路径（归一化） */
  function collectFilePaths(nodes: ScriptFileNode[], out: Set<string> = new Set()): Set<string> {
    for (const n of nodes) {
      if (n.isDir) collectFilePaths(n.children, out)
      else out.add(n.path)
    }
    return out
  }

  /** Rust 原始节点 -> 前端节点（归一化路径、过滤 index.json） */
  function mapEntries(entries: ScriptFileEntryRaw[]): ScriptFileNode[] {
    return entries
      .filter(e => !(e.name === INDEX_FILE && !e.is_dir))
      .map(e => ({
        name: e.name,
        path: norm(e.path),
        isDir: e.is_dir,
        children: e.is_dir ? mapEntries(e.children) : [],
      }))
  }

  /** 持久化绑定索引到 scripts/index.json */
  async function persistIndex() {
    if (!scriptsDir.value) return
    const indexPath = norm(`${scriptsDir.value}/${INDEX_FILE}`)
    await tauri_request<ResponseData<string>>('write_script_file', {
      path: indexPath,
      content: JSON.stringify(bindings.value, null, 2),
    })
  }

  /** 加载绑定索引并做一次对账：目录内已不存在的条目清除，目录外的条目（另存的绝对路径）保留 */
  async function loadIndex() {
    if (!scriptsDir.value) return
    const indexPath = norm(`${scriptsDir.value}/${INDEX_FILE}`)
    let raw: Record<string, ScriptFileBinding> = {}
    try {
      const res = await tauri_request<ResponseData<string>>('read_note_file', { path: indexPath })
      if (res?.code === 200 && res.data) {
        raw = JSON.parse(res.data)
      }
    } catch {
      // 索引不存在或损坏：视为空索引
    }
    const existing = collectFilePaths(tree.value)
    const dirPrefix = norm(scriptsDir.value) + '/'
    const reconciled: Record<string, ScriptFileBinding> = {}
    let changed = false
    for (const [key, val] of Object.entries(raw)) {
      const nk = norm(key)
      if (!nk.startsWith(dirPrefix) || existing.has(nk)) {
        reconciled[nk] = val
      } else {
        changed = true
      }
    }
    bindings.value = reconciled
    if (changed) await persistIndex()
  }

  /** 刷新目录树 + 绑定索引 */
  async function refresh() {
    loading.value = true
    try {
      scriptsDir.value = await pluginDir.getSubDir('scripts')
      if (!scriptsDir.value) return
      const res = await tauri_request<ResponseData<ScriptFileEntryRaw[]>>('list_script_files', {
        path: scriptsDir.value,
      })
      tree.value = res?.code === 200 ? mapEntries(res.data || []) : []
      await loadIndex()
    } finally {
      loading.value = false
    }
  }

  /** 保存文件与连接/数据库的绑定（保存 SQL 时自动调用） */
  async function bindFile(filePath: string, configId?: string, database?: string) {
    if (!filePath) return
    bindings.value[norm(filePath)] = {
      configId: configId || undefined,
      database: database || undefined,
      updatedAt: Date.now(),
    }
    await persistIndex()
  }

  /** 清除文件绑定 */
  async function unbindFile(filePath: string) {
    delete bindings.value[norm(filePath)]
    await persistIndex()
  }

  /** 新建查询文件，返回归一化路径 */
  async function createFile(dirPath: string, name: string): Promise<string> {
    const fileName = name.endsWith('.sql') ? name : `${name}.sql`
    const filePath = norm(`${dirPath}/${fileName}`)
    // 重名检查：避免静默覆盖已有文件
    const parent = tree.value
    const exists = checkExists(parent, filePath)
    if (exists) throw new Error(`已存在同名文件「${fileName}」`)
    const res = await tauri_request<ResponseData<string>>('write_script_file', {
      path: filePath,
      content: '-- 输入 SQL 语句\n',
    })
    if (res?.code !== 200) throw new Error(res?.msg || '新建查询文件失败')
    await refresh()
    return filePath
  }

  /** 在树中查找指定路径的文件是否存在 */
  function checkExists(nodes: ScriptFileNode[], filePath: string): boolean {
    for (const n of nodes) {
      if (n.path === filePath) return true
      if (n.isDir && checkExists(n.children, filePath)) return true
    }
    return false
  }

  /** 新建文件夹 */
  async function createFolder(dirPath: string, name: string) {
    const res = await tauri_request<ResponseData<string>>('create_folder', {
      fatherPath: dirPath,
      newFolderName: name,
    })
    if (res?.code !== 200) throw new Error(res?.msg || '新建文件夹失败')
    await refresh()
  }

  /** 重命名文件（保留 .sql 扩展名，同步迁移索引 key） */
  async function renameFile(oldPath: string, newName: string) {
    const oldKey = norm(oldPath)
    const res = await tauri_request<ResponseData<string>>('rename_script_file', {
      path: oldPath,
      newName,
    })
    if (res?.code !== 200) throw new Error(res?.msg || '重命名失败')
    // data 返回重命名后的新路径，迁移绑定
    const binding = bindings.value[oldKey]
    if (binding) {
      delete bindings.value[oldKey]
      if (res.data) bindings.value[norm(res.data)] = binding
      await persistIndex()
    }
    await refresh()
  }

  /** 重命名文件夹（同步迁移该目录下所有索引 key） */
  async function renameFolder(oldPath: string, newName: string) {
    const oldPrefix = norm(oldPath) + '/'
    const parent = norm(oldPath).slice(0, norm(oldPath).lastIndexOf('/'))
    const newPrefix = `${parent}/${newName}/`
    const res = await tauri_request<ResponseData<string>>('up_folder_name', {
      path: oldPath,
      upFolderName: newName,
    })
    if (res?.code !== 200) throw new Error(res?.msg || '重命名文件夹失败')
    for (const key of Object.keys(bindings.value)) {
      if (key.startsWith(oldPrefix)) {
        bindings.value[newPrefix + key.slice(oldPrefix.length)] = bindings.value[key]
        delete bindings.value[key]
      }
    }
    await persistIndex()
    await refresh()
  }

  /** 删除文件（同步清理绑定） */
  async function deleteFile(filePath: string) {
    const res = await tauri_request<ResponseData<string>>('remove_file', { path: filePath })
    if (res?.code !== 200) throw new Error(res?.msg || '删除失败')
    delete bindings.value[norm(filePath)]
    await persistIndex()
    await refresh()
  }

  /** 删除文件夹（同步清理目录下所有绑定） */
  async function deleteFolder(dirPath: string) {
    const res = await tauri_request<ResponseData<string>>('remove_folder', { path: dirPath })
    if (res?.code !== 200) throw new Error(res?.msg || '删除失败')
    const prefix = norm(dirPath) + '/'
    for (const key of Object.keys(bindings.value)) {
      if (key.startsWith(prefix)) delete bindings.value[key]
    }
    await persistIndex()
    await refresh()
  }

  // 工作区切换时重新加载（目录本身按工作区隔离）
  function onWorkspaceChanged() {
    refresh()
  }

  onMounted(() => {
    window.addEventListener('workspace-changed', onWorkspaceChanged)
  })

  onUnmounted(() => {
    window.removeEventListener('workspace-changed', onWorkspaceChanged)
  })

  return {
    scriptsDir,
    tree,
    bindings,
    loading,
    getBinding,
    refresh,
    bindFile,
    unbindFile,
    createFile,
    createFolder,
    renameFile,
    renameFolder,
    deleteFile,
    deleteFolder,
  }
})
