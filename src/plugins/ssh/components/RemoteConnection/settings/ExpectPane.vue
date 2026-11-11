<template>
  <div class="pane">
    <div class="pane-toolbar">
      <app-button size="small" type="primary" plain @click="add">新增规则</app-button>
      <span class="pane-hint">
        连接在收到匹配输出时自动发送内容。变量：${password} / ${passphrase} / ${username} / ${host}。
        含口令的规则请勾选「敏感」，这类输入不会写入会话日志。
      </span>
    </div>
    <div class="ex-head">
      <span>名称</span><span>期望（正则）</span><span>发送内容</span><span>仅一次</span><span>敏感</span><span>启用</span><span />
    </div>
    <div v-for="r in prefs.expectRules" :key="r.id" class="ex-row">
      <el-input v-model="r.name" size="small" placeholder="密码提示" />
      <el-input v-model="r.pattern" size="small" placeholder="[Pp]assword:" />
      <el-input v-model="r.send" size="small" placeholder="${password}" />
      <input v-model="r.once" type="checkbox" />
      <input v-model="r.sensitive" type="checkbox" />
      <input v-model="r.enabled" type="checkbox" />
      <button class="pane-icon-btn" title="删除" @click="remove(r.id)">×</button>
    </div>
    <div v-if="!prefs.expectRules.length" class="pane-empty">暂无自动登录规则</div>
    <div class="pane-notice">
      规则默认关闭：请确认「仅一次」与正则范围后再启用，避免把密码发到非预期的提示上。
    </div>
  </div>
</template>

<script setup lang="ts">
import { useSshPreferencesStore, uid } from '../../../store'
import type { ExpectRule } from '../../../types'

const prefs = useSshPreferencesStore()

function add() {
  const rule: ExpectRule = {
    id: uid('ex'), name: '新规则', pattern: '[Pp]assword:', send: '${password}',
    once: true, enabled: false, sensitive: true
  }
  prefs.expectRules.push(rule)
}

function remove(id: string) {
  const idx = prefs.expectRules.findIndex(r => r.id === id)
  if (idx >= 0) prefs.expectRules.splice(idx, 1)
}
</script>
