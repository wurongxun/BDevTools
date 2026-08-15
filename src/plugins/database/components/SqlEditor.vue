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
import { CaretRight, FolderOpened } from '@element-plus/icons-vue'
import { dbExecuteSql } from '../api/operations'
import { dbListDatabases } from '../api/metadata'
import { useConnectionStore } from '../store'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { usePluginDir } from '@/composables/usePluginDir'
import type { SqlResult, DatabaseInfo } from '../types'
import * as monaco from 'monaco-editor'

const props = defineProps<{
  configId?: string
  initialContent?: string
  savedFilePath?: string
}>()

const emit = defineEmits<{
  'executed': [result: SqlResult]
  'saved': [filePath: string]
}>()

const connectionStore = useConnectionStore()
const pluginDir = usePluginDir('database')
const editorContainer = ref<HTMLElement>()
const monacoContainer = ref<HTMLElement>()
const selectedConfigId = ref<string>(props.configId || '')
const selectedDb = ref('')
const databases = ref<DatabaseInfo[]>([])
const lastResult = ref<SqlResult | null>(null)
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
      if (res.data.length > 0) {
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

async function handleExecute() {
  if (!selectedConfigId.value || !editor) return
  const sql = editor.getValue().trim()
  if (!sql) return

  try {
    const db = selectedDb.value || undefined
    console.log('[SqlEditor] SQL :', selectedConfigId.value, sql, db)
    const res = await dbExecuteSql(selectedConfigId.value, sql, db)
    console.log('[SqlEditor] SQL 执行结果:', res.data)
    if (res.code === 200) {
      lastResult.value = res.data
      emit('executed', res.data)
    } else {
      console.error('[SqlEditor] SQL 执行失败:', res.msg)
      lastResult.value = { columns: [], rows: [], affected_rows: 0, execution_time_ms: 0, is_query: false }
      emit('executed', { columns: ['Error'], rows: [[res.msg]], affected_rows: 0, execution_time_ms: 0, is_query: false })
    }
  } catch (e: any) {
    console.error('[SqlEditor] SQL 执行异常:', e)
    emit('executed', { columns: ['Error'], rows: [[e.message || String(e)]], affected_rows: 0, execution_time_ms: 0, is_query: false })
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
    emit('saved', filePath)
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
