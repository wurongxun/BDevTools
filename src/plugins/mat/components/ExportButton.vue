<template>
  <app-button
    size="small"
    text
    :disabled="disabled || rows.length === 0"
    :aria-label="`导出 ${label} 为 CSV`"
    @click="doExport"
  >
    <el-icon><Download /></el-icon>
    <span>导出{{ label }}</span>
  </app-button>
</template>

<script setup lang="ts">
import { Download } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { exportCsv } from '../utils/export'
import type { ExportColumn } from '../utils/export'

/**
 * 表格结果导出按钮（CSV）。
 * 只导出传入的 rows —— 服务端分页视图导当前页，前端全量视图导全部已加载行，
 * 调用方不要为了导出而额外拉取数据。
 */
const props = withDefaults(
  defineProps<{
    rows: unknown[]
    columns: ExportColumn[]
    filename: string
    label?: string
    disabled?: boolean
  }>(),
  { label: 'CSV', disabled: false }
)

function doExport() {
  try {
    exportCsv(props.filename, props.columns, props.rows as Record<string, unknown>[])
    ElMessage.success(`已导出 ${props.rows.length} 行`)
  } catch (e: any) {
    ElMessage.error('导出失败：' + (e?.message || String(e)))
  }
}
</script>
