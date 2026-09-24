<template>
  <div class="main-box">
    <!-- 沉浸式：标题栏整条隐藏（v-show：空间让给主区，内部状态保留） -->
    <AppNavbar v-show="!isImmersive" />
    <div class="main-area" :class="{ 'is-resizing': isDragging }">
      <aside ref="sideBoxRef" class="side-box" :class="{ 'is-collapse': isCollapse }"
        :style="!isCollapse ? { width: sidebarWidth + 'px', minWidth: sidebarWidth + 'px', maxWidth: sidebarWidth + 'px', transition: isDragging ? 'none' : '' } : {}">
        <app-sidebar :hideActivityBar="isFocusMode" :immersive="isImmersive" />
      </aside>
      <div v-show="!isCollapse && !useLayoutStore.sidebarPanelCollapsed && hasSidebar" class="resize-handle" :class="{ 'is-dragging': isDragging }" @mousedown="onResizeStart" role="separator" aria-orientation="vertical" tabindex="0" :aria-valuenow="sidebarWidth" :aria-valuemin="activeSidebarCfg.min" :aria-valuemax="activeSidebarCfg.max" aria-label="调整侧边栏宽度" @keydown="onResizeKeydown">
        <div class="resize-handle-line"></div>
      </div>
      <main ref="mainContentRef" class="app-content"
        :class="{ 'is-collapse': isCollapse || isFocusMode, 'has-sidebar': !isCollapse && !isFocusMode && windowWidth >= 1024 }">
        <!-- 主区内容（编辑器/各插件主面板）。停靠面板「占满主区」时整体隐藏 -->
        <div v-show="!dockExpanded" class="editor-wrapper">
          <app-editor-main></app-editor-main>
        </div>

        <!--
          停靠面板（ToolPlugin.dockPanel，如终端）：
          用 v-show 而不是 v-if —— 面板必须常驻挂载，否则每次显隐都会销毁重建 PTY 会话/xterm 实例。
          显隐与高度由 layout store 持有；拖到顶 = 面板占满主区（editor 隐藏），拖到底 = 隐藏面板。
        -->
        <template v-if="dockPlugin">
          <div
            v-show="dockVisible || dockDragging"
            class="dock-resize-handle"
            :class="{ 'is-dragging': dockDragging }"
            role="separator"
            aria-orientation="horizontal"
            tabindex="0"
            :aria-valuenow="dockHeight"
            :aria-valuemin="DOCK_MIN_HEIGHT"
            :aria-valuemax="dockMaxHeight"
            aria-label="调整底部面板高度"
            @mousedown="onDockResizeStart"
            @keydown="onDockResizeKeydown"
          >
            <div class="resize-handle-line"></div>
          </div>
          <div
            v-show="dockVisible"
            class="dock-panel"
            :class="{ 'is-expanded': dockExpanded }"
            :style="dockExpanded ? {} : { height: dockHeight + 'px' }"
          >
            <component :is="dockComponent" />
          </div>
        </template>
      </main>
    </div>
    <AppStatusBar v-show="!isImmersive" />

    <!-- 沉浸式：鼠标移到窗口顶部才滑下来的退出条（含拖拽区与窗口控制） -->
    <AppImmersiveBar />
  </div>
</template>

<script setup lang="ts" name="Layout">
import AppNavbar from './components/AppNavbar/index.vue'
import AppEditorMain from './components/AppEditorMain/index.vue'
import AppImmersiveBar from './components/AppImmersiveBar/index.vue'
import AppSidebar from './components/AppSidebar/index.vue'
import AppStatusBar from './components/AppStatusBar/index.vue'
import useStore from '@/store/index'
import { pluginManager } from '@/core/pluginManager'
import { storeToRefs } from 'pinia'
import { watch, nextTick, onMounted, onUnmounted, ref, reactive, computed } from 'vue'
import { useResizeObserver } from '@vueuse/core'

const { system, useWorkspaceStore, useLayoutStore } = useStore()
const { isCollapse } = storeToRefs(system)

/** 当前窗口是否处于专注模式（窗口预设声明：隐藏活动栏，固定显示第一个插件） */
const isFocusMode = computed(() => useLayoutStore.focusMode)

/**
 * 当前窗口是否处于沉浸式（标题栏按钮随时切：只留插件界面，隐藏标题栏/活动栏/状态栏）
 * 与 focusMode 的区别见 `store/modules/layout.ts` 的 `immersiveMode` 注释
 */
const isImmersive = computed(() => useLayoutStore.immersiveMode)

// const isEditorRoute = computed(() => {
//   return !route.path.startsWith('/App')
// })





const sideBoxRef = ref<HTMLElement | null>(null)
const mainContentRef = ref<HTMLElement | null>(null)
const windowWidth = ref(window.innerWidth)

// ==================== 停靠面板（ToolPlugin.dockPanel） ====================
/** 当前窗口里停靠在底部的插件（如终端）。受窗口预设的插件清单过滤 */
const dockPlugin = computed(() =>
  useLayoutStore.filteredPlugins.find(p => p.dockPanel?.position === 'bottom'),
)
const dockComponent = computed(() => dockPlugin.value?.dockPanel?.component)
const dockVisible = computed(() => useLayoutStore.dockPanelVisible.bottom)
const dockExpanded = computed(() => useLayoutStore.dockPanelExpanded.bottom)
const dockHeight = computed(() => useLayoutStore.dockPanelHeight.bottom)
const dockDragging = ref(false)
/** 面板可拖拽的最大高度：主区至少留 EDITOR_MIN_HEIGHT */
const EDITOR_MIN_HEIGHT = 80
const DOCK_MIN_HEIGHT = dockPlugin.value?.dockPanel?.minHeight ?? 80
const dockMaxHeight = computed(() => {
  const mainH = mainContentRef.value?.clientHeight ?? window.innerHeight
  return Math.max(DOCK_MIN_HEIGHT, mainH - EDITOR_MIN_HEIGHT)
})

const SIDEBAR_MIN_WIDTH = 60
const SIDEBAR_MAX_WIDTH = 700
const SIDEBAR_DEFAULT_WIDTH = 220
/** 活动栏（activity-bar）固定宽度：插件面板收起时侧边栏收缩到该宽度 */
const ACTIVITY_BAR_WIDTH = 48

/** 当前激活插件的侧边栏布局配置（插件可声明 sidebarMinWidth/MaxWidth/DefaultWidth，未声明用全局默认） */
const activeSidebarCfg = computed(() => {
  const plugin = pluginManager.getPlugin(pluginManager.activeToolId.value)
  return {
    min: plugin?.sidebarMinWidth ?? SIDEBAR_MIN_WIDTH,
    max: plugin?.sidebarMaxWidth ?? SIDEBAR_MAX_WIDTH,
    def: plugin?.sidebarDefaultWidth ?? SIDEBAR_DEFAULT_WIDTH,
  }
})

/**
 * 侧栏面板区当前实际显示的插件
 * 专注模式下面板区固定显示第一个启用插件（与 AppSidebar 的判定保持一致），其余情况用激活插件
 */
const activeSidebarPlugin = computed(() => {
  const list = useLayoutStore.filteredPlugins
  if (isFocusMode.value && list.length > 0) return list[0]
  const active = pluginManager.getPlugin(pluginManager.activeToolId.value)
  // 激活插件不在本窗口的显示清单里 → 按「无面板」处理（hasSidebar 为 false）
  // 否则侧栏会为一块并不渲染的面板留出宽度（判定必须与 AppSidebar 的 showPanelArea 对齐）
  if (active && !list.some(p => p.id === active.id)) return undefined
  return active
})

/**
 * 当前插件是否提供侧栏面板（ToolPlugin.sidebarPanel 可选）
 * 未提供时：侧栏收缩到活动栏宽度、隐藏拖拽手柄，主面板占满宽度
 */
const hasSidebar = computed(() => Boolean(activeSidebarPlugin.value?.sidebarPanel))

/** 每个插件独立的侧边栏宽度（持久化存储） */
const pluginSidebarWidths: Record<string, number> = {}

/** 当前侧边栏宽度（直接驱动 DOM） */
const sidebarWidth = ref(SIDEBAR_DEFAULT_WIDTH)

/**
 * 无动画设置侧栏宽度：宽度相同则跳过
 * 加 no-transition 避免 300ms transition 造成「切换后布局慢慢变化」的滞后感
 */
const setSidebarWidthNow = (target: number) => {
  if (sidebarWidth.value === target) return
  const asideEl = sideBoxRef.value
  asideEl?.classList.add('no-transition')
  sidebarWidth.value = target
  requestAnimationFrame(() => asideEl?.classList.remove('no-transition'))
}

/** 插件切换时恢复该插件记住的宽度：无动画、宽度相同则跳过；切换后即时通知一次布局更新（消除延迟链） */
watch(() => pluginManager.activeToolId.value, async (newId) => {
  // 面板收起、或该插件未提供 sidebarPanel：保持活动栏宽度，不恢复插件记住的宽度
  if (useLayoutStore.sidebarPanelCollapsed || !hasSidebar.value) {
    setSidebarWidthNow(ACTIVITY_BAR_WIDTH)
    await nextTick()
    triggerLayoutUpdate('plugin-switch')
    return
  }
  setSidebarWidthNow(pluginSidebarWidths[newId] ?? activeSidebarCfg.value.def)
  // 切换后立即（非 setTimeout 延迟）通知子组件按新容器重新测量布局（Monaco/xterm 等）
  await nextTick()
  triggerLayoutUpdate('plugin-switch')
})

/** 插件面板收起/展开：收起时缩到活动栏宽度，展开时恢复该插件记住的宽度 */
watch(() => useLayoutStore.sidebarPanelCollapsed, async (collapsed) => {
  const activeId = pluginManager.activeToolId.value
  if (collapsed) {
    // 记住收起前的宽度，展开时恢复（无侧栏插件不记，避免把活动栏宽度存成该插件的宽度）
    if (hasSidebar.value) pluginSidebarWidths[activeId] = sidebarWidth.value
    setSidebarWidthNow(ACTIVITY_BAR_WIDTH)
  } else if (hasSidebar.value) {
    setSidebarWidthNow(pluginSidebarWidths[activeId] ?? activeSidebarCfg.value.def)
  }
  await nextTick()
  triggerLayoutUpdate('panel-toggle')
})

/**
 * 活动栏是否被隐藏：专注模式（窗口预设）与沉浸式都会隐藏它
 * 活动栏被隐藏且当前插件没有侧栏面板时，侧栏整条收起（0）——留一个活动栏的宽度会变成一条空白带
 */
const activityBarHidden = computed(() => isFocusMode.value || isImmersive.value)

/** 按「有无侧栏面板 + 活动栏是否隐藏」把侧栏宽度收敛到该有的值 */
const syncSidebarWidth = () => {
  const activeId = pluginManager.activeToolId.value
  if (!hasSidebar.value) {
    setSidebarWidthNow(activityBarHidden.value ? 0 : ACTIVITY_BAR_WIDTH)
  } else if (!useLayoutStore.sidebarPanelCollapsed) {
    setSidebarWidthNow(pluginSidebarWidths[activeId] ?? activeSidebarCfg.value.def)
  }
}

/**
 * 侧栏有无会随激活插件、启用插件列表（布局预设）、沉浸式开关变化
 * 无侧栏：收敛到活动栏宽度（活动栏也被隐藏时为 0）；恢复有侧栏：回到该插件记住的宽度
 */
watch([hasSidebar, isImmersive], () => {
  syncSidebarWidth()
  nextTick(() => triggerLayoutUpdate('sidebar-availability'))
}, { immediate: true })

/**
 * 沉浸式：只留插件界面（插件侧栏面板 + 主面板）。
 *
 * 进入时把折叠的侧边栏与插件面板展开 —— 否则「只显示插件界面」会显示成一片空；退出时**还原**
 * 用户原来的折叠状态，不把临时视图态写进用户的显隐偏好里。
 */
const immersiveRestore = ref<{ sidebarCollapsed: boolean; panelCollapsed: boolean } | null>(null)

watch(isImmersive, async (on) => {
  if (on) {
    immersiveRestore.value = {
      sidebarCollapsed: isCollapse.value,
      panelCollapsed: useLayoutStore.sidebarPanelCollapsed,
    }
    if (isCollapse.value) system.expandCollapse()
    if (useLayoutStore.sidebarPanelCollapsed) useLayoutStore.setSidebarPanelCollapsed(false)
  } else if (immersiveRestore.value) {
    if (immersiveRestore.value.sidebarCollapsed) system.packCollapse()
    useLayoutStore.setSidebarPanelCollapsed(immersiveRestore.value.panelCollapsed)
    immersiveRestore.value = null
  }
  await nextTick()
  triggerLayoutUpdate('immersive-toggle')
})
const isDragging = ref(false)

const onResizeStart = (e: MouseEvent) => {
  e.preventDefault()
  isDragging.value = true
  const startX = e.clientX
  const startWidth = sidebarWidth.value
  const asideEl = sideBoxRef.value
  if (!asideEl) return
  // 拖拽钳制范围取当前激活插件的声明值
  const { min, max } = activeSidebarCfg.value

  let currentWidth = startWidth

  const onMouseMove = (moveEvent: MouseEvent) => {
    currentWidth = Math.min(max, Math.max(min, startWidth + (moveEvent.clientX - startX)))
    // 拖拽期间只操作 DOM，不触发 Vue 响应式更新
    asideEl.style.width = currentWidth + 'px'
    asideEl.style.minWidth = currentWidth + 'px'
    asideEl.style.maxWidth = currentWidth + 'px'
  }

  const onMouseUp = () => {
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    // 同步更新 ref，Vue 重渲染后 :style 绑定会接管宽度
    sidebarWidth.value = currentWidth
    pluginSidebarWidths[pluginManager.activeToolId.value] = currentWidth
    isDragging.value = false
    triggerLayoutUpdate('drag-resize')
  }

  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}

let prevWindowWidth = window.innerWidth

// ==================== 停靠面板拖拽（横向分隔条） ====================

/**
 * 拖拽底部停靠面板的分隔条：
 * - 拖到底（新高度 ≤ 最小高度）→ 隐藏面板（「拖到底隐藏 Terminal」）
 * - 拖到顶（剩余主区 ≤ EDITOR_MIN_HEIGHT）→ 面板占满主区、隐藏 editor（「拖到顶隐藏 editor」）
 * - 中间：正常设高度
 */
const onDockResizeStart = (e: MouseEvent) => {
  e.preventDefault()
  const mainEl = mainContentRef.value
  if (!mainEl) return
  const rect = mainEl.getBoundingClientRect()
  dockDragging.value = true
  document.body.style.cursor = 'row-resize'
  document.body.style.userSelect = 'none'

  let rafId = 0
  const notify = () => {
    if (rafId) return
    rafId = requestAnimationFrame(() => {
      rafId = 0
      triggerLayoutUpdate('dock-resize')
    })
  }

  const onMouseMove = (moveEvent: MouseEvent) => {
    const next = rect.bottom - moveEvent.clientY
    const maxH = rect.height - EDITOR_MIN_HEIGHT
    if (next <= DOCK_MIN_HEIGHT) {
      useLayoutStore.setDockPanelExpanded('bottom', false)
      useLayoutStore.setDockPanelVisible('bottom', false)
    } else if (next >= maxH) {
      useLayoutStore.setDockPanelVisible('bottom', true)
      useLayoutStore.setDockPanelExpanded('bottom', true)
    } else {
      useLayoutStore.setDockPanelExpanded('bottom', false)
      useLayoutStore.setDockPanelVisible('bottom', true)
      useLayoutStore.setDockPanelHeight('bottom', next)
    }
    notify()
  }

  const onMouseUp = () => {
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
    if (rafId) cancelAnimationFrame(rafId)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    dockDragging.value = false
    triggerLayoutUpdate('dock-resize-end')
  }

  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}

/** 键盘调整停靠面板高度（WCAG 2.1.1）：↑↓ ±16px，Home 最小、End 最大 */
const onDockResizeKeydown = (e: KeyboardEvent) => {
  let next = dockHeight.value
  if (e.key === 'ArrowUp') next = Math.min(dockMaxHeight.value, dockHeight.value + 16)
  else if (e.key === 'ArrowDown') next = dockHeight.value - 16
  else if (e.key === 'Home') next = DOCK_MIN_HEIGHT
  else if (e.key === 'End') next = dockMaxHeight.value
  else return
  e.preventDefault()
  if (next <= DOCK_MIN_HEIGHT) {
    useLayoutStore.setDockPanelExpanded('bottom', false)
    useLayoutStore.setDockPanelVisible('bottom', false)
  } else if (next >= dockMaxHeight.value) {
    useLayoutStore.setDockPanelVisible('bottom', true)
    useLayoutStore.setDockPanelExpanded('bottom', true)
  } else {
    useLayoutStore.setDockPanelExpanded('bottom', false)
    useLayoutStore.setDockPanelVisible('bottom', true)
    useLayoutStore.setDockPanelHeight('bottom', next)
  }
  triggerLayoutUpdate('dock-resize-keyboard')
}

/** 停靠面板显隐/占满状态变化后通知子组件按新容器重排（Monaco / xterm） */
watch([dockVisible, dockExpanded], async () => {
  await nextTick()
  triggerLayoutUpdate('dock-toggle')
})

// 触发自定义事件的辅助函数
const triggerLayoutUpdate = (reason: string, collapseState?: boolean) => {
  const event = new CustomEvent('layout-update', {
    detail: {
      isCollapse: collapseState !== undefined ? collapseState : isCollapse.value,
      timestamp: Date.now(),
      reason: reason,
      sidebarWidth: sideBoxRef.value?.offsetWidth || 0,
      mainWidth: mainContentRef.value?.offsetWidth || 0,
      windowWidth: windowWidth.value
    }
  })
  window.dispatchEvent(event)
}

// 监听窗口大小变化，跨过 720px 阈值时自动折叠/展开侧边栏（仅按方向切换，避免缩放抖动）
const handleWindowResize = () => {
  const w = window.innerWidth
  windowWidth.value = w
  if (w < 720 && !isCollapse.value && prevWindowWidth >= 720) {
    system.packCollapse()
  } else if (w >= 720 && isCollapse.value && prevWindowWidth < 720) {
    system.expandCollapse()
  }
  prevWindowWidth = w

  // 触发重新计算
  setTimeout(() => {
    triggerLayoutUpdate('window-resize')
  }, 100)
}

/** 键盘调节侧边栏宽度（WCAG 2.1.1/2.1.2）：分隔条聚焦后 ← → ±16px，Home/End 跳到最小/最大 */
const onResizeKeydown = (e: KeyboardEvent) => {
  const { min, max } = activeSidebarCfg.value
  let next = sidebarWidth.value
  if (e.key === 'ArrowLeft') next = Math.max(min, sidebarWidth.value - 16)
  else if (e.key === 'ArrowRight') next = Math.min(max, sidebarWidth.value + 16)
  else if (e.key === 'Home') next = min
  else if (e.key === 'End') next = max
  else return
  e.preventDefault()
  sidebarWidth.value = next
  pluginSidebarWidths[pluginManager.activeToolId.value] = next
  triggerLayoutUpdate('keyboard-resize')
}

// 监听侧边栏折叠状态变化
watch(isCollapse, async (newValue) => {
  await nextTick()

  // 等待过渡动画完成后再触发事件
  setTimeout(() => {
    triggerLayoutUpdate('collapse-change', newValue)
  }, newValue ? 400 : 350)
})

// 使用 ResizeObserver 监听侧边栏实际宽度变化
useResizeObserver(sideBoxRef, async (entries) => {
  if (isDragging.value) return
  for (const entry of entries) {
    if (entry.target === sideBoxRef.value) {
      await nextTick()

      setTimeout(() => {
        triggerLayoutUpdate('sidebar-resize')
      }, 100)

      console.log('[Layout] Sidebar width changed:', entry.contentRect.width)
      break
    }
  }
})

// 组件挂载时触发初始化事件并设置初始状态
onMounted(async () => {
  await nextTick()

  // 加载工作区列表（用于菜单显示，currentWorkspace 由 App.vue 负责加载）
  await useWorkspaceStore.loadAllWorkspaces()

  // 初始窗口宽度小于 720px 时自动隐藏侧边栏，跨过阈值再展开
  if (windowWidth.value < 720 && !isCollapse.value) {
    system.packCollapse()
  } else if (windowWidth.value >= 720 && isCollapse.value) {
    system.expandCollapse()
  }

  setTimeout(() => {
    triggerLayoutUpdate('initial-mount')
  }, 200)

  // 监听窗口大小变化（仅保留一个 resize 处理，避免双份监听器互相覆盖，审查报告 #33）
  window.addEventListener('resize', handleWindowResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleWindowResize)
})
</script>


<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss';

.main-box {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;
  box-sizing: border-box;

  .main-area {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;

    &.is-resizing {

      .side-box,
      .app-content {
        transition: none !important;
      }
    }

    .side-box {
      flex-shrink: 0;
      height: 100%;
      transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1),
        min-width 0.3s cubic-bezier(0.4, 0, 0.2, 1),
        max-width 0.3s cubic-bezier(0.4, 0, 0.2, 1),
        opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      z-index: 1000;
      box-sizing: border-box;
      overflow: hidden;

      // 插件切换时无动画恢复宽度（避免切换后布局慢慢变化）
      &.no-transition {
        transition: none !important;
      }

      @include theme.useTheme {
        background-color: theme.getVar('bgColor');
        border-right: none;
      }

      &.is-collapse {
        width: 0;
        min-width: 0;
        max-width: 0;
        opacity: 0;
      }
    }

    .resize-handle {
      flex-shrink: 0;
      width: var(--resize-handle-v-line-width);
      cursor: col-resize;
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 1001;
      position: relative;
      background-color: var(--resize-handle-color);
      transition: background-color 0.2s;

      // 扩大命中热区（透明、不占布局）：1px 的线太难精确点中，
      // 向左右各扩 hit-area 让鼠标更容易触发 hover / mousedown。
      &::before {
        content: '';
        position: absolute;
        top: 0;
        bottom: 0;
        left: calc(-1 * var(--resize-handle-hit-area));
        right: calc(-1 * var(--resize-handle-hit-area));
      }

      &:hover,
      &.is-dragging {
        background-color: var(--resize-handle-color-active);
      }

      .resize-handle-line {
        width: var(--resize-handle-v-line-width);
        height: var(--resize-handle-v-line-height);
        border-radius: var(--radius-xs);
        background-color: var(--resize-handle-color-active);
        opacity: 0;
        transition: width 0.2s ease, opacity 0.2s ease;
      }

      // 悬停 / 拖拽中：中线增粗（1px → active 宽度）并显现，提示「此处可拖拽」
      &:hover .resize-handle-line,
      &.is-dragging .resize-handle-line {
        width: var(--resize-handle-v-line-width-active);
        opacity: 1;
      }
    }

    .app-content {
      flex: 1;
      min-width: 0;
      display: flex;
      flex-direction: column;
      height: 100%;
      overflow: hidden;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      box-sizing: border-box;

      &.has-sidebar {
        margin-left: 0;
      }

      .editor-wrapper,
      .page-wrapper {
        flex: 1;
        min-height: 0;
        display: flex;
        flex-direction: column;
        height: 100%;
        overflow: hidden;
      }

      // ---------- 停靠面板（底部终端等） ----------
      // 横向分隔条：与竖向分隔条共用同一组令牌，但**刻意不使用 .resize-handle 类名**——
      // `.main-area .resize-handle` 是后代选择器，会连带命中嵌在 app-content 里的横向手柄，
      // 把 width/cursor 覆盖成竖向的值。独立类名避免这类隐性继承。
      .dock-resize-handle {
        flex-shrink: 0;
        width: 100%;
        height: var(--resize-handle-h-line-height);
        cursor: row-resize;
        display: flex;
        align-items: center;
        justify-content: center;
        position: relative;
        background-color: var(--resize-handle-color);
        transition: background-color 0.2s;

        // 上下各扩一条不可见热区（1px 的线太细，不扩几乎点不中）
        &::before {
          content: '';
          position: absolute;
          left: 0;
          right: 0;
          top: calc(-1 * var(--resize-handle-hit-area));
          bottom: calc(-1 * var(--resize-handle-hit-area));
        }

        &:hover,
        &.is-dragging {
          background-color: var(--resize-handle-color-active);
        }

        .resize-handle-line {
          width: var(--resize-handle-v-line-height);
          height: var(--resize-handle-h-line-height);
          border-radius: var(--radius-xs);
          background-color: var(--resize-handle-color-active);
          opacity: 0;
          transition: height 0.2s ease, opacity 0.2s ease;
        }

        &:hover .resize-handle-line,
        &.is-dragging .resize-handle-line {
          height: var(--resize-handle-h-line-height-active);
          opacity: 1;
        }
      }

      // 停靠面板容器：高度由 JS 直接给（拖拽期间需要逐帧跟手，不适合走 CSS 过渡）
      .dock-panel {
        flex: 0 0 auto;
        min-height: 0;
        overflow: hidden;
        display: flex;
        box-sizing: border-box;

        &.is-expanded {
          flex: 1 1 auto;
          height: auto !important;
        }
      }
    }
  }
}
</style>