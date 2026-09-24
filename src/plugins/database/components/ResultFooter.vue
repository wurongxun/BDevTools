<template>
  <div class="result-footer">
    <!-- 左侧统计信息（父组件经 slot 注入，如共 N 行 / 耗时 / 当前页） -->
    <div class="result-info">
      <slot name="info" />
    </div>

    <!-- 右侧操作区：单行左对齐（上一页/下一页、每页行数输入框、导出、刷新） -->
    <div class="result-actions">
      <template v-if="showPager">
        <label class="refresh-interval-field" v-if="allowAutoRefresh">
          <span class="field-label" title="自动刷新间隔（秒），0 表示关闭">定时</span>
          <el-input
            :model-value="String(intervalSeconds)"
            size="small"
            class="interval-input"
            type="number"
            min="0"
            aria-label="自动刷新间隔（秒）"
            @update:model-value="onIntervalInput"
          />
          <span class="field-label">秒</span>
        </label>
        <el-tooltip content="刷新" placement="top">
          <app-button
            size="small"
            bare
            :icon="Refresh"
            @click="emit('refresh')"
            :loading="loading"
          ></app-button>
        </el-tooltip>
        <el-tooltip content="删除行" placement="top">
          <app-button
            v-if="Deleteline"
            bare
            :icon="Deleteline"
            size="small"
            @click="showExportDialog = true"
          ></app-button>
        </el-tooltip>
        <el-tooltip content="新增行" placement="top">
          <app-button
            v-if="Addline"
            bare
            :icon="Addline"
            size="small"
            @click="showExportDialog = true"
          ></app-button>
        </el-tooltip>
        <el-tooltip content="导出" placement="top">
          <app-button
            v-if="exportSql"
            bare
            :icon="Export"
            size="small"
            @click="showExportDialog = true"
          ></app-button>
        </el-tooltip>
        <div class="el-button-group">
          <el-tooltip content="上一页" placement="top">
            <app-button
              size="small"
              bare
              icon="ArrowLeft"
              :disabled="!canGoPrev || loading"
              @click="emit('prev')"
            ></app-button>
          </el-tooltip>
          <el-tooltip content="下一页" placement="top">
            <app-button
              size="small"
              bare
              icon="ArrowRight"
              :disabled="!canGoNext || loading"
              @click="emit('next')"
            ></app-button>
          </el-tooltip>
        </div>

        <label class="page-size-field">
          <el-input
            :model-value="pageSizeDraft"
            size="small"
            class="page-size-input"
            aria-label="每页数据行数"
            @update:model-value="onPageSizeInput"
            @keyup.enter="onPageSizeCommit"
            @blur="onPageSizeCommit"
          />
        </label>

        <el-tooltip content="合计" placement="top">
          <app-button
            bare
            :icon="Total"
            size="small"
            @click="emit('totalQuantity')"
          ></app-button>
        </el-tooltip>

        <span class="result-info" :title="`共 ${total} 行`">{{ total.toLocaleString() }}</span>
      </template>
    </div>

    <!-- 导出对话框：按当前查询 SQL 导出为 CSV / Excel -->
    <GlobalDialog v-model="showExportDialog" title="导出查询结果" width="480px">
      <el-form label-width="100px">
        <el-form-item label="导出格式">
          <el-radio-group v-model="exportFormat">
            <el-radio value="csv">CSV</el-radio>
            <el-radio value="excel">Excel</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="保存路径">
          <el-input v-model="exportPath" placeholder="选择文件保存路径" />
          <app-button size="small" style="margin-left: 8px" @click="chooseExportPath"
            >浏览</app-button
          >
        </el-form-item>
      </el-form>
      <template #footer>
        <app-button size="small" @click="showExportDialog = false">取消</app-button>
        <app-button size="small" type="primary" :loading="exporting" @click="handleExport"
          >导出</app-button
        >
      </template>
    </GlobalDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { ElMessage } from 'element-plus'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { save } from '@/core/runtime'
import { dbExportSqlCsv, dbExportSqlExcel } from '../api/importExport'
import Export from '@/components/Icon/Export.vue'
import Addline from '@/components/Icon/Addline.vue'
import Deleteline from '@/components/Icon/Deleteline.vue'

import { Refresh } from '@element-plus/icons-vue'
import Total from '@/components/Icon/Total.vue'

const props = withDefaults(
  defineProps<{
    /** 当前页 */
    page: number
    /** 每页行数 */
    pageSize: number
    /** 是否显示分页控件（仅查询结果） */
    showPager?: boolean
    /** 是否还有下一页 */
    hasMore?: boolean
    /** 加载中 */
    loading?: boolean
    /** 当前查询 SQL（提供则启用导出按钮） */
    exportSql?: string
    /** 连接 ID（导出需要） */
    configId?: string
    /** 是否允许自动刷新（显示定时输入框） */
    allowAutoRefresh?: boolean
    /** 总数量 */
    total: number
  }>(),
  {
    showPager: true,
    hasMore: false,
    loading: false,
    exportSql: '',
    configId: '',
    allowAutoRefresh: true,
    total: 0
  }
)

const emit = defineEmits<{
  prev: []
  next: []
  'page-size-change': [size: number]
  refresh: []
  totalQuantity: []
}>()

const canGoPrev = () => props.page > 1
const canGoNext = () => props.hasMore

// ==================== 每页行数输入框 ====================
/** 本地输入缓冲（允许临时输入非法值，失焦/回车时校验提交） */
const pageSizeDraft = ref(String(props.pageSize))

watch(
  () => props.pageSize,
  (v) => {
    pageSizeDraft.value = String(v)
  }
)

function onPageSizeInput(v: string) {
  // 仅保留数字
  pageSizeDraft.value = v.replace(/\D/g, '')
}

function onPageSizeCommit() {
  const n = parseInt(pageSizeDraft.value, 10)
  if (!Number.isFinite(n) || n <= 0) {
    // 非法则回退到当前有效值
    pageSizeDraft.value = String(props.pageSize)
    return
  }
  const clamped = Math.min(Math.max(n, 1), 10000)
  pageSizeDraft.value = String(clamped)
  if (clamped !== props.pageSize) {
    emit('page-size-change', clamped)
  }
}

// ==================== 定时刷新 ====================
const intervalSeconds = ref(0)
let timer: ReturnType<typeof setInterval> | null = null

function onIntervalInput(v: string) {
  const s = parseInt(v, 10)
  intervalSeconds.value = Number.isFinite(s) && s > 0 ? s : 0
  restartTimer()
}

function restartTimer() {
  if (timer) {
    clearInterval(timer)
    timer = null
  }
  if (intervalSeconds.value > 0) {
    timer = setInterval(() => {
      emit('refresh')
    }, intervalSeconds.value * 1000)
  }
}

onMounted(() => {
  restartTimer()
})
onUnmounted(() => {
  if (timer) clearInterval(timer)
})

// ==================== 导出 ====================
const showExportDialog = ref(false)
const exporting = ref(false)
const exportFormat = ref<'csv' | 'excel'>('csv')
const exportPath = ref('')

async function chooseExportPath() {
  const ext = exportFormat.value === 'csv' ? 'csv' : 'xlsx'
  const path = await save({
    filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
    defaultPath: `query_result.${ext}`
  })
  if (path) exportPath.value = path
}

async function handleExport() {
  if (!props.exportSql || !props.configId) {
    ElMessage.warning('缺少查询语句或连接信息')
    return
  }
  if (!exportPath.value) {
    ElMessage.warning('请选择保存路径')
    return
  }
  exporting.value = true
  try {
    const fn = exportFormat.value === 'csv' ? dbExportSqlCsv : dbExportSqlExcel
    const res = await fn(props.configId, props.exportSql, exportPath.value)
    if (res.code === 200) {
      ElMessage.success(`导出成功，共 ${res.data.row_count} 行`)
      showExportDialog.value = false
      exportPath.value = ''
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e: any) {
    ElMessage.error('导出失败: ' + (e.message || e))
  } finally {
    exporting.value = false
  }
}
</script>

<style scoped>
.result-footer {
  flex-shrink: 0;
  padding: 6px 12px;
  background: var(--el-fill-color-light);
  border-top: 1px solid var(--el-border-color-lighter);
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}
.result-info {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}
.result-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: auto;
}
.page-size-field,
.refresh-interval-field {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.field-label {
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
  white-space: nowrap;
}
.page-size-input {
  width: 64px;
}
.interval-input {
  width: 56px;
}

.stat-rows {
  font-family: var(--font-family-mono);
  white-space: nowrap;
}
</style>
