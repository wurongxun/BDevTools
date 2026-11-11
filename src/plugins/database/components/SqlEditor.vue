<template>
  <div class="sql-editor" ref="editorContainer">
    <div class="editor-toolbar">
      <app-button
        size="small"
        type="primary"
        title="执行选中的语句（无选区则执行全文）· 右键语句可执行 / 在新标签页中执行"
        @click="handleExecute"
        :disabled="!selectedConfigId"
      >
        <el-icon><CaretRight /></el-icon> 执行 (Ctrl+Enter)
      </app-button>
      <app-button size="small" @click="handleSave">
        <el-icon><FolderOpened /></el-icon> 保存 (Ctrl+S)
      </app-button>
      <el-select v-model="selectedConfigId" size="small" placeholder="选择连接" style="width: 140px; margin-left: 8px;" @change="handleConnectionChange">
        <el-option v-for="conn in connections" :key="conn.id" :label="conn.name" :value="conn.id" />
      </el-select>
      <el-select v-model="selectedDb" size="small" placeholder="选择数据库" style="width: 140px; margin-left: 8px;" :disabled="!selectedConfigId">
        <el-option v-for="db in databases" :key="db.name" :label="db.name" :value="db.name" />
      </el-select>
    </div>
    <div class="editor-wrapper" ref="monacoContainer"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { CaretRight, FolderOpened } from '@element-plus/icons-vue'
import { dbExecuteSql } from '../api/operations'
import { dbListDatabases } from '../api/metadata'
import { useConnectionStore } from '../store'
import { save } from '@/core/runtime'
import { tauri_request } from '@/utils/tauri_request'
import type { ResponseData } from '@/Types/entitys'
import { usePluginDir } from '@/composables/usePluginDir'
import { isDark } from '@/composables/useDark'
import type { SqlExecuteMode, SqlResult, DatabaseInfo } from '../types'
import * as monaco from 'monaco-editor'

const props = defineProps<{
  configId?: string
  database?: string
  initialContent?: string
  savedFilePath?: string
}>()

const emit = defineEmits<{
  /** mode 决定结果落在已有同名子结果标签上，还是另开一个（见 `SqlExecuteMode`） */
  'executed': [entry: { sql: string; result: SqlResult; mode: SqlExecuteMode }]
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
// 分页统一在 QueryResultView 中按当前 SQL 独立管理，编辑器执行时仅取第一页（每页 200）
// 分页大小必须与 QueryResultView.DEFAULT_PAGE_SIZE 一致：本组件执行的结果会经
// SqlTab 作为 initialResult 直接交给 QueryResultView 展示，两边不一致会让后续翻页的
// offset 错位（例如这里取 100 行、那边按 200 的页大小翻页 ⇒ 第二页跳错位置）。
const PAGE_SIZE = 200
const executing = ref(false)
let editor: monaco.editor.IStandaloneCodeEditor | null = null
/** 右键菜单两个执行入口的注册句柄（卸载时 dispose，见 initEditor 里的注释） */
let runActionDisposables: monaco.IDisposable[] | null = null

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
  // 完整释放：editor 实例 + 其持有的 model（不显式 dispose model 会残留在 Monaco 内存中）
  const model = editor?.getModel()
  const disposables = runActionDisposables
  runActionDisposables = null
  disposables?.forEach(d => d.dispose())
  editor?.dispose()
  editor = null
  if (model) model.dispose()
})

function initEditor() {
  if (!monacoContainer.value) return
  editor = monaco.editor.create(monacoContainer.value, {
    value: props.initialContent || '-- 输入 SQL 语句\n',
    language: 'sql',
    theme: isDark.value ? 'vs-dark' : 'vs',
    minimap: { enabled: false },
    fontSize: 14,
    lineNumbers: 'on',
    scrollBeyondLastLine: false,
    automaticLayout: true,
    tabSize: 2,
    wordWrap: 'on',
  })

  // Ctrl+Enter 执行（与右键「执行 SQL 语句」同一语义：复用同语句的子结果标签）
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
    void runSelectedOrAll('reuse')
  })

  // Ctrl+S 保存
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
    handleSave()
  })

  // 右键菜单的两个执行入口。
  // 走 Monaco 自己的上下文菜单（`editor.addAction` + `contextMenuGroupId`）而不是拦 contextmenu
  // 自己弹一个：这样菜单出现在编辑器自己的层级里、位置由 Monaco 算，也就不会和别处的
  // 右键菜单/原生菜单守卫打架。
  // instance 级 `addAction` 是安全的：standalone 实现会给每条 registrations 拼上自身 editorId
  // 作 precondition（`editorId == 'xxx'`），所以多个 SQL tab 同时活着也**不会**互相串，只会显示
  // 被右击的那个编辑器自己的条目。disposable 在卸载时 dispose，避免 destroyed 的实例留菜单项。
  runActionDisposables = [
    editor.addAction({
      id: 'bdevtools.sql.runStatement',
      label: '执行 SQL 语句',
      contextMenuGroupId: 'navigation',
      contextMenuOrder: 1,
      run: () => { void runSelectedOrAll('reuse') },
    }),
    editor.addAction({
      id: 'bdevtools.sql.runStatementInNewTab',
      label: '在新标签页中执行',
      contextMenuGroupId: 'navigation',
      contextMenuOrder: 2,
      run: () => { void runSelectedOrAll('new-tab') },
    }),
  ]
}

// 暗黑模式切换：Monaco 主题跟随全局单例（setTheme 为全局操作，已创建的实例实时生效）
watch(isDark, (dark) => {
  monaco.editor.setTheme(dark ? 'vs-dark' : 'vs')
})

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

async function handleExecute() {
  await runSelectedOrAll('reuse')
}

/** 按 `;` 拆分为多条独立 SQL 语句（简单实现，不处理字符串内的 `;`） */
function splitSqlStatements(sql: string): string[] {
  return sql.split(';').map(s => s.trim()).filter(Boolean)
}

/** 获取当前 Monaco 选区文本；无选区返回空字符串 */
function getSelectedText(): string {
  if (!editor) return ''
  const sel = editor.getSelection()
  if (!sel || sel.isEmpty()) return ''
  return editor.getModel()?.getValueInRange(sel) ?? ''
}

/**
 * 选区优先：选中文本（按 `;` 拆分）→ 否则全文（按 `;` 拆分）→ 逐条串行执行并 emit（每页固定 200）
 *
 * mode 透传给 SqlTab / store：同一条语句重复执行时决定「复用已有子结果标签」还是「另开一个」，
 * 逐语句各自判定 —— 多选了几条跑一次，每条都会落到它自己那 tab 上。
 */
async function runSelectedOrAll(mode: SqlExecuteMode) {
  if (!selectedConfigId.value || !editor || executing.value) return
  const selected = getSelectedText()
  const text = selected || editor.getValue()
  const statements = splitSqlStatements(text)
  if (statements.length === 0) return

  executing.value = true
  try {
    for (const stmt of statements) {
      const db = selectedDb.value || undefined
      try {
        const res = await dbExecuteSql(selectedConfigId.value, stmt, db, PAGE_SIZE, 0)
        if (res.code === 200) {
          lastResult.value = res.data
          // 透传执行的 SQL 原文，便于 SqlTab 存为子结果标签/展示
          emit('executed', { sql: stmt, result: res.data, mode })
        } else {
          const errResult = { columns: ['Error'], rows: [[res.msg]], affected_rows: 0, execution_time_ms: 0, is_query: false }
          emit('executed', { sql: stmt, result: errResult, mode })
        }
      } catch (e: any) {
        const errResult = { columns: ['Error'], rows: [[e.message || String(e)]], affected_rows: 0, execution_time_ms: 0, is_query: false }
        emit('executed', { sql: stmt, result: errResult, mode })
      }
    }
  } finally {
    executing.value = false
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
    // 走后端 write_script_file（tokio 直接写，无 fs scope 权限限制，本地/外部脚本均可保存）
    const res = await tauri_request<ResponseData<string>>('write_script_file', {
      path: filePath,
      content,
    })
    if (res?.code !== 200) {
      console.error('[SqlEditor] 保存失败:', res?.msg)
      ElMessage.error('保存失败: ' + (res?.msg || '未知错误'))
      return
    }
    console.log('[SqlEditor] SQL 已保存至:', filePath)
    ElMessage.success('保存成功')
    emit('saved', filePath, selectedConfigId.value, selectedDb.value)
  } catch (e: any) {
    console.error('[SqlEditor] 保存失败:', e)
    ElMessage.error('保存失败: ' + (e.message || e))
  }
}

defineExpose({ getContent, setContent })
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.sql-editor {
  display: flex;
  flex-direction: column;
  height: 100%;
  // 编辑器面背景跟随主面板（appMainBgColor），保持与下方内容区连体
  @include theme.useTheme {
    background: theme.getVar('appMainBgColor');
  }
}
.editor-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  row-gap: 4px;
  padding: 6px 8px;

  @include theme.useTheme {
    background: theme.getVar('sidebarBg');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}
.editor-wrapper {
  flex: 1;
  overflow: hidden;
}
</style>
