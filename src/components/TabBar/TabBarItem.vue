<template>
  <div
    class="tab-bar-item"
    :class="{
      'tab-bar-item-active': active,
      'tab-bar-item-dragging': dragging,
      'tab-bar-item-drag-over': dragOver
    }"
    :draggable="draggable"
    @click="emit('click')"
    @mousedown.middle.prevent="emit('close')"
    @dragstart="emit('dragstart', $event)"
    @dragover="emit('dragover', $event)"
    @drop="emit('drop', $event)"
    @dragend="emit('dragend')"
    @mouseenter="emit('mouseenter', $event)"
    @mouseleave="emit('mouseleave')"
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

const emit = defineEmits<{
  (e: 'click'): void
  (e: 'close'): void
  (e: 'dragstart', event: DragEvent): void
  (e: 'dragover', event: DragEvent): void
  (e: 'drop', event: DragEvent): void
  (e: 'dragend'): void
  (e: 'mouseenter', event: MouseEvent): void
  (e: 'mouseleave'): void
}>()
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;
@use '@/styles/variables' as variables;
@use 'sass:color';

.tab-bar-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
  min-width: 100px;
  max-width: 200px;
  height: 100%;
  flex-shrink: 0;
  cursor: pointer;
  white-space: nowrap;
  position: relative;
  transition: background-color 0.15s, color 0.15s;
  font-size: variables.$font-size;

  @include theme.useTheme {
    background-color: color.adjust(theme.getVar('bgColor'), $lightness: 3%);
    border-right: 1px solid theme.getVar('borderColor');
    color: color.adjust(theme.getVar('textColor'), $lightness: -30%);
  }

  &:hover {
    @include theme.useTheme {
      background-color: color.adjust(theme.getVar('bgColor'), $lightness: 8%);
      color: theme.getVar('textColor');
    }
  }
}

.tab-bar-item-active {
  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    color: theme.getVar('textColor');
    border-bottom: 2px solid theme.getVar('hoverColor');
  }

  &::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 2px;
    background-color: theme.getVar('hoverColor');
  }
}

.tab-bar-item-dragging {
  opacity: 0.5;

  @include theme.useTheme {
    background-color: color.adjust(theme.getVar('bgColor'), $lightness: 15%);
  }
}

.tab-bar-item-drag-over {
  border-left: 1px solid #007acc;
}
</style>
