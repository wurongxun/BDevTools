<template>
  <div class="sql-tab">
    <div class="sql-editor-wrap">
      <SqlEditor
        :config-id="tab.configId"
        :database="tab.database"
        :initial-content="tab.content"
        :saved-file-path="tab.filePath"
        @executed="onSqlExecuted"
        @saved="onSqlSaved"
      />
    </div>
    <div
      class="resize-handle"
      :class="{ dragging: isDragging }"
      @mousedown.prevent="onResizeStart"
    ></div>
    <div
      ref="resultPanelRef"
      class="sql-result-wrap"
      :style="{ height: resultHeight + 'px', transition: isDragging ? 'none' : '' }"
    >
      <!-- ============ 多语句执行结果：子结果 Tab 栏（由 SqlTab 持有） ============ -->
      <TabBarContainer v-if="entries.length > 0" ref="barRef" aria-label="SQL 执行结果">
        <TabBarItem
          v-for="(entry, idx) in entries"
          :key="idx"
          :active="activeIdx === idx"
          :title="entry.sql"
          @click="setActive(idx)"
          @close="closeSubResult(idx)"
        >
          <el-icon class="tab-icon"><Document /></el-icon>
          <span class="tab-text">{{ summarizeSql(entry.sql) }}</span>
          <TabCloseButton title="关闭此结果" @close="closeSubResult(idx)" />
        </TabBarItem>

        <!-- 子结果多到横向溢出时，滚动按钮挨个找很低效；这里给出跳转 / 单关 / 全关的入口 -->
        <template #append>
          <TabBarOverflowMenu
            :items="resultMenuItems"
            :active-id="String(activeIdx)"
            button-title="显示所有执行结果"
            header-label="执行结果"
            @select="(id) => setActive(Number(id))"
            @close="(id) => closeSubResult(Number(id))"
            @close-all="closeAllResults"
          />
        </template>
      </TabBarContainer>

      <!-- ============ 当前激活子结果：交给 QueryResultView 执行并展示 ============ -->
      <!-- KeepAlive 按 entry.id 缓存实例：切走再切回直接恢复，**不再重跑一遍 SQL**
           （原来 :key="activeIdx" 每次切换都换 key ⇒ 组件重建 ⇒ immediate watch 触发 ⇒ 重新查询）。
           两个细节：
           ① key 必须用**稳定的 entry.id**，不能用下标 —— 关掉中间某个结果会让下标平移，
              旧下标的缓存实例会被错当成新条目复用，显示张冠李戴的结果；
           ② max 限住缓存数量：结果大多上千行，不设上限会一直堆在内存里。 -->
      <div v-if="activeEntry" class="result-body">
        <KeepAlive :max="SUB_RESULT_CACHE_MAX">
          <QueryResultView
            :key="activeEntry.id"
            :config-id="tab.configId"
            :database="tab.database"
            :sql="activeEntry.sql"
            :initial-result="activeEntry.result"
            :table="tab.table"
          />
        </KeepAlive>
      </div>
      <div v-else class="no-result">
        <el-empty description="执行 SQL 查看结果" :image-size="60" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { Document } from '@element-plus/icons-vue'
import { TabBarContainer, TabBarItem, TabBarOverflowMenu, TabCloseButton } from '@/components/TabBar'
import { useEditorStore, useScriptsStore } from '../store'
import SqlEditor from './SqlEditor.vue'
import QueryResultView from './QueryResultView.vue'
import type { EditorTab, SqlExecuteMode, SqlResult } from '../types'

/**
 * SQL 编辑器 Tab 内容
 * 由 MainPanel 的 KeepAlive 按 tab.id 缓存实例：切走/切回时
 * 编辑内容、查询结果、结果面板高度均保持，无需重新执行。
 *
 * 多语句执行产生的「子结果」Tab 栏（TabBarContainer）现由本组件持有，
 * 每个子结果把其 SQL 传给 QueryResultView 去执行并展示。
 */
const props = defineProps<{
  tab: EditorTab
}>()

/** 子结果实例缓存上限（KeepAlive max）。结果常上千行，不能无限堆积 */
const SUB_RESULT_CACHE_MAX = 8

const editorStore = useEditorStore()
const scriptsStore = useScriptsStore()

// ==================== 多语句子结果 Tab 栏 ====================
const entries = computed(() => editorStore.tabResults[props.tab.id] ?? [])
const activeIdx = computed(() => editorStore.activeResultIndex[props.tab.id] ?? 0)
const activeEntry = computed(() => {
  const arr = entries.value
  const i = activeIdx.value
  return arr.length > 0 && i < arr.length ? arr[i] : null
})

const barRef = ref<InstanceType<typeof TabBarContainer> | null>(null)

/** 把当前激活的子结果标签滚进横向可视区（"跳转"到那个标签时不必手动找） */
function scrollActiveIntoView() {
  const list = barRef.value?.tabListRef
  const el = list?.querySelector('.tab-bar-item-active') as HTMLElement | null
  el?.scrollIntoView({ inline: 'nearest', block: 'nearest' })
}

// 激活项从别处变化时（例如编辑器里再次执行某条语句 ⇒ store 直接把子结果切过去）
// 也要滚动到可见。`nextTick` 等 TabBar 先把 active class 迁到新标签上。
watch(activeIdx, () => nextTick(scrollActiveIntoView))

// 激活索引变化后，若其超出横向可视区则自动滚动到可见
function setActive(idx: number) {
  editorStore.setActiveResultIndex(props.tab.id, idx)
  nextTick(scrollActiveIntoView)
}

function closeSubResult(idx: number) {
  editorStore.closeResultAt(props.tab.id, idx)
}

/** 把 SQL 摘要为短标签（去除换行/多余空格，超过 30 字符省略） */
function summarizeSql(sql: string): string {
  if (!sql) return '(空)'
  const oneLine = sql.replace(/\s+/g, ' ').trim()
  return oneLine.length > 30 ? oneLine.slice(0, 30) + '…' : oneLine
}

/** 收纳菜单的条目：id 用**下标字符串**（store 的下层 API 以索引定位），提示给完整 SQL */
const resultMenuItems = computed(() =>
  entries.value.map((entry, idx) => ({
    id: String(idx),
    label: summarizeSql(entry.sql),
    title: entry.sql,
  }))
)

/**
 * 关闭本 tab 的全部子结果（≒ 清掉 result 面板）。
 * 不弹确认：结果是可重建的（重跑一遍就有），不像主 tab 里可能有没保存的 SQL。
 */
function closeAllResults() {
  editorStore.clearTabResults(props.tab.id)
}

// ==================== 垂直 resize（编辑器 / 结果面板） ====================
const RESULT_HEIGHT_DEFAULT = 250
const RESULT_HEIGHT_MIN = 100

const resultPanelRef = ref<HTMLElement>()
const resultHeight = ref(RESULT_HEIGHT_DEFAULT)
const isDragging = ref(false)

function onResizeStart(e: MouseEvent) {
  const sqlTab = resultPanelRef.value?.parentElement
  if (!sqlTab) return
  const tabRect = sqlTab.getBoundingClientRect()
  const startY = e.clientY
  const startH = resultHeight.value
  isDragging.value = true
  document.body.style.cursor = 'ns-resize'
  document.body.style.userSelect = 'none'

  const resultEl = resultPanelRef.value!

  const onMouseMove = (moveEvent: MouseEvent) => {
    const newH = Math.max(
      RESULT_HEIGHT_MIN,
      Math.min(tabRect.height - 150, startH - (moveEvent.clientY - startY))
    )
    resultEl.style.height = newH + 'px'
  }
  const onMouseUp = () => {
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    // 读取 DOM 最终值，交给 Vue 接管
    resultHeight.value = parseInt(resultEl.style.height) || RESULT_HEIGHT_DEFAULT
    isDragging.value = false
  }
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}

/**
 * 编辑器执行完一条语句：把结果落到子结果标签上。
 *
 * 落位由 store 完成（`recordExecution`）：`reuse` 会在已有同语句的标签里就地刷新，
 * 不再堆一个同名标签 —— 反复跑同一条语句原本会让标签栏越跑越长。
 */
function onSqlExecuted(entry: { sql: string; result: SqlResult; mode: SqlExecuteMode }) {
  if (!props.tab.id || !entry) return
  editorStore.recordExecution(props.tab.id, { sql: entry.sql, result: entry.result }, entry.mode)
}

async function onSqlSaved(filePath: string, configId: string, database: string) {
  if (props.tab.id) {
    editorStore.setTabFilePath(props.tab.id, filePath)
    // 保存时自动落绑定：该文件默认对应的连接与数据库
    await scriptsStore.bindFile(filePath, configId, database)
    await scriptsStore.refresh()
  }
}
</script>

<style scoped>
.sql-tab {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.sql-editor-wrap {
  flex: 1;
  min-height: 150px;
  overflow: hidden;
}
.resize-handle {
  height: var(--resize-handle-h-line-height);
  background: transparent;
  cursor: ns-resize;
  flex-shrink: 0;
  position: relative;
}
/* 扩大命中热区（透明、不占布局）：1px 的线难以精确点中，向上下各扩 hit-area */
.resize-handle::before {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  top: calc(-1 * var(--resize-handle-hit-area));
  bottom: calc(-1 * var(--resize-handle-hit-area));
}
/* 视觉线：绝对定位居中，常态 1px，悬停/拖拽中增粗到 active 高度 */
.resize-handle::after {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  top: 50%;
  height: var(--resize-handle-h-line-height);
  transform: translateY(-50%);
  background: var(--resize-handle-color);
  transition:
    height 0.15s ease,
    background-color 0.15s ease;
}
.resize-handle:hover::after,
.resize-handle.dragging::after {
  height: var(--resize-handle-h-line-height-active);
  background: var(--resize-handle-color-active);
}
.sql-result-wrap {
  flex-shrink: 0;
  overflow: hidden;
  border-top: 1px solid var(--el-border-color);
  display: flex;
  flex-direction: column;
}
.result-body {
  flex: 1;
  min-height: 0;
}
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
.no-result {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
