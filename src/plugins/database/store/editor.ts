/**
 * SQL 编辑器 Tab 状态管理
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { EditorTab, SqlResult } from '../types'

let tabCounter = 0

export const useEditorStore = defineStore('dbEditor', () => {
  /** 打开的 Tab 列表 */
  const tabs = ref<EditorTab[]>([])
  /** 当前激活的 Tab ID */
  const activeTabId = ref<string>('')
  /** 每个 Tab 的 SQL 执行结果（按 tabId 存储） */
  const tabResults = ref<Record<string, SqlResult | null>>({})

  /** 当前激活的 Tab */
  const activeTab = computed(() =>
    tabs.value.find(t => t.id === activeTabId.value)
  )

  /** 当前激活 Tab 的执行结果 */
  const activeTabResult = computed(() =>
    tabResults.value[activeTabId.value] ?? null
  )

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
    }
    // 检查是否已存在相同的 Tab
    const existing = tabs.value.find(
      t => t.type === newTab.type && t.configId === newTab.configId && t.database === newTab.database && t.table === newTab.table
    )
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
    delete tabResults.value[tabId]
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

  /** 设置 Tab 的执行结果 */
  function setTabResult(tabId: string, result: SqlResult | null) {
    tabResults.value[tabId] = result
  }

  /** 设置 Tab 的文件路径 */
  function setTabFilePath(tabId: string, filePath: string) {
    const tab = tabs.value.find(t => t.id === tabId)
    if (tab) {
      tab.filePath = filePath
      tab.dirty = false
    }
  }

  return {
    tabs,
    activeTabId,
    activeTab,
    activeTabResult,
    tabResults,
    openTab,
    closeTab,
    updateTabContent,
    setTabResult,
    setTabFilePath,
  }
})
