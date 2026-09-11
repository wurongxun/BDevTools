/**
 * SQL 编辑器 Tab 状态管理
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { EditorTab, SqlResult } from '../types'

let tabCounter = 0

/** 脚本 tab 单条执行结果项（含执行语句 + 结果） */
export interface ScriptResultEntry {
  sql: string
  result: SqlResult | null
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

  /** 打开新 Tab */
  function openTab(tab: Partial<EditorTab>) {
    const id = tab.id || `tab_${++tabCounter}`
    const newTab: EditorTab = {
      id,
      title: tab.title || '新查询',
      type: tab.type || 'sql',
      configId: tab.configId,
      database: tab.database,
      table: tab.table,
      content: tab.content || '',
      dirty: false,
      filePath: tab.filePath,
    }
    // 检查是否已存在相同的 Tab：sql 文件按路径去重，其余按类型+连接+库+表去重
    const existing = tabs.value.find(t => {
      if (t.type === 'sql' && t.filePath) {
        return newTab.type === 'sql' && t.filePath === newTab.filePath
      }
      return (
        t.type === newTab.type &&
        t.configId === newTab.configId &&
        t.database === newTab.database &&
        t.table === newTab.table
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

  /** 更新 Tab 内容 */
  function updateTabContent(tabId: string, content: string) {
    const tab = tabs.value.find(t => t.id === tabId)
    if (tab) {
      tab.content = content
      tab.dirty = true
    }
  }

  /** 追加一条执行结果（多语句分次调用，自动激活最新子结果） */
  function appendTabResult(tabId: string, entry: ScriptResultEntry) {
    if (!tabResults.value[tabId]) tabResults.value[tabId] = []
    tabResults.value[tabId].push(entry)
    activeResultIndex.value[tabId] = tabResults.value[tabId].length - 1
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
    updateTabContent,
    appendTabResult,
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
