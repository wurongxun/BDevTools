<template>
  <div ref="containerRef" class="monaco-editor-host"></div>
</template>

<script setup lang="ts">
/**
 * 公共 Monaco 代码编辑器封装（全项目唯一允许 import 'monaco-editor' 的组件）。
 *
 * 职责边界：
 * - 收敛 monaco 的全部样板：worker 配置（MonacoEnvironment）、实例创建/销毁、
 *   深浅色主题跟随、resize 自适应、v-model 双向内容
 * - 不绑定任何业务概念（不知道"文件/SQL/保存"），父组件拿到 editor 实例做高级操作
 *
 * 约定：新增任何"代码/SQL 编辑器"一律复用本组件；不要在其他 .vue 里直接 import 'monaco-editor'。
 */
import { ref, watch, onMounted, onBeforeUnmount } from 'vue'
import * as monaco from 'monaco-editor'
import { isDark } from '@/composables/useDark'
import { isHighContrast } from '@/composables/useHighContrast'

// ==================== monaco worker 配置（全应用只需一次） ====================
// vite 通过 `?worker` 把 worker 打成独立 chunk；MonacoEnvironment 按语言 label 分发。
// 说明：monaco-editor 主入口 import 后不会立即启动 worker，worker 在首次 create 时按需
// new，因此此处模块顶层设置环境（先于任何 editor.create）是安全的。
import EditorWorker from 'monaco-editor/editor/editor.worker?worker'
import JsonWorker from 'monaco-editor/language/json/json.worker?worker'
import CssWorker from 'monaco-editor/language/css/css.worker?worker'
import HtmlWorker from 'monaco-editor/language/html/html.worker?worker'
import TsWorker from 'monaco-editor/language/typescript/ts.worker?worker'

if (!(self as any).MonacoEnvironment) {
  ;(self as any).MonacoEnvironment = {
    getWorker(_workerId: string, label: string) {
      if (label === 'json') return new JsonWorker()
      if (label === 'css' || label === 'scss' || label === 'less') return new CssWorker()
      if (label === 'html' || label === 'handlebars' || label === 'razor') return new HtmlWorker()
      if (label === 'typescript' || label === 'javascript') return new TsWorker()
      return new EditorWorker()
    }
  }
}

// monaco 语言 id 归一化：把部分"业务语言名"映射到 monaco 内置可高亮的语言，
// 避免 create 时传入未知语言导致无高亮/告警。
// - vue：monaco 无内置 vue 语法，模板以 HTML 高亮为主（script/style 块内为近似高亮）
// - jsx/tsx：monaco 通过 typescript worker 支持，语言 id 用 javascript/typescript 即含 JSX
const LANGUAGE_ALIAS: Record<string, string> = {
  vue: 'html',
  javascriptreact: 'javascript',
  typescriptreact: 'typescript'
}

// ==================== 编辑器主题：与外壳同源 ====================
// monaco 的 defineTheme 只吃具体色值、不认 CSS 变量，所以从 :root 上把编辑器令牌
// （theme.scss 的 editorBg/editorFg/...）读出来喂给它 —— 改 theme.scss 编辑器就跟着变，
// 不再各自维护一套。
// colors 只覆盖「UI 外壳色」，`inherit: true` 保留内置 vs/vs-dark 成熟的语法高亮着色
// （自绘 20+ 条 token 规则成本高、且很难做得比内置好）。
const EDITOR_THEME = 'bdev-tools'
const EDITOR_FALLBACK: Record<string, [string, string]> = {
  // [亮色兜底, 暗色兜底]：令牌缺失时不至于让编辑器变黑底黑字
  editorBg: ['#ffffff', '#2b2b2b'],
  editorFg: ['#212121', '#bbbbbb'],
  editorGutterBg: ['#ffffff', '#2b2b2b'],
  editorLineNumberFg: ['#666666', '#949494'],
  editorSelectionBg: ['#add6ff', '#234060'],
  editorLineHighlightBg: ['#f0f0f0', '#323537'],
  editorCursorFg: ['#005a9e', '#79c0ff'],
  editorIndentGuideBg: ['#e6e6e6', '#373a3c'],
  editorWidgetBorder: ['#808080', '#8c9094'],
}

/** 读取当前主题下的编辑器令牌值（必须在主题 class 已生效后调用，见下方 flush:'post'） */
function readEditorTokens(dark: boolean): Record<string, string> {
  const style = getComputedStyle(document.documentElement)
  const out: Record<string, string> = {}
  for (const key of Object.keys(EDITOR_FALLBACK)) {
    const raw = style.getPropertyValue(`--${key}`).trim()
    out[key] = raw || EDITOR_FALLBACK[key][dark ? 1 : 0]
  }
  return out
}

/**
 * 定义/重定义编辑器主题。
 * 主题切换时必须重新 define：monaco 的主题是「定义时的快照」，
 * 不会因为我们改了 CSS 变量而自动更新。
 */
function defineEditorTheme(dark: boolean) {
  const c = readEditorTokens(dark)
  monaco.editor.defineTheme(EDITOR_THEME, {
    base: dark ? 'vs-dark' : 'vs',
    inherit: true,
    rules: [],
    colors: {
      'editor.background': c.editorBg,
      'editor.foreground': c.editorFg,
      'editorGutter.background': c.editorGutterBg,
      'editorLineNumber.foreground': c.editorLineNumberFg,
      'editorLineNumber.activeForeground': c.editorFg,
      'editor.selectionBackground': c.editorSelectionBg,
      'editor.lineHighlightBackground': c.editorLineHighlightBg,
      'editorCursor.foreground': c.editorCursorFg,
      'editorIndentGuide.background': c.editorIndentGuideBg,
      'editorIndentGuide.activeBackground': c.editorIndentGuideBg,
      // 悬浮/建议类浮层：跟随面板色，避免与外壳割裂
      'editorWidget.background': c.editorBg,
      'editorWidget.border': c.editorWidgetBorder,
      'editorSuggestWidget.background': c.editorBg,
      'editorSuggestWidget.border': c.editorWidgetBorder,
      'editorHoverWidget.background': c.editorBg,
      'editorHoverWidget.border': c.editorWidgetBorder
    }
  })
}

/** 文本比较入参：传入即创建 DiffEditor（只读），不传则创建普通编辑器。 */
interface MonacoDiffInput {
  original: string
  modified: string
  language?: string
  renderSideBySide?: boolean
}

const props = withDefaults(
  defineProps<{
    modelValue?: string
    language?: string
    readonly?: boolean
    /** 透传给 monaco.create 的覆盖项（优先于下方默认值） */
    options?: monaco.editor.IStandaloneEditorConstructionOptions
    /** 存在时切换为只读 diff 视图（M3：文本比较） */
    diff?: MonacoDiffInput | null
  }>(),
  {
    modelValue: '',
    language: 'plaintext',
    readonly: false,
    options: () => ({}),
    diff: null
  }
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
  /** editor 就绪后触发（父组件可拿实例 addCommand / 设选区等） */
  'ready': [editor: monaco.editor.IStandaloneCodeEditor]
  /** 本次编辑插入的文本片段（M4 宏录制用；外部 setValue 与纯删除不派发） */
  'content-change': [texts: string[]]
}>()

const containerRef = ref<HTMLElement>()
let editor: monaco.editor.IStandaloneCodeEditor | null = null
let diffEditor: monaco.editor.IStandaloneDiffEditor | null = null
let diffModels: [monaco.editor.ITextModel, monaco.editor.ITextModel] | null = null
let bookmarkDecorations: monaco.editor.IEditorDecorationsCollection | null = null
/** 标记是否是编辑器自身触发的变更（避免外部 modelValue watch 回写导致光标跳动） */
let applyingExternal = false

function resolveLanguage(lang: string): string {
  return LANGUAGE_ALIAS[lang] ?? lang
}

/** 普通编辑器与比较编辑器共用的默认选项；props.options 优先覆盖。 */
function baseOptions(): monaco.editor.IStandaloneEditorConstructionOptions {
  return {
    theme: EDITOR_THEME,
    minimap: { enabled: false },
    fontSize: 14,
    lineNumbers: 'on',
    scrollBeyondLastLine: false,
    automaticLayout: true,
    tabSize: 2,
    wordWrap: 'off',
    readOnly: props.readonly,
    glyphMargin: false,
    folding: true,
    renderLineHighlight: 'all'
  }
}

onMounted(() => {
  if (!containerRef.value) return
  const language = resolveLanguage(props.language)
  defineEditorTheme(isDark.value)
  const options = { ...baseOptions(), ...props.options }

  if (props.diff) {
    // 比较视图：两个 model 都归本组件所有，卸载时必须显式 dispose
    const lang = resolveLanguage(props.diff.language || props.language)
    const origModel = monaco.editor.createModel(props.diff.original, lang)
    const modModel = monaco.editor.createModel(props.diff.modified, lang)
    diffModels = [origModel, modModel]
    diffEditor = monaco.editor.createDiffEditor(containerRef.value, {
      ...options,
      // diff 是查看语义，不接受「只读」被外部 options 意外解除
      readOnly: true,
      renderSideBySide: props.diff.renderSideBySide ?? true
    } as monaco.editor.IStandaloneDiffEditorConstructionOptions)
    diffEditor.setModel({ original: origModel, modified: modModel })
  } else {
    editor = monaco.editor.create(containerRef.value, {
      value: props.modelValue,
      language,
      ...options
    })

    // 内容变更 → 上抛（业务侧据此更新 isDirty 等）
    editor.onDidChangeModelContent((e) => {
      if (applyingExternal) return
      const value = editor?.getValue() ?? ''
      if (value !== props.modelValue) {
        emit('update:modelValue', value)
        // 宏只录「插入了什么」：纯删除（change.text 为空）在不同上下文里回放会误删内容
        const texts = (e.changes || [])
          .map((c) => c.text)
          .filter((t): t is string => typeof t === 'string' && t.length > 0)
        if (texts.length) emit('content-change', texts)
      }
    })
  }

  // 外观偏好热更新（M3）：不重建 editor，避免光标/滚动位置丢失
  watch(
    () => props.options,
    (opts) => {
      editor?.updateOptions(opts || {})
      diffEditor?.updateOptions(opts || {})
    },
    { deep: true }
  )

  // 语言（扩展名改变时切换 model 语言）
  watch(
    () => props.language,
    (lang) => {
      const model = editor?.getModel()
      if (model) monaco.editor.setModelLanguage(model, resolveLanguage(lang))
      if (diffModels) {
        monaco.editor.setModelLanguage(diffModels[0], resolveLanguage(lang))
        monaco.editor.setModelLanguage(diffModels[1], resolveLanguage(lang))
      }
    }
  )

  // 只读切换
  watch(
    () => props.readonly,
    (ro) => {
      editor?.updateOptions({ readOnly: ro })
    }
  )

  // 比较内容变化（切换比较对象时只换 model 内容，不重建 editor）
  watch(
    () => props.diff,
    (next) => {
      if (!next || !diffModels) return
      diffModels[0].setValue(next.original)
      diffModels[1].setValue(next.modified)
      diffEditor?.updateOptions({ renderSideBySide: next.renderSideBySide ?? true })
    },
    { deep: true }
  )

  // diff 模式没有 standalone editor，不派发 ready（业务组件只在普通模式下注册命令）
  if (editor) emit('ready', editor)
})

// 暗黑模式：monaco 主题跟随全局单例（setTheme 为全局操作，实例实时生效）
//
// flush:'post' 是硬要求：useDark 先改 isDark 再给 <html> 加 class，
// 默认（pre）时机读到的还是**旧主题**的 CSS 变量 ⇒ 编辑器会拿着上一套颜色。
// 明暗切换 + 高对比度切换都会改变编辑器令牌的取值，两件事都要重新 define 再 setTheme
//（monaco 的主题是「定义时的快照」，只 setTheme 不会重读变量）。
// flush:'post' 是硬要求：useDark 先改 isDark 再给 <html> 加 class，
// 默认（pre）时机读到的还是**旧主题**的 CSS 变量 ⇒ 编辑器会拿着上一套颜色。
watch(
  [isDark, isHighContrast],
  ([dark]) => {
    defineEditorTheme(dark)
    monaco.editor.setTheme(EDITOR_THEME)
  },
  { flush: 'post' }
)

// 外部内容变化（如切换文档 / 重新加载）→ 仅在值不同时回写，避免滚动与光标跳动
watch(
  () => props.modelValue,
  (val) => {
    const model = editor?.getModel()
    if (!model) return
    const current = editor!.getValue()
    if (val !== current) {
      applyingExternal = true
      editor!.setValue(val ?? '')
      applyingExternal = false
    }
  }
)

onBeforeUnmount(() => {
  // 完整释放：editor 实例 + 其持有的 model（不显式 dispose model 会残留在 Monaco 内存中）
  bookmarkDecorations?.clear()
  bookmarkDecorations = null
  const model = editor?.getModel()
  editor?.dispose()
  editor = null
  if (model) model.dispose()
  diffEditor?.dispose()
  diffEditor = null
  if (diffModels) {
    diffModels[0].dispose()
    diffModels[1].dispose()
    diffModels = null
  }
})

/** 获取底层 editor 实例（父组件高级操作入口） */
function getEditor(): monaco.editor.IStandaloneCodeEditor | null {
  return editor
}

function getValue(): string {
  return editor?.getValue() ?? ''
}

function setValue(value: string) {
  if (editor) {
    applyingExternal = true
    editor.setValue(value)
    applyingExternal = false
  }
}

function focus() {
  editor?.focus()
}

/**
 * 业务侧需要 monaco 常量的键位/装饰操作一律收在这里：
 * `editor-and-tabs.md:8` 规定本组件是全项目唯一 import 'monaco-editor' 的地方，
 * 业务组件不得为了让 Ctrl+S / F2 生效而自己 import（M1 的 D-7 遗留）。
 */
function addSaveCommand(handler: () => void) {
  editor?.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, handler)
}

function addBookmarkCommand(handler: () => void) {
  editor?.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.F2, handler)
}

/** 行书签的 glyph margin 装饰（只改传入的行号集合，不重建 editor）。 */
function setBookmarkLines(lines: number[]) {
  if (!editor) return
  const decorations = lines.map((line) => ({
    range: new monaco.Range(line, 1, line, 1),
    options: {
      glyphMarginClassName: 'files-bookmark-glyph',
      glyphMarginHoverMessage: { value: `行书签：第 ${line} 行` }
    }
  }))
  if (!bookmarkDecorations) {
    bookmarkDecorations = editor.createDecorationsCollection(decorations)
  } else {
    bookmarkDecorations.set(decorations)
  }
}

/** 行操作（A6）：执行 Monaco 内建 action（复制/移动/删除行），业务侧不 import monaco。 */
function runAction(actionId: string) {
  void editor?.getAction(actionId)?.run()
}

/**
 * 行操作（A6）：对当前选区文本做变换后回写。
 * 只在 MonacoEditor 内接触 executeEdits / IEditOperation，业务侧只传纯函数。
 * 变换后 EOL 可能被归一成 \n，但写盘仍由 file_meta 按原 EOL 还原（M1 口径）。
 */
function transformSelection(transform: (text: string) => string): boolean {
  if (!editor) return false
  const sel = editor.getSelection()
  const model = editor.getModel()
  if (!sel || !model || sel.isEmpty()) return false
  const text = model.getValueInRange(sel)
  const next = transform(text)
  if (next === text) return false
  editor.executeEdits('files-line-ops', [{ range: sel, text: next }])
  return true
}

/** 宏回放（M4 / F2）：在当前光标/选区处插入文本，业务侧不接触 executeEdits。 */
function insertTextAtCursor(text: string): boolean {
  if (!editor || !text) return false
  const sel = editor.getSelection()
  if (!sel) return false
  editor.executeEdits('files-macro', [{ range: sel, text, forceMoveMarkers: true }])
  editor.focus()
  return true
}

/** 选区统计（A8）：返回选中字符数与覆盖行数，无选区时为 0。 */
function getSelectionStats(): { chars: number; lines: number } {
  if (!editor) return { chars: 0, lines: 0 }
  const model = editor.getModel()
  if (!model) return { chars: 0, lines: 0 }
  let chars = 0
  let lines = 0
  for (const sel of editor.getSelections() || []) {
    if (sel.isEmpty()) continue
    // 按 Unicode code point 计数：中文/emoji 用 .length 会算成 2，状态栏字数会翻倍
    chars += [...model.getValueInRange(sel)].length
    lines += sel.endLineNumber - sel.startLineNumber + 1
  }
  return { chars, lines }
}

defineExpose({
  getEditor,
  getValue,
  setValue,
  focus,
  addSaveCommand,
  addBookmarkCommand,
  setBookmarkLines,
  getSelectionStats,
  runAction,
  transformSelection,
  insertTextAtCursor
})
</script>

<!-- glyph margin 里的书签圆点由 monaco 动态插入 DOM，scoped 样式选择器到不了，必须全局 -->
<style>
.files-bookmark-glyph::after {
  content: '';
  display: block;
  width: 8px;
  height: 8px;
  margin: 5px auto 0;
  border-radius: 50%;
  background: var(--stateInfoFg, #409eff);
}
</style>

<style scoped>
.monaco-editor-host {
  width: 100%;
  height: 100%;
  min-height: 0;
}
</style>
