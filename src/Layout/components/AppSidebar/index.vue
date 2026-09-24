<template>
  <div class="sidebar-container">
    <!-- 活动栏（左侧图标列）：hideActivityBar（专注模式）或 immersive（沉浸式）时隐藏
         语义：垂直 tablist + roving tabindex，支持 ↑↓/Home/End 键盘切换（WCAG 2.1.1 / 4.1.2） -->
    <div v-show="!hideActivityBar && !immersive" class="activity-bar">
      <!-- 插件图标列表（roving tabindex 的 tablist；固定在底部的宿主入口不在此列） -->
      <div class="activity-list" role="tablist" aria-label="插件面板" aria-orientation="vertical">
        <button
          v-for="(plugin, index) in listPlugins"
          :key="plugin.id"
          :ref="(el) => setItemRef(el, index)"
          class="activity-item"
          type="button"
          role="tab"
          :class="{ 'activity-item-active': isPluginActive(plugin.id) }"
          :aria-selected="isPluginActive(plugin.id)"
          :tabindex="isPluginActive(plugin.id) ? 0 : -1"
          :title="activityTitle(plugin)"
          :aria-label="activityTitle(plugin)"
          @click="onActivityClick(plugin.id)"
          @keydown="onActivityKeydown($event, index)"
        >
          <component :is="plugin.icon" class="activity-icon" aria-hidden="true" />
        </button>
      </div>

      <!-- 底部常驻入口（pinnedBottom，如「设置」）：对齐 VS Code 活动栏底部的齿轮位 -->
      <div v-if="bottomPlugins.length" class="activity-bottom">
        <button
          v-for="plugin in bottomPlugins"
          :key="plugin.id"
          class="activity-item"
          type="button"
          :class="{ 'activity-item-active': pluginManager.activeToolId.value === plugin.id }"
          :aria-pressed="pluginManager.activeToolId.value === plugin.id"
          :title="plugin.label"
          :aria-label="plugin.label"
          @click="onActivityClick(plugin.id)"
        >
          <component :is="plugin.icon" class="activity-icon" aria-hidden="true" />
        </button>
      </div>
    </div>

    <!-- 面板区域：动态渲染当前激活插件的侧边栏面板（KeepAlive 常驻：切走 deactivated 缓存实例、卸载 DOM，切回 activated 恢复，不重建）
         收起（sidebarPanelCollapsed）时隐藏；专注模式（hideActivityBar）下始终显示；
         插件未提供 sidebarPanel 时整个面板区不渲染（由 Layout 把侧栏收缩到活动栏宽度，主面板占满） -->
    <div v-show="showPanelArea" class="panel-area">
      <template v-for="plugin in displayPlugins" :key="plugin.id">
        <KeepAlive>
          <component :is="plugin.sidebarPanel" v-if="plugin.sidebarPanel && isPanelVisible(plugin.id)" />
        </KeepAlive>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts" name="AppSidebar">
import { computed, ref, nextTick } from 'vue'
import { pluginManager } from '@/core/pluginManager'
import { launchStandalonePlugin } from '@/core/plugin/launch'
import { isPluginWindowOpen } from '@/core/plugin/window'
import useStore from '@/store/index'

const { useLayoutStore } = useStore()

const props = withDefaults(defineProps<{
  /** 是否隐藏活动栏（专注模式时只隐藏图标列，保留插件面板） */
  hideActivityBar?: boolean
  /**
   * 沉浸式：同样隐藏活动栏，但**保留当前激活插件**的面板并直显（不会像专注模式那样改成第一个插件）。
   * 与 hideActivityBar 的差别只在面板归属判定上，故不复用同一个 prop。
   */
  immersive?: boolean
}>(), {
  hideActivityBar: false,
  immersive: false
})

/** 当前窗口显示的插件列表（根据 layout store 的 enabledPlugins 过滤） */
const displayPlugins = computed(() => useLayoutStore.filteredPlugins)

/** 活动栏图标列表（不含固定在底部的宿主入口，如「设置」） */
const listPlugins = computed(() => displayPlugins.value.filter(p => !p.pinnedBottom))

/** 固定在活动栏底部的常驻入口（pinnedBottom，如「设置」） */
const bottomPlugins = computed(() => displayPlugins.value.filter(p => p.pinnedBottom))

/** 插件面板是否收起（仅隐藏面板区，保留活动栏） */
const sidebarPanelCollapsed = computed(() => useLayoutStore.sidebarPanelCollapsed)

/**
 * 面板区当前实际渲染的插件
 * 专注模式固定显示第一个启用插件（与下面的 isPanelVisible 判定保持一致）
 */
const visiblePanelPlugin = computed(() => {
  if (props.hideActivityBar) return displayPlugins.value[0]
  const active = pluginManager.getPlugin(pluginManager.activeToolId.value)
  // 当前激活插件不在本窗口的显示清单里（本窗口把它「移出」了，或窗口预设的清单没勾选它）
  // → 不能按「有面板」处理：那会渲染出一块没有图标可回去的孤儿面板
  if (active && !displayPlugins.value.some(p => p.id === active.id)) return undefined
  return active
})

/**
 * 面板区是否需要渲染
 * 当前插件未提供 sidebarPanel 时整体不显示，避免留下一片空白条
 * （宽度收敛由 Layout 负责：收缩到活动栏宽度，主面板占满）
 */
const showPanelArea = computed(() =>
  Boolean(visiblePanelPlugin.value?.sidebarPanel)
  && (props.hideActivityBar || !sidebarPanelCollapsed.value)
)

/**
 * 点击活动栏图标
 * - 停靠面板插件（`dockPanel`，如终端）：图标是**显示/隐藏开关**，不切主面板
 * - 独立窗口型插件（`standaloneWindow`，如游戏）：图标是**打开/前置独立窗口**
 *   （主面板仍切过去 —— 那是宿主生成的启动卡，用于关掉窗口后重开）
 * - 普通插件：切换插件；若面板当前处于收起状态则自动展开（否则点了看不到面板）
 *   目标插件没有侧栏时不展开——避免无侧栏插件把用户的面板显隐偏好改掉
 *
 * @param activate 是否算「用户主动激活」。鼠标点击与 Enter/Space 为 true；
 *   **↑↓/Home/End 导航为 false** —— 键盘扫过一遍列表不该顺手开出一堆窗口、
 *   也不该顺手把终端面板开开关关。
 */
function onActivityClick(pluginId: string, activate = true) {
  const plugin = pluginManager.getPlugin(pluginId)
  if (plugin?.dockPanel) {
    if (activate) useLayoutStore.toggleDockPanel(plugin.dockPanel.position)
    return
  }
  pluginManager.switchTool(pluginId)
  if (plugin?.standaloneWindow && activate) {
    void launchStandalonePlugin(pluginId)
  }
  if (useLayoutStore.sidebarPanelCollapsed && plugin?.sidebarPanel) {
    useLayoutStore.setSidebarPanelCollapsed(false)
  }
}

/** 活动栏图标的激活态：停靠面板插件看面板显隐，独立窗口插件看窗口是否开着，其余看是否被激活 */
function isPluginActive(pluginId: string): boolean {
  const plugin = pluginManager.getPlugin(pluginId)
  if (plugin?.dockPanel) return !!useLayoutStore.dockPanelVisible[plugin.dockPanel.position]
  return pluginManager.activeToolId.value === pluginId
}

/**
 * 图标 tooltip。
 * - 停靠面板插件补一句当前动作，避免「点了没反应」的困惑
 * - 独立窗口插件同理：它的窗口开着时点击是「前置」，关着时是「打开」
 */
function activityTitle(plugin: { label: string; dockPanel?: { position: string }; standaloneWindow?: boolean; id: string }): string {
  if (plugin.dockPanel) {
    return useLayoutStore.dockPanelVisible[plugin.dockPanel.position]
      ? `${plugin.label}（点击隐藏）`
      : `${plugin.label}（点击显示）`
  }
  if (plugin.standaloneWindow) {
    return isPluginWindowOpen(plugin.id)
      ? `${plugin.label}（独立窗口已打开，点击前置）`
      : `${plugin.label}（点击在独立窗口打开）`
  }
  return plugin.label
}

/**
 * 键盘导航：↑↓ 循环切换、Home/End 跳首尾（垂直 tablist 标准交互）
 * 切换后立即把焦点移到新项上，保持 roving tabindex 的焦点跟随
 */
function onActivityKeydown(e: KeyboardEvent, index: number) {
  const count = listPlugins.value.length
  if (count === 0) return

  let next = -1
  switch (e.key) {
    case 'ArrowDown':
      next = (index + 1) % count
      break
    case 'ArrowUp':
      next = (index - 1 + count) % count
      break
    case 'Home':
      next = 0
      break
    case 'End':
      next = count - 1
      break
    default:
      return
  }

  e.preventDefault()
  onActivityClick(listPlugins.value[next].id, false)
  nextTick(() => activityItemRefs.value[next]?.focus())
}

/** 收集活动栏按钮 DOM，供方向键导航后移动焦点 */
const activityItemRefs = ref<HTMLElement[]>([])
function setItemRef(el: unknown, index: number) {
  if (el) activityItemRefs.value[index] = el as HTMLElement
}

/**
 * 判断插件面板是否可见
 * - 活动栏隐藏时（专注模式）：显示第一个插件的面板
 * - 正常模式：显示当前激活插件的面板
 */
function isPanelVisible(pluginId: string): boolean {
  if (props.hideActivityBar) {
    // 专注模式：自动显示第一个插件的面板
    return displayPlugins.value.length > 0 && displayPlugins.value[0].id === pluginId
  }
  return pluginManager.activeToolId.value === pluginId
}
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss' as theme;

.sidebar-container {
  display: flex;
  height: 100%;
  overflow: hidden;
}

// 活动栏（左侧图标列）：上方为插件列表，下方为固定在底部的宿主入口
.activity-bar {
  display: flex;
  flex-direction: column;
  width: 48px;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('activityBarBg');
    // 分隔线用 inset 阴影画，**不占布局**：border-right 会把按钮挤成 47px，
    // 图标随之内缩（实测左 12.5 / 右 13.5）。阴影落在同一个像素上，视觉完全一致。
    box-shadow: inset -1px 0 0 theme.getVar('borderColor');
  }
}

.activity-list {
  display: flex;
  flex-direction: column;
}

// 底部常驻入口（如「设置」）：推到底部并与上方插件列表做视觉分隔
.activity-bottom {
  margin-top: auto;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  padding-top: 4px;

  @include theme.useTheme {
    border-top: 1px solid theme.getVar('borderColor');
  }
}

.activity-item {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 48px;
  cursor: pointer;
  transition: all 0.15s;
  // 语义化 <button> 需要抹平浏览器默认外观
  appearance: none;
  border: none;
  padding: 0;
  font-family: inherit;

  @include theme.useTheme {
    background-color: theme.getVar('activityBarBg');
    color: theme.getVar('textSecondary');

    &:hover {
      color: theme.getVar('textColor');
      background-color: theme.getVar('activityBarHover');
    }
  }
}

// 选中指示条（左侧 2px）用 inset 阴影画，**不能用 border-left**：
// 边框会占掉内容宽度，把 justify-content:center 的图标挤出中心
// （历史 bug：左 13.5 / 右 11.5，偏右 1px）。
.activity-item-active {
  @include theme.useTheme {
    box-shadow: inset 2px 0 0 theme.getVar('hoverColor');
    color: theme.getVar('activityBarActive') !important;
    background-color: theme.getVar('activityBarHover');
  }
}

.activity-icon {
  width: 22px;
  height: 22px;
}

// 面板区域
.panel-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
  }
}

.panel-content {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
</style>
