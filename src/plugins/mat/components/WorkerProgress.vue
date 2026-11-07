<template>
  <div v-if="job && !job.finished" class="wprog">
    <div class="wprog-tip">{{ job.msg || '解析对象引用图…' }}</div>
    <el-progress :percentage="pct" :stroke-width="6" :show-text="false" class="wprog-bar" />
    <div v-if="hint" class="wprog-sub">{{ hint }}</div>
    <div v-if="counters" class="wprog-sub">{{ counters }}</div>
    <app-button v-if="cancelable" size="small" text type="danger" @click="onCancel">
      取消分析
    </app-button>
  </div>
</template>

<script setup lang="ts">
/**
 * 后台分析进度（sidecar 子进程）。
 *
 * 「重」视图在等索引时都挂这个组件 —— 否则用户看到的只是一个转圈，
 * 分不清「在算」还是「卡死了」。取消 = 直接杀子进程（见 `mat_worker_host`）。
 *
 * 进度百分比这里用**阶段权重**：HPROF 不记录总记录数，阶段内部只能报
 * 「已处理 N 条」，给不出真实比例；有 `total`（支配树按可达对象数）时才用真比例。
 */
import { computed } from 'vue'
import { useMatStore } from '../store'
import { workerPct } from '../workerPhase'

const props = withDefaults(
  defineProps<{
    dumpId: string
    /** 说明文案（各视图的等待原因不同，默认给通用一句） */
    hint?: string
    /** 是否显示「已处理 N 条」计数 */
    counters?: boolean
    /** 是否允许在这里取消（默认允许） */
    cancelable?: boolean
  }>(),
  {
    hint: '分析跑在独立子进程里，主窗口可以正常操作；完成后本页会自动重新加载。',
    counters: false,
    cancelable: true,
  },
)

const store = useMatStore()

const job = computed(() => store.workerJobOf(props.dumpId))

const pct = computed(() => workerPct(job.value))

const counters = computed(() => {
  const j = job.value
  if (!props.counters || !j || j.finished) return ''
  const parts: string[] = []
  if (j.total) parts.push(`${j.done} / ${j.total}`)
  else if (j.done) parts.push(`已处理 ${j.done} 条`)
  if (j.elapsed_ms) parts.push(`${(j.elapsed_ms / 1000).toFixed(1)} s`)
  return parts.join(' · ')
})

function onCancel() {
  void store.cancelWorkerOf(props.dumpId)
}
</script>

<style scoped>
.wprog {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  max-width: 460px;
  text-align: center;
}

.wprog-tip {
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}

.wprog-bar {
  width: 260px;
}

.wprog-sub {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
}
</style>
