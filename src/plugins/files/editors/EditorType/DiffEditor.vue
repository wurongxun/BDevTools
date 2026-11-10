<template>
  <div class="diff-editor">
    <div class="diff-editor-bar">
      <span class="diff-title" :title="props.originalPath">
        比较：<b>{{ originalName }}</b> ↔ <b>{{ modifiedName }}</b>
      </span>
      <span v-if="errorMsg" class="diff-error">{{ errorMsg }}</span>
      <app-button size="small" class="diff-close" @click="split.closeDiff()">退出比较</app-button>
    </div>
    <MonacoEditor
      v-if="ready"
      class="diff-monaco"
      :diff="{ original, modified, language }"
      :options="diffOptions"
    />
    <div v-else class="diff-loading">正在读取两份文件…</div>
  </div>
</template>

<script setup lang="ts">
/**
 * 文本比较视图（D6）。
 *
 * 不 import monaco：createDiffEditor 由唯一的 `MonacoEditor.vue` 通过 diff prop 分支创建。
 * 内容优先取已打开文档的**当前内容**（含未保存编辑，比较更有意义），
 * 未打开或 >1MB 的才回落到 `read_note_file`。
 */
import { computed, ref, watch } from 'vue'
import MonacoEditor from '@/components/MonacoEditor.vue'
import { call } from '@/api/rpc'
import { useDocumentManagerStore } from '@/store/modules/documents'
import { useSplitView } from '../../view/useSplitView'
import { editorOptions, prefs } from '../../preferences/useEditorPreferences'

const props = defineProps<{
  originalPath: string
  modifiedPath: string
}>()

const docStore = useDocumentManagerStore()
const split = useSplitView()

const original = ref('')
const modified = ref('')
const ready = ref(false)
const errorMsg = ref('')
let loadToken = 0

const originalName = computed(() => baseName(props.originalPath))
const modifiedName = computed(() => baseName(props.modifiedPath))
const language = computed(
  () => docStore.documents.get(props.modifiedPath)?.language || 'plaintext'
)
/** 比较视图恒只读（diff 本身不接受编辑），外观偏好仍可复用但关掉 glyphMargin */
const diffOptions = computed(() => ({
  ...editorOptions.value,
  glyphMargin: false,
  readOnly: true,
  renderSideBySide: !prefs.pureViewMode
}))

function baseName(path: string): string {
  return path.split(/[/\\]/).pop() || path
}

const DIFF_MAX_SIZE_BYTES = 1024 * 1024

async function readText(path: string): Promise<string> {
  const doc = docStore.documents.get(path)
  if (doc && typeof doc.content === 'string') return doc.content
  // >1MB 文档在 store 里是 content:null（TextView 分块读），整读会卡界面：
  // M4 之前不把大文件纳入比较范围（路线图决策 9）
  if (doc && doc.content === null) {
    throw new Error(`${baseName(path)} 是大文件，暂不支持比较`)
  }
  // 未在编辑器中打开的文件同样要先问体积：read_note_file 会整读，
  // 否则「比较一份没打开过的大文件」会绕过上面的 content:null 保护把界面卡住。
  const meta = await call<{ code: number; msg: string; data: { size: number } | null }>(
    'read_file_meta', { path }
  )
  if (meta?.code === 200 && meta.data && meta.data.size > DIFF_MAX_SIZE_BYTES) {
    throw new Error(`${baseName(path)} 超过 1MB，暂不支持比较`)
  }
  const res = await call<{ code: number; msg: string; data: string }>('read_note_file', { path })
  if (res?.code !== 200) throw new Error(res?.msg || `读取失败：${baseName(path)}`)
  return typeof res.data === 'string' ? res.data : ''
}

async function load() {
  const token = ++loadToken
  ready.value = false
  errorMsg.value = ''
  try {
    const [a, b] = await Promise.all([readText(props.originalPath), readText(props.modifiedPath)])
    if (token !== loadToken) return
    original.value = a
    modified.value = b
    ready.value = true
  } catch (e: any) {
    if (token !== loadToken) return
    errorMsg.value = e?.message || '读取文件失败'
    ready.value = false
  }
}

watch(() => [props.originalPath, props.modifiedPath], load, { immediate: true })
</script>

<style scoped lang="scss">
.diff-editor {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.diff-editor-bar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-2);
  flex: none;
  border-bottom: 1px solid var(--borderColor);
  background: var(--bgColor);
  font-size: var(--fontSizeSmall);
}

.diff-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--el-text-color-secondary);
}

.diff-error {
  color: var(--stateErrorFg);
  flex: none;
}

.diff-close {
  margin-left: auto;
  flex: none;
}

.diff-monaco {
  flex: 1;
  min-height: 0;
}

.diff-loading {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--el-text-color-secondary);
  font-size: var(--fontSizeSmall);
}
</style>
