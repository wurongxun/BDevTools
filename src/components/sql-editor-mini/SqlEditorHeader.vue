<template>
  <div class="sql-header" :class="{ 'is-expanded': expanded }">
    <!-- ========== 折叠态单行头部 ========== -->
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
        <button class="icon-btn expand-btn" @click.stop="toggleExpand">
          <el-icon><Expand /></el-icon>
        </button>
      </el-tooltip>

      <!-- 分隔符 -->
      <div class="divider-v"></div>

      <!-- 中间：快速过滤框（单行） -->
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

      <!-- 右侧：工具栏按钮组（仿 DBeaver 风格，图标按钮 + 下拉箭头） -->
      <div class="toolbar" @click.stop>
        <!-- 运行按钮（主按钮，带下拉） -->
        <el-dropdown trigger="click" @command="handleRunCommand">
          <el-button size="small" type="primary" class="run-btn" :loading="loading">
            <el-icon class="play-icon"><VideoPlay /></el-icon>
            <span class="run-label">{{ runLabel }}</span>
            <el-icon class="arrow-down"><ArrowDown /></el-icon>
          </el-button>
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

        <!-- 保存 / 导出 -->
        <ToolBtn icon="Document" :title="'保存SQL (Ctrl+S)'" @click="emit('save')" />
        <ToolDropdown icon="Download" title="导出结果">
          <el-dropdown-menu>
            <el-dropdown-item command="csv">导出为 CSV</el-dropdown-item>
            <el-dropdown-item command="excel">导出为 Excel</el-dropdown-item>
            <el-dropdown-item command="json">导出为 JSON</el-dropdown-item>
            <el-dropdown-item command="insert">导出为 INSERT 语句</el-dropdown-item>
          </el-dropdown-menu>
        </ToolDropdown>

        <div class="divider-v thin"></div>

        <!-- SQL 模板 / 代码片段 -->
        <ToolDropdown icon="EditPen" title="SQL 模板">
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

        <div class="divider-v thin"></div>

        <!-- 结果集切换 -->
        <ToolDropdown icon="Tickets" title="结果集展示">
          <el-dropdown-menu>
            <el-dropdown-item command="grid">网格视图</el-dropdown-item>
            <el-dropdown-item command="text">文本视图</el-dropdown-item>
            <el-dropdown-item command="json-view">JSON 视图</el-dropdown-item>
          </el-dropdown-menu>
        </ToolDropdown>
      </div>
    </div>

    <!-- ========== 展开态：多行 SQL 编辑器 ========== -->
    <div v-else class="qr-header-expanded">
      <div class="sql-editor-wrap">
        <!-- 编辑器顶部：SQL 标签/语言标识 -->
        <div class="editor-topbar">
          <span class="lang-tag">
            <el-icon><Document /></el-icon>
            <span>SQL</span>
          </span>
          <span class="charset-tag">{{ tableName || 'untitled' }}</span>
          <div class="editor-actions">
            <el-tooltip content="格式化 SQL" placement="bottom">
              <button class="icon-btn small" @click="handleFormat">
                <el-icon><MagicStick /></el-icon>
              </button>
            </el-tooltip>
            <el-tooltip content="折叠 (Esc)" placement="bottom">
              <button class="icon-btn small" @click="toggleExpand">
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
          @keydown.ctrl.s.prevent="emit('save')"
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
              placeholder="输入表达式过滤结果 (Ctrl+Space 补全)"
              clearable
              @keydown.enter="handleFilter"
              @keydown.ctrl.space.prevent="emit('complete')"
            />
            <el-button size="small" @click="emit('cancel')">取消</el-button>
            <el-button size="small" type="primary" :loading="loading" @click="handleRun">
              <el-icon><VideoPlay /></el-icon>
              <span style="margin-left:4px">运行 (Ctrl+Enter)</span>
            </el-button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import {
  Expand, Fold, VideoPlay, ArrowDown, Document, Download,
  EditPen, Back, Right, Tickets, MagicStick,
} from '@element-plus/icons-vue'
import ToolBtn from './ToolBtn.vue'
import ToolDropdown from './ToolDropdown.vue'

interface Props {
  modelValue: string
  loading?: boolean
  tableName?: string
  rows?: number
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
  filterPlaceholder: '输入一个SQL表达式来过滤结果 (使用 Ctrl+Space)',
})

const emit = defineEmits<{
  (e: 'update:modelValue', v: string): void
  (e: 'run', sql: string): void
  (e: 'run-command', cmd: string): void
  (e: 'filter', keyword: string): void
  (e: 'save'): void
  (e: 'undo'): void
  (e: 'redo'): void
  (e: 'format'): void
  (e: 'complete'): void
  (e: 'cancel'): void
  (e: 'template', cmd: string): void
  (e: 'export', fmt: string): void
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

// 外部可调用：关闭折叠
defineExpose({
  collapse: () => (expanded.value = false),
  expand: () => { expanded.value = true; nextTick(() => sqlInputRef.value?.textarea?.focus()) },
  setFilter: (v: string) => (filterText.value = v),
})

// 默认允许父组件用 esc 关闭
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
.sql-header {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', sans-serif;
  border: 1px solid #d0d7de;
  border-radius: 4px;
  background: #fafbfc;
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
  background: linear-gradient(to bottom, #ffffff, #f6f8fa);
  transition: background .15s;
  &:hover { background: linear-gradient(to bottom, #f0f6ff, #e8f0fe); }
}

.sql-preview {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 0 0 auto;
  min-width: 260px;
  max-width: 520px;
  padding: 0 8px;
  font-family: 'JetBrains Mono', 'Consolas', 'Monaco', monospace;
  font-size: 13px;
  color: #24292f;
  overflow: hidden;
  .sql-icon {
    display: inline-flex;
    align-items: center;
    color: #f59e0b;
    flex-shrink: 0;
  }
  .sql-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: #0550ae; // SQL 关键字蓝
    letter-spacing: 0;
  }
  .sql-ellipsis { color: #8c959f; }
}

.expand-btn {
  flex-shrink: 0;
  margin: 0 2px;
}

.divider-v {
  width: 1px;
  height: 22px;
  background: #d0d7de;
  margin: 0 4px;
  &.thin { background: #e5e9ef; margin: 0 2px; height: 18px; }
}

.quick-filter {
  flex: 1 1 auto;
  min-width: 180px;
  padding: 0 4px;
  .filter-input :deep(.el-input__wrapper) {
    background: transparent;
    box-shadow: none;
    font-size: 12px;
    &:hover, &.is-focus { box-shadow: 0 0 0 1px #0969da inset; background: #fff; }
  }
  .filter-input :deep(.el-input__inner) { color: #57606a; }
  .filter-input :deep(input::placeholder) { color: #8c959f; font-style: italic; }
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
  .play-icon { font-size: 12px; }
  .run-label { font-size: 12px; font-weight: 500; }
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
  color: #57606a;
  border-radius: 3px;
  cursor: pointer;
  transition: all .15s;
  &:hover { background: #eaeef2; color: #0969da; }
  &:disabled { opacity: .4; cursor: not-allowed; &:hover { background: transparent; color: #57606a; } }
  &.small { width: 24px; height: 24px; font-size: 13px; }
}

/* ========== 展开态 ========== */
.qr-header-expanded { padding: 0; background: #fff; }

.sql-editor-wrap { display: flex; flex-direction: column; }

.editor-topbar {
  display: flex;
  align-items: center;
  height: 28px;
  padding: 0 8px;
  background: #f6f8fa;
  border-bottom: 1px solid #e5e9ef;
  gap: 8px;
  font-size: 12px;
  .lang-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    background: #ddf4ff;
    color: #0969da;
    border-radius: 3px;
    font-weight: 600;
  }
  .charset-tag {
    color: #8c959f;
    font-family: monospace;
  }
  .editor-actions {
    margin-left: auto;
    display: flex;
    gap: 2px;
  }
}

.sql-textarea {
  :deep(.el-textarea__inner) {
    font-family: 'JetBrains Mono', 'Consolas', 'Monaco', 'Fira Code', monospace;
    font-size: 13px;
    line-height: 1.6;
    padding: 8px 12px;
    border: none;
    border-radius: 0;
    background: #fff;
    color: #24292f;
    resize: none;
    box-shadow: none;
    &:focus { box-shadow: none; }
  }
}

.editor-bottombar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 34px;
  padding: 0 8px;
  background: #f6f8fa;
  border-top: 1px solid #e5e9ef;
  gap: 12px;

  .status-info {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
    .badge {
      padding: 2px 8px;
      border-radius: 3px;
      font-weight: 500;
      &.single { background: #dafbe1; color: #1a7f37; }
      &.multi { background: #fff8c5; color: #9a6700; }
    }
    .line-info { color: #8c959f; font-family: monospace; }
  }
  .bottom-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    .filter-input-expanded {
      width: 320px;
      :deep(.el-input__wrapper) { font-size: 12px; }
    }
  }
}

/* 下拉菜单项样式统一 */
:deep(.el-dropdown-menu__item) {
  font-size: 13px;
}
</style>
