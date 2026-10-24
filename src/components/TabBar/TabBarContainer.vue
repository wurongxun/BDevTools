<template>
  <div class="tab-bar-container" :style="containerStyle">
    <!-- 左侧额外内容插槽 -->
    <slot name="prepend" />

    <!-- 向左滚动：标签溢出时提供可发现的滚动入口（原先只计算状态、从未渲染按钮） -->
    <button
      v-if="canScrollLeft"
      type="button"
      class="tab-scroll-btn"
      aria-label="向左滚动标签"
      @click="scrollTabs(-160)"
    >
      ‹
    </button>

    <!-- Tab 列表区域：tablist 语义 + ← →/Home/End 键盘导航 -->
    <div
      ref="tabListRef"
      class="tab-bar-list"
      role="tablist"
      :aria-label="ariaLabel"
      @wheel="handleWheel"
      @keydown="onTabListKeydown"
    >
      <slot />
    </div>

    <button
      v-if="canScrollRight"
      type="button"
      class="tab-scroll-btn"
      aria-label="向右滚动标签"
      @click="scrollTabs(160)"
    >
      ›
    </button>

    <!-- 右侧额外内容插槽（如下拉菜单等） -->
    <slot name="append" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

/**
 * 通用 Tab 栏容器组件
 * 提供统一的高度、背景、边框等基础样式约束
 * 各插件通过 slot 填充自己的 Tab 项内容
 */

defineOptions({ name: 'TabBarContainer' })

const props = withDefaults(defineProps<{
  /** Tab 栏高度，默认 24px */
  height?: string
  /** 无障碍标签，描述该组 Tab 的用途 */
  ariaLabel?: string
}>(), {
  height: '24px',
  ariaLabel: '标签栏'
})

const emit = defineEmits<{
  (e: 'wheel', event: WheelEvent): void
}>()

const tabListRef = ref<HTMLElement | null>(null)

const containerStyle = {
  '--tab-bar-height': props.height
}

function handleWheel(event: WheelEvent) {
  // Ctrl/⌘ + 滚轮是页面缩放快捷键，不拦截（WCAG 1.4.4 缩放）
  if (event.ctrlKey || event.metaKey) return
  const container = tabListRef.value
  if (!container) return
  // 垂直滚动转水平滚动
  event.preventDefault()
  container.scrollLeft += event.deltaY
  emit('wheel', event)
}

/** 标签栏内的方向键导航：在 role="tab" 之间移动焦点（roving tabindex） */
function onTabListKeydown(event: KeyboardEvent) {
  const keys = ['ArrowLeft', 'ArrowRight', 'Home', 'End']
  if (!keys.includes(event.key)) return

  const container = tabListRef.value
  if (!container) return

  const tabs = Array.from(
    container.querySelectorAll<HTMLElement>('[role="tab"]:not([aria-disabled="true"])')
  )
  if (tabs.length === 0) return

  const current = tabs.indexOf(document.activeElement as HTMLElement)
  if (current === -1) return

  event.preventDefault()
  let next = current
  if (event.key === 'ArrowLeft') next = (current - 1 + tabs.length) % tabs.length
  else if (event.key === 'ArrowRight') next = (current + 1) % tabs.length
  else if (event.key === 'Home') next = 0
  else next = tabs.length - 1

  tabs[next].focus()
  tabs[next].scrollIntoView({ block: 'nearest', inline: 'nearest' })
}

// 滚动按钮相关
const canScrollLeft = ref(false)
const canScrollRight = ref(false)

function updateScrollButtons() {
  const container = tabListRef.value
  if (!container) return
  canScrollLeft.value = container.scrollLeft > 0
  canScrollRight.value = container.scrollLeft + container.clientWidth < container.scrollWidth - 1
}

function scrollTabs(delta: number) {
  tabListRef.value?.scrollBy({ left: delta, behavior: 'smooth' })
}

defineExpose({
  tabListRef,
  canScrollLeft,
  canScrollRight,
  updateScrollButtons
})

// 标签增删（MutationObserver）与容器尺寸变化（ResizeObserver）时刷新滚动按钮状态
let resizeObserver: ResizeObserver | null = null
let mutationObserver: MutationObserver | null = null

onMounted(() => {
  const container = tabListRef.value
  if (!container) return
  container.addEventListener('scroll', updateScrollButtons, { passive: true })

  resizeObserver = new ResizeObserver(updateScrollButtons)
  resizeObserver.observe(container)

  mutationObserver = new MutationObserver(updateScrollButtons)
  mutationObserver.observe(container, { childList: true, subtree: true })

  window.addEventListener('resize', updateScrollButtons)
  updateScrollButtons()
})

onUnmounted(() => {
  const container = tabListRef.value
  if (container) {
    container.removeEventListener('scroll', updateScrollButtons)
  }
  resizeObserver?.disconnect()
  resizeObserver = null
  mutationObserver?.disconnect()
  mutationObserver = null
  window.removeEventListener('resize', updateScrollButtons)
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.tab-bar-container {
  display: flex;
  align-items: stretch;
  height: var(--tab-bar-height, 24px);
  user-select: none;
  position: relative;
  flex-shrink: 0;
  // Chrome 风格：标签条底色用专用 tabBarBg（比内容区 appMainBgColor 明显深/亮一档），
  // 激活标签用 appMainBgColor 与下方内容连体——保证 side / tab / main 三段色板协调，
  // 同时让未激活 tab（透出标签条底色）与激活 tab 形成清晰底色对比。
  // 跟随 light/dark 设计令牌，不再用 EP --el-bg-color-page（暗色下 ≈#0a0a0a，与设计色板脱节）
  @include theme.useTheme {
    background-color: theme.getVar('tabBarBg');
  }

  // 底线用 ::after 绘制而非 border-bottom：border 在内容盒之外，
  // 激活标签的背景无法盖住它；::after 位于内容盒内，可被激活标签覆盖，
  // 实现 Chrome 式「激活标签与下方内容区连体、无分隔线」的效果
  &::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 1px;
    @include theme.useTheme {
      background: theme.getVar('borderColor');
    }
  }
}

// 标签溢出时的滚动按钮：24×24 命中区，符合 WCAG 2.5.8 目标尺寸下限
.tab-scroll-btn {
  flex-shrink: 0;
  width: 24px;
  height: 100%;
  min-height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-size: var(--font-size-lg);
  font-family: inherit;
  line-height: 1;
  appearance: none;
  border: none;
  padding: 0;

  @include theme.useTheme {
    background-color: theme.getVar('tabBarBg');
    color: theme.getVar('textSecondary');
  }

  &:hover {
    @include theme.useTheme {
      color: theme.getVar('textColor');
      background-color: theme.getVar('activityBarHover');
    }
  }
}

.tab-bar-list {
  flex: 1;
  display: flex;
  align-items: stretch;
  overflow-x: auto;
  overflow-y: hidden;
  scroll-behavior: smooth;
  min-width: 0;
  // 提升到容器 ::after 底线之上，激活标签（含连体凹脚）才能盖住底线
  position: relative;
  z-index: 1;

  /* 隐藏滚动条 */
  scrollbar-width: none;
  -ms-overflow-style: none;
  &::-webkit-scrollbar {
    display: none;
  }
}
</style>
