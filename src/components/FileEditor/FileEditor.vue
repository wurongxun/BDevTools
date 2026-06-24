<template>
    <KeepAlive :include="cachedPaths">
      <component
        :is="currentEditorComponent"
        :key="activePathId"
        :path="activePath"
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
import EditorTab from '../../Layout/components/AppEditorMain/EditorTab.vue'
import ToggleSwitch from '@/Layout/components/AppNavbar/components/ToggleSwitch.vue'
import FunctionalArea from '@/Layout/components/AppNavbar/components/FunctionalArea/index.vue'
import BreadCrumb from '@/Layout/components/AppNavbar/components/BreadCrumb.vue'
const { routerNoteStore, tagview, useDocumentManagerStore } = useStore()

const activePath = computed(() => useDocumentManagerStore.activeDocument)
const activePathId = computed(() => activePath.value?.id || '')
// 根据文件类型动态选择组件
const currentEditorComponent = computed(() => {
  if (!activePath.value) return null
  const doc = activePath.value;
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
  if (activePath.value) {
    useDocumentManagerStore.updateViewState(useDocumentManagerStore.activeDocumentId, state)
  }
}
</script>
