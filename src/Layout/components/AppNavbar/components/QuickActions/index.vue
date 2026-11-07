<template>
  <!-- 标题栏左侧快捷操作区（菜单左边）： 侧边栏 / 插件面板 / 沉浸式 / 主题 。
       原先这组按钮在右侧 FunctionalArea 里，现整体搬到菜单左侧，四个按钮排在一起。 -->
  <div class="quick-actions">


    <!-- 切换侧边栏 -->
    <button
      type="button"
      class="qa-icon"
      :class="{ 'is-active': !isCollapse }"
      :aria-pressed="!isCollapse"
      :aria-label="isCollapse ? '显示侧边栏' : '隐藏侧边栏'"
      :title="isCollapse ? '显示侧边栏' : '隐藏侧边栏'"
      @click="toggleSidebar"
    >
      <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor" aria-hidden="true">
        <rect x="3" y="3" width="18" height="18" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/>
        <rect x="3" y="3" width="6" height="18" rx="1" :fill="!isCollapse ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="1.2" opacity="0.9"/>
      </svg>
    </button>

    <!-- 隐藏/显示插件面板（仅收起 sidebarPanel，保留活动栏图标列）；当前插件没有侧栏时禁用。
         图标与「切换侧边栏」同源：外框 + 右侧栏。面板显示时右侧栏实心、隐藏时空心，
         与 VS Code「切换辅助侧栏」语义一致（面板在右侧）。 -->
    <button
      type="button"
      class="qa-icon"
      :class="{ 'is-active': !sidebarPanelCollapsed }"
      :disabled="!hasSidebar"
      :aria-pressed="!sidebarPanelCollapsed"
      :aria-label="sidebarToggleLabel"
      :title="sidebarToggleLabel"
      @click="toggleSidebarPanel"
    >
      <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor" aria-hidden="true">
        <rect x="3" y="3" width="18" height="18" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/>
        <rect x="9" y="3" width="6" height="18" rx="1" :fill="!sidebarPanelCollapsed ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="1.2" opacity="0.9"/>
      </svg>
    </button>

    <!-- 沉浸式：只留插件界面（插件侧栏面板 + 主面板），隐藏标题栏 / 活动栏 / 状态栏。
         进入后本按钮随标题栏一起隐藏，退出走窗口顶部弹出的「退出沉浸式」、Esc，
         或再次按下同一个切换键（Windows/Linux F11、macOS Control+Command+F）。 -->
    <button
      type="button"
      class="qa-icon"
      :class="{ 'is-active': immersiveMode }"
      :aria-pressed="immersiveMode"
      :aria-label="immersiveMode ? '退出沉浸式' : '进入沉浸式，只显示插件界面'"
      :aria-keyshortcuts="IMMERSIVE_TOGGLE_ARIA"
      :title="immersiveTitle"
      @click="toggleImmersive"
    >
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" aria-hidden="true">
        <!-- 外壳（标题栏/活动栏/状态栏）：沉浸式下被隐藏，故画成淡色虚线。
             虚线不透明度取 0.75 而不是更低 —— 图形要素属「非文本」，需 ≥3:1（WCAG 1.4.11），
             实测 0.75 在浅色 ≈3.35:1、暗色 ≈3.2:1，再低就不达标了 -->
        <rect x="2.5" y="4.5" width="19" height="15" rx="2" stroke="currentColor" stroke-width="1.3" opacity="0.75" stroke-dasharray="3 2.4"/>
        <!-- 插件界面（侧栏面板 + 主面板）：沉浸式下保留，故实心 -->
        <rect x="6" y="8" width="12" height="8" rx="1.5" fill="currentColor"/>
      </svg>
    </button>
    <!-- 主题切换：开关语义用 aria-pressed 表达（WCAG 4.1.2） -->
    <button
      type="button"
      class="qa-icon"
      :aria-label="isDark ? '切换到浅色主题' : '切换到深色主题'"
      :aria-pressed="isDark"
      :title="isDark ? '切换到浅色主题' : '切换到深色主题'"
      @click="dark_change"
    >
      <el-icon class="qa-el-icon">
        <Moon v-if="isDark" />
        <Sunny v-else />
      </el-icon>
    </button>
  </div>
</template>

<script lang="ts" setup name="QuickActions">
import { computed, onMounted } from 'vue'
import { storeToRefs } from 'pinia'
import { Sunny, Moon } from '@element-plus/icons-vue'
import { load, Store } from '@/core/runtime'
import { pluginManager } from '@/core/pluginManager'
import { isDark, toggleDark } from '@/composables/useDark'
import useStore from '@/store'
import {
  IMMERSIVE_EXIT_LABEL,
  IMMERSIVE_TOGGLE_ARIA,
  IMMERSIVE_TOGGLE_LABEL,
} from '@/composables/useImmersiveShortcut'

const store = useStore()

const { isCollapse } = storeToRefs(store.system)
const { sidebarPanelCollapsed, immersiveMode } = storeToRefs(store.useLayoutStore)

/** 当前激活插件是否提供侧栏面板（未提供时面板切换按钮无意义，禁用并给出说明） */
const hasSidebar = computed(() => pluginManager.activeHasSidebar)

/** 面板切换按钮的文案（无障碍标签与 tooltip 共用，避免两处判断分叉） */
const sidebarToggleLabel = computed(() => {
  if (!hasSidebar.value) return '当前插件没有侧栏面板'
  return sidebarPanelCollapsed.value ? '显示插件面板' : '隐藏插件面板'
})

/**
 * 沉浸式按钮的 tooltip：两种状态 + 对应的键盘快捷键。
 * 快捷键文案来自 `@/composables/useImmersiveShortcut`（与全局按键处理同一事实源），
 * 平台相关的键名（F11 / Control+Command+F）不在这里各写一份。
 */
const immersiveTitle = computed(() =>
  immersiveMode.value
    ? `退出沉浸式（${IMMERSIVE_EXIT_LABEL}）`
    : `进入沉浸式，只显示插件界面（${IMMERSIVE_TOGGLE_LABEL}）`
)

// ========== 侧边栏切换 ==========
function toggleSidebar(){
  if (isCollapse.value) {
    store.system.expandCollapse()
  } else {
    store.system.packCollapse()
  }
}

// ========== 插件面板切换（仅隐藏 sidebarPanel，保留活动栏） ==========
function toggleSidebarPanel(){
  store.useLayoutStore.toggleSidebarPanel()
}

// ========== 沉浸式 ==========
function toggleImmersive(){
  store.useLayoutStore.toggleImmersive()
}

// ========== 主题切换 ==========

let piniaStore: Store | undefined
async function dark_change() {
  if (piniaStore) {
    toggleDark()
    await piniaStore.set('dark', { value: isDark.value })
    await piniaStore.save()
    store.system.toggleDark(isDark.value)
  }
}
onMounted(async () => {
  piniaStore = await load('settings.json', { autoSave: false, defaults: {} })
  const dark_val = await piniaStore.get<{ value: boolean }>('dark')
  if (dark_val !== undefined && dark_val.value !== isDark.value) {
    toggleDark()
    store.system.toggleDark(isDark.value)
  }
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.quick-actions {
  display: flex;
  align-items: center;
  height: 100%;
  gap: 2px;
  flex-shrink: 0;
  // 与右侧菜单文字拉开一点距离，避免图标贴住「文件(F)」
  margin-right: 6px;
}

// 与窗口控制按钮（WindowControls）同规格的紧凑图标按钮；类名沿用原 FunctionalArea 的 .fa-icon 尺寸约定
.qa-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  cursor: pointer;
  border-radius: var(--radius-md);
  transition: background-color 0.15s;
  flex-shrink: 0;
  // 语义化 <button> 需要抹平浏览器默认外观
  appearance: none;
  border: none;
  padding: 0;
  background: transparent;
  font-family: inherit;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &:hover {
    @include theme.useTheme {
      color: theme.getVar('textColor');
      background-color: theme.getVar('activityBarHover');
    }
  }

  &.is-active {
    @include theme.useTheme {
      color: theme.getVar('activityBarActive');
    }
  }

  // 当前插件无侧栏面板：按钮不可用（保持 24×24 命中区，仅降低对比度并阻断 hover 反馈）
  &:disabled {
    cursor: default;
    opacity: 0.4;

    &:hover {
      @include theme.useTheme {
        color: theme.getVar('textSecondary');
        background-color: transparent;
      }
    }
  }

  svg {
    width: 14px;
    height: 14px;
  }

  // Element Plus 图标按钮：固定 14px，与自绘 SVG 图标视觉一致
  .qa-el-icon {
    font-size: 14px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
}
</style>
