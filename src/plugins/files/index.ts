import { markRaw } from 'vue'
import { Folder } from '@element-plus/icons-vue'
import type { ToolPlugin } from '@/core/pluginManager'
import SidebarPanel from './sidebar/SidebarPanel.vue'
import FileEditor from './editors/FileEditor.vue'
import EditorTab from './tabs/EditorTab.vue'

export default {
  id: 'files',
  label: '文件',
  icon: markRaw(Folder),
  sidebarPanel: markRaw(SidebarPanel),
  mainPanel: markRaw(FileEditor),
  tabBar: markRaw(EditorTab),
  order: 1
} satisfies ToolPlugin
