import { markRaw } from 'vue'
import { Folder } from '@element-plus/icons-vue'
import type { BuiltinPluginImpl } from '@/core/pluginManager'
import { lazyView } from '@/core/plugin/lazy'
import { registerCommands } from '@/core/commands'
import { filesCommands } from './commands/editorCommands'

/**
 * 面板一律 `lazyView`：图标同步（活动栏要立刻画出来），面板按需（用到才拉 chunk）。
 * `lazyView` 的第一个参数是本插件 id，必须与 `plugin.json` 一致（守卫脚本会查）。
 *
 * 命令随插件 id 注册，`withdrawPlugin('files')` 会整组撤销，无需手动注销；
 * `contributionRegistry.add` 按命令 id 幂等，重复 onActivate 不会堆重复项。
 * 命令表与快捷键执行体共用 `commands/editorCommands.ts` 一处真源（M3）。
 */
export default {
  icon: markRaw(Folder),
  sidebarPanel: lazyView('files', 'sidebar', () => import('./sidebar/SidebarPanel.vue')),
  mainPanel: lazyView('files', 'main', () => import('./editors/FileEditor.vue')),
  tabBar: lazyView('files', 'tabBar', () => import('./tabs/EditorTab.vue')),
  workspaceDirs: ['temp'],
  order: 1,
  onActivate() {
    registerCommands('files', filesCommands())
  }
} satisfies BuiltinPluginImpl
