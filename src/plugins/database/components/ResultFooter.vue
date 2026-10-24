<template>
  <div class="result-footer">
    <!-- 左侧统计信息（父组件经 slot 注入，如共 N 行 / 耗时 / 当前页） -->
    <div class="result-info">
      <slot name="info" />
    </div>

    <!-- 右侧操作区：单行左对齐（上一页/下一页、每页行数输入框、导出、刷新） -->
    <div class="result-actions">
      <template v-if="showPager">
        <!--
          编辑类控件集中在这里：新增 / 删除 / 导入 / 只读开关。
          是否显示由父组件判定（Footer 不懂「单表 / 行定位」那些规则）——
          尤其是「只读」开关的显示条件**不能反过来依赖只读状态**，
          否则一开就把它自己藏起来，用户再也关不掉。
        -->
        <el-tooltip
          v-if="showReadOnlySwitch"
          content="开启后可防止误改：网格不再进入编辑态"
          placement="top"
        >
          <el-switch
            :model-value="gridReadOnly"
            size="small"
            active-text="只读"
            inline-prompt
            style="--el-switch-off-color: var(--el-color-info)"
            @update:model-value="onGridReadOnly"
          />
        </el-tooltip>
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
        <el-tooltip content="删除勾选的行（未勾选时不可用）" placement="top">
          <app-button
            v-if="canEdit"
            bare
            :icon="Deleteline"
            size="small"
            :disabled="!hasSelection"
            @click="emit('delete-selected')"
          ></app-button>
        </el-tooltip>
        <el-tooltip content="新增行：定位到表格尾部的空行" placement="top">
          <app-button
            v-if="canInsert"
            bare
            :icon="Addline"
            size="small"
            @click="emit('add-row')"
          ></app-button>
        </el-tooltip>
        <el-tooltip content="导入数据到当前表" placement="top">
          <app-button
            v-if="canEdit"
            bare
            :icon="Import"
            size="small"
            @click="emit('import')"
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
        <!--
          当前页码（可直接输入跳转）+ 总页数。
          总页数是**算出来的**：`ceil(总行数 / 每页行数)`。总行数未知时显示 `?`
          —— 这时候点「合计」统计一次就有数了，绝不能把「不知道」写成「共 1 页」。
        -->
        <span class="page-field" :title="pageFieldTitle">
          <span class="field-label">第</span>
          <el-input
            :model-value="pageDraft"
            size="small"
            class="page-input"
            type="number"
            min="1"
            :max="totalPages ?? undefined"
            aria-label="当前页码（可直接输入跳转）"
            @update:model-value="onPageInput"
            @keyup.enter="onPageCommit"
            @blur="onPageCommit"
          />
          <span class="field-label">/{{ totalPagesText }}</span>
        </span>
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

        <label class="page-size-field" title="每页行数（不设上限，数值由你自己控制）">
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

        <!--
          读取所有行：只在**确实还有没取回来的数据**时出现（hasMore=false 说明已经在看全部，
          再给一个「读取所有行」就是噪音）。取多大批、要不要确认由父组件决定 —— 这里只是入口。
        -->
        <el-tooltip content="读取所有行（忽略每页行数，一次取回全部结果）" placement="top">
          <app-button
            v-if="hasMore"
            bare
            :icon="Bottom"
            size="small"
            @click="emit('fetch-all')"
          ></app-button>
        </el-tooltip>

        <!-- 总行数未知时（非查询语句 / 表浏览）不显示任何数字，别用 0 冒充 -->
        <span
          v-if="total != null"
          class="result-info stat-rows"
          :title="`共 ${total} 行`"
          >{{ total.toLocaleString() }}</span
        >
      </template>
    </div>

    <!--
      导出对话框：按当前查询 SQL 导出为 CSV / Excel。
      版式（label 宽度、路径行、底部按钮）**与导入弹窗 ImportExportDialog 对齐**，
      路径输入直接复用同一个 PathField —— 两套写法迟早会长歪。
    -->
    <GlobalDialog
      v-model="showExportDialog"
      title="导出查询结果"
      width="480px"
      @close="exportPath = ''"
    >
      <el-form label-width="100px">
        <el-form-item label="导出范围">
          <span class="form-static">{{ exportScope || '当前查询的全部结果' }}</span>
        </el-form-item>
        <el-form-item label="导出格式">
          <el-radio-group v-model="exportFormat">
            <el-radio value="csv">CSV</el-radio>
            <el-radio value="excel">Excel</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="保存路径">
          <PathField
            v-model="exportPath"
            placeholder="选择文件保存路径"
            browse-label="浏览保存路径"
            @browse="chooseExportPath"
          />
        </el-form-item>
      </el-form>
      <div class="dialog-note">
        导出由后端重新执行查询语句后写文件：<b>不受分页影响</b>，也不含网格里未保存的改动。
      </div>
      <template #footer>
        <app-button size="small" @click="showExportDialog = false">取消</app-button>
        <app-button
          size="small"
          type="primary"
          :loading="exporting"
          :disabled="!exportPath"
          @click="handleExport"
          >导出</app-button
        >
      </template>
    </GlobalDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { ElMessage } from 'element-plus'
import GlobalDialog from '@/components/GlobalDialog.vue'
import PathField from './PathField.vue'
import { save } from '@/core/runtime'
import { dbExportSqlCsv, dbExportSqlExcel } from '../api/importExport'
import {
  computeTotalPages,
  normalizeIntervalSeconds,
  normalizePageNumber,
  normalizePageSize,
} from '../utils/paging'
import Export from '@/components/Icon/Export.vue'
import Import from '@/components/Icon/Import.vue'
import Addline from '@/components/Icon/Addline.vue'
import Deleteline from '@/components/Icon/Deleteline.vue'

import { Refresh, Bottom } from '@element-plus/icons-vue'
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
    /**
     * 总行数。**null = 未知**（后端只在纯 SELECT 首页统计），这与「0 行」是两回事：
     * 未知时不显示数字，也不能据此算出总页数。
     */
    total?: number | null
    /**
     * 能不能**新增**行：由父组件判定（单表 + 未开只读）。
     * Footer 不自己去推导这些规则 —— 它只负责把入口摆出来。
     */
    canInsert?: boolean
    /** 能不能改 / 删**已有**行（需要可靠的行定位手段），同时决定「导入」按钮 */
    canEdit?: boolean
    /**
     * 是否显示「只读」开关。
     * 刻意与 `gridReadOnly` 分开：**开关的显示条件不能依赖它自己的状态**，
     * 否则一旦打开（canInsert/canEdit 随之变 false）就会把自己藏起来，再也关不掉。
     */
    showReadOnlySwitch?: boolean
    /** 只读状态（v-model:grid-read-only） */
    gridReadOnly?: boolean
    /** 是否有勾选行 —— 没有勾选时「删除」按钮禁用 */
    hasSelection?: boolean
    /**
     * 导出范围说明（由调用方给出，例如「仅勾选的 5 行」）。
     * Footer 只负责显示：真正的选中行过滤是调用方把 SQL 换成只查选中项来做的。
     */
    exportScope?: string
  }>(),
  {
    showPager: true,
    hasMore: false,
    loading: false,
    exportSql: '',
    configId: '',
    allowAutoRefresh: true,
    total: null,
    canInsert: false,
    canEdit: false,
    showReadOnlySwitch: false,
    gridReadOnly: false,
    hasSelection: false,
    exportScope: ''
  }
)

const emit = defineEmits<{
  prev: []
  next: []
  /** 跳转到指定页（页码输入框；由调用方决定怎么取数，Footer 只报页码） */
  'page-change': [page: number]
  'page-size-change': [size: number]
  refresh: []
  totalQuantity: []
  /** 读取所有行（确认与取数都由调用方负责；Footer 只报「用户点了这个按钮」） */
  'fetch-all': []
  /** 新增行：怎么呈现由调用方决定（本项目是滚到表格尾部的空行，不弹窗） */
  'add-row': []
  /** 删除当前勾选的行 */
  'delete-selected': []
  /** 导入数据到当前表（对话框由调用方持有） */
  import: []
  /** 网格只读开关（v-model:grid-read-only） */
  'update:gridReadOnly': [v: boolean]
}>()

const canGoPrev = () => props.page > 1
/**
 * 下一页只认 `hasMore`。**刻意不看总页数** —— 总行数是首页那次查询的快照，
 * 期间数据可能变多；拿快照算出「已经是最后一页」去禁用按钮，只会把新数据挡在门外。
 */
const canGoNext = () => props.hasMore

// ==================== 页码 / 总页数 ====================
/** 总页数；总行数未知 ⇒ null（界面显示 `?`，而不是撒谎说「共 1 页」） */
const totalPages = computed(() => computeTotalPages(props.total, props.pageSize))
const totalPagesText = computed(() => (totalPages.value == null ? '?' : String(totalPages.value)))

const pageFieldTitle = computed(() => {
  const where = `第 ${props.page} 页`
  const pages =
    totalPages.value == null
      ? '总页数未知（点「合计」统计总行数后可算出）'
      : `共 ${totalPages.value} 页`
  return `${where}，${pages}；可直接输入页码后回车跳转`
})

/** 页码输入框的本地缓冲（与每页行数同一个套路：允许临时非法，失焦 / 回车时提交） */
const pageDraft = ref(String(props.page))

watch(
  () => props.page,
  v => {
    pageDraft.value = String(v)
  }
)

function onPageInput(v: string) {
  pageDraft.value = v.replace(/\D/g, '')
}

function onPageCommit() {
  const { page, changed } = normalizePageNumber(pageDraft.value, props.page, totalPages.value)
  // 无论成不成都要把输入框拉回真实页码：不然越界输入会一直挂在界面上
  pageDraft.value = String(page)
  if (changed) emit('page-change', page)
}

/** 只读开关：el-switch 的值类型在模板里拿不到精确收窄，这里显式转成 boolean */
function onGridReadOnly(v: unknown) {
  emit('update:gridReadOnly', v === true)
}

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
  // 仅保留数字（不截断长度：位数多少由使用者决定，超界的会在提交时被挡住）
  pageSizeDraft.value = v.replace(/\D/g, '')
}

/**
 * 提交每页行数。**不设上限**：能拉多少行由使用者判断，界面不替他改数字
 * （偷偷钳到某个魔法数字会造成「填了 5000 只回来 1000」的假象）。
 * 规则实现在 `utils/paging.ts`，可被仿真脚本直接 import 源码验证。
 */
function onPageSizeCommit() {
  const { size, changed } = normalizePageSize(pageSizeDraft.value, props.pageSize)
  pageSizeDraft.value = String(size)
  if (changed) {
    emit('page-size-change', size)
  }
}

// ==================== 定时刷新 ====================
const intervalSeconds = ref(0)
let timer: ReturnType<typeof setInterval> | null = null

function onIntervalInput(v: string) {
  const s = normalizeIntervalSeconds(v)
  intervalSeconds.value = s
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
.refresh-interval-field,
.page-field {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.page-input {
  width: 56px;
}
.field-label {
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
  white-space: nowrap;
}
.page-size-input {
  /* 上限放开后可能出现六、七位数，宽度要能容下 */
  width: 84px;
}
.interval-input {
  width: 56px;
}

.stat-rows {
  font-family: var(--font-family-mono);
  white-space: nowrap;
}

/* ↓↓↓ 导出弹窗：与 ImportExportDialog 同一套版式（label 100px / 路径行 / 说明文字） */
.form-static {
  font-size: var(--font-size-md);
  color: var(--el-text-color-primary);
  word-break: break-all;
}
.dialog-note {
  margin-top: 12px;
  font-size: var(--font-size-sm);
  line-height: 1.6;
  color: var(--el-text-color-secondary);
}
</style>
