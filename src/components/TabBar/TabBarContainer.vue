<template>
  <div class="tab-bar-container" :style="containerStyle">
    <!-- 左侧额外内容插槽 -->
    <slot name="prepend" />

    <!-- Tab 列表区域 -->
    <div
      ref="tabListRef"
      class="tab-bar-list"
      @wheel.prevent="handleWheel"
    >
      <slot />
    </div>

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
  /** Tab 栏高度，默认 36px */
  height?: string
}>(), {
  height: '36px'
})

const emit = defineEmits<{
  (e: 'wheel', event: WheelEvent): void
}>()

const tabListRef = ref<HTMLElement | null>(null)

const containerStyle = {
  '--tab-bar-height': props.height
}

function handleWheel(event: WheelEvent) {
  const container = tabListRef.value
  if (!container) return
  // 垂直滚动转水平滚动
  container.scrollLeft += event.deltaY
  emit('wheel', event)
}

// 滚动按钮相关（可选功能）
const canScrollLeft = ref(false)
const canScrollRight = ref(false)

function updateScrollButtons() {
  const container = tabListRef.value
  if (!container) return
  canScrollLeft.value = container.scrollLeft > 0
  canScrollRight.value = container.scrollLeft + container.clientWidth < container.scrollWidth - 1
}

defineExpose({
  tabListRef,
  canScrollLeft,
  canScrollRight,
  updateScrollButtons
})

onMounted(() => {
  const container = tabListRef.value
  if (container) {
    container.addEventListener('scroll', updateScrollButtons, { passive: true })
    updateScrollButtons()
  }
})

onUnmounted(() => {
  const container = tabListRef.value
  if (container) {
    container.removeEventListener('scroll', updateScrollButtons)
  }
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.tab-bar-container {
  display: flex;
  align-items: stretch;
  height: var(--tab-bar-height, 36px);
  user-select: none;
  position: relative;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-bottom: 1px solid theme.getVar('borderColor');
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

  /* 隐藏滚动条 */
  scrollbar-width: none;
  -ms-overflow-style: none;
  &::-webkit-scrollbar {
    display: none;
  }
}
</style>
