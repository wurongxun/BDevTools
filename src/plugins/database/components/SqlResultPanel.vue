<template>
  <div class="sql-result-panel">
    <!-- 顶部子结果 Tab 栏（复用全局标准 TabBar：Chrome 风格，溢出可横向滚动） -->
    <TabBarContainer
      v-if="results.length > 0"
      ref="barRef"
      aria-label="SQL 执行结果"
    >
      <TabBarItem
        v-for="(entry, idx) in results"
        :key="idx"
        :active="Number(activeIdx) === idx"
        :title="entry.sql"
        @click="handleTabChange(idx)"
        @close="closeSubResult(idx)"
      >
        <el-icon class="tab-icon"><Document /></el-icon>
        <span class="tab-text">{{ summarizeSql(entry.sql) }}</span>
        <TabCloseButton
          v-if="results.length > 1"
          title="关闭此结果"
          @close="closeSubResult(idx)"
        />
      </TabBarItem>
    </TabBarContainer>

    <!-- 结果表（复用公共只读结果表格组件） -->
    <ResultTable
      v-if="activeEntry && activeEntry.result && activeEntry.result.columns.length > 0"
      :columns="activeEntry.result.columns"
      :rows="tableData"
      :loading="pageLoading"
      :aria-busy="pageLoading"
    />

    <!-- 空结果占位 -->
    <div class="no-result" v-else-if="results.length === 0">
      <el-empty description="执行 SQL 查看结果" :image-size="60" />
    </div>
    <div class="no-result" v-else>
      <el-empty description="此语句无结果数据" :image-size="50" />
    </div>

    <!-- 底部状态 + 分页（复用公共结果 footer，每个子结果独立分页，默认 200/页） -->
    <ResultFooter
      v-if="activeEntry && activeEntry.result"
      :page="activePageState.page"
      :page-size="activePageState.pageSize"
      :show-pager="activeEntry.result.is_query"
      :has-more="!!activeEntry.result.has_more"
      :loading="pageLoading"
      :export-sql="activeEntry.result.is_query ? activeEntry.sql : ''"
      :config-id="tab?.configId"
      :total="activeEntry.result.total || 0"
      @prev="goPrev"
      @next="goNext"
      @page-size-change="handlePageSizeChange"
      @refresh="handleRefresh"
      @totalQuantity="handleTotalQuantity"
    >
      <template #info>
        <el-icon class="info-icon"><DataAnalysis /></el-icon>
        <template v-if="activeEntry.result.is_query">
          <span class="stat-rows" :title="hasTotal ? `共 ${activeEntry.result.total} 行` : ''">共 {{ totalRowsText }} 行</span>
          <span class="sep">·</span>
          <span>第 {{ activePageState.page }} 页 / 当前 {{ activeEntry.result.rows.length }} 行</span>
          <span class="sep">·</span>
          <span>耗时 {{ activeEntry.result.execution_time_ms }} ms</span>
        </template>
        <template v-else>
          <span class="stat-rows">共 {{ (activeEntry.result.affected_rows ?? 0).toLocaleString() }} 行</span>
          <span class="sep">·</span>
          <span>耗时 {{ activeEntry.result.execution_time_ms }} ms</span>
        </template>
      </template>
    </ResultFooter>
    <!-- 屏幕阅读器状态播报（WCAG 4.1.3 状态消息） -->
    <span class="sr-only" role="status" aria-live="polite">{{ pageLoading ? '正在加载数据，请稍候' : '' }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { DataAnalysis, Document } from '@element-plus/icons-vue'
import { TabBarContainer, TabBarItem, TabCloseButton } from '@/components/TabBar'
import ResultTable from './ResultTable.vue'
import ResultFooter from './ResultFooter.vue'
import { useEditorStore } from '../store'
import { dbExecuteSql } from '../api/operations'
import type { ScriptResultEntry } from '../store/editor'

/**
 * 多结果子 Tab 结果面板
 * 通过 store.tabResults[tabId] 数组展示一个 SQL tab 的多次执行结果，每个子 tab 对应一条 SQL；
 * 分页按子结果独立管理（切换子 tab 保留各自页码），翻页用该子 tab 的 SQL 重新查询
 */
const props = defineProps<{
  tabId: string
}>()

const editorStore = useEditorStore()

/** 默认每页行数 */
const DEFAULT_PAGE_SIZE = 200
const total = ref(0)
/** 把 SQL 摘要为短标签（去除换行/多余空格，超过 30 字符省略） */
function summarizeSql(sql: string): string {
  if (!sql) return '(空)'
  const oneLine = sql.replace(/\s+/g, ' ').trim()
  return oneLine.length > 30 ? oneLine.slice(0, 30) + '…' : oneLine
}

// 防御：兼容 store 中可能残留的旧单结果格式（非数组视为空）
const results = computed<ScriptResultEntry[]>(() => {
  const v = editorStore.tabResults[props.tabId]
  return Array.isArray(v) ? v as ScriptResultEntry[] : []
})

/** 当前 Tab（拿 configId/database 供翻页查询） */
const tab = computed(() => editorStore.tabs.find(t => t.id === props.tabId))



/** 当前激活的子结果索引（本地副本，store 中也存一份用于持久化） */
const activeIdx = ref('0')

/** 标准 TabBar 容器实例（暴露 tabListRef，用于把激活 tab 滚动到可见区域） */
const barRef = ref<InstanceType<typeof TabBarContainer> | null>(null)

// 激活索引变化（点击/执行新结果）后，若其超出横向可视区则自动滚动到可见
watch(activeIdx, () => {
  nextTick(() => {
    const list = barRef.value?.tabListRef
    const el = list?.querySelector('.tab-bar-item-active') as HTMLElement | null
    el?.scrollIntoView({ inline: 'nearest', block: 'nearest' })
  })
})

// 同步 store 中的活跃索引到本地
// 注意：监听 length（appendTabResult 是 push 到已有数组，引用不变，监听数组引用不会触发）
watch(() => results.value.length, () => {
  const arr = results.value
  if (arr.length === 0) return
  const storeIdx = editorStore.activeResultIndex[props.tabId] ?? 0
  const safe = String(Math.min(storeIdx, arr.length - 1))
  if (safe !== activeIdx.value) activeIdx.value = safe
}, { immediate: true })

function handleTabChange(idx: number) {
  // 同时更新本地激活索引（TabBarItem 无 v-model，点击/键盘选中都走这里）与 store 持久化
  activeIdx.value = String(idx)
  editorStore.setActiveResultIndex(props.tabId, idx)
}

function closeSubResult(idx: number) {
  editorStore.closeResultAt(props.tabId, idx)
}

const activeEntry = computed(() => {
  const idx = Number(activeIdx.value)
  return results.value[idx] ?? null
})

/** 后端是否统计到总行数（仅纯 SELECT 首页返回） */
const hasTotal = computed(() => {
  const r = activeEntry.value?.result
  return r != null && r.total != null
})

/** 总行数文案：后端统计到 total 显示总数；否则显示当前已载入行数 */
const totalRowsText = computed(() => {
  const r = activeEntry.value?.result
  if (!r) return '0'
  const t = r.total ?? null
  return (t != null ? t : r.rows.length).toLocaleString()
})

// ==================== 每个子结果独立分页 ====================
/** 子结果索引 -> 独立分页状态（page/pageSize），切换子 tab 时各自保留 */
const pageStates = reactive<Record<number, { page: number; pageSize: number }>>({})
const pageLoading = ref(false)

function getPageState(idx: number) {
  if (!pageStates[idx]) {
    pageStates[idx] = { page: 1, pageSize: DEFAULT_PAGE_SIZE }
  }
  return pageStates[idx]
}

/** 当前激活子结果的分页状态 */
const activePageState = computed(() => getPageState(Number(activeIdx.value)))

/** 翻页：用当前子 tab 的 SQL 重新查询指定页并更新该子结果 */
async function fetchPage(page: number, pageSize: number) {
  const entry = activeEntry.value
  if (!entry || !tab.value?.configId || pageLoading.value) return
  pageLoading.value = true
  try {
    const res = await dbExecuteSql(
      tab.value.configId,
      entry.sql,
      tab.value.database,
      pageSize,
      (page - 1) * pageSize
    )
    if (res.code === 200) {
      const idx = Number(activeIdx.value)
      getPageState(idx).page = page
      // 翻页时后端不再统计 total（仅首页统计），从旧结果继承总行数
      editorStore.updateResultAt(props.tabId, idx, {
        sql: entry.sql,
        result: { ...res.data, total: entry.result?.total ?? res.data.total },
      })
    } else {
      ElMessage.error('查询失败: ' + res.msg)
    }
  } catch (e: any) {
    ElMessage.error('查询失败: ' + (e.message || e))
  } finally {
    pageLoading.value = false
  }
}

/** 每页大小变化：重置到第一页重新查询 */
function handlePageSizeChange(size: number) {
  const idx = Number(activeIdx.value)
  getPageState(idx).pageSize = size
  getPageState(idx).page = 1
  fetchPage(1, size)
}

function goPrev() {
  const st = activePageState.value
  if (st.page > 1) fetchPage(st.page - 1, st.pageSize)
}

function goNext() {
  if (activeEntry.value?.result?.has_more) {
    const st = activePageState.value
    fetchPage(st.page + 1, st.pageSize)
  }
}

/** 刷新：用当前子 tab 的 SQL 重跑第一页（重新统计 total），并重置到第一页 */
async function handleRefresh() {
  const entry = activeEntry.value
  if (!entry || !tab.value?.configId || pageLoading.value) return
  const idx = Number(activeIdx.value)
  getPageState(idx).page = 1
  await fetchPage(1, activePageState.value.pageSize)
}
/** 获取总行数：用当前子 tab 的 SQL 重新查询并更新该子结果 */
async function handleTotalQuantity() {
  const entry = activeEntry.value
  if (!entry || !entry.result || !tab.value?.configId) return
  const countSql = `SELECT COUNT(*) AS total FROM (${entry.sql}) AS _count`
  const res = await dbExecuteSql(tab.value.configId, countSql, tab.value.database)
  if (res.code === 200) {
    // COUNT(*) 返回一行一列，rows = [[count]]，取第一行第一列
    let total = Number(res.data.rows[0][0])
    editorStore.updateResultTotal(props.tabId, Number(activeIdx.value), total)
  } else {
    ElMessage.error('查询失败: ' + res.msg)
  }
}




/** 前端表格数据：按当前子结果的 columns 映射行 */
const tableData = computed(() => {
  const r = activeEntry.value?.result
  if (!r || !r.is_query) return []
  return r.rows.map(row => {
    const obj: Record<string, any> = {}
    r.columns.forEach((col, idx) => {
      obj[String(idx)] = row[idx]
    })
    return obj
  })
})
</script>

<style scoped>
.sql-result-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
/* tab 外壳（底色/圆角/凹脚/关闭按钮/滚动按钮）由标准 TabBar 组件负责，这里只写内容元素 */
.tab-icon {
  flex: none;
  font-size: var(--font-size-md);
  color: var(--el-text-color-secondary);
}
.tab-text {
  /* SQL 摘要用等宽字体，避免阿里巴巴普惠体 13px 下 _ 等字形不可见 */
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 160px;
}
.result-info {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}
.info-icon {
  font-size: 13px;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}
.stat-rows {
  font-family: var(--font-family-mono);
  white-space: nowrap;
}
.sep {
  color: var(--el-border-color);
  flex-shrink: 0;
}
.no-result {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
  border: 0;
}
</style>
