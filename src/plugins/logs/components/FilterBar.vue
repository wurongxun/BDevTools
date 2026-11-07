<template>
  <div class="filter-bar">
    <!-- 级别多选 -->
    <div class="fb-group fb-levels">
      <span class="fb-label">级别</span>
      <el-checkbox
        v-for="lv in LEVELS"
        :key="lv"
        class="fb-level"
        :model-value="isChecked(lv)"
        @change="(v: boolean | string | number) => toggleLevel(lv, Boolean(v))"
      >
        <span class="fb-level-tag" :class="`lv-text-${lv.toLowerCase()}`">{{ lv }}</span>
        <span class="fb-count">{{ formatCount(countOf(lv)) }}</span>
      </el-checkbox>
    </div>

    <span class="fb-sep" />

    <!-- 时间范围 -->
    <div class="fb-group fb-time">
      <span class="fb-label">时间</span>
      <input
        class="fb-dt"
        type="datetime-local"
        :value="fromLocal"
        :disabled="!hasTime"
        :step="1"
        @change="(e: Event) => onFromInput(e)"
      />
      <span class="fb-tilde">~</span>
      <input
        class="fb-dt"
        type="datetime-local"
        :value="toLocal"
        :disabled="!hasTime"
        :step="1"
        @change="(e: Event) => onToInput(e)"
      />
      <span v-if="hasTime" class="fb-quicks">
        <button class="fb-quick" @click="quickRange(1)">近1h</button>
        <button class="fb-quick" @click="quickRange(6)">近6h</button>
        <button class="fb-quick" @click="quickRange(24)">近24h</button>
        <button class="fb-quick" @click="quickRange(null)">全部</button>
      </span>
    </div>

    <!-- 时间滑块 -->
    <div v-if="hasTime" class="fb-group fb-slider">
      <input
        class="fb-range"
        type="range"
        :min="sliderMin"
        :max="sliderMax"
        :step="sliderStep"
        :value="fromSlider"
        @change="(e: Event) => onSlider(e, 'from')"
      />
      <input
        class="fb-range"
        type="range"
        :min="sliderMin"
        :max="sliderMax"
        :step="sliderStep"
        :value="toSlider"
        @change="(e: Event) => onSlider(e, 'to')"
      />
    </div>

    <span class="fb-sep" />

    <!-- 反向过滤 -->
    <label class="fb-invert">
      <el-switch
        size="small"
        :model-value="store.filter.invert"
        @change="(v: boolean | string | number) => store.applyFilter({ invert: Boolean(v) })"
      />
      <span>反向过滤</span>
    </label>

    <!-- 状态栏 -->
    <div v-if="store.filterActive" class="fb-status">
      <span class="fb-status-text">{{ statusText }}</span>
      <app-button size="small" text type="primary" @click="store.clearFilter()">清除筛选</app-button>
    </div>

    <div v-if="store.filterError" class="fb-error">
      <el-icon><WarningFilled /></el-icon>
      <span>{{ store.filterError }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { WarningFilled } from '@element-plus/icons-vue'
import { useLogStore } from '../store/logs'
import { LEVEL_ORDER, type LogLevel } from '../types'
import { LEVEL_CODES } from '../core/indexApi'
import { formatCount, fromLocalInput, toLocalInput } from '../core/filter'

const store = useLogStore()
const LEVELS = LEVEL_ORDER

const hasTime = computed(() => store.indexMeta?.ts_min != null && store.indexMeta?.ts_max != null)

/** 默认（未设级别筛选）表示全选；选中集 = 全部 7 级时后端收到空数组（不限级别） */
const selectedAll = computed(() => store.filter.levels.length === 0)
function isChecked(lv: LogLevel): boolean {
  return selectedAll.value ? true : store.filter.levels.includes(lv)
}
function toggleLevel(lv: LogLevel, val: boolean) {
  const base = selectedAll.value ? new Set(LEVEL_ORDER) : new Set(store.filter.levels)
  if (val) base.add(lv)
  else base.delete(lv)
  const arr = [...base]
  store.applyFilter({ levels: arr.length === LEVEL_ORDER.length ? [] : arr })
}

function countOf(lv: LogLevel): number {
  const m = store.indexMeta
  if (!m || !Array.isArray(m.levels)) return 0
  const i = LEVEL_CODES.indexOf(lv)
  return i < 0 ? 0 : m.levels[i] ?? 0
}

// ==================== 时间：输入框 ====================
const fromLocal = computed(() => toLocalInput(store.filter.tsFrom))
const toLocal = computed(() => toLocalInput(store.filter.tsTo))
function onFromInput(e: Event) {
  const v = (e.target as HTMLInputElement).value
  store.applyFilter({ tsFrom: fromLocalInput(v) })
}
function onToInput(e: Event) {
  const v = (e.target as HTMLInputElement).value
  store.applyFilter({ tsTo: fromLocalInput(v) })
}

// ==================== 时间：滑块 ====================
const sliderMin = computed(() => store.indexMeta?.ts_min ?? 0)
const sliderMax = computed(() => store.indexMeta?.ts_max ?? 0)
const sliderStep = computed(() => {
  const span = sliderMax.value - sliderMin.value
  return span > 0 ? Math.max(1, Math.floor(span / 200)) : 1
})
const fromSlider = computed(() => store.filter.tsFrom ?? sliderMin.value)
const toSlider = computed(() => store.filter.tsTo ?? sliderMax.value)
function onSlider(e: Event, kind: 'from' | 'to') {
  const v = Number((e.target as HTMLInputElement).value)
  store.applyFilter(kind === 'from' ? { tsFrom: v } : { tsTo: v })
}

/** 快捷区间：以日志最新时间为锚点往前推 */
const anchor = computed(() => store.indexMeta?.ts_max ?? Date.now())
function quickRange(hours: number | null) {
  if (hours === null) {
    store.applyFilter({ tsFrom: null, tsTo: null })
    return
  }
  store.applyFilter({ tsFrom: anchor.value - hours * 3_600_000, tsTo: anchor.value })
}

// ==================== 状态栏 ====================
const statusText = computed(() => {
  if (!store.filterActive) return ''
  return `命中 ${formatCount(store.filterMatched)} 行${store.filterTruncated ? '（结果已截断，建议缩小范围）' : ''}`
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.filter-bar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 2px 6px;
  padding: 3px 6px;
  font-size: var(--font-size-sm);
  // 筛选条用 sidebarBg（与 TabBar/LogToolbar 同源：工具栏条）
  @include theme.useTheme {
    background: theme.getVar('sidebarBg');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.fb-group {
  display: flex;
  align-items: center;
  gap: 3px;
}

.fb-label {
  color: var(--el-text-color-secondary);
  font-weight: 500;
}

.fb-levels {
  flex-wrap: wrap;
}

.fb-level {
  margin-right: 0;
  height: 20px;

  :deep(.el-checkbox__label) {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding-left: 3px;
  }
}

.fb-level-tag {
  font-size: var(--font-size-xs);
  font-weight: 600;
  letter-spacing: 0.3px;
}

.fb-count {
  color: var(--el-text-color-placeholder);
  font-size: var(--font-size-xs);
}

.fb-dt {
  width: 136px;
  height: 22px;
  padding: 0 6px;
  border: 1px solid var(--el-border-color);
  border-radius: var(--radius-sm);
  background: var(--el-bg-color);
  color: var(--el-text-color-primary);
  font-size: var(--font-size-sm);
  font-family: var(--font-family-mono);

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  &:focus {
    border-color: var(--el-color-primary);
  }

  &:focus-visible {
    outline: 2px solid var(--focus-ring-color);
    outline-offset: -1px;
  }
}

.fb-tilde {
  color: var(--el-text-color-secondary);
}

/* 快捷区间：合并相邻边框做成紧凑分段控件（桌面工具栏风格），避免每个按钮各自留一圈空白 */
.fb-quicks {
  display: inline-flex;
}

.fb-quick {
  height: 22px;
  padding: 0 6px;
  margin-left: -1px;
  border: 1px solid var(--el-border-color);
  background: var(--el-bg-color);
  color: var(--el-text-color-regular);
  font-size: var(--font-size-xs);
  line-height: 1;
  cursor: pointer;

  &:first-child {
    margin-left: 0;
    border-radius: var(--radius-sm) 0 0 var(--radius-sm);
  }

  &:last-child {
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  }

  &:hover {
    /* 悬停时抬起，让主色边框完整盖住被相邻按钮压住的那一边 */
    position: relative;
    z-index: 1;
    border-color: var(--el-color-primary);
    color: var(--el-color-primary);
  }
}

.fb-slider {
  flex-direction: column;
  align-items: stretch;
  gap: 0;
  min-width: 112px;
}

/* 原生 range 自带较高 thumb 与上下外边距，压薄后两个滑块才能塞进单行高度 */
.fb-range {
  width: 100%;
  height: 12px;
  margin: 0;
  accent-color: var(--el-color-primary);
  cursor: pointer;

  &::-webkit-slider-thumb {
    width: 10px;
    height: 10px;
    border-radius: var(--radius-full);
    background: var(--el-color-primary);
  }
}

.fb-sep {
  width: 1px;
  align-self: stretch;
  background: var(--el-border-color-lighter);
}

.fb-invert {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--el-text-color-regular);
  cursor: pointer;
  user-select: none;
}

.fb-status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
}

.fb-status-text {
  color: var(--el-text-color-secondary);
  font-family: var(--font-family-mono);
}

.fb-error {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--stateErrorFg);
}
</style>
