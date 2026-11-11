<template>
  <nav class="settings-nav" :aria-label="$t('settingsPlugin.nav.title')">
    <div class="settings-nav-head">{{ $t('settingsPlugin.nav.title') }}</div>
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
        {{ $t('settingsPlugin.nav.' + item.id) }}
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
  { id: 'app', label: 'app' },
  { id: 'window', label: 'window' },
  { id: 'plugin', label: 'plugin' },
  { id: 'help', label: 'help' },
  { id: 'about', label: 'about' },
]
</script>
