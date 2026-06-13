<!-- 分页表格组件 -->
<template>
  <div>
    <el-table v-bind="attrs" ref="ElTableRef">
      <!-- 插槽支持 -->
      <template #empty>
        <slot name="empty"></slot>
      </template>
      <template #append>
        <slot name="append"></slot>
      </template>

      <el-table-column
        v-for="(item, index) in (attrs as propsType).columns || []"
        :key="index"
        v-bind="item"
        :align="item.align || 'center'"
      >
        <template #default="scope" v-if="item.type !== 'selection'">
          <div v-if="item.slotName" class="el-td-box">
            <slot :name="item.slotName" :row="scope.row" />
          </div>
          <div v-else-if="!item.formatter" class="el-td-box">
            {{ scope.row[item.prop!] || '--' }}
          </div>
        </template>
      </el-table-column>
      <slot name="default"></slot>
    </el-table>
    <div class="pager-box" v-if="isShowPager">
      <el-pagination
        ref="PaginationRef"
        v-model:current-page="(attrs as propsType).pager!.currentPage"
        v-model:page-size="(attrs as propsType).pager!.pageSize"
        :page-sizes="(attrs as propsType).pager?.pageSizes || [10, 20, 30, 50]"
        :total="(attrs as propsType).pager?.total || 0"
        background
        layout="total, sizes, prev, pager, next, jumper"
      />
    </div>
  </div>
</template>

<script lang="ts" setup name="PagerTable">
import { ref, useAttrs, computed } from 'vue'
import type { propsType } from './types'

let ElTableRef = ref(null)
let PaginationRef = ref(null)
const attrs = useAttrs()

const isShowPager = computed((): boolean => {
  const pagerAttrs = attrs as propsType
  return !!pagerAttrs.pager?.currentPage && !!pagerAttrs.pager?.pageSize
})

defineExpose({
  ElTableRef,
  PaginationRef
})
</script>

<style scoped lang="scss">
.pager-box {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
