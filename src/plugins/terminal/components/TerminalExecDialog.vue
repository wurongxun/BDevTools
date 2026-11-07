<template>
  <!--
    快速执行：在激活窗格里跑一条命令并等它结束（异步执行 + 超时）
    —— 等价于 TerminalSession.exec()，把「拿退出码/输出」这件事做成 UI 入口。
  -->
  <GlobalDialog
    :model-value="store.execOpen"
    title="快速执行命令"
    width="640px"
    @update:model-value="onVisibleChange"
  >
    <div class="ted-body">
      <div class="ted-row">
        <span class="ted-label">命令</span>
        <el-input
          ref="cmdInputRef"
          v-model="command"
          class="ted-input"
          size="small"
          placeholder="例如：git status"
          spellcheck="false"
          @keydown.enter="run"
        />
      </div>

      <div class="ted-row">
        <span class="ted-label">超时</span>
        <el-input-number v-model="timeoutMs" class="ted-num" size="small" :min="1000" :max="1800000" :step="5000" />
        <span class="ted-unit">ms</span>
        <app-button class="ted-run" size="small" type="primary" :loading="store.execRunning" :disabled="!canRun" @click="run">
          执行
        </app-button>
      </div>

      <p v-if="store.integrationHint" class="ted-warn">
        {{ store.integrationHint }}
      </p>

      <p v-if="store.execError" class="ted-warn ted-warn--error">{{ store.execError }}</p>

      <!-- 最近一次结果 -->
      <div v-if="latest" class="ted-result">
        <div class="ted-result-head">
          <span class="ted-status" :class="statusClass">{{ statusText }}</span>
          <span class="ted-meta">退出码 {{ latest.result.exitCode ?? '未知' }}</span>
          <span class="ted-meta">耗时 {{ formatMs(latest.result.elapsedMs) }}</span>
        </div>
        <pre class="ted-output">{{ latest.result.output || '（无输出）' }}</pre>
      </div>

      <!-- 历史（本次会话内） -->
      <div v-if="store.execHistory.length > 1" class="ted-history">
        <p class="ted-history-title">执行历史</p>
        <ul class="ted-history-list">
          <li v-for="(item, i) in store.execHistory.slice(1)" :key="`${i}-${item.command}`">
            <app-button size="small" text @click="command = item.command">{{ item.command }}</app-button>
            <span class="ted-history-meta">
              {{ item.result.timedOut ? '超时' : `退出码 ${item.result.exitCode ?? '未知'}` }}
            </span>
          </li>
        </ul>
      </div>
    </div>

    <template #footer>
      <app-button size="small" @click="store.execOpen = false">关闭</app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts" name="TerminalExecDialog">
import { computed, ref, watch } from 'vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { useTerminalStore } from '../store'

const store = useTerminalStore()

const command = ref('')
const timeoutMs = ref(30_000)
const cmdInputRef = ref<{ focus?: () => void } | null>(null)

const latest = computed(() => store.execHistory[0] ?? null)
const canRun = computed(() => Boolean(store.activePaneId) && command.value.trim().length > 0 && !store.execRunning)

const statusText = computed(() => {
  const r = latest.value?.result
  if (!r) return ''
  if (r.timedOut) return '超时未结束'
  if (r.exitCode === null) return '已结束（退出码未知）'
  return r.exitCode === 0 ? '成功' : '失败'
})

const statusClass = computed(() => {
  const r = latest.value?.result
  if (!r || r.timedOut || r.exitCode === null) return 'is-unknown'
  return r.exitCode === 0 ? 'is-ok' : 'is-fail'
})

function formatMs(ms: number): string {
  return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(2)} s`
}

function onVisibleChange(visible: boolean) {
  store.execOpen = visible
}

async function run() {
  if (!canRun.value) return
  try {
    const paneId = store.activePaneId
    const s = store.getSession(paneId)
    // 直接调 exec（而不是 runInActivePane）以便把焦点还给终端
    await store.exec(paneId, command.value.trim(), timeoutMs.value)
    s?.focus()
  } catch {
    /* 错误已写入 store.execError */
  }
}

watch(
  () => store.execOpen,
  (open) => {
    if (!open) return
    store.execError = ''
    void Promise.resolve().then(() => cmdInputRef.value?.focus?.())
  },
)
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.ted-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}

.ted-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.ted-label {
  width: 44px;
  flex-shrink: 0;
  font-size: var(--font-size-base);

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.ted-input {
  flex: 1;
  min-width: 0;
}

.ted-num {
  width: 130px;
}

.ted-unit {
  font-size: var(--font-size-sm);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.ted-run {
  margin-left: auto;
}

.ted-warn {
  margin: 0;
  font-size: var(--font-size-xs);
  line-height: 1.5;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &--error {
    color: var(--stateErrorFg);
  }
}

.ted-result {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.ted-result-head {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  font-size: var(--font-size-sm);
}

.ted-status {
  font-weight: 600;

  &.is-ok {
    color: var(--stateSuccessFg);
  }

  &.is-fail {
    color: var(--stateErrorFg);
  }

  &.is-unknown {
    @include theme.useTheme {
      color: theme.getVar('textSecondary');
    }
  }
}

.ted-meta {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.ted-output {
  margin: 0;
  max-height: 260px;
  overflow: auto;
  padding: var(--space-2);
  border-radius: var(--radius-sm);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    color: theme.getVar('textColor');
    border: 1px solid theme.getVar('borderColor');
  }
}

.ted-history-title {
  margin: 0 0 2px;
  font-size: var(--font-size-xs);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.ted-history-list {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  max-height: 120px;
  overflow: auto;

  li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }
}

.ted-history-meta {
  font-size: var(--font-size-xs);
  flex-shrink: 0;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}
</style>
