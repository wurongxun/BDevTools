<template>
  <div class="pane">
    <div class="pane-toolbar">
      <app-button size="small" type="primary" plain @click="addSet">新建高亮集</app-button>
      <span class="pane-hint">启用集合中的规则会对终端输出实时着色；非法正则会被自动忽略。历史回滚内容不重新着色。</span>
    </div>
    <div class="hl-layout">
      <div class="hl-sets">
        <div
          v-for="s in prefs.highlightSets"
          :key="s.id"
          class="hl-set"
          :class="{ active: s.id === currentId }"
          @click="currentId = s.id"
        >
          <el-switch v-model="s.enabled" size="small" @click.stop />
          <span class="hl-set-name" :title="s.name">{{ s.name }}</span>
          <span class="hl-count">{{ s.rules.length }}</span>
          <button class="pane-icon-btn" title="删除集合" @click.stop="removeSet(s.id)">×</button>
        </div>
        <div v-if="!prefs.highlightSets.length" class="pane-empty">暂无高亮集</div>
      </div>

      <div class="hl-rules" v-if="current">
        <div class="pane-row">
          <span class="pane-label">集合名称</span>
          <el-input v-model="current.name" size="small" />
          <app-button size="small" @click="addRule">新增规则</app-button>
        </div>
        <div class="hl-rule-head">
          <span>匹配文本 / 正则</span><span>前景</span><span>背景</span>
          <span>选项</span><span />
        </div>
        <div v-for="r in current.rules" :key="r.id" class="hl-rule">
          <el-input v-model="r.pattern" size="small" placeholder="ERROR 或 (error|fail)" />
          <div class="color-cell">
            <input type="color" :value="r.foreground || '#ff6b6b'" @input="setFg(r, $event)" />
            <button class="pane-icon-btn" title="不设前景色" @click="r.foreground = ''">×</button>
          </div>
          <div class="color-cell">
            <input type="color" :value="r.background || '#ffd166'" @input="setBg(r, $event)" />
            <button class="pane-icon-btn" title="不设背景色" @click="r.background = ''">×</button>
          </div>
          <div class="hl-options">
            <label title="正则表达式"><input v-model="r.regex" type="checkbox" />正则</label>
            <label title="区分大小写"><input v-model="r.caseSensitive" type="checkbox" />Aa</label>
            <label title="整词匹配"><input v-model="r.wholeWord" type="checkbox" />整词</label>
            <label title="加粗"><input v-model="r.bold" type="checkbox" />粗</label>
            <label title="启用"><input v-model="r.enabled" type="checkbox" />启用</label>
          </div>
          <button class="pane-icon-btn" title="删除规则" @click="removeRule(r.id)">×</button>
        </div>
        <div v-if="!current.rules.length" class="pane-empty">该集合暂无规则</div>
      </div>
      <div v-else class="hl-rules pane-empty">选择左侧集合以编辑规则</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useSshPreferencesStore, uid } from '../../../store'
import type { HighlightRule, HighlightSet } from '../../../types'

const prefs = useSshPreferencesStore()
const currentId = ref(prefs.highlightSets[0]?.id || '')
const current = computed<HighlightSet | undefined>(() =>
  prefs.highlightSets.find(s => s.id === currentId.value)
)

function addSet() {
  const set: HighlightSet = {
    id: uid('hs'), name: `高亮集 ${prefs.highlightSets.length + 1}`,
    enabled: true, rules: []
  }
  prefs.highlightSets.push(set)
  currentId.value = set.id
}

function removeSet(id: string) {
  const idx = prefs.highlightSets.findIndex(s => s.id === id)
  if (idx < 0) return
  prefs.highlightSets.splice(idx, 1)
  if (currentId.value === id) currentId.value = prefs.highlightSets[0]?.id || ''
}

function addRule() {
  if (!current.value) return
  const rule: HighlightRule = {
    id: uid('hl'), pattern: 'ERROR', regex: false, caseSensitive: false, wholeWord: false,
    foreground: '#ff6b6b', background: '', bold: true, enabled: true
  }
  current.value.rules.push(rule)
}

function removeRule(id: string) {
  if (!current.value) return
  const idx = current.value.rules.findIndex(r => r.id === id)
  if (idx >= 0) current.value.rules.splice(idx, 1)
}

function setFg(rule: HighlightRule, event: Event) {
  rule.foreground = (event.target as HTMLInputElement).value
}
function setBg(rule: HighlightRule, event: Event) {
  rule.background = (event.target as HTMLInputElement).value
}
</script>
