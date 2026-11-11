/**
 * SQL 编辑器 Tab 状态管理
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { EditorTab, SqlExecuteMode, SqlResult } from '../types'

let tabCounter = 0
/** 「新建查询」强制新建的递增序号，用于标题后缀（新查询 1 / 新查询 2 …） */
let newQuerySeq = 0

/** 脚本 tab 单条执行结果项（含执行语句 + 结果） */
export interface ScriptResultEntry {
  /** 唯一标识。**不能用数组下标当组件 key** —— 关闭中间某个子结果会让下标平移，
   *  KeepAlive 里旧下标的缓存实例会被错当成新条目复用，显示张冠李戴的结果。 */
  id: string
  sql: string
  result: SqlResult | null
}

/** 新增子结果时省略 id（由 store 统一生成） */
export type NewScriptResultEntry = Omit<ScriptResultEntry, 'id'>

/** 子结果 id 自增序号（同一会话内唯一即可） */
let resultSeq = 0

/**
 * 「是不是同一条 SQL」的比较形态：折叠空白 + 去首尾空 + 去语句末分号。
 * **刻意不做大小写归一**：MySQL 的 `=` 在非二进制排序规则下不区分大小写，PG / Oracle 区分，
 * 一旦把 `where name='X'` 和 `where name='x'` 判成同一条，这两种库上会看到互相覆盖的结果。
 * 空白差异却是同一个意图（换个缩进不该被当成新语句），所以只归空白。
 */
function normalizeSqlKey(sql: string): string {
  return sql.replace(/\s+/g, ' ').trim().replace(/;\s*$/, '')
}

export const useEditorStore = defineStore('dbEditor', () => {
  /** 打开的 Tab 列表 */
  const tabs = ref<EditorTab[]>([])
  /** 当前激活的 Tab ID */
  const activeTabId = ref<string>('')
  /** 每个 Tab 的多次执行结果（按 tabId 存储，结果数组对应子 tab） */
  const tabResults = ref<Record<string, ScriptResultEntry[]>>({})
  /** 每个 Tab 当前激活的子结果索引 */
  const activeResultIndex = ref<Record<string, number>>({})

  /** 当前激活的 Tab */
  const activeTab = computed(() =>
    tabs.value.find(t => t.id === activeTabId.value)
  )

  /** 当前激活 Tab 当前激活子结果 */
  const activeTabResult = computed<ScriptResultEntry | null>(() => {
    const tabId = activeTabId.value
    if (!tabId) return null
    const arr = tabResults.value[tabId]
    if (!arr || arr.length === 0) return null
    const idx = activeResultIndex.value[tabId] ?? 0
    return arr[idx] ?? arr[0]
  })

  /** 打开新 Tab
   * @param tab.forceNew 为 true 时跳过去重，始终新建（用于「新建查询」每次点击都开新 tab）
   */
  function openTab(tab: Partial<EditorTab> & { forceNew?: boolean }) {
    const id = tab.id || `tab_${++tabCounter}`
    // 强制新建的「新查询」tab：追加递增后缀，避免多个同名 tab 分不清
    const baseTitle = tab.title || '新查询'
    const finalTitle = tab.forceNew && baseTitle === '新查询'
      ? `${baseTitle} ${++newQuerySeq}`
      : baseTitle
    const newTab: EditorTab = {
      id,
      title: finalTitle,
      type: tab.type || 'sql',
      configId: tab.configId,
      database: tab.database,
      table: tab.table,
      content: tab.content || '',
      dirty: false,
      filePath: tab.filePath,
      // 对象列表 / 系统信息 tab 的细分类型（object-list 按 objectType 区分视图/索引/…，
      // system-info 按 subType 区分会话状态/全局变量…）
      objectType: tab.objectType,
      objectName: tab.objectName,
      subType: tab.subType,
    }
    // 检查是否已存在相同的 Tab：sql 文件按路径去重，其余按类型+连接+库+表+细分类型去重
    const existing = tab.forceNew ? undefined : tabs.value.find(t => {
      if (t.type === 'sql' && t.filePath) {
        return newTab.type === 'sql' && t.filePath === newTab.filePath
      }
      return (
        t.type === newTab.type &&
        t.configId === newTab.configId &&
        t.database === newTab.database &&
        t.table === newTab.table &&
        t.objectType === newTab.objectType &&
        t.objectName === newTab.objectName &&
        t.subType === newTab.subType
      )
    })
    if (existing) {
      activeTabId.value = existing.id
      return
    }
    tabs.value.push(newTab)
    activeTabId.value = id
  }

  /** 关闭 Tab */
  function closeTab(tabId: string) {
    const idx = tabs.value.findIndex(t => t.id === tabId)
    if (idx === -1) return
    tabs.value.splice(idx, 1)
    clearTabResults(tabId)
    if (activeTabId.value === tabId) {
      activeTabId.value = tabs.value[Math.min(idx, tabs.value.length - 1)]?.id || ''
    }
  }

  /**
   * 关闭全部 Tab。
   *
   * 逐个调 `closeTab` 也行，但那会对同一个数组做 n 次 splice（中间态还有激活项反复易主），
   * 直接整体清空，再一次性清掉各 tab 的子结果缓存（否则 tabResults 会留下一堆孤儿结果）。
   */
  function closeAllTabs() {
    const ids = tabs.value.map(t => t.id)
    ids.forEach(id => clearTabResults(id))
    tabs.value = []
    activeTabId.value = ''
  }

  /** 更新 Tab 内容 */
  function updateTabContent(tabId: string, content: string) {
    const tab = tabs.value.find(t => t.id === tabId)
    if (tab) {
      tab.content = content
      tab.dirty = true
    }
  }

  /**
   * 记录一次执行结果，返回它落在哪个下标（并自动激活该子结果）。
   *
   * - `reuse`（「执行 SQL 语句」）：同一条语句已经有子结果就**就地替换**它，不再追加新的
   *   子 tab —— 反复跑同一条语句时不会堆出一串同名标签。没有同语句时才追加。
   * - `new-tab`（「在新标签页中执行」）：忽略同语句，**永远追加**。这是用户明确要留多份结果
   *   做对比时才点的，静默复用会让他以为「第二份没出来」。
   *
   * 同语句命中**最后一个**：用户可能已经用 new-tab 攒了好几个同语句结果，「执行 SQL 语句」
   * 应该落到最新的那个（DBeaver / DataGrip 的「重跑当前语句」也是这个语义）。
   *
   * ⚠ 复用时是「替换数组元素 + **换发新 id**」，不是改其中的字段：SqlTab 里
   * QueryResultView 以 `:key="entry.id"` 挂在 KeepAlive 上，id 不变 ⇒ 复用的还是旧实例，
   * 而它只在挂载那次消费 `initialResult` ⇒ 新结果没人认领，用户会以为「点了没反应」。
   * 换 id 会换出新实例，结果 / 页码 / 列过滤都从这一次执行重新开始；旧实例交给 KeepAlive
   * 的 `max` 淘汰（这本来就是「关闭子 tab 不会自动出缓存」那条约定的兜底）。
   */
  function recordExecution(
    tabId: string,
    entry: NewScriptResultEntry,
    mode: SqlExecuteMode = 'reuse',
  ): number {
    if (!tabResults.value[tabId]) tabResults.value[tabId] = []
    const arr = tabResults.value[tabId]

    let idx = -1
    if (mode === 'reuse') {
      const key = normalizeSqlKey(entry.sql)
      for (let i = arr.length - 1; i >= 0; i--) {
        if (normalizeSqlKey(arr[i].sql) === key) {
          idx = i
          break
        }
      }
    }

    const next: ScriptResultEntry = { ...entry, id: `res_${++resultSeq}` }
    if (idx >= 0) {
      arr[idx] = next
    } else {
      arr.push(next)
      idx = arr.length - 1
    }
    activeResultIndex.value[tabId] = idx
    return idx
  }

  /** 设置激活的子结果索引 */
  function setActiveResultIndex(tabId: string, idx: number) {
    if (tabResults.value[tabId] && idx >= 0 && idx < tabResults.value[tabId].length) {
      activeResultIndex.value[tabId] = idx
    }
  }

  /** 关闭某个子结果（按索引） */
  function closeResultAt(tabId: string, idx: number) {
    const arr = tabResults.value[tabId]
    if (!arr || idx < 0 || idx >= arr.length) return
    arr.splice(idx, 1)
    const cur = activeResultIndex.value[tabId] ?? 0
    if (arr.length === 0) {
      delete activeResultIndex.value[tabId]
    } else if (cur >= arr.length) {
      activeResultIndex.value[tabId] = arr.length - 1
    } else if (idx < cur) {
      activeResultIndex.value[tabId] = cur - 1
    }
  }

  /** 更新指定子结果（翻页时替换该子 tab 的 result，保留 sql） */
  function updateResultAt(tabId: string, idx: number, entry: ScriptResultEntry) {
    const arr = tabResults.value[tabId]
    if (arr && idx >= 0 && idx < arr.length) {
      arr[idx] = entry
    }
  }

  /** 仅更新指定子结果的总行数（就地覆盖 result.total，不动其他字段） */
  function updateResultTotal(tabId: string, idx: number, total: number) {
    const arr = tabResults.value[tabId]
    const result = arr?.[idx]?.result
    if (result) {
      result.total = total
    }
  }

  /** 清理 Tab 的所有结果（关闭 Tab 时调用） */
  function clearTabResults(tabId: string) {
    delete tabResults.value[tabId]
    delete activeResultIndex.value[tabId]
  }

  /** 设置 Tab 的文件路径（同步更新标题为文件名） */
  function setTabFilePath(tabId: string, filePath: string) {
    const tab = tabs.value.find(t => t.id === tabId)
    if (tab) {
      tab.filePath = filePath
      tab.dirty = false
      const fileName = filePath.split(/[\\/]/).pop()
      if (fileName && tab.type === 'sql') tab.title = fileName
    }
  }

  /** 指定连接是否仍有打开的 Tab（表数据/表结构/查询等），用于连接树图标状态：
   * 该连接的所有相关 Tab 全部关闭后返回 false，连接图标回到未连接状态 */
  function hasTabsForConfig(configId?: string): boolean {
    if (!configId) return false
    return tabs.value.some(t => t.configId === configId)
  }

  /** 指定表是否有打开的 Tab（表数据/表结构），用于表节点图标状态：
   * 打开该表的视图后表名变绿，关闭该表全部相关 Tab 后变灰 */
  function hasTabForTable(configId?: string, database?: string, table?: string): boolean {
    if (!configId || !database || !table) return false
    return tabs.value.some(
      t =>
        (t.type === 'table' || t.type === 'metadata') &&
        t.configId === configId &&
        t.database === database &&
        t.table === table
    )
  }

  return {
    tabs,
    activeTabId,
    activeTab,
    activeTabResult,
    tabResults,
    activeResultIndex,
    openTab,
    closeTab,
    closeAllTabs,
    updateTabContent,
    recordExecution,
    setActiveResultIndex,
    closeResultAt,
    updateResultAt,
    updateResultTotal,
    clearTabResults,
    setTabFilePath,
    hasTabsForConfig,
    hasTabForTable,
  }
})
