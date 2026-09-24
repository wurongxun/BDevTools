<template>
  <nav class="settings-nav" aria-label="设置分类">
    <div class="settings-nav-head">设置</div>
    <div class="settings-nav-list" role="tablist" aria-orientation="vertical">
      <button
        v-for="item in NAV_ITEMS"
        :key="item.id"
        type="button"
        class="settings-nav-item"
        role="tab"
        :aria-selected="modelValue === item.id"
        @click="emit('update:modelValue', item.id)"
      >
        {{ item.label }}
      </button>
    </div>
  </nav>
</template>

<script setup lang="ts">
// 左侧分类导航。样式在 setting/settings.scss（非 scoped，与右侧面板共用基元）。
import type { SettingTab } from './types'

defineProps<{ modelValue: SettingTab }>()

const emit = defineEmits<{ 'update:modelValue': [SettingTab] }>()

const NAV_ITEMS: ReadonlyArray<{ id: SettingTab; label: string }> = [
  { id: 'app', label: '应用设置' },
  { id: 'window', label: '窗口设置' },
  { id: 'plugin', label: '插件' },
  { id: 'help', label: '帮助' },
  { id: 'about', label: '关于' },
]
</script>
