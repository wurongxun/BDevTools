<template>
  <div class="sql-editor" ref="editorContainer">
    <div class="editor-toolbar">
      <el-button size="small" type="primary" @click="handleExecute" :disabled="!selectedConfigId">
        <el-icon><CaretRight /></el-icon> 执行 (Ctrl+Enter)
      </el-button>
      <el-button size="small" @click="handleSave">
        <el-icon><FolderOpened /></el-icon> 保存 (Ctrl+S)
      </el-button>
      <el-select v-model="selectedConfigId" size="small" placeholder="选择连接" style="width: 140px; margin-left: 8px;" @change="handleConnectionChange">
        <el-option v-for="conn in connections" :key="conn.id" :label="conn.name" :value="conn.id" />
      </el-select>
      <el-select v-model="selectedDb" size="small" placeholder="选择数据库" style="width: 140px; margin-left: 8px;" :disabled="!selectedConfigId">
        <el-option v-for="db in databases" :key="db.name" :label="db.name" :value="db.name" />
      </el-select>
      <el-select v-model="pageSize" size="small" style="width: 108px; margin-left: 8px;" @change="handlePageSizeChange">
        <el-option v-for="s in PAGE_SIZE_OPTIONS" :key="s" :label="`${s} 行/页`" :value="s" />
      </el-select>
      <span class="execution-info" v-if="lastResult">
        {{ lastResult.is_query ? `${lastResult.rows.length} 行` : `${lastResult.affected_rows} 行受影响` }}
        | {{ lastResult.execution_time_ms }} ms
        <template v-if="lastResult.is_query"> | 第 {{ currentPage }} 页</template>
      </span>
      <el-button-group v-if="lastResult && lastResult.is_query" class="pager">
        <el-button size="small" :disabled="currentPage <= 1 || executing" @click="goPrevPage">
          <el-icon><ArrowLeft /></el-icon> 上一页
        </el-button>
        <el-button size="small" :disabled="!canGoNext || executing" @click="goNextPage">
          {{ prefetching ? '加载中' : '下一页' }} <el-icon><ArrowRight /></el-icon>
        </el-button>
      </el-button-group>
    </div>
    <div class="editor-wrapper" ref="monacoContainer"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import { CaretRight, FolderOpened, ArrowLeft, ArrowRight } from '@element-plus/icons-vue'
import { dbExecuteSql } from '../api/operations'
import { dbListDatabases } from '../api/metadata'
import { useConnectionStore } from '../store'
import { save } from '@/core/runtime'
import { writeTextFile } from '@/core/runtime'
import { usePluginDir } from '@/composables/usePluginDir'
import type { SqlResult, DatabaseInfo } from '../types'
import * as monaco from 'monaco-editor'

const props = defineProps<{
  configId?: string
  database?: string
  initialContent?: string
  savedFilePath?: string
}>()

const emit = defineEmits<{
  'executed': [result: SqlResult]
  'saved': [filePath: string, configId: string, database: string]
}>()

const connectionStore = useConnectionStore()
const pluginDir = usePluginDir('database')
const editorContainer = ref<HTMLElement>()
const monacoContainer = ref<HTMLElement>()
const selectedConfigId = ref<string>(props.configId || '')
const selectedDb = ref('')
const databases = ref<DatabaseInfo[]>([])
const lastResult = ref<SqlResult | null>(null)
// 分页：默认 200 行/页（前后端一致），后端单次最大 1000 行
const PAGE_SIZE_OPTIONS = [50, 100, 200, 500, 1000]
const pageSize = ref(200)
const currentPage = ref(1)
const executing = ref(false)
// 下一页预取缓存：当前页加载完成后后台预取，翻页时即时展示
const nextPageCache = ref<SqlResult | null>(null)
const prefetching = ref(false)
// 预取时记录的查询快照，SQL/连接/库/行数变化后缓存作废
let prefetchSnapshot = ''
let editor: monaco.editor.IStandaloneCodeEditor | null = null

// 从 store 获取连接列表
const connections = connectionStore.connections

onMounted(async () => {
  await nextTick()
  initEditor()
  // 确保连接列表已加载
  if (connectionStore.connections.length === 0) {
    await connectionStore.loadConnections()
  }
  if (selectedConfigId.value) {
    await loadDatabases(selectedConfigId.value)
  }
})

onBeforeUnmount(() => {
  editor?.dispose()
})

function initEditor() {
  if (!monacoContainer.value) return
  editor = monaco.editor.create(monacoContainer.value, {
    value: props.initialContent || '-- 输入 SQL 语句\n',
    language: 'sql',
    theme: 'vs-dark',
    minimap: { enabled: false },
    fontSize: 14,
    lineNumbers: 'on',
    scrollBeyondLastLine: false,
    automaticLayout: true,
    tabSize: 2,
    wordWrap: 'on',
  })

  // Ctrl+Enter 执行
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
    handleExecute()
  })

  // Ctrl+S 保存
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
    handleSave()
  })
}

async function loadDatabases(configId: string) {
  if (!configId) return
  try {
    // 确保已连接到该数据库
    if (!connectionStore.isConnected(configId)) {
      const connectRes = await connectionStore.connect(configId)
      if (connectRes.code !== 200) {
        console.error('连接失败:', connectRes.msg)
        return
      }
    }
    const res = await dbListDatabases(configId)
    if (res.code === 200) {
      databases.value = res.data
      // 优先使用文件绑定的数据库，其次回退到第一个库
      const bound = props.database ? res.data.find(d => d.name === props.database) : undefined
      if (bound) {
        selectedDb.value = bound.name
      } else if (res.data.length > 0) {
        selectedDb.value = res.data[0].name
      } else {
        selectedDb.value = ''
      }
    }
  } catch (e) {
    console.error('加载数据库列表失败', e)
  }
}

// 切换连接时重新加载数据库列表
async function handleConnectionChange(configId: string) {
  selectedDb.value = ''
  databases.value = []
  if (configId) {
    await loadDatabases(configId)
  }
}

function querySnapshot(): string {
  return `${selectedConfigId.value}|${selectedDb.value}|${pageSize.value}|${editor?.getValue().trim() || ''}`
}

function invalidatePageCache() {
  nextPageCache.value = null
  prefetching.value = false
  prefetchSnapshot = ''
}

async function handleExecute() {
  currentPage.value = 1
  await executePage(1)
}

async function executePage(page: number) {
  if (!selectedConfigId.value || !editor || executing.value) return
  const sql = editor.getValue().trim()
  if (!sql) return

  executing.value = true
  invalidatePageCache()
  const snapshot = querySnapshot()
  try {
    const db = selectedDb.value || undefined
    const offset = (page - 1) * pageSize.value
    console.log('[SqlEditor] SQL :', selectedConfigId.value, sql, db, `页=${page} 行数=${pageSize.value}`)
    const res = await dbExecuteSql(selectedConfigId.value, sql, db, pageSize.value, offset)
    if (res.code === 200) {
      currentPage.value = page
      lastResult.value = res.data
      emit('executed', res.data)
      // 还有下一页：后台预取，保证翻页即时展示
      if (res.data.is_query && res.data.has_more) {
        prefetchNext(page + 1, snapshot)
      }
    } else {
      console.error('[SqlEditor] SQL 执行失败:', res.msg)
      lastResult.value = { columns: [], rows: [], affected_rows: 0, execution_time_ms: 0, is_query: false }
      emit('executed', { columns: ['Error'], rows: [[res.msg]], affected_rows: 0, execution_time_ms: 0, is_query: false })
    }
  } catch (e: any) {
    console.error('[SqlEditor] SQL 执行异常:', e)
    emit('executed', { columns: ['Error'], rows: [[e.message || String(e)]], affected_rows: 0, execution_time_ms: 0, is_query: false })
  } finally {
    executing.value = false
  }
}

/** 后台预取指定页（静默失败，不影响当前页） */
async function prefetchNext(page: number, snapshot: string) {
  if (prefetching.value || !selectedConfigId.value || !editor) return
  prefetching.value = true
  prefetchSnapshot = snapshot
  try {
    const db = selectedDb.value || undefined
    const offset = (page - 1) * pageSize.value
    const res = await dbExecuteSql(selectedConfigId.value, editor.getValue().trim(), db, pageSize.value, offset)
    // 预取期间查询条件未变才保留缓存，否则作废（用户改了 SQL/连接/行数）
    if (res.code === 200 && prefetchSnapshot === snapshot && querySnapshot() === snapshot) {
      nextPageCache.value = res.data
    }
  } catch (e) {
    console.warn('[SqlEditor] 预取下一页失败:', e)
  } finally {
    prefetching.value = false
  }
}

const canGoNext = computed(() => {
  return !!lastResult.value?.has_more || nextPageCache.value !== null
})

function goNextPage() {
  if (nextPageCache.value) {
    // 命中预取缓存：即时展示，并继续预取下下页
    currentPage.value += 1
    lastResult.value = nextPageCache.value
    nextPageCache.value = null
    emit('executed', lastResult.value)
    if (lastResult.value.has_more) {
      prefetchNext(currentPage.value + 1, querySnapshot())
    }
  } else {
    executePage(currentPage.value + 1)
  }
}

function goPrevPage() {
  if (currentPage.value > 1) {
    executePage(currentPage.value - 1)
  }
}

/** 行数变化后重新查询（以前端设定为准），从第一页开始 */
function handlePageSizeChange() {
  invalidatePageCache()
  if (lastResult.value && lastResult.value.is_query) {
    currentPage.value = 1
    executePage(1)
  }
}

function getContent(): string {
  return editor?.getValue() || ''
}

function setContent(content: string) {
  editor?.setValue(content)
}

async function handleSave() {
  if (!editor) return
  const content = editor.getValue()
  try {
    let filePath = props.savedFilePath
    if (!filePath) {
      // 获取插件工作目录作为默认保存路径
      const scriptsDir = await pluginDir.getSubDir('scripts')
      const defaultPath = scriptsDir ? `${scriptsDir}/query.sql` : 'query.sql'
      // 弹出保存对话框
      const selected = await save({
        filters: [{ name: 'SQL 文件', extensions: ['sql'] }],
        defaultPath,
      })
      if (!selected) return
      filePath = selected
    }
    await writeTextFile(filePath, content)
    console.log('[SqlEditor] SQL 已保存至:', filePath)
    emit('saved', filePath, selectedConfigId.value, selectedDb.value)
  } catch (e: any) {
    console.error('[SqlEditor] 保存失败:', e)
  }
}

defineExpose({ getContent, setContent })
</script>

<style scoped>
.sql-editor {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #1e1e1e;
}
.editor-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  row-gap: 4px;
  padding: 6px 8px;
  background: #252526;
  border-bottom: 1px solid #3c3c3c;
}
.execution-info {
  margin-left: auto;
  font-size: var(--font-size-base);
  color: #888;
}
.pager {
  margin-left: 8px;
}
.editor-wrapper {
  flex: 1;
  overflow: hidden;
}
</style>
