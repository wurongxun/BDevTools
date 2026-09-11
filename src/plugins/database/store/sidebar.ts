/**
 * 数据库插件侧边栏 UI 状态：
 * - scriptsExpanded：底部脚本区(ScriptExplorer)是否展开。
 *   默认折叠以给连接树更多空间；连接树右键「新建查询」等入口会自动展开。
 */
import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useDbSidebarStore = defineStore('dbSidebar', () => {
  /** 脚本区(ScriptExplorer)是否展开，默认折叠 */
  const scriptsExpanded = ref(false)

  function toggleScripts() {
    scriptsExpanded.value = !scriptsExpanded.value
  }

  function setScriptsExpanded(v: boolean) {
    scriptsExpanded.value = v
  }

  return { scriptsExpanded, toggleScripts, setScriptsExpanded }
})
