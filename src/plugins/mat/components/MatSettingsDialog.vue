<template>
  <!-- 分析预算与规模上限设置：控制 MAT 分析愿意占用的内存（影响可分析堆的最大规模） -->
  <el-dialog
    :model-value="store.settingsOpen"
    title="分析预算与规模上限"
    width="560px"
    append-to-body
    class="msd-dialog"
    @update:model-value="onVisibleChange"
  >
    <!-- 加载 / 失败 -->
    <div v-if="!info" class="msd-body">
      <div v-if="limitState.loading" v-loading="true" class="msd-loading">正在读取本机内存信息…</div>
      <div v-else-if="limitState.error" class="msd-err">
        {{ limitState.error }}
        <app-button size="small" text @click="store.loadLimitInfo()">重试</app-button>
      </div>
    </div>

    <!-- 预算明细与设置 -->
    <div v-else class="msd-body">
      <p class="msd-desc">
        上限随「分析内存预算」换算：预算越大，能分析的堆越大，但分析时内存峰值也越高。
        默认按本机物理内存自动估算；超大堆（数千万元素级）被超限拦截时，调高预算后会自动重试。
      </p>

      <!-- 机器与预算现状 -->
      <div class="msd-stats">
        <div class="msd-stat">
          <span class="msd-stat-v">{{ fmtGb(info.phys_gb) }}</span>
          <span class="msd-stat-k">本机物理内存</span>
        </div>
        <div class="msd-stat">
          <span class="msd-stat-v">{{ fmtGb(info.auto_budget_gb) }}</span>
          <span class="msd-stat-k">自动预算（默认）</span>
        </div>
        <div class="msd-stat cur">
          <span class="msd-stat-v">{{ fmtGb(info.effective_budget_gb) }}</span>
          <span class="msd-stat-k">当前生效预算</span>
        </div>
      </div>

      <!-- 自定义预算输入 -->
      <div class="msd-row">
        <span class="msd-label">自定义预算（GB）</span>
        <el-input
          v-model="budgetText"
          class="msd-input"
          placeholder="留空 = 自动（推荐）"
          clearable
          inputmode="decimal"
        />
        <span class="msd-hint">范围 2 – 48 GB</span>
      </div>

      <!-- 换算上限预览 -->
      <div class="msd-preview">
        <div class="msd-preview-row">
          <span class="msd-preview-k">引用追踪 · 泄漏检测建图</span>
          <span class="msd-preview-v">约 {{ fmtCnt(info.refs_nodes) }} 对象 / {{ fmtCnt(info.refs_edges) }} 引用边</span>
        </div>
        <div class="msd-preview-row">
          <span class="msd-preview-k">泄漏检测支配树（追加）</span>
          <span class="msd-preview-v">约 {{ fmtCnt(info.dom_nodes) }} 可达对象 / {{ fmtCnt(info.dom_edges) }} 边</span>
        </div>
        <div class="msd-preview-tip">调高预算后，对之前「规模超限」的 dump 会自动重新分析；已成功的分析结果保留。</div>
      </div>
    </div>

    <template #footer>
      <div v-if="info" class="msd-footer">
        <app-button size="small" @click="onResetAuto">恢复自动</app-button>
        <span class="msd-flex" />
        <app-button size="small" @click="store.closeSettings()">取消</app-button>
        <app-button size="small" type="primary" :disabled="saving" @click="onSave">
          {{ saving ? '保存中…' : '保存' }}
        </app-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { useMatStore } from '../store'

defineOptions({ name: 'MatSettingsDialog' })

const store = useMatStore()
const limitState = computed(() => store.limitInfo)
const info = computed(() => store.limitInfo.data)

/** 输入框内容（空 = 自动）。仅在弹窗打开、数据首次到达时预填一次，用户输入后不再覆盖 */
const budgetText = ref('')
const saving = ref(false)
let prefilled = false

watch(
  () => [store.settingsOpen, store.limitInfo.data?.override_gb] as const,
  ([open]) => {
    if (open && store.limitInfo.data && !prefilled) {
      prefilled = true
      budgetText.value = store.limitInfo.data.override_gb != null ? String(store.limitInfo.data.override_gb) : ''
    }
    if (!open) prefilled = false
  },
  { immediate: true }
)

function onVisibleChange(v: boolean) {
  if (!v) store.closeSettings()
}

function fmtGb(gb: number): string {
  return gb >= 10 ? `${gb.toFixed(0)} GB` : `${gb.toFixed(1)} GB`
}

function fmtCnt(n: number): string {
  if (n >= 1e8) return `${+(n / 1e8).toFixed(2)} 亿`
  return `${Math.round(n / 1e4)} 万`
}

async function onSave() {
  const t = budgetText.value.trim()
  if (t === '') {
    saving.value = true
    const ok = await store.saveBudget(null)
    saving.value = false
    if (ok) store.closeSettings()
    return
  }
  const num = Number(t)
  if (!Number.isFinite(num) || num < 2 || num > 48) {
    ElMessage.warning('预算需在 2 – 48 GB 之间（留空 = 自动）')
    return
  }
  saving.value = true
  const ok = await store.saveBudget(num)
  saving.value = false
  if (ok) store.closeSettings()
}

async function onResetAuto() {
  budgetText.value = ''
  saving.value = true
  const ok = await store.saveBudget(null)
  saving.value = false
  if (ok) store.closeSettings()
}
</script>

<!-- 弹窗 teleport 到 body，样式需全局（非 scoped） -->
<style>
.msd-body {
  padding: 2px 2px 0;
}

.msd-desc {
  margin: 0 0 14px;
  font-size: var(--font-size-sm);
  line-height: 1.7;
  color: var(--el-text-color-secondary);
}

.msd-stats {
  display: flex;
  gap: 10px;
  margin-bottom: 16px;
}

.msd-stat {
  flex: 1;
  border: 1px solid var(--el-border-color);
  border-radius: var(--radius-md);
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: var(--bgColor);
}

.msd-stat.cur {
  border-color: var(--el-color-primary);
  background: var(--el-color-primary-light-9);
}

.msd-stat-v {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xl);
  color: var(--el-text-color-primary);
}

.msd-stat.cur .msd-stat-v {
  color: var(--el-color-primary);
}

.msd-stat-k {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.msd-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}

.msd-label {
  font-size: var(--font-size-sm);
  color: var(--el-text-color-primary);
  white-space: nowrap;
}

.msd-input {
  width: 180px;
}

.msd-hint {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.msd-preview {
  margin-top: 14px;
  border: 1px dashed var(--el-border-color);
  border-radius: var(--radius-md);
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  background: var(--bgColor);
}

.msd-preview-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 12px;
}

.msd-preview-k {
  font-size: var(--font-size-sm);
  color: var(--el-text-color-regular);
}

.msd-preview-v {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  color: var(--el-text-color-primary);
  text-align: right;
}

.msd-preview-tip {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  border-top: 1px dashed var(--el-border-color);
  padding-top: 6px;
  line-height: 1.6;
}

.msd-footer {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.msd-flex {
  flex: 1;
}

.msd-loading,
.msd-err {
  min-height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--el-text-color-secondary);
  font-size: var(--font-size-sm);
}
</style>
