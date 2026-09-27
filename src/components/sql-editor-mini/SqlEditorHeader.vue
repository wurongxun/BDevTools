<template>
  <div class="sql-header" :class="{ 'is-expanded': expanded }">
    <!-- ========== 折叠态：单行头部 ========== -->
    <div v-if="!expanded" class="qr-header-collapsed" @click="toggleExpand">
      <!-- 左侧：SQL 语句预览（截断显示） + 展开按钮 -->
      <div class="sql-preview" :title="modelValue">
        <span class="sql-icon">
          <svg viewBox="0 0 16 16" width="18" height="18" fill="currentColor">
            <path d="M2 3h12l-2 4 2 6H2l2-6z" opacity=".2"/>
            <path d="M3 4h10M3 7h10M3 10h6" stroke="currentColor" stroke-width="1.2" fill="none" stroke-linecap="round"/>
          </svg>
        </span>
        <span class="sql-text">{{ trimmedSql }}</span>
        <span v-if="modelValue.length > 120" class="sql-ellipsis">…</span>
      </div>

      <!-- 展开/折叠图标 -->
      <el-tooltip content="展开编辑器" placement="bottom">
        <button class="icon-btn expand-btn" aria-label="展开编辑器" @click.stop="toggleExpand">
          <el-icon><Expand /></el-icon>
        </button>
      </el-tooltip>

      <div class="divider-v"></div>

      <!-- 中间：快速过滤框（单行）：仅过滤当前页结果，不改 SQL -->
      <div class="quick-filter" @click.stop>
        <el-input
          v-model="filterText"
          size="small"
          class="filter-input"
          :placeholder="filterPlaceholder"
          clearable
          @keydown.enter="handleFilter"
        />
      </div>

      <div class="divider-v"></div>

      <!-- 右侧：工具栏（运行 / 模板 / 撤销 · 重做）
           导入、导出、保存一律不放这里：导出在结果区底部 ResultFooter，保存由编辑器 Tab 负责 -->
      <div class="toolbar" @click.stop>
        <!-- 运行按钮（主按钮，带下拉）
             用原生 button 而非 el-button：el-dropdown 需要拿到真实 DOM 作为触发锚点，
             app-button 不透传 DOM 引用；这里与 ToolDropdown 保持同一写法 -->
        <el-dropdown trigger="click" @command="handleRunCommand">
          <button
            class="run-btn"
            type="button"
            :disabled="loading"
            :aria-label="`${runLabel} (Ctrl+Enter)`"
          >
            <el-icon class="play-icon"><VideoPlay /></el-icon>
            <span class="run-label">{{ runLabel }}</span>
            <el-icon class="arrow-down"><ArrowDown /></el-icon>
          </button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="run">执行 (Ctrl+Enter)</el-dropdown-item>
              <el-dropdown-item command="run-all">执行所有语句</el-dropdown-item>
              <el-dropdown-item command="explain">解释执行计划</el-dropdown-item>
              <el-dropdown-item command="count">统计行数</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>

        <div class="divider-v thin"></div>

        <!-- SQL 模板 / 代码片段：追加到编辑器末尾 -->
        <ToolDropdown icon="EditPen" title="SQL 模板" @command="cmd => emit('template', cmd)">
          <el-dropdown-menu>
            <el-dropdown-item command="select">SELECT * FROM</el-dropdown-item>
            <el-dropdown-item command="insert">INSERT INTO</el-dropdown-item>
            <el-dropdown-item command="update">UPDATE … SET</el-dropdown-item>
            <el-dropdown-item command="delete">DELETE FROM</el-dropdown-item>
            <el-dropdown-item divided command="count-rows">SELECT COUNT(*) FROM</el-dropdown-item>
          </el-dropdown-menu>
        </ToolDropdown>

        <!-- 历史：撤销 / 重做 -->
        <ToolBtn icon="Back" title="撤销 (Ctrl+Z)" :disabled="!canUndo" @click="emit('undo')" />
        <ToolBtn icon="Right" title="重做 (Ctrl+Y)" :disabled="!canRedo" @click="emit('redo')" />
      </div>
    </div>

    <!-- ========== 展开态：多行 SQL 编辑器 ========== -->
    <div v-else class="qr-header-expanded">
      <div class="sql-editor-wrap">
        <!-- 编辑器顶部：SQL 标签/表名标识 -->
        <div class="editor-topbar">
          <span class="lang-tag">
            <el-icon><Document /></el-icon>
            <span>SQL</span>
          </span>
          <span class="charset-tag">{{ tableName || 'untitled' }}</span>
          <div class="editor-actions">
            <el-tooltip content="格式化 SQL" placement="bottom">
              <button class="icon-btn small" aria-label="格式化 SQL" @click="handleFormat">
                <el-icon><MagicStick /></el-icon>
              </button>
            </el-tooltip>
            <el-tooltip content="折叠 (Esc)" placement="bottom">
              <button class="icon-btn small" aria-label="折叠编辑器" @click="toggleExpand">
                <el-icon><Fold /></el-icon>
              </button>
            </el-tooltip>
          </div>
        </div>

        <!-- SQL 编辑区 -->
        <el-input
          ref="sqlInputRef"
          :model-value="modelValue"
          type="textarea"
          :rows="rows"
          class="sql-textarea"
          :class="{ 'is-single': isSingleTable }"
          placeholder="输入 SQL 语句，点击「运行」或按 Ctrl/Cmd + Enter 执行..."
          resize="none"
          @update:model-value="emit('update:modelValue', $event)"
          @keydown.ctrl.enter.prevent="handleRun"
          @keydown.meta.enter.prevent="handleRun"
          @keydown.esc.prevent="toggleExpand"
          @keydown.ctrl.z.prevent="emit('undo')"
          @keydown.ctrl.y.prevent="emit('redo')"
        />

        <!-- 编辑器底栏：状态 + 辅助操作 -->
        <div class="editor-bottombar">
          <div class="status-info">
            <span v-if="isSingleTable" class="badge single">单表查询 · 支持增删改</span>
            <span v-else class="badge multi">多表查询 · 仅可查询/导出</span>
            <span class="line-info">{{ lineCount }} 行 · {{ charCount }} 字符</span>
          </div>
          <div class="bottom-actions">
            <!-- 过滤框（展开态也保留） -->
            <el-input
              v-model="filterText"
              size="small"
              class="filter-input-expanded"
              :placeholder="filterPlaceholder"
              clearable
              @keydown.enter="handleFilter"
            />
            <app-button size="small" @click="emit('cancel')">取消</app-button>
            <app-button size="small" type="primary" :loading="loading" @click="handleRun">
              <el-icon><VideoPlay /></el-icon>
              <span style="margin-left:4px">运行 (Ctrl+Enter)</span>
            </app-button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import {
  Expand, Fold, VideoPlay, ArrowDown, Document,
  MagicStick,
} from '@element-plus/icons-vue'
import ToolBtn from './ToolBtn.vue'
import ToolDropdown from './ToolDropdown.vue'

/**
 * SQL 结果区的头部编辑器（DBeaver 风格）
 *
 * 职责边界：**只负责「编辑 + 执行动作」，不负责数据落盘与结果导出**。
 * 保存由编辑器 Tab 负责，导出/分页/统计在结果区底部 ResultFooter，
 * 因此这里刻意不提供保存、导出、导入按钮（重复入口会造成行为分歧）。
 *
 * 折叠态单行预览 + 展开态多行编辑；两种形态都保留过滤框。
 */
interface Props {
  modelValue: string
  loading?: boolean
  /** 展示用的表名/标识（顶部标签栏） */
  tableName?: string
  rows?: number
  /** 单表查询 → 展示「可增删改」徽标 */
  isSingleTable?: boolean
  canUndo?: boolean
  canRedo?: boolean
  runLabel?: string
  filterPlaceholder?: string
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  tableName: '',
  rows: 6,
  isSingleTable: true,
  canUndo: false,
  canRedo: false,
  runLabel: '运行',
  filterPlaceholder: '输入关键字过滤当前页结果',
})

const emit = defineEmits<{
  (e: 'update:modelValue', v: string): void
  (e: 'run', sql: string): void
  (e: 'run-command', cmd: string): void
  (e: 'filter', keyword: string): void
  (e: 'undo'): void
  (e: 'redo'): void
  (e: 'format'): void
  (e: 'cancel'): void
  (e: 'template', cmd: string): void
  (e: 'expand-change', expanded: boolean): void
}>()

const expanded = ref(false)
const filterText = ref('')
const sqlInputRef = ref()

const trimmedSql = computed(() => {
  const s = props.modelValue.replace(/\s+/g, ' ').trim()
  return s.length > 120 ? s.slice(0, 120) : s
})

const lineCount = computed(() => props.modelValue.split('\n').length)
const charCount = computed(() => props.modelValue.length)

const toggleExpand = async () => {
  expanded.value = !expanded.value
  emit('expand-change', expanded.value)
  if (expanded.value) {
    await nextTick()
    const textarea = sqlInputRef.value?.textarea as HTMLTextAreaElement
    textarea?.focus()
  }
}

const handleRun = () => emit('run', props.modelValue)
const handleFilter = () => emit('filter', filterText.value)
const handleFormat = () => emit('format')
const handleRunCommand = (cmd: string) => emit('run-command', cmd)

// 外部可调用：折叠 / 展开 / 写入过滤词
defineExpose({
  collapse: () => (expanded.value = false),
  expand: () => { expanded.value = true; nextTick(() => sqlInputRef.value?.textarea?.focus()) },
  setFilter: (v: string) => (filterText.value = v),
})

// 展开态允许用 Esc 收起（编辑区内 Esc 已在模板里 prevent 掉，这里兜住点击别处的场景）
watch(expanded, (v) => {
  if (v) {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        expanded.value = false
        emit('expand-change', false)
        window.removeEventListener('keydown', onKey)
      }
    }
    window.addEventListener('keydown', onKey)
  }
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

/* 全部取主题令牌 / EP 语义变量：禁止硬编码底色，暗色主题下才不会白得刺眼 */
.sql-header {
  font-family: var(--font-family-base);
  font-size: var(--font-size-sm);
  border: 1px solid var(--el-border-color);
  border-radius: var(--radius-sm, 4px);
  @include theme.useTheme {
    background: theme.getVar('bgColor');
  }
  overflow: hidden;
  user-select: none;
}

/* ========== 折叠态 ========== */
.qr-header-collapsed {
  display: flex;
  align-items: center;
  height: 36px;
  padding: 0 4px;
  gap: 0;
  cursor: pointer;
  background: var(--el-fill-color-lighter);
  transition: background .15s;
  &:hover { background: var(--el-fill-color-light); }
}

.sql-preview {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 0 0 auto;
  min-width: 200px;
  max-width: 520px;
  padding: 0 8px;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  color: var(--el-text-color-primary);
  overflow: hidden;
  .sql-icon {
    display: inline-flex;
    align-items: center;
    color: var(--el-color-warning);
    flex-shrink: 0;
  }
  .sql-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--el-color-primary); // SQL 关键字蓝
    letter-spacing: 0;
  }
  .sql-ellipsis { color: var(--el-text-color-placeholder); }
}

.expand-btn {
  flex-shrink: 0;
  margin: 0 2px;
}

.divider-v {
  width: 1px;
  height: 22px;
  background: var(--el-border-color);
  margin: 0 4px;
  flex-shrink: 0;
  &.thin { background: var(--el-border-color-lighter); margin: 0 2px; height: 18px; }
}

.quick-filter {
  flex: 1 1 auto;
  min-width: 140px;
  padding: 0 4px;
  .filter-input :deep(.el-input__wrapper) {
    background: transparent;
    box-shadow: none;
    font-size: var(--font-size-sm);
    &:hover, &.is-focus { box-shadow: 0 0 0 1px var(--el-color-primary) inset; background: var(--el-bg-color); }
  }
  .filter-input :deep(.el-input__inner) { color: var(--el-text-color-regular); }
  .filter-input :deep(input::placeholder) { color: var(--el-text-color-placeholder); font-style: italic; }
}

/* ========== 工具栏 ========== */
.toolbar {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
  padding: 0 2px;
}
.run-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border: none;
  border-radius: 3px;
  background: var(--el-color-primary);
  color: #fff;
  cursor: pointer;
  transition: background .15s;
  &:hover:not(:disabled) { background: var(--el-color-primary-light-3); }
  &:disabled { background: var(--el-color-primary-light-5); cursor: not-allowed; }
  .play-icon { font-size: 12px; }
  .run-label { font-size: var(--font-size-sm); font-weight: 500; }
  .arrow-down { font-size: 10px; margin-left: 2px; }
}

.icon-btn {
  width: 28px;
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--el-text-color-secondary);
  border-radius: 3px;
  cursor: pointer;
  transition: all .15s;
  &:hover { background: var(--el-fill-color); color: var(--el-color-primary); }
  &:disabled { opacity: .4; cursor: not-allowed; &:hover { background: transparent; color: var(--el-text-color-secondary); } }
  &.small { width: 24px; height: 24px; font-size: 13px; }
}

/* ========== 展开态 ========== */
.qr-header-expanded { padding: 0; background: var(--el-bg-color); }

.sql-editor-wrap { display: flex; flex-direction: column; }

.editor-topbar {
  display: flex;
  align-items: center;
  height: 28px;
  padding: 0 8px;
  background: var(--el-fill-color-lighter);
  border-bottom: 1px solid var(--el-border-color-lighter);
  gap: 8px;
  font-size: var(--font-size-sm);
  .lang-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    background: var(--el-color-info-light-9);
    color: var(--el-color-info);
    border-radius: 3px;
    font-weight: 600;
  }
  .charset-tag {
    color: var(--el-text-color-placeholder);
    font-family: var(--font-family-mono);
  }
  .editor-actions {
    margin-left: auto;
    display: flex;
    gap: 2px;
  }
}

.sql-textarea {
  :deep(.el-textarea__inner) {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-base);
    line-height: 1.6;
    padding: 8px 12px;
    border: none;
    border-radius: 0;
    background: var(--el-bg-color);
    color: var(--el-text-color-primary);
    resize: none;
    box-shadow: none;
    &:focus { box-shadow: none; }
  }
}

.editor-bottombar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 34px;
  padding: 0 8px;
  background: var(--el-fill-color-lighter);
  border-top: 1px solid var(--el-border-color-lighter);
  gap: 12px;
  flex-wrap: wrap;

  .status-info {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: var(--font-size-sm);
    .badge {
      padding: 2px 8px;
      border-radius: 3px;
      font-weight: 500;
      &.single { background: var(--el-color-success-light-9); color: var(--el-color-success); }
      &.multi { background: var(--el-color-warning-light-9); color: var(--el-color-warning); }
    }
    .line-info { color: var(--el-text-color-placeholder); font-family: var(--font-family-mono); }
  }
  .bottom-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    .filter-input-expanded {
      width: 260px;
      :deep(.el-input__wrapper) { font-size: var(--font-size-sm); }
    }
  }
}

/* 下拉菜单项样式统一 */
:deep(.el-dropdown-menu__item) {
  font-size: var(--font-size-base);
}
</style>
