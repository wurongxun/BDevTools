<template>
  <div class="context-menu-trigger" ref="triggerRef" @contextmenu.prevent="handleContextMenu">
    <slot></slot>
    <ContextMenu
      :visible="menuVisible"
      :menu-items="menuItems"
      :position="menuPosition"
      @select="handleSelect"
      @close="menuVisible = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import ContextMenu from './ContextMenu.vue'
import type { MenuItem } from './ContextMenu.vue'

interface Props {
  menuItems: MenuItem[]
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  disabled: false
})

const emit = defineEmits<{
  (e: 'select', item: MenuItem): void
}>()

const triggerRef = ref<HTMLElement | null>(null)
const menuVisible = ref(false)
const menuPosition = ref({ x: 0, y: 0 })

const handleContextMenu = (event: MouseEvent) => {
  if (props.disabled) return

  event.preventDefault()
  event.stopPropagation()

  // 获取鼠标点击位置
  menuPosition.value = {
    x: event.clientX,
    y: event.clientY
  }

  menuVisible.value = true
}

const handleSelect = (item: MenuItem) => {
  emit('select', item)
  menuVisible.value = false
}
</script>

<style scoped>
.context-menu-trigger {
  display: inline-block;
}
</style>
