<template>
  <div class="pane">
    <div class="pane-toolbar">
      <label class="file-button">
        导入 .xcs / JSON
        <input type="file" accept=".xcs,.json,.txt,.ini" multiple @change="onFiles" />
      </label>
      <app-button size="small" @click="exportCurrent" :disabled="!currentCustom">导出当前方案</app-button>
      <app-button size="small" @click="removeCurrent" :disabled="!currentCustom">删除方案</app-button>
      <span class="pane-hint">Xshell .xcs 的 Color0–15 / 前景 / 背景会被映射成 xterm 主题；残缺字段回落到深色默认值。</span>
    </div>

    <div class="scheme-list">
      <div
        v-for="s in prefs.allSchemes"
        :key="s.id"
        class="scheme-item"
        :class="{ active: prefs.activeSchemeId === s.id }"
        @click="prefs.activeSchemeId = s.id"
      >
        <span class="scheme-radio">{{ prefs.activeSchemeId === s.id ? '◉' : '○' }}</span>
        <span class="scheme-name">{{ s.name }}</span>
        <span class="scheme-source">{{ s.source === 'builtin' ? '内置' : '导入' }}</span>
        <span class="scheme-swatches">
          <i v-for="(c, i) in swatches(s.theme)" :key="i" :style="{ background: c }" />
        </span>
      </div>
    </div>

    <div v-if="notice" class="pane-notice">{{ notice }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useSshPreferencesStore } from '../../../store'
import NotificationUtil from '@/utils/NotificationUtil'
import { exportScheme, parseSchemeFile } from '../../../utils/theme'
import type { TerminalColorScheme, TerminalTheme } from '../../../types'

const prefs = useSshPreferencesStore()
const notice = ref('')
const currentCustom = computed<TerminalColorScheme | undefined>(() =>
  prefs.colorSchemes.find(s => s.id === prefs.activeSchemeId)
)

function swatches(theme: TerminalTheme): string[] {
  return [theme.background, theme.foreground, theme.red, theme.green, theme.yellow, theme.blue, theme.magenta, theme.cyan]
}

async function onFiles(event: Event) {
  const input = event.target as HTMLInputElement
  const files = Array.from(input.files || [])
  let ok = 0
  for (const file of files) {
    try {
      const text = await file.text()
      const scheme = parseSchemeFile(text, file.name)
      if (!scheme) continue
      prefs.colorSchemes.push(scheme)
      prefs.activeSchemeId = scheme.id
      ok++
    } catch { /* 单个坏文件不影响其它文件 */ }
  }
  input.value = ''
  notice.value = ok
    ? `成功导入 ${ok} 个配色方案，已应用最后一个。`
    : '未识别到有效配色（.xcs 需至少包含前景 / 背景色）。'
}

async function exportCurrent() {
  if (!currentCustom.value) return
  try {
    await navigator.clipboard.writeText(exportScheme(currentCustom.value))
    NotificationUtil.success('配色 JSON 已复制到剪贴板')
  } catch {
    notice.value = '剪贴板不可用，请在设置中心的导入导出页复制（或手动保存）'
  }
}

function removeCurrent() {
  const idx = prefs.colorSchemes.findIndex(s => s.id === prefs.activeSchemeId)
  if (idx < 0) return
  prefs.colorSchemes.splice(idx, 1)
  prefs.activeSchemeId = 'auto'
}
</script>
