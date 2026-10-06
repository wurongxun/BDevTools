<template>
  <div class="code-editor">
    <!-- 顶栏：状态与保存 -->
    <div class="code-editor-bar">
      <span class="code-editor-status">
        <span v-if="doc?.isLoading" class="status-item">加载中…</span>
        <template v-else>
          <span v-if="doc?.isDirty" class="status-item dirty">已修改</span>
          <span v-else class="status-item">已保存</span>
          <span v-if="doc?.isReadOnly" class="status-item readonly">只读</span>
        </template>
      </span>
      <span class="code-editor-language">{{ languageLabel }}</span>
      <app-button
        v-if="!doc?.isReadOnly"
        size="small"
        type="primary"
        :loading="saving"
        :disabled="!doc?.isDirty"
        @click="handleSave"
      >
        保存 (Ctrl+S)
      </app-button>
    </div>
    <!-- Monaco 编辑器主体 -->
    <MonacoEditor
      ref="monacoRef"
      v-model="content"
      :language="language"
      :readonly="isReadonly"
      @update:model-value="onContentChange"
      @ready="onEditorReady"
    />
  </div>
</template>

<script setup lang="ts" name="CodeEditor">
import { ref, computed, watch, nextTick, onMounted, onActivated, onDeactivated, onBeforeUnmount } from 'vue'
import * as monaco from 'monaco-editor'
import MonacoEditor from '@/components/MonacoEditor.vue'
import NotificationUtil from '@/utils/NotificationUtil'
import { useDocumentManagerStore } from '@/store/modules/documents'
import type { DocumentState } from '@/store/modules/documents'

const props = defineProps<{
  path: string
}>()

const emit = defineEmits<{
  'update-state': [state: Record<string, unknown>]
}>()

const docStore = useDocumentManagerStore()

const monacoRef = ref<InstanceType<typeof MonacoEditor>>()
const content = ref('')
const saving = ref(false)

/** 当前路径对应的文档状态 */
const doc = computed<DocumentState | undefined>(() => props.path ? docStore.documents.get(props.path) : undefined)
const language = computed(() => doc.value?.language || 'plaintext')
const isReadonly = computed(() => doc.value?.isReadOnly || false)
const languageLabel = computed(() => language.value || 'plaintext')

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
  const cursor = vs.cursor as { line?: number; column?: number } | undefined
  if (typeof vs.scrollTop === 'number' && vs.scrollTop > 0) {
    if (cursor?.line) editor.setPosition({ lineNumber: cursor.line, column: cursor.column || 1 })
    editor.setScrollTop(vs.scrollTop)
  } else if (cursor?.line) {
    editor.setPosition({ lineNumber: cursor.line, column: cursor.column || 1 })
    editor.revealLineInCenter(cursor.line)
  }
}

/**
 * 编辑器就绪：注册 Ctrl+S + 还原上次视图状态。
 *
 * 历史坑：MonacoEditor 一直会 emit('ready')，但本组件模板里没有 @ready 绑定，
 * 于是这个函数从未被执行 —— 「Ctrl+S 保存」与「位置恢复」都只是看起来写了。
 * 父组件不监听子组件事件 = 死代码，改动子组件 API 时务必回头核对调用点。
 */
function onEditorReady() {
  const editor = monacoRef.value?.getEditor()
  if (!editor) return
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
    handleSave()
  })
  applyViewState()
}

// 文档路径切换（KeepAlive 复用实例时，props.path 会变化）
watch(() => props.path, async () => {
  syncContentFromDoc()
  // 内容回写会走子组件的 modelValue watch → editor.setValue，这一步会把滚动位置清零，
  // 所以必须等这次更新跑完再落位置，否则刚恢复就被覆盖回顶部
  await nextTick()
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

// setup 顶层立即同步一次：确保子组件 MonacoEditor 创建前 content 已有值（父 onMounted 晚于子组件创建）
syncContentFromDoc()

onMounted(() => {
  syncContentFromDoc()
})

onActivated(() => {
  syncContentFromDoc()
  // 切回时 DOM 已重新插回文档，重新落一次位置：KeepAlive detach 期间视图尺寸为 0，
  // monaco 的 automaticLayout 按 0 高度重排过，滚动位置不保证保留
  nextTick(() => applyViewState())
})

onDeactivated(() => {
  emitState()
})

onBeforeUnmount(() => {
  emitState()
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
  gap: 8px;
  padding: 4px 8px;
  flex: none;
  border-bottom: 1px solid var(--el-border-color-lighter);
  background: var(--el-fill-color-light);
}

.code-editor-status {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--font-size-xs);
}

.status-item {
  color: var(--el-text-color-secondary);
}

.status-item.dirty {
  color: var(--el-color-warning);
}

.status-item.readonly {
  color: var(--el-color-danger);
}

.code-editor-language {
  margin-left: auto;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  font-family: var(--font-family-mono);
}
</style>
