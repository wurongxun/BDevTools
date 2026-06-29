<template>
    <KeepAlive :include="cachedPaths">
      <component
        :is="currentEditorComponent"
        :key="activePathId"
        :path="activePathId"
        @update-state="handleStateUpdate"
      />
    </KeepAlive>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import useStore from '@/store'
import CodeEditor from './EditorType/CodeEditor.vue'
import ImageEditor from './EditorType/ImageEditor.vue'
import PdfEditor from './EditorType/PdfEditor.vue'
import MdEditor from './EditorType/MdEditor.vue'
const { routerNoteStore, tagview, useDocumentManagerStore } = useStore()

const activeDocument = computed(() => useDocumentManagerStore.activeDocument)
const activePathId = computed(() => activeDocument.value?.id || '')
// 根据文件类型动态选择组件
const currentEditorComponent = computed(() => {
  if (!activeDocument.value) return null
  const doc = activeDocument.value;
  if (!doc) return null

  switch (doc.type) {
    case 'markdown': return MdEditor
    case 'pdf': return PdfEditor
    default: return CodeEditor // 默认处理代码和文本
  }
})

// 缓存所有已打开的文件路径，确保 KeepAlive 生效
const cachedPaths = computed(() => Array.from(useDocumentManagerStore.documents.keys()))

function handleStateUpdate(state: any) {
  if (activeDocument.value) {
    useDocumentManagerStore.updateViewState(useDocumentManagerStore.activeDocumentId, state)
  }
}
</script>
