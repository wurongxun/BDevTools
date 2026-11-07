<template>
  <GlobalDialog v-model="visible" title="导出日志" width="520px">
    <div v-if="!store.activeSource" class="ex-empty">请先打开一个日志源再导出</div>

    <template v-else>
      <div class="ex-row">
        <span class="ex-label">保存路径</span>
        <el-input v-model="destPath" placeholder="点击右侧按钮选择目标文件" readonly />
        <app-button @click="pickPath">选择…</app-button>
      </div>

      <div class="ex-row">
        <span class="ex-label">导出范围</span>
        <el-radio-group v-model="scope">
          <el-radio label="filter">当前筛选结果</el-radio>
          <el-radio label="all">整个文件</el-radio>
        </el-radio-group>
      </div>
      <p v-if="scope === 'filter' && !store.filterActive" class="ex-hint">
        当前未启用筛选，将按"整个文件"导出。
      </p>

      <div class="ex-row">
        <span class="ex-label">选项</span>
        <el-checkbox v-model="withLineNumber">包含行号</el-checkbox>
        <el-checkbox v-model="withBom">写入 BOM</el-checkbox>
      </div>

      <div v-if="result" class="ex-result">
        <el-icon class="ex-ok"><CircleCheckFilled /></el-icon>
        <span>
          已导出 <b>{{ formatCount(result.lines) }}</b> 行 ·
          {{ formatBytes(result.bytes) }} · 耗时 {{ result.elapsed_ms }} ms
        </span>
      </div>
      <div v-if="error" class="ex-error">
        <el-icon><WarningFilled /></el-icon>
        <span>{{ error }}</span>
      </div>
    </template>

    <template #footer>
      <app-button size="small" text @click="visible = false">关闭</app-button>
      <app-button
        size="small"
        type="primary"
        :disabled="!store.activeSource || !destPath"
        :loading="store.exporting"
        @click="doExport"
      >
        导出
      </app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { CircleCheckFilled, WarningFilled } from '@element-plus/icons-vue'
import { save } from '@/core/runtime'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { useLogStore } from '../store/logs'
import { formatCount } from '../core/filter'
import type { LogExportResult } from '../core/indexApi'

const props = defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [boolean] }>()

const store = useLogStore()

/** 透传外部 v-model：GlobalDialog 的开关直接映射为父级的 modelValue */
const visible = computed({
  get: () => props.modelValue,
  set: (v: boolean) => emit('update:modelValue', v),
})

const destPath = ref('')
const scope = ref<'filter' | 'all'>('filter')
const withLineNumber = ref(true)
const withBom = ref(false)
const result = ref<LogExportResult | null>(null)
const error = ref('')

/** 打开时根据当前源预填默认文件名 */
watch(
  () => props.modelValue,
  open => {
    if (!open) return
    error.value = ''
    result.value = null
    scope.value = store.filterActive ? 'filter' : 'all'
    if (!destPath.value && store.activeSource) {
      const base = store.activeSource.path
      const dot = base.lastIndexOf('.')
      destPath.value = (dot > 0 ? base.slice(0, dot) : base) + '.exported.log'
    }
  },
)

async function pickPath() {
  const picked = await save({
    defaultPath: destPath.value || store.activeSource?.name,
    filters: [{ name: '日志文件', extensions: ['log', 'txt', 'out'] }],
  })
  if (typeof picked === 'string' && picked) destPath.value = picked
}

async function doExport() {
  if (!destPath.value) return
  error.value = ''
  result.value = null
  try {
    const r = await store.exportLogs({
      destPath: destPath.value,
      scope: scope.value,
      withLineNumber: withLineNumber.value,
      withBom: withBom.value,
    })
    result.value = r
  } catch (e) {
    error.value = (e as Error).message || '导出失败'
  }
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / 1024 / 1024).toFixed(2)} MB`
}
</script>

<style scoped lang="scss">
.ex-empty {
  padding: 12px 0;
  text-align: center;
  color: var(--el-text-color-secondary);
}

.ex-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.ex-label {
  flex: none;
  width: 64px;
  color: var(--el-text-color-secondary);
}

.ex-hint {
  margin: -8px 0 12px 74px;
  font-size: var(--font-size-sm);
  color: var(--stateWarningFg);
}

.ex-result {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  background: var(--el-color-success-light-9);
  color: var(--stateSuccessFg);
  font-size: var(--font-size-sm);
}

.ex-ok {
  font-size: var(--font-size-xl);
}

.ex-error {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  background: var(--el-color-danger-light-9);
  color: var(--stateErrorFg);
  font-size: var(--font-size-sm);
}
</style>
