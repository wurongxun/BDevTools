<template>
    <KeepAlive>
      <component
        :is="currentEditorComponent"
        :key="activeKey"
        :path="activePathId"
        @update-state="handleStateUpdate"
      />
    </KeepAlive>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import useStore from '@/store'
import CodeEditor from './EditorType/CodeEditor.vue'
import ImageEditor from './EditorType/ImageEditor.vue'
import PdfEditor from './EditorType/PdfEditor.vue'
import MdEditor from './EditorType/MdEditor.vue'
const { routerNoteStore,  useDocumentManagerStore } = useStore()

const activeDocument = computed(() => useDocumentManagerStore.activeDocument)
const activePathId = computed(() => activeDocument.value?.id || '')
const activeKey = computed(() => {
  const doc = activeDocument.value
  if (!doc) return ''
  return `${doc.type}:${doc.key}`
})
const currentEditorComponent = computed(() => {
  if (!activeDocument.value) return null
  const doc = activeDocument.value;

  switch (doc.type) {
    case 'markdown': return MdEditor
    case 'pdf': return PdfEditor
    default: return CodeEditor
  }
})

function handleStateUpdate(state: any) {
  if (activeDocument.value&&useDocumentManagerStore.activeDocumentId) {
      useDocumentManagerStore.updateViewState(useDocumentManagerStore.activeDocumentId, state)
  }
}
</script>
