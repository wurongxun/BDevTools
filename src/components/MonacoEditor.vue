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
  editor = monaco.editor.create(containerRef.value, {
    value: props.modelValue,
    language,
    theme: isDark.value ? 'vs-dark' : 'vs',
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
watch(isDark, (dark) => {
  monaco.editor.setTheme(dark ? 'vs-dark' : 'vs')
})

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
