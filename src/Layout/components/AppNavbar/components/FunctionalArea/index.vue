<template>
  <div class="functional-area">
    <!-- 主题切换：开关语义用 aria-pressed 表达（WCAG 4.1.2） -->
    <button
      type="button"
      class="fa-icon"
      :aria-label="isDark ? '切换到浅色主题' : '切换到深色主题'"
      :aria-pressed="isDark"
      :title="isDark ? '切换到浅色主题' : '切换到深色主题'"
      @click="dark_change"
    >
      <el-icon class="fa-el-icon">
        <Moon v-if="isDark" />
        <Sunny v-else />
      </el-icon>
    </button>

    <!-- 切换侧边栏 -->
    <button
      type="button"
      class="fa-icon"
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

    <!-- 隐藏/显示插件面板（仅收起 sidebarPanel，保留活动栏图标列） -->
    <button
      type="button"
      class="fa-icon"
      :class="{ 'is-active': !sidebarPanelCollapsed }"
      :aria-pressed="!sidebarPanelCollapsed"
      :aria-label="sidebarPanelCollapsed ? '显示插件面板' : '隐藏插件面板'"
      :title="sidebarPanelCollapsed ? '显示插件面板' : '隐藏插件面板'"
      @click="toggleSidebarPanel"
    >
      <el-icon v-if="sidebarPanelCollapsed" class="fa-el-icon"><Expand /></el-icon>
      <el-icon v-else class="fa-el-icon"><Fold /></el-icon>
    </button>

    <!-- 功能图标与窗口控制按钮的视觉分隔 -->
    <div class="fa-divider" aria-hidden="true"></div>

    <!-- 窗口控制 -->
    <WindowControls />
  </div>
</template>

<script lang="ts" setup name="FunctionalArea">
import { onMounted } from 'vue'
import { storeToRefs } from 'pinia'
import { Sunny, Moon, Fold, Expand } from '@element-plus/icons-vue'
import { load, Store } from '@/core/runtime'
import { isDark, toggleDark } from '@/composables/useDark'
import useStore from '@/store'
import WindowControls from '@/components/WindowControls.vue'

const store = useStore()

const { isCollapse } = storeToRefs(store.system)
const { sidebarPanelCollapsed } = storeToRefs(store.useLayoutStore)

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

.functional-area {
  display: flex;
  align-items: center;
  height: 100%;
  gap: 2px;
}

.fa-divider {
  width: 1px;
  height: 14px;
  margin: 0 5px;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('borderColor');
  }
}

.fa-icon {
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

  svg {
    width: 14px;
    height: 14px;
  }

  // Element Plus 图标按钮：固定 14px，与自绘 SVG 图标视觉一致
  .fa-el-icon {
    font-size: 14px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
}
</style>
