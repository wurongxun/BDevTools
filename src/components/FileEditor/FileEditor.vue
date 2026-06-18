<template>
  <div class="h-full w-full overflow-hidden">
    <KeepAlive :include="cachedPaths">
      <component
        :is="currentEditorComponent"
        :key="activePath"
        :path="activePath"
        @update-state="handleStateUpdate"
      />
    </KeepAlive>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import useStore from '@/store'
import CodeEditor from './CodeEditor.vue'
import ImageEditor from './ImageEditor.vue'
import PdfEditor from './PdfEditor.vue'
const { routerNoteStore, tagview, useDocumentManagerStore } = useStore()

const activePath = computed(() => useDocumentManagerStore.activePath)

// 根据文件类型动态选择组件
const currentEditorComponent = computed(() => {
  if (!activePath.value) return null
  const doc = useDocumentManagerStore.documents.get(activePath.value)
  if (!doc) return null

  switch (doc.type) {
    case 'md': return ImageEditor
    case 'pdf': return PdfEditor
    default: return CodeEditor // 默认处理代码和文本
  }
})

// 缓存所有已打开的文件路径，确保 KeepAlive 生效
const cachedPaths = computed(() => Array.from(useDocumentManagerStore.documents.keys()))

function handleStateUpdate(state: any) {
  if (activePath.value) {
    useDocumentManagerStore.updateViewState(activePath.value, state)
  }
}
</script>
