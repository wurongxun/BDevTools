<template>
  <div class="log-toolbar">
    <!-- 搜索输入 -->
    <div class="tb-search">
      <el-input
        v-model="keyword"
        size="small"
        clearable
        :placeholder="search.mode === 'regex' ? '正则表达式，如 (Timeout|Exception)' : '关键词，回车跳到下一个'"
        @input="onInput"
        @keyup.enter="onEnter"
      >
        <template #prefix>
          <el-icon><Search /></el-icon>
        </template>
      </el-input>
      <span v-if="searchError" class="tb-error" :title="searchError">正则无效</span>
    </div>

    <!-- 匹配模式 -->
    <el-checkbox v-model="isRegex" size="small" @change="onModeChange">正则</el-checkbox>
    <el-checkbox v-model="ignoreCase" size="small" @change="onFlagChange">忽略大小写</el-checkbox>

    <!-- 正则模板 -->
    <el-dropdown trigger="click" @command="applyTemplate">
      <app-button size="small">
        模板
        <el-icon class="el-icon--right"><ArrowDown /></el-icon>
      </app-button>
      <template #dropdown>
        <el-dropdown-menu>
          <el-dropdown-item v-for="t in REGEX_TEMPLATES" :key="t.label" :command="t">
            {{ t.label }}
          </el-dropdown-item>
        </el-dropdown-menu>
      </template>
    </el-dropdown>

    <!-- 命中导航 -->
    <div class="tb-nav">
      <app-button size="small" :disabled="!navTotal" title="上一个匹配" @click="emit('goto', store.gotoPrev())">
        <el-icon><ArrowUp /></el-icon>
      </app-button>
      <app-button size="small" :disabled="!navTotal" title="下一个匹配" @click="emit('goto', store.gotoNext())">
        <el-icon><ArrowDown /></el-icon>
      </app-button>
      <span class="tb-count">
        <el-input
          v-model="jumpText"
          size="small"
          class="tb-jump"
          :disabled="!navTotal"
          @keyup.enter="onJump"
        />
        <span class="tb-sep">/</span>
        <span class="tb-total">{{ navTotal }}</span>
        <!-- 全文件结果可用时以"命中行"为口径，不再叠加本地的"处"计数（两者口径不同，混着显示会误导） -->
        <span v-if="!store.viewActive && total !== hitLines" class="tb-total-sub">（{{ total }} 处）</span>
      </span>
      <!-- 查询中 / 降级说明 -->
      <span v-if="store.viewScanning" class="tb-scan">查询中…</span>
      <!-- 跳转打不通时的原因（原先静默失败，用户只看到"点了没反应"） -->
      <span v-else-if="store.jumpHint" class="tb-scan tb-jump-hint" :title="store.jumpHint">
        {{ store.jumpHint }}
      </span>
      <span v-else-if="store.viewNote" class="tb-scan" :title="store.viewNote">
        {{ store.viewNote }}
      </span>
    </div>

    <div class="tb-spacer"></div>

    <!-- 文件编码控制 -->
    <el-select
      :model-value="currentEncoding"
      size="small"
      class="tb-encoding"
      :disabled="!activeSource"
      title="切换日志文件编码后重新加载"
      @change="onEncodingChange"
    >
      <el-option v-for="enc in ENCODINGS" :key="enc" :label="enc" :value="enc" />
    </el-select>

    <span class="tb-stat">
      已加载 {{ loadedCount }} / {{ activeSource?.totalLines ?? 0 }} 行
    </span>

    <app-button size="small" @click="emit('manage-rules')">
      <el-icon><Brush /></el-icon>
      高亮规则
    </app-button>
    <app-button size="small" :disabled="!activeSource" @click="emit('export')">
      <el-icon><Download /></el-icon>
      导出
    </app-button>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { ArrowDown, ArrowUp, Brush, Download, Search } from '@element-plus/icons-vue'
import { useLogStore } from '../store/logs'
import { REGEX_TEMPLATES, type RegexTemplate } from '../core/patterns'

const emit = defineEmits<{
  (e: 'goto', lineNo: number | null): void
  (e: 'manage-rules'): void
  (e: 'export'): void
  (e: 'toggle-timeline'): void
}>()

const props = defineProps<{ timelineVisible?: boolean }>()

const store = useLogStore()
const keyword = ref('')
const isRegex = ref(false)
const ignoreCase = ref(false)
const jumpText = ref('')

const search = computed(() => store.search)
const searchError = computed(() => store.searchError)
const total = computed(() => store.result.total)
const hitLines = computed(() => store.result.hits.length)
/** 导航总数：全文件命中行数（不可用时回退为已加载命中行数） */
const navTotal = computed(() => store.navTotal)
const loadedCount = computed(() => store.loadedCount)
const activeSource = computed(() => store.activeSource)

/** 编码选项（与后端 encoding_rs 支持范围对齐） */
const ENCODINGS = [
  'UTF-8',
  'GBK',
  'GB18030',
  'GB2312',
  'BIG5',
  'UTF-16LE',
  'UTF-16BE',
  'ISO-8859-1',
  'WINDOWS-1252',
  'SHIFT_JIS',
  'EUC-JP',
  'ASCII',
]

/** 当前源的编码（编码选择器跟随当前激活源） */
const currentEncoding = computed(() => store.encoding || 'UTF-8')

function onEncodingChange(val: string) {
  store.setEncoding(val)
}

/** 输入防抖：保证输入后 30ms 内完成扫描与高亮刷新（需求要求 40ms 内） */
let timer: number | null = null
function onInput(val: string) {
  if (timer) window.clearTimeout(timer)
  timer = window.setTimeout(() => {
    store.setSearch({ keyword: val })
  }, 30)
}

function onModeChange(val: boolean) {
  store.setSearch({ mode: val ? 'regex' : 'text', keyword: keyword.value })
}

function onFlagChange(val: boolean) {
  store.setSearch({ ignoreCase: val })
}

function onEnter() {
  if (timer) window.clearTimeout(timer)
  store.setSearch({ keyword: keyword.value })
  emit('goto', store.gotoNext())
}

function onJump() {
  const n = Number(jumpText.value)
  if (!Number.isFinite(n) || n < 1) return
  emit('goto', store.gotoMatch(n))
}

function applyTemplate(t: RegexTemplate) {
  keyword.value = t.pattern
  isRegex.value = true
  store.setSearch({ keyword: t.pattern, mode: 'regex' })
}

/** 当前命中序号回写到输入框（跳转后同步显示） */
watch(
  () => store.currentMatchNo,
  n => {
    jumpText.value = n > 0 ? String(n) : ''
  },
)

watch(
  () => store.search.keyword,
  v => {
    if (v !== keyword.value) keyword.value = v
  },
)

/** 切换 tab 时同步正则 / 忽略大小写开关，使每个 tab 各自记忆搜索模式 */
watch(
  () => store.search.mode === 'regex',
  v => {
    if (v !== isRegex.value) isRegex.value = v
  },
)
watch(
  () => store.search.ignoreCase,
  v => {
    if (v !== ignoreCase.value) ignoreCase.value = v
  },
)
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.log-toolbar {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: nowrap;
  padding: 4px 8px;
  // 工具条用 sidebarBg（与 TabBar 容器同源：作工具栏条，比内容区略深一档）
  @include theme.useTheme {
    background: theme.getVar('sidebarBg');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.tb-search {
  position: relative;
  width: 220px;
  flex: none;
}

.tb-error {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  font-size: var(--font-size-xs);
  color: var(--el-color-danger);
  cursor: help;
}

.tb-nav {
  display: flex;
  align-items: center;
  gap: 4px;
}

.tb-count {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-regular);
}

.tb-jump {
  width: 56px;
}

.tb-sep {
  color: var(--el-text-color-placeholder);
}

.tb-total {
  font-variant-numeric: tabular-nums;
}

.tb-total-sub {
  color: var(--el-text-color-placeholder);
}

/* 全文件扫描中 / 降级说明：与"处"计数同一视觉层级，避免抢主计数的注意力 */
.tb-scan {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
  white-space: nowrap;
  cursor: help;
}

/* 跳转失败是"操作没生效"，用告警色以区别于普通的降级说明 */
.tb-jump-hint {
  color: var(--el-color-warning);
}

.tb-spacer {
  flex: 1;
}

.tb-encoding {
  width: 124px;
  flex-shrink: 0;
}

.tb-stat {
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}
</style>
