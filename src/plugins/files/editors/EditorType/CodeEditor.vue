<template>
  <div class="code-editor">
    <!-- 顶栏：状态、行列/选区、编码/EOL、语言切换、导航工具与保存 -->
    <div class="code-editor-bar">
      <span class="code-editor-status">
        <span v-if="doc?.isLoading" class="status-item">加载中…</span>
        <template v-else>
          <span v-if="doc?.externalModified" class="status-item error">外部已修改</span>
          <span v-else-if="doc?.isDirty" class="status-item dirty">已修改</span>
          <span v-else class="status-item">已保存</span>
          <span v-if="isReadonly" class="status-item readonly">只读</span>
        </template>
        <span class="status-item">行 {{ cursor.line }}，列 {{ cursor.column }}</span>
        <span v-if="selection.chars > 0" class="status-item">
          选中 {{ selection.chars }} 字符 / {{ selection.lines }} 行
        </span>
        <button
          type="button"
          class="status-item status-action"
          title="点击转换文件编码 / 换行符 / BOM（会先备份）"
          @click="openConvertDialog(props.path)"
        >{{ doc?.encoding || 'UTF-8' }}</button>
        <button
          type="button"
          class="status-item status-action"
          title="点击转换文件编码 / 换行符 / BOM（会先备份）"
          @click="openConvertDialog(props.path)"
        >{{ doc?.eol || 'LF' }}</button>
        <span class="status-item" :title="'字号 ' + prefs.fontSize">{{ prefs.fontSize }}px</span>
      </span>

      <!-- 工具栏：一律 bare 图标按钮（24×24），文字信息交给 title/aria-label，
           避免带文字的按钮把顶栏撑高、挤压正文可用高度
           （信息密度约定见 docs/conventions/ui-style-and-a11y.md「信息密度」） -->
      <div class="code-editor-tools">
        <app-button
          bare
          :icon="Collection"
          :disabled="isReadonly"
          title="行书签（Ctrl+F2）"
          aria-label="行书签"
          @click="showBookmarkMenu"
        />
        <app-button bare :icon="Tickets" title="文档大纲" aria-label="文档大纲" @click="showOutlineMenu" />
        <app-button
          bare
          :icon="Setting"
          title="编辑器偏好与快捷键"
          aria-label="编辑器偏好"
          @click="editorPreferencesVisible = true"
        />
        <app-button bare :icon="ZoomOut" title="缩小字号" aria-label="缩小字号" @click="zoomBy(-1)" />
        <app-button bare :icon="ZoomIn" title="放大字号" aria-label="放大字号" @click="zoomBy(1)" />
        <el-select
          v-model="selectedLanguage"
          size="small"
          class="code-editor-language-select"
          :disabled="isReadonly"
        >
          <el-option v-for="lang in LANGUAGE_OPTIONS" :key="lang" :label="lang" :value="lang" />
        </el-select>
        <!-- 保存：图标按钮，有未保存修改时高亮（title 里带快捷键，不占横向空间） -->
        <app-button
          v-if="!isReadonly"
          bare
          class="code-editor-save"
          :class="{ dirty: doc?.isDirty }"
          :icon="DocumentChecked"
          :loading="saving"
          :disabled="!doc?.isDirty"
          title="保存 (Ctrl+S)"
          aria-label="保存"
          @click="handleSave"
        />
      </div>
    </div>

    <!-- Monaco 编辑器主体：外观偏好通过 options 热更新，不重建实例 -->
    <MonacoEditor
      ref="monacoRef"
      v-model="content"
      :language="language"
      :readonly="isReadonly"
      :options="monacoOptions"
      @update:model-value="onContentChange"
      @content-change="onMonacoContentChange"
      @ready="onEditorReady"
    />

    <!-- 外部改动冲突三选一（M1 遗留：此前只有 409 报错，没有恢复路径） -->
    <GlobalDialog v-model="showConflict" title="文件已在外部被修改" width="460px">
      <div class="conflict-message">
        「{{ doc?.name || props.path }}」在磁盘上被其它程序改写，已阻止保存以免覆盖。
        重新加载会丢弃本编辑器里未保存的更改；强制保存会用当前内容覆盖磁盘。
      </div>
      <template #footer>
        <app-button size="small" @click="showConflict = false">稍后处理</app-button>
        <app-button size="small" plain @click="reloadFromDisk">重新加载</app-button>
        <app-button size="small" type="danger" @click="overwriteDisk">强制保存</app-button>
      </template>
    </GlobalDialog>
  </div>
</template>

<script setup lang="ts" name="CodeEditor">
import { ref, computed, watch, nextTick, onMounted, onActivated, onDeactivated, onBeforeUnmount } from 'vue'
import MonacoEditor from '@/components/MonacoEditor.vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { showContextMenu } from '@/components/context-menu/useContextMenu'
import NotificationUtil from '@/utils/NotificationUtil'
import { useDocumentManagerStore } from '@/store/modules/documents'
import type { DocumentState } from '@/store/modules/documents'
import {
  editorOptions,
  editorPreferencesVisible,
  prefs,
  zoomBy
} from '../../preferences/useEditorPreferences'
import { useSplitView } from '../../view/useSplitView'
import { useBookmarks } from '../../navigation/useBookmarks'
import { onRevealLine, onLineOp, revealLine } from '../../navigation/editorEvents'
import type { LineOp, LineOpDetail } from '../../navigation/editorEvents'
import { buildOutline } from '../../navigation/outline'
import { setEditorCursor } from '../../commands/editorCommands'
import { recordMacroStep, onMacroInsert } from '../../macros/useMacros'
import { openConvertDialog } from '../../encoding/useEncodingConvert'
import { Collection, DocumentChecked, Setting, Tickets, ZoomIn, ZoomOut } from '@element-plus/icons-vue'

const props = defineProps<{
  path: string
}>()

const emit = defineEmits<{
  'update-state': [state: Record<string, unknown>]
}>()

const docStore = useDocumentManagerStore()
const split = useSplitView()
const bookmarks = useBookmarks()

const monacoRef = ref<InstanceType<typeof MonacoEditor>>()
const content = ref('')
const saving = ref(false)
const showConflict = ref(false)

/** 本组件不得 import monaco（editor-and-tabs.md:8），options 交给公共封装透传 */
const monacoOptions = computed(() => editorOptions.value as any)

/** 当前路径对应的文档状态 */
const doc = computed<DocumentState | undefined>(() => props.path ? docStore.documents.get(props.path) : undefined)
const language = computed(() => doc.value?.language || 'plaintext')
/** 纯查看模式是 M3 全部增强的总回退开关：关掉增强并强制只读 */
const isReadonly = computed(() => !!doc.value?.isReadOnly || prefs.pureViewMode)
/** 状态栏语言下拉：值与 monaco 的 language id 对齐 */
const LANGUAGE_OPTIONS = [
  'plaintext', 'javascript', 'typescript', 'json', 'html', 'css', 'scss',
  'python', 'java', 'sql', 'markdown', 'yaml', 'xml', 'shell', 'rust',
  'go', 'cpp', 'csharp', 'php', 'ini', 'bat'
]
const cursor = ref({ line: 1, column: 1 })
const selection = ref({ chars: 0, lines: 0 })
const selectedLanguage = computed({
  get: () => doc.value?.language || 'plaintext',
  set: (value: string) => {
    if (props.path) docStore.setDocumentLanguage(props.path, value)
  }
})

/** 同步滚动：每个 CodeEditor 实例一个 id，接收端据此忽略自己的回声（setup 每次实例都跑，不能用模块级计数） */
const sourceId = `code-${Math.random().toString(36).slice(2)}`
let unsubScroll: (() => void) | null = null
let unsubReveal: (() => void) | null = null
let unsubLineOp: (() => void) | null = null
let unsubMacroInsert: (() => void) | null = null
let syncing = false

/** 行操作（A6）：Monaco 内建 action 的 id 表，避免业务侧碰 monaco 常量。 */
const LINE_ACTIONS: Partial<Record<LineOp, string>> = {
  copyDown: 'editor.action.copyLinesDownAction',
  copyUp: 'editor.action.copyLinesUpAction',
  moveUp: 'editor.action.moveLinesUpAction',
  moveDown: 'editor.action.moveLinesDownAction',
  delete: 'editor.action.deleteLines'
}

/** 选区文本变换：只处理文本，EOL 由写盘时按原值还原。 */
const LINE_TRANSFORMS: Partial<Record<LineOp, (text: string) => string>> = {
  sortAsc: (t) => t.split(/\r?\n/).sort((a, b) => a.localeCompare(b)).join('\n'),
  sortDesc: (t) => t.split(/\r?\n/).sort((a, b) => b.localeCompare(a)).join('\n'),
  dedup: (t) => [...new Set(t.split(/\r?\n/))].join('\n'),
  removeEmpty: (t) => t.split(/\r?\n/).filter((l) => l.trim().length > 0).join('\n'),
  upper: (t) => t.toUpperCase(),
  lower: (t) => t.toLowerCase()
}

function handleLineOp(detail: LineOpDetail) {
  if (detail.path !== props.path) return
  if (isReadonly.value) {
    NotificationUtil.warn('当前文档只读，不能执行行操作')
    return
  }
  const action = LINE_ACTIONS[detail.op]
  if (action) {
    monacoRef.value?.runAction(action)
    return
  }
  const transform = LINE_TRANSFORMS[detail.op]
  if (!transform) return
  if (!monacoRef.value?.transformSelection(transform)) {
    NotificationUtil.warn('请先选中要处理的文本行')
  }
}

/** 从 store 取出文档内容（openDocument 已对 ≤1MB 文件整读；此处兜底读取） */
function syncContentFromDoc() {
  const d = doc.value
  if (!d) return
  if (typeof d.content === 'string') {
    content.value = d.content
  } else if (d.content instanceof ArrayBuffer) {
    content.value = new TextDecoder('utf-8').decode(d.content)
  } else {
    content.value = ''
  }
}

/** 编辑器内容变更 → 同步 dirty 状态到 store（tab 圆点/关闭确认据此工作） */
function onContentChange(value: string) {
  if (!props.path) return
  content.value = value
  docStore.updateDocumentContent(props.path, value)
  // 记录光标/滚动（视图状态快照由父级 FileEditor handleStateUpdate 收）
  emitState()
}

/** 宏录制（M4 / F2）：Monaco 直接给出本次插入片段，不对整篇文本做 diff（大文件会卡）。 */
function onMonacoContentChange(texts: string[]) {
  for (const text of texts || []) {
    recordMacroStep({ kind: 'text', text })
  }
}

async function handleSave() {
  if (!props.path || saving.value) return
  saving.value = true
  try {
    const ok = await docStore.saveDocument(props.path)
    if (ok) NotificationUtil.success('保存成功')
  } finally {
    saving.value = false
  }
}

/** 视图状态上报（光标、滚动位置，切走时恢复用） */
function emitState() {
  const editor = monacoRef.value?.getEditor()
  if (!editor) return
  const pos = editor.getPosition()
  const scroll = editor.getScrollTop()
  emit('update-state', {
    // 带上自身 path：父级若用 activeDocumentPath 收口，切 tab 时会把位置写到新文档上（串号）
    path: props.path,
    cursor: pos ? { line: pos.lineNumber, column: pos.column } : undefined,
    scrollTop: scroll
  })
}

/**
 * 把 docStore 里记录的光标 / 滚动位置应用到编辑器。
 *
 * 位置真源是 docStore.viewState（会话快照里 exportOpenFiles 的 cursor_* / scroll_top 就是它），
 * 因此「切 tab / 切插件 / 重启」三条路径共用同一份数据，组件内不再另存一套。
 * 注意 scrollTop 与 cursor 是二选一还原：revealLineInCenter 会把视口重新居中，
 * 两个都恢复的话滚动位置会被光标位置覆盖。
 */
function applyViewState() {
  const editor = monacoRef.value?.getEditor()
  if (!editor) return
  const vs = doc.value?.viewState || {}
  const savedCursor = vs.cursor as { line?: number; column?: number } | undefined
  if (typeof vs.scrollTop === 'number' && vs.scrollTop > 0) {
    if (savedCursor?.line) editor.setPosition({ lineNumber: savedCursor.line, column: savedCursor.column || 1 })
    editor.setScrollTop(vs.scrollTop)
  } else if (savedCursor?.line) {
    editor.setPosition({ lineNumber: savedCursor.line, column: savedCursor.column || 1 })
    editor.revealLineInCenter(savedCursor.line)
  }
}

/** 当前行书签（Ctrl+F2 也走这里）。 */
function toggleCurrentBookmark() {
  if (!props.path) return
  const added = bookmarks.toggle(props.path, cursor.value.line)
  NotificationUtil.info(added ? `已标记第 ${cursor.value.line} 行` : `已取消第 ${cursor.value.line} 行书签`)
  refreshBookmarkDecorations()
}

function refreshBookmarkDecorations() {
  monacoRef.value?.setBookmarkLines(bookmarks.linesFor(props.path))
}

function gotoBookmark(direction: 'next' | 'prev') {
  const line = direction === 'next'
    ? bookmarks.next(props.path, cursor.value.line)
    : bookmarks.prev(props.path, cursor.value.line)
  if (line == null) {
    NotificationUtil.warn('当前文件没有行书签')
    return
  }
  revealLine(props.path, line)
}

function showBookmarkMenu(e: MouseEvent) {
  const lines = bookmarks.linesFor(props.path)
  const customItems = lines.slice(0, 30).map((line) => ({
    label: `第 ${line} 行`,
    onClick: () => revealLine(props.path, line)
  }))
  const items: any[] = [
    { label: '切换当前行书签 (Ctrl+F2)', onClick: toggleCurrentBookmark },
    { label: '下一个书签 (F2)', onClick: () => gotoBookmark('next') },
    { label: '上一个书签 (Shift+F2)', onClick: () => gotoBookmark('prev') },
    { label: '清除本文件书签', onClick: () => { bookmarks.clear(props.path); refreshBookmarkDecorations() } }
  ]
  if (lines.length > 0) items.push({ divider: true })
  items.push(...customItems)
  showContextMenu({ x: e.clientX, y: e.clientY, items })
}

function showOutlineMenu(e: MouseEvent) {
  const items = buildOutline(content.value, language.value)
  if (items.length === 0) {
    NotificationUtil.warn('当前文档没有可识别的大纲条目')
    return
  }
  showContextMenu({
    x: e.clientX,
    y: e.clientY,
    items: items.slice(0, 60).map((item) => ({
      label: `${'  '.repeat(Math.max(0, item.level - 1))}L${item.line}  ${item.label}`,
      onClick: () => revealLine(props.path, item.line)
    }))
  })
}

/**
 * 编辑器就绪：注册键位 + 还原上次视图状态。
 *
 * 历史坑：MonacoEditor 一直会 emit('ready')，但本组件模板里曾没有 @ready 绑定，
 * 于是这个函数从未被执行 —— 「Ctrl+S 保存」与「位置恢复」都只是看起来写了。
 * monaco 常量已全部收进 MonacoEditor.vue 的 expose（D-7），本组件不再 import。
 */
function onEditorReady() {
  const editor = monacoRef.value?.getEditor()
  if (!editor) return
  monacoRef.value?.addSaveCommand(() => { void handleSave() })
  monacoRef.value?.addBookmarkCommand(toggleCurrentBookmark)

  // 状态栏行列：Monaco 的 position 是内部状态，不受 KeepAlive detach 影响
  editor.onDidChangeCursorPosition((e) => {
    cursor.value = { line: e.position.lineNumber, column: e.position.column }
    // 命令面板/全局快捷键不持有 Monaco 实例，靠这里拿到「当前行」
    setEditorCursor(props.path, e.position.lineNumber)
  })
  editor.onDidChangeCursorSelection(() => {
    selection.value = monacoRef.value?.getSelectionStats() || { chars: 0, lines: 0 }
  })
  // 分屏同步滚动：只发「顶部可见行」，接收端按行对齐（两文件行数不同也能跟上）
  editor.onDidScrollChange(() => {
    if (syncing) return
    const top = editor.getVisibleRanges()[0]?.startLineNumber ?? 1
    split.publishScroll({
      source: sourceId,
      path: props.path,
      line: top,
      scrollTop: editor.getScrollTop()
    })
  })
  refreshBookmarkDecorations()
  applyViewState()
}

async function reloadFromDisk() {
  await docStore.reloadDocument(props.path)
  showConflict.value = false
  NotificationUtil.success('已重新加载磁盘内容')
}

async function overwriteDisk() {
  const ok = await docStore.saveDocument(props.path, { force: true })
  showConflict.value = false
  if (ok) NotificationUtil.success('已用当前内容覆盖磁盘')
}

// 文档路径切换（KeepAlive 复用实例时，props.path 会变化）
watch(() => props.path, async () => {
  syncContentFromDoc()
  // 内容回写会走子组件的 modelValue watch → editor.setValue，这一步会把滚动位置清零，
  // 所以必须等这次更新跑完再落位置，否则刚恢复就被覆盖回顶部
  await nextTick()
  refreshBookmarkDecorations()
  applyViewState()
})

// 文档从外部被 reload 后，内容已由 store 更新 → 回写编辑器
watch(
  () => doc.value?.content,
  (val) => {
    if (typeof val === 'string' && val !== content.value && !doc.value?.isDirty) {
      content.value = val
    }
  }
)

// 外部修改被后端 409 拦下时主动弹三选一，用户不必再去状态栏找入口
watch(() => doc.value?.externalModified, (flag) => {
  if (flag) showConflict.value = true
})

// 书签由命令面板/快捷键改动时，装饰也要跟着刷新（不依赖调用方记得通知）
watch(
  () => bookmarks.linesFor(props.path),
  () => refreshBookmarkDecorations(),
  { deep: true }
)

// setup 顶层立即同步一次：确保子组件 MonacoEditor 创建前 content 已有值（父 onMounted 晚于子组件创建）
syncContentFromDoc()

onMounted(() => {
  syncContentFromDoc()
  unsubScroll = split.subscribeScroll((p) => {
    if (p.source === sourceId || p.path === props.path || syncing) return
    const editor = monacoRef.value?.getEditor()
    if (!editor) return
    syncing = true
    editor.revealLineInCenter(Math.max(1, p.line))
    window.setTimeout(() => { syncing = false }, 80)
  })
  unsubLineOp = onLineOp(handleLineOp)
  // 宏回放只作用于当前活动文档：分屏时两个 CodeEditor 都会收到事件，必须自己过滤
  unsubMacroInsert = onMacroInsert((text) => {
    if (docStore.activeDocumentPath !== props.path) return
    monacoRef.value?.insertTextAtCursor(text)
  })
  unsubReveal = onRevealLine((d) => {
    if (d.path !== props.path) return
    const editor = monacoRef.value?.getEditor()
    if (!editor) {
      // 组件尚未挂载：写入 viewState，挂载后 applyViewState 会落位
      docStore.updateViewState(props.path, { cursor: { line: d.line, column: d.column || 1 } })
      return
    }
    editor.setPosition({ lineNumber: d.line, column: d.column || 1 })
    editor.revealLineInCenter(d.line)
    editor.focus()
  })
})

onActivated(() => {
  syncContentFromDoc()
  // 切回时 DOM 已重新插回文档，重新落一次位置：KeepAlive detach 期间视图尺寸为 0，
  // monaco 的 automaticLayout 按 0 高度重排过，滚动位置不保证保留
  nextTick(() => {
    refreshBookmarkDecorations()
    applyViewState()
  })
})

onDeactivated(() => {
  emitState()
})

onBeforeUnmount(() => {
  emitState()
  unsubScroll?.()
  unsubReveal?.()
  unsubLineOp?.()
  unsubMacroInsert?.()
})
</script>

<style scoped lang="scss">
.code-editor {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.code-editor-bar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-2);
  flex: none;
  border-bottom: 1px solid var(--borderColor);
  background: var(--bgColor);
}

.code-editor-status {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--font-size-xs);
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
}

.code-editor-tools {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  margin-left: auto;
  flex: none;
}

/* 保存图标：仅在有未保存修改时用强调色；平时是次级色（disabled 时为禁用色） */
.code-editor-save.dirty {
  color: var(--stateInfoFg);
}

.code-editor-save.dirty:hover:not(:disabled) {
  color: var(--stateInfoFg);
}

.status-item {
  color: var(--el-text-color-secondary);
}

.status-item.dirty {
  color: var(--stateWarningFg);
}

/* 编码 / EOL 是可点击的「转换」入口，按钮语义要保留键盘可达与焦点环 */
.status-action {
  padding: 0 3px;
  border: 0;
  border-radius: 3px;
  background: transparent;
  font: inherit;
  cursor: pointer;
  text-decoration: underline dotted;
  text-underline-offset: 2px;
}
.status-action:hover {
  color: var(--textColor);
  background: var(--hoverColor);
}
.status-action:focus-visible {
  outline: 2px solid var(--focusRing);
  outline-offset: 1px;
}

.status-item.error,
.status-item.readonly {
  color: var(--stateErrorFg);
}

.code-editor-language-select {
  /* 语言 id 最长 typescript，96px 足够；收窄让工具栏整体不超过顶栏高度 */
  width: 96px;
  flex: none;
}

.conflict-message {
  font-size: var(--fontSizeSmall);
  line-height: 1.7;
  color: var(--el-text-color-primary);
}
</style>
