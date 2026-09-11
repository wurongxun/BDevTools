<template>
  <button
    type="button"
    class="tab-bar-close"
    @click.stop="$emit('close', $event)"
    :title="title"
  >
    <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
      <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.5"/>
    </svg>
  </button>
</template>

<script setup lang="ts">
/**
 * 通用 Tab 关闭按钮
 */

defineOptions({ name: 'TabCloseButton' })

withDefaults(defineProps<{
  title?: string
}>(), {
  title: '关闭'
})

// 事件名用 close 而非 click：
// 1) click 是原生事件名，父组件写 @click.stop 时 Vue 的 withModifiers 包装函数会以 emit 的
//    负载（无负载即 undefined）为参数调用，进而 e.stopPropagation() 抛 TypeError；
// 2) 声明 click 后父组件的 @click 会被当作组件事件而非 DOM 监听，语义与原生点击混淆。
// 关闭语义始终携带原生事件，父组件可安全使用 .stop/.prevent 修饰符。
defineEmits<{
  (e: 'close', event: MouseEvent): void
}>()
</script>

<style scoped lang="scss">
.tab-bar-close {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  border-radius: var(--radius-full);
  cursor: pointer;
  // 常显（次级灰）：禁止 opacity:0 仅 hover 显现（键盘用户看不到，违反无障碍约定），
  // 悬停/聚焦时加深并出现圆形底
  opacity: 0.55;
  transition: opacity 0.15s, background-color 0.15s;
  color: var(--el-text-color-secondary);

  &:hover,
  &:focus-visible {
    opacity: 1;
    background-color: var(--el-fill-color);
    color: var(--el-text-color-primary);
  }
}
</style>
