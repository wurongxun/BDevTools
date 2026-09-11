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
    />
  </div>
</template>

<script setup lang="ts" name="CodeEditor">
import { ref, computed, watch, onMounted, onActivated, onDeactivated, onBeforeUnmount } from 'vue'
import * as monaco from 'monaco-editor'
import MonacoEditor from '@/components/MonacoEditor.vue'
import NotificationUtil from '@/utils/NotificationUtil'
import { useDocumentManagerStore } from '../../store/documents'
import type { DocumentState } from '../../store/documents'

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
    cursor: pos ? { line: pos.lineNumber, column: pos.column } : undefined,
    scrollTop: scroll
  })
}

/** 编辑器内 Ctrl+S 保存 */
function onEditorReady() {
  const editor = monacoRef.value?.getEditor()
  if (editor) {
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
      handleSave()
    })
    // 恢复上次视图状态（如果存在）
    const vs = doc.value?.viewState || {}
    if (typeof vs.scrollTop === 'number' && vs.scrollTop > 0) {
      editor.setScrollTop(vs.scrollTop)
    }
    const line = (vs.cursor as { line?: number } | undefined)?.line
    if (line) {
      editor.setPosition({ lineNumber: line, column: ((vs.cursor as { column?: number })?.column) || 1 })
      editor.revealLineInCenter(line)
    }
  }
}

// 文档路径切换（KeepAlive 复用实例时，props.path 会变化）
watch(() => props.path, () => {
  syncContentFromDoc()
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
