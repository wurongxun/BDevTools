import { markRaw } from 'vue'
import { Folder } from '@element-plus/icons-vue'
import type { ToolPlugin } from '@/core/pluginManager'
import SidebarPanel from './sidebar/SidebarPanel.vue'
import FileEditor from './editors/FileEditor.vue'
import EditorTab from './tabs/EditorTab.vue'

export default {
  id: 'files',
  label: '文件',
  description: '文件管理：目录浏览、打开与编辑、最近打开与会话恢复',
  icon: markRaw(Folder),
  sidebarPanel: markRaw(SidebarPanel),
  mainPanel: markRaw(FileEditor),
  tabBar: markRaw(EditorTab),
  workspaceDirs: ['temp'],
  order: 1
} satisfies ToolPlugin
