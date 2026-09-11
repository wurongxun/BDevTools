<template>
  <div class="result-table">
    <el-table
      :data="rows"
      border
      size="small"
      style="width: 100%; height: 100%;"
      stripe
      v-loading="loading"
      element-loading-text="加载中…"
    >
      <el-table-column
        v-for="(col, idx) in columns"
        :key="idx"
        :prop="String(idx)"
        :label="col"
        min-width="240"
        show-overflow-tooltip
      />
    </el-table>
  </div>
</template>

<script setup lang="ts">
/**
 * 只读 SQL 查询结果表格
 * 数据已由父组件映射为列名数组 + 行对象数组（键为列索引字符串）
 */
defineProps<{
  /** 列名 */
  columns: string[]
  /** 行数据（键为 String(列索引)） */
  rows: Record<string, any>[]
  /** 加载中 */
  loading?: boolean
}>()
</script>

<style scoped>
.result-table {
  flex: 1;
  overflow: auto;
  min-height: 0;
}
</style>
