<template>
  <div class="pane">
    <div class="macro-save-strip">
      <span class="save-dot" :class="`save-${prefs.saveState}`" />
      <span class="save-text" :class="`save-${prefs.saveState}`">{{ saveText }}</span>
      <span class="pane-hint">改动自动写入本地，无需点保存；可随时关闭窗口</span>
      <app-button size="small" link @click="saveNow">立即保存</app-button>
    </div>

    <div class="macro-layout">
      <div class="macro-list">
        <div class="pane-toolbar pane-toolbar-stack">
          <app-button size="small" type="primary" plain @click="addMacro">新建宏</app-button>
        </div>
        <div class="macro-items">
          <div
            v-for="m in prefs.macros"
            :key="m.id"
            class="macro-item"
            :class="{ active: m.id === currentId }"
            @click="selectMacro(m.id)"
          >
            <span class="macro-name" :title="m.name">{{ m.name || '未命名宏' }}</span>
            <span class="macro-count" :title="`${m.steps.length} 个步骤`">{{ m.steps.length }} 步</span>
            <span v-if="m.autoRun" class="macro-auto">自动</span>
            <button class="pane-icon-btn" title="删除宏" @click.stop="removeMacro(m.id)">×</button>
          </div>
          <div v-if="!prefs.macros.length" class="pane-empty">暂无宏</div>
        </div>
      </div>

      <div v-if="current" class="macro-editor">
        <div class="pane-row">
          <span class="pane-label">名称</span>
          <el-input v-model="current.name" size="small" class="macro-name-input" placeholder="宏名称" />
          <label class="inline-check"><input v-model="current.enabled" type="checkbox" />启用</label>
          <label class="inline-check"><input v-model="current.autoRun" type="checkbox" />连接后自动运行</label>
        </div>
        <div class="pane-toolbar">
          <app-button size="small" @click="addStep('send')">+ 发送</app-button>
          <app-button size="small" @click="addStep('wait')">+ 等待</app-button>
          <app-button size="small" @click="addStep('expect')">+ 等待输出</app-button>
        </div>
        <div class="pane-hint">
          声明式步骤，不使用脚本求值。「发送」框支持多行（每行作为一条命令发出），
          变量仅在发送内容里展开；点击步骤右侧的 ⤢ 可把编辑框放大到 14 行。
        </div>

        <div class="macro-steps">
          <div v-for="(s, i) in current.steps" :key="s.id" class="macro-step-card">
            <div class="step-head">
              <span class="step-index">{{ i + 1 }}</span>
              <el-select v-model="s.kind" size="small" class="step-kind">
                <el-option label="发送" value="send" />
                <el-option label="等待(ms)" value="wait" />
                <el-option label="等待输出" value="expect" />
              </el-select>
              <el-input-number
                v-if="s.kind !== 'send'"
                v-model="s.arg"
                size="small"
                :min="0"
                :max="120000"
                :step="100"
                controls-position="right"
                class="step-arg"
              />
              <label v-if="s.kind === 'send'" class="inline-check">
                <input v-model="s.newline" type="checkbox" />末尾回车
              </label>
              <span class="step-spacer" />
              <button
                class="pane-icon-btn"
                :title="expandedId === s.id ? '收起编辑框' : '放大编辑框到 14 行'"
                @click="toggleExpand(s.id)"
              >{{ expandedId === s.id ? '⤡' : '⤢' }}</button>
              <button class="pane-icon-btn" title="上移" @click="move(i, -1)">↑</button>
              <button class="pane-icon-btn" title="下移" @click="move(i, 1)">↓</button>
              <button class="pane-icon-btn" title="删除步骤" @click="current.steps.splice(i, 1)">×</button>
            </div>
            <el-input
              v-if="s.kind !== 'wait'"
              v-model="s.value"
              type="textarea"
              :rows="expandedId === s.id ? 14 : s.kind === 'send' ? 4 : 2"
              resize="vertical"
              spellcheck="false"
              class="step-text"
              :placeholder="stepPlaceholder(s.kind)"
            />
            <div v-else class="step-hint">等待上方指定的毫秒数，无需输入内容。</div>
          </div>
          <div v-if="!current.steps.length" class="pane-empty">暂无步骤，点击上方按钮添加</div>
        </div>
      </div>
      <div v-else class="macro-editor pane-empty">选择左侧宏以编辑步骤</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import NotificationUtil from '@/utils/NotificationUtil'
import { useSshPreferencesStore, uid } from '../../../store'
import type { MacroScript, MacroStepKind } from '../../../types'

const prefs = useSshPreferencesStore()
const currentId = ref(prefs.macros[0]?.id || '')
/** 单个步骤临时放大，避免所有步骤同时变高把列表撑得没法浏览 */
const expandedId = ref('')
const current = computed<MacroScript | undefined>(() => prefs.macros.find(m => m.id === currentId.value))

const saveText = computed(() => {
  if (prefs.saveState === 'saving') return '正在保存…'
  if (prefs.saveState === 'error') return `保存失败：${prefs.saveError || '本地存储不可用'}`
  return prefs.lastSavedAt
    ? `已自动保存 · ${new Date(prefs.lastSavedAt).toLocaleTimeString('zh-CN', { hour12: false })}`
    : '已自动保存'
})

function saveNow() {
  if (prefs.saveNow()) NotificationUtil.success('宏与偏好已保存到本地')
  else NotificationUtil.error(`保存失败：${prefs.saveError}`)
}

function selectMacro(id: string) {
  currentId.value = id
  expandedId.value = ''
}

function toggleExpand(id: string) {
  expandedId.value = expandedId.value === id ? '' : id
}

function stepPlaceholder(kind: MacroStepKind): string {
  if (kind === 'expect') return '等待输出匹配的正则（单行），如 \\$ '
  return '发送内容：支持多行，每行一条命令；可用 ${password} / ${username} 等变量'
}

function addMacro() {
  const macro: MacroScript = {
    id: uid('macro'), name: `宏 ${prefs.macros.length + 1}`, enabled: true, autoRun: false, steps: []
  }
  prefs.macros.push(macro)
  selectMacro(macro.id)
}

function removeMacro(id: string) {
  const idx = prefs.macros.findIndex(m => m.id === id)
  if (idx < 0) return
  prefs.macros.splice(idx, 1)
  if (currentId.value === id) selectMacro(prefs.macros[0]?.id || '')
}

function addStep(kind: MacroStepKind) {
  if (!current.value) return
  current.value.steps.push({
    id: uid('ms'), kind,
    value: kind === 'expect' ? '\\$ ' : '',
    arg: kind === 'wait' ? 500 : 15000,
    newline: kind === 'send'
  })
}

function move(index: number, delta: number) {
  if (!current.value) return
  const target = index + delta
  const steps = current.value.steps
  if (target < 0 || target >= steps.length) return
  const [item] = steps.splice(index, 1)
  steps.splice(target, 0, item)
}
</script>
