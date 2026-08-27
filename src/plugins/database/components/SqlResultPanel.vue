<template>
  <div class="sql-result-panel">
    <div class="result-table" v-if="result && result.columns.length > 0">
      <el-table :data="tableData" border size="small" style="width: 100%; height: 100%;">
        <el-table-column
          v-for="(col, idx) in result.columns"
          :key="idx"
          :prop="String(idx)"
          :label="col"
          min-width="120"
          show-overflow-tooltip
        />
      </el-table>
    </div>
    <div class="result-header" v-if="result">
      <span class="result-info">
        <template v-if="result.is_query">
          查询结果: {{ result.rows.length }} 行, 耗时 {{ result.execution_time_ms }} ms
          <span v-if="result.has_more" class="limit-hint">（还有更多数据，点击工具栏“下一页”继续查看）</span>
          <span v-else-if="result.rows.length > displayLimit" class="limit-hint">（仅显示前 {{ displayLimit }} 行）</span>
        </template>
        <template v-else>
          执行完成: {{ result.affected_rows }} 行受影响, 耗时 {{ result.execution_time_ms }} ms
        </template>
      </span>
    </div>
    <div class="no-result" v-else-if="!result">
      <el-empty description="执行 SQL 查看结果" :image-size="60" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { SqlResult } from '../types'

const props = defineProps<{
  result?: SqlResult | null
}>()

/** 前端最大显示行数（防止大数据集卡死浏览器） */
const displayLimit = 1000

const tableData = computed(() => {
  if (!props.result || !props.result.is_query) return []
  const rows = props.result.rows.slice(0, displayLimit)
  return rows.map(row => {
    const obj: Record<string, any> = {}
    props.result!.columns.forEach((col, idx) => {
      obj[String(idx)] = row[idx]
    })
    return obj
  })
})
</script>

<style scoped>
.sql-result-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.result-header {
  padding: 6px 12px;
  background: #f5f7fa;
  border-bottom: 1px solid #e4e7ed;
}
.result-info {
  font-size: var(--font-size-base);
  color: #606266;
}
.limit-hint {
  color: #e6a23c;
  margin-left: 4px;
}
.result-table {
  flex: 1;
  overflow: hidden;
  min-height: 0;
}
.no-result {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
