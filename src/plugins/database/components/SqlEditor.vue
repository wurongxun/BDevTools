<template>
  <div class="sql-editor" ref="editorContainer">
    <div class="editor-toolbar">
      <el-button size="small" type="primary" @click="handleExecute" :disabled="!configId">
        <el-icon><CaretRight /></el-icon> 执行 (Ctrl+Enter)
      </el-button>
      <el-select v-model="selectedDb" size="small" placeholder="选择数据库" style="width: 160px; margin-left: 8px;">
        <el-option v-for="db in databases" :key="db.name" :label="db.name" :value="db.name" />
      </el-select>
      <span class="execution-info" v-if="lastResult">
        {{ lastResult.is_query ? `${lastResult.rows.length} 行` : `${lastResult.affected_rows} 行受影响` }}
        | {{ lastResult.execution_time_ms }} ms
      </span>
    </div>
    <div class="editor-wrapper" ref="monacoContainer"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import { CaretRight } from '@element-plus/icons-vue'
import { dbExecuteSql } from '../api/operations'
import { dbListDatabases } from '../api/metadata'
import type { SqlResult, DatabaseInfo } from '../types'
import * as monaco from 'monaco-editor'

const props = defineProps<{
  configId?: string
  initialContent?: string
}>()

const emit = defineEmits<{
  'executed': [result: SqlResult]
}>()

const editorContainer = ref<HTMLElement>()
const monacoContainer = ref<HTMLElement>()
const selectedDb = ref('')
const databases = ref<DatabaseInfo[]>([])
const lastResult = ref<SqlResult | null>(null)
let editor: monaco.editor.IStandaloneCodeEditor | null = null

onMounted(async () => {
  await nextTick()
  initEditor()
  if (props.configId) {
    await loadDatabases()
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
}

async function loadDatabases() {
  if (!props.configId) return
  try {
    const res = await dbListDatabases(props.configId)
    if (res.code === 200) {
      databases.value = res.data
      if (res.data.length > 0 && !selectedDb.value) {
        selectedDb.value = res.data[0].name
      }
    }
  } catch (e) {
    console.error('加载数据库列表失败', e)
  }
}

watch(() => props.configId, () => {
  if (props.configId) {
    loadDatabases()
  }
})

async function handleExecute() {
  if (!props.configId || !editor) return
  const sql = editor.getValue().trim()
  if (!sql) return

  try {
    const res = await dbExecuteSql(props.configId, sql)
    if (res.code === 200) {
      lastResult.value = res.data
      emit('executed', res.data)
    } else {
      lastResult.value = { columns: [], rows: [], affected_rows: 0, execution_time_ms: 0, is_query: false }
      emit('executed', { columns: ['Error'], rows: [[res.msg]], affected_rows: 0, execution_time_ms: 0, is_query: false })
    }
  } catch (e: any) {
    emit('executed', { columns: ['Error'], rows: [[e.message || String(e)]], affected_rows: 0, execution_time_ms: 0, is_query: false })
  }
}

function getContent(): string {
  return editor?.getValue() || ''
}

function setContent(content: string) {
  editor?.setValue(content)
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
  padding: 6px 8px;
  background: #252526;
  border-bottom: 1px solid #3c3c3c;
}
.execution-info {
  margin-left: auto;
  font-size: 12px;
  color: #888;
}
.editor-wrapper {
  flex: 1;
  overflow: hidden;
}
</style>
