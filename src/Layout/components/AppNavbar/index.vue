<template>
  <!-- 沉浸式下整条标题栏隐藏（v-show 而非 v-if：保留内部状态与已加载的主题设置，
       空间由 flex 列自动让给主区；拖拽区与窗口控制由 AppImmersiveBar 临时接管）
       双击 / 右键：无边框窗口没有 OS 那条标题栏，这两个手势得自己补 -->
  <header
    v-show="!immersive"
    class="topbar-box"
    data-tauri-drag-region
    @dblclick="onTitleBarDblClick"
    @contextmenu="onTitleBarContextMenu"
  >
    <!-- 快捷操作区（菜单左侧）：主题 / 侧边栏 / 插件面板 / 沉浸式 -->
    <quick-actions></quick-actions>
    <div class="menu-box" data-tauri-drag-region>
      <menu-bar></menu-bar>
    </div>
    <div class="functional-area-box">
      <functional-area></functional-area>
    </div>

    <!-- 标题栏右键的「系统菜单」：还原 / 最小化 / 最大化 / 关闭。
         原生窗口这一条由 OS 提供（图标右键、双击最大化都靠它），无边框窗口要自己实现。
         没有「移动 / 大小」：那两项是 Win32 的键盘移动模式，Tauri 无对应 API。 -->
    <ContextMenu
      :visible="sysMenuVisible"
      :menu-items="sysMenuItems"
      :position="sysMenuPosition"
      @select="sysMenuVisible = false"
      @close="sysMenuVisible = false"
    />
  </header>
</template>

<script lang="ts" setup name="AppNavbar">import { computed, ref } from 'vue'
import { storeToRefs } from 'pinia'
import ContextMenu from '@/components/context-menu/ContextMenu.vue'
import type { MenuItem } from '@/components/context-menu/ContextMenu.vue'
import { useWindowCommands } from '@/composables/useWindowCommands'
import FunctionalArea from './components/FunctionalArea/index.vue'
import MenuBar from './components/MenuBar/index.vue'
import QuickActions from './components/QuickActions/index.vue'
import useStore from '@/store'

const store = useStore()

/** 沉浸式：只显示插件界面，整条标题栏（含菜单、快捷操作、窗口控制）隐藏 */
const { immersiveMode: immersive } = storeToRefs(store.useLayoutStore)

// ==================== 无边框窗口补的两条原生手势 ====================

// 最大化状态与窗口命令都走同一个出口（与 WindowControls / 系统菜单共一份事实源）
const { isMaximized: windowMaximized, syncMaximized, runWindowCommand } = useWindowCommands()
const sysMenuVisible = ref(false)
const sysMenuPosition = ref({ x: 0, y: 0 })

/**
 * 命中可交互元素时不接管手势：按钮（快捷操作 / 窗口控制）、菜单项、以及已展开的下拉面板
 * 上的双击与右键归它自己 —— 否则双击主题按钮会顺带把窗口最大化。
 */
function isInteractive(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false
  return Boolean(target.closest('button, a, input, textarea, select, [role="menuitem"], [role="menu"]'))
}

/** 双击标题栏切换最大化（原生标题栏白送的手势，无边框窗口自己补） */
function onTitleBarDblClick(e: MouseEvent) {
  if (isInteractive(e.target)) return
  void runWindowCommand('toggle')
}

/** 右键标题栏弹出系统菜单 */
async function onTitleBarContextMenu(e: MouseEvent) {
  if (isInteractive(e.target)) return
  // 压掉 WebView2 自带的浏览器右键菜单
  e.preventDefault()
  e.stopPropagation()
  sysMenuPosition.value = { x: e.clientX, y: e.clientY }
  // 先取真实状态再显示：菜单里哪一项置灰取决于当前是否最大化
  await syncMaximized()
  sysMenuVisible.value = true
}

/** 系统菜单项（对齐 Windows 系统菜单的语义与排序） */
const sysMenuItems = computed<MenuItem[]>(() => [
  { label: '还原', disabled: !windowMaximized.value, onClick: () => void runWindowCommand('unmaximize') },
  { label: '最小化', onClick: () => void runWindowCommand('minimize') },
  { label: '最大化', disabled: windowMaximized.value, onClick: () => void runWindowCommand('maximize') },
  { divider: true },
  { label: '关闭', shortcut: 'Alt+F4', onClick: () => void runWindowCommand('close') },
])
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as variables;
@use '@/styles/theme.scss';

.topbar-box {
  height: $header-height;
  // 必须的：flex 列布局里分项默认可收缩（flex-shrink:1），主区内容一旦撑过剩余空间，
  // 标题栏就会被压扁/被挤出顶部。标题栏是固定尺寸的壳，永远不参与收缩。
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 4px;
  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.logo-box {
  user-select: none;
  overflow: hidden;
  width: 24px;
  height: $header-height;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  flex-shrink: 0;
}

// 固定尺寸：百分比写法在收缩布局中会按原图尺寸撑开，破坏标题栏比例
.logo {
  width: 20px;
  height: 20px;
}

.menu-box {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  height: 100%;
}

.functional-area-box {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  // 与窗口控制按钮拉开距离，避免图标贴边
  margin-right: 1px;
}
</style>
