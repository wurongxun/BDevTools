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

const props = withDefaults(
  defineProps<{
    modelValue?: string
    language?: string
    readonly?: boolean
    /** 透传给 monaco.create 的覆盖项（优先于下方默认值） */
    options?: monaco.editor.IStandaloneEditorConstructionOptions
  }>(),
  {
    modelValue: '',
    language: 'plaintext',
    readonly: false,
    options: () => ({})
  }
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
  /** editor 就绪后触发（父组件可拿实例 addCommand / 设选区等） */
  'ready': [editor: monaco.editor.IStandaloneCodeEditor]
}>()

const containerRef = ref<HTMLElement>()
let editor: monaco.editor.IStandaloneCodeEditor | null = null
/** 标记是否是编辑器自身触发的变更（避免外部 modelValue watch 回写导致光标跳动） */
let applyingExternal = false

function resolveLanguage(lang: string): string {
  return LANGUAGE_ALIAS[lang] ?? lang
}

onMounted(() => {
  if (!containerRef.value) return
  const language = resolveLanguage(props.language)
  defineEditorTheme(isDark.value)
  editor = monaco.editor.create(containerRef.value, {
    value: props.modelValue,
    language,
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
    renderLineHighlight: 'all',
    ...props.options
  })

  // 内容变更 → 上抛（业务侧据此更新 isDirty 等）
  editor.onDidChangeModelContent(() => {
    if (applyingExternal) return
    const value = editor?.getValue() ?? ''
    if (value !== props.modelValue) {
      emit('update:modelValue', value)
    }
  })

  // 语言（扩展名改变时切换 model 语言）
  watch(
    () => props.language,
    (lang) => {
      const model = editor?.getModel()
      if (model) {
        monaco.editor.setModelLanguage(model, resolveLanguage(lang))
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

  emit('ready', editor)
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
  const model = editor?.getModel()
  editor?.dispose()
  editor = null
  if (model) model.dispose()
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

defineExpose({ getEditor, getValue, setValue, focus })
</script>

<style scoped>
.monaco-editor-host {
  width: 100%;
  height: 100%;
  min-height: 0;
}
</style>
