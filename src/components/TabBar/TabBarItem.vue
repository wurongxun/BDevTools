<template>
  <div
    class="tab-bar-item"
    role="tab"
    :aria-selected="active"
    :tabindex="active ? 0 : -1"
    :class="{
      'tab-bar-item-active': active,
      'tab-bar-item-dragging': dragging,
      'tab-bar-item-drag-over': dragOver
    }"
    :draggable="draggable"
    @click="emit('click', $event)"
    @mousedown.middle.prevent="emit('close', $event)"
    @keydown="onKeydown"
    @dragstart="emit('dragstart', $event)"
    @dragover="emit('dragover', $event)"
    @drop="emit('drop', $event)"
    @dragend="emit('dragend', $event)"
    @mouseenter="emit('mouseenter', $event)"
    @mouseleave="emit('mouseleave', $event)"
  >
    <slot />
  </div>
</template>

<script setup lang="ts">
/**
 * 通用 Tab 项组件
 * 提供统一的 Tab 项基础样式和交互行为
 */

defineOptions({ name: 'TabBarItem' })

withDefaults(defineProps<{
  /** 是否激活状态 */
  active?: boolean
  /** 是否可拖拽 */
  draggable?: boolean
  /** 拖拽中 */
  dragging?: boolean
  /** 拖拽经过 */
  dragOver?: boolean
}>(), {
  active: false,
  draggable: false,
  dragging: false,
  dragOver: false
})

// click 必须携带原生 MouseEvent：
// 声明 click 后父组件的 @click 会走组件事件通道（emit 的负载即参数）。
// 若 emit 不带负载，父组件写 @click.stop/@click.prevent 时，Vue 的 withModifiers
// 包装函数会收到 undefined 事件并在 e.stopPropagation() 处抛 TypeError。
const emit = defineEmits<{
  // 鼠标点击传 MouseEvent，Enter/空格（键盘等价选中）传 KeyboardEvent
  (e: 'click', event: MouseEvent | KeyboardEvent): void
  (e: 'close', event: MouseEvent | KeyboardEvent): void
  (e: 'dragstart', event: DragEvent): void
  (e: 'dragover', event: DragEvent): void
  (e: 'drop', event: DragEvent): void
  (e: 'dragend', event: DragEvent): void
  (e: 'mouseenter', event: MouseEvent): void
  (e: 'mouseleave', event: MouseEvent): void
  (e: 'keydown', event: KeyboardEvent): void
}>()

/**
 * 键盘交互（WCAG 2.1.1）：
 * - Enter/空格 = 选中该标签（等价于点击）
 * - Delete = 关闭该标签（等价于中键点击）
 * - ← → / Home / End 由外层 tablist 容器统一处理焦点移动
 */
function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    emit('click', event)
    return
  }
  if (event.key === 'Delete') {
    event.preventDefault()
    emit('close', event)
    return
  }
  emit('keydown', event)
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.tab-bar-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 6px;
  min-width: 100px;
  max-width: 200px;
  height: 100%;
  flex-shrink: 0;
  cursor: pointer;
  white-space: nowrap;
  position: relative;
  z-index: 1;
  transition: background-color 0.15s, color 0.15s;
  font-size: var(--font-size-base);
  // Chrome 风格基础：未激活标签透明底、仅顶部圆角，与标签条底色融为一体
  border-radius: var(--radius-lg) var(--radius-lg) 0 0;
  background: transparent;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &:hover {
    @include theme.useTheme {
      background: theme.getVar('activityBarHover');
      color: theme.getVar('textColor');
    }
  }
}

.tab-bar-item-active {
  // 压过相邻未激活标签与容器底线，保证连体凹脚完整显示
  z-index: 2;

  @include theme.useTheme {
    background: theme.getVar('appMainBgColor');
    color: theme.getVar('textColor');
  }

  &:hover {
    @include theme.useTheme {
      background: theme.getVar('appMainBgColor');
    }
  }

  // Chrome 式凹脚：激活标签底部两侧各一个 6×6 方块，
  // radial-gradient 以方块外上角为圆心挖掉四分之一圆，
  // 剩余的弧形填充与标签同色（appMainBgColor），视觉上与下方内容区连为一体。
  &::before,
  &::after {
    content: '';
    position: absolute;
    bottom: 0;
    width: 6px;
    height: 6px;
    pointer-events: none;
  }

  &::before {
    left: -6px;
    background: radial-gradient(circle 6px at 0 0, transparent 6px, var(--appMainBgColor) 6.5px);
  }

  &::after {
    right: -6px;
    background: radial-gradient(circle 6px at 100% 0, transparent 6px, var(--appMainBgColor) 6.5px);
  }
}

.tab-bar-item-dragging {
  opacity: 0.5;
  @include theme.useTheme {
    background: theme.getVar('activityBarHover');
  }
}

.tab-bar-item-drag-over {
  // 拖拽落点指示：inset 阴影画主题色左边线（不参与盒模型，避免出现时的内容抖动）
  box-shadow: inset 2px 0 0 var(--el-color-primary);
}
</style>
