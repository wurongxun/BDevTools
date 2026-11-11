<template>
  <div class="pane">
    <div class="pane-toolbar">
      <app-button size="small" type="primary" plain @click="add">新增键位</app-button>
      <span class="pane-hint">在终端聚焦时生效；内置快捷键（Ctrl+Shift+C/V/F、Ctrl+±）优先，不会被覆盖。发送内容里 \r 表示回车。</span>
    </div>
    <div class="kb-head">
      <span>说明</span><span>按键</span><span>修饰键</span><span>动作</span><span>参数</span><span>启用</span><span />
    </div>
    <div v-for="b in prefs.keyBindings" :key="b.id" class="kb-row">
      <el-input v-model="b.label" size="small" placeholder="说明" />
      <el-input v-model="b.key" size="small" placeholder="如 c / F1" />
      <div class="kb-mods">
        <label><input v-model="b.ctrl" type="checkbox" />Ctrl</label>
        <label><input v-model="b.alt" type="checkbox" />Alt</label>
        <label><input v-model="b.shift" type="checkbox" />Shift</label>
      </div>
      <el-select v-model="b.action" size="small">
        <el-option v-for="a in ACTIONS" :key="a.value" :label="a.label" :value="a.value" />
      </el-select>
      <el-input
        v-model="b.payload"
        size="small"
        :disabled="!needsPayload(b.action)"
        :placeholder="b.action === 'runMacro' ? '宏 ID' : '发送文本 / 宏 ID'"
      />
      <input v-model="b.enabled" type="checkbox" />
      <button class="pane-icon-btn" title="删除" @click="remove(b.id)">×</button>
    </div>
    <div v-if="!prefs.keyBindings.length" class="pane-empty">暂无自定义键位</div>
  </div>
</template>

<script setup lang="ts">
import { useSshPreferencesStore, uid } from '../../../store'
import type { KeyAction, KeyBinding } from '../../../types'

const prefs = useSshPreferencesStore()
const ACTIONS: { label: string; value: KeyAction }[] = [
  { label: '发送文本', value: 'send' },
  { label: '运行宏', value: 'runMacro' },
  { label: '切换录制', value: 'toggleRecord' },
  { label: '复制', value: 'copy' },
  { label: '粘贴', value: 'paste' },
  { label: '查找', value: 'search' },
  { label: '全选', value: 'selectAll' },
  { label: '清屏', value: 'clear' },
  { label: '重连', value: 'reconnect' }
]

function needsPayload(action: KeyAction): boolean {
  return action === 'send' || action === 'runMacro'
}

function add() {
  const binding: KeyBinding = {
    id: uid('kb'), label: '新键位', key: 'k', ctrl: false, alt: true, shift: false,
    meta: false, action: 'send', payload: '', enabled: true
  }
  prefs.keyBindings.push(binding)
}

function remove(id: string) {
  const idx = prefs.keyBindings.findIndex(b => b.id === id)
  if (idx >= 0) prefs.keyBindings.splice(idx, 1)
}
</script>
