import { markRaw } from 'vue'
import { Folder } from '@element-plus/icons-vue'
import type { ToolPlugin } from '@/core/pluginManager'
import FileTree from '@/Layout/components/AppSidebar/components/FileTree.vue'
import FileEditor from '@/components/FileEditor/FileEditor.vue'
import EditorTab from '@/Layout/components/AppEditorMain/EditorTab.vue'

export default {
  id: 'files',
  label: '文件',
  icon: markRaw(Folder),
  sidebarPanel: markRaw(FileTree),
  mainPanel: markRaw(FileEditor),
  tabBar: markRaw(EditorTab),
  order: 1
} satisfies ToolPlugin
