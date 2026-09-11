<template>
  <div class="file-editor-container">
    <!-- 有激活文档时显示编辑器：KeepAlive 缓存切换过的文档实例（max 限制缓存数，超出淘汰最久未访问，
         避免大量文档来回切换后编辑器实例无上限常驻导致内存膨胀） -->
    <KeepAlive v-if="activeDocument" :max="8">
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
import { useDocumentManagerStore } from '../store/documents'
import CodeEditor from './EditorType/CodeEditor.vue'
import ImageEditor from './EditorType/ImageEditor.vue'
import PdfEditor from './EditorType/PdfEditor.vue'
import MdEditor from './EditorType/MdEditor.vue'
import TextView from './EditorType/TextView.vue'

const { fileTreeStore } = useStore()
const docStore = useDocumentManagerStore()

const activeDocument = computed(() => docStore.activeDocument)
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
    case 'code':
    case 'text':
      // 大文件分诊：content 为 string（≤1MB 已整读 / 新建 / 远程已下载）→ monaco 代码编辑器；
      // content 为 null（>1MB，openDocument 未整读）→ TextView 只读分块查看（虚拟滚动，避免卡顿）
      return typeof doc.content === 'string' ? CodeEditor : TextView
    default: return CodeEditor
  }
})

function handleStateUpdate(state: any) {
  if (activeDocument.value&&docStore.activeDocumentPath) {
      docStore.updateViewState(docStore.activeDocumentPath, state)
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
    // 主区占位跟随主面板 appMainBgColor（与 Layout/.app-main 同源）
    background-color: theme.getVar('appMainBgColor');
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
  font-size: theme.getVar('fontSizeXl');
  font-weight: 500;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.welcome-desc {
  margin: 0;
  font-size: theme.getVar('fontSize');

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}
</style>
