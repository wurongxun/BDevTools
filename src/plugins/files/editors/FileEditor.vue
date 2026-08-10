<template>
  <div class="file-editor-container">
    <!-- 有激活文档时显示编辑器 -->
    <KeepAlive v-if="activeDocument">
      <component
        :is="currentEditorComponent"
        :key="activeKey"
        :path="activePathId"
        @update-state="handleStateUpdate"
      />
    </KeepAlive>
    <!-- 无文档时显示占位页 -->
    <div v-else class="editor-welcome-placeholder">
      <div class="welcome-content">
        <el-icon :size="64" class="welcome-icon">
          <Document />
        </el-icon>
        <p class="welcome-desc">从左侧文件树中选择文件开始编辑</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Document } from '@element-plus/icons-vue'
import useStore from '@/store'
import CodeEditor from './EditorType/CodeEditor.vue'
import ImageEditor from './EditorType/ImageEditor.vue'
import PdfEditor from './EditorType/PdfEditor.vue'
import MdEditor from './EditorType/MdEditor.vue'
import TextView from './EditorType/TextView.vue'

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
    case 'image': return ImageEditor
    case 'code': return TextView
    case 'text': return TextView
    default: return CodeEditor
  }
})

function handleStateUpdate(state: any) {
  if (activeDocument.value&&useDocumentManagerStore.activeDocumentPath) {
      useDocumentManagerStore.updateViewState(useDocumentManagerStore.activeDocumentPath, state)
  }
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.file-editor-container {
  width: 100%;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.editor-welcome-placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
  }
}

.welcome-content {
  text-align: center;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.welcome-icon {
  margin-bottom: 16px;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.welcome-title {
  margin: 0 0 8px 0;
  font-size: 20px;
  font-weight: 500;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.welcome-desc {
  margin: 0;
  font-size: 14px;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}
</style>
