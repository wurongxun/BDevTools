<template>
  <div class="sql-result-panel">
    <div class="result-header" v-if="result">
      <span class="result-info">
        <template v-if="result.is_query">
          查询结果: {{ result.rows.length }} 行, 耗时 {{ result.execution_time_ms }} ms
        </template>
        <template v-else>
          执行完成: {{ result.affected_rows }} 行受影响, 耗时 {{ result.execution_time_ms }} ms
        </template>
      </span>
    </div>
    <div class="result-table" v-if="result && result.columns.length > 0">
      <el-table :data="tableData" border size="small" max-height="400" style="width: 100%">
        <el-table-column
          v-for="(col, idx) in result.columns"
          :key="idx"
          :prop="idx"
          :label="col"
          min-width="120"
          show-overflow-tooltip
        />
      </el-table>
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

const tableData = computed(() => {
  if (!props.result || !props.result.is_query) return []
  return props.result.rows.map(row => {
    const obj: Record<string, any> = {}
    props.result!.columns.forEach((col, idx) => {
      obj[idx] = row[idx]
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
}
.result-header {
  padding: 6px 12px;
  background: #f5f7fa;
  border-bottom: 1px solid #e4e7ed;
}
.result-info {
  font-size: 12px;
  color: #606266;
}
.result-table {
  flex: 1;
  overflow: auto;
}
.no-result {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
