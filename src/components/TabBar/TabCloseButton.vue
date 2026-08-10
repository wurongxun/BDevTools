<template>
  <button
    class="tab-bar-close"
    @click.stop="$emit('click')"
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

defineEmits<{
  (e: 'click'): void
}>()
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.tab-bar-close {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s, background-color 0.15s;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }

  *:hover > &,
  &:hover {
    opacity: 1;
  }

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('activityBarHover');
      color: theme.getVar('textColor');
    }
  }
}
</style>
