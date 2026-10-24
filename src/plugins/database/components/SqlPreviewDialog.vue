<template>
  <GlobalDialog
    v-model="visible"
    title="确认要执行的 SQL"
    width="760px"
    :close-on-click-modal="false"
  >
    <div class="pv-head">
      共 <b>{{ statements.length }}</b> 条语句，涉及 <b>{{ rowCount }}</b> 行。
      全部语句在<b>同一个事务</b>里执行：任意一条失败则整体回滚。
    </div>
    <div v-if="invalidCount > 0" class="pv-warn">
      有 {{ invalidCount }} 条语句无法生成（数据不完整），已排除在提交之外
    </div>
    <div v-if="exactOneCount > 0" class="pv-note">
      其中 {{ exactOneCount }}
      条要求<b>恰好命中一行</b>：这张表没有可用的主键 / 唯一索引，行是用原值拼出来的。
      命中 0 行（数据已被改过）或多行（条件不足以定位）都会判定失败并回滚整批。
    </div>
    <div class="pv-list">
      <div
        v-for="(s, i) in statements"
        :key="i"
        class="pv-item"
        :class="{ 'is-invalid': !!s.error }"
      >
        <span class="pv-idx mono">{{ i + 1 }}</span>
        <span v-if="s.expect_one" class="pv-flag" title="要求恰好命中一行，否则回滚整批">=1</span>
        <pre class="pv-sql mono">{{ s.error || s.sql }}</pre>
      </div>
    </div>
    <template #footer>
      <app-button size="small" @click="visible = false">取消</app-button>
      <app-button size="small" type="primary" :loading="saving" @click="emit('confirm')">
        执行
      </app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import type { EditStatement } from '../types'

/**
 * 写库前的 SQL 预览。
 *
 * 为什么要这一步：对着（尤其是生产库的）表批量 UPDATE，**不看到具体语句就提交是不可接受的**。
 * 语句由后端 `db_preview_edits` 生成 —— 与真正执行的那条命令共用同一份拼装逻辑，
 * 所以不存在「预览与实际不一样」的情况（前端自己拼一份迟早会跑偏）。
 */
const props = defineProps<{
  modelValue: boolean
  statements: EditStatement[]
  rowCount: number
  saving?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  confirm: []
}>()

const visible = computed({
  get: () => props.modelValue,
  set: v => emit('update:modelValue', v),
})

const invalidCount = computed(() => props.statements.filter(s => !!s.error).length)
/** 有多少条语句带着「必须命中恰好一行」的校验（无身份表的写操作） */
const exactOneCount = computed(() => props.statements.filter(s => s.expect_one).length)
</script>

<style scoped>
.pv-head {
  font-size: var(--font-size-sm);
  color: var(--el-text-color-regular);
  margin-bottom: 8px;
}
.pv-head b {
  font-family: var(--font-family-mono);
  color: var(--el-color-primary);
}
.pv-warn {
  font-size: var(--font-size-sm);
  color: var(--el-color-danger);
  margin-bottom: 8px;
}
.pv-note {
  font-size: var(--font-size-sm);
  color: var(--el-color-warning);
  margin-bottom: 8px;
}
.pv-flag {
  flex-shrink: 0;
  font-family: var(--font-family-mono);
  font-size: 10px;
  line-height: 14px;
  padding: 0 3px;
  border-radius: 3px;
  color: var(--el-color-warning);
  border: 1px solid var(--el-color-warning-light-5);
}
.pv-list {
  max-height: 420px;
  overflow: auto;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 4px;
}
.pv-item {
  display: flex;
  gap: 8px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--el-border-color-lighter);
}
.pv-item:last-child {
  border-bottom: none;
}
.pv-item.is-invalid .pv-sql {
  color: var(--el-color-danger);
}
.pv-idx {
  flex-shrink: 0;
  width: 24px;
  color: var(--el-text-color-secondary);
  text-align: right;
}
.pv-sql {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  font-size: var(--font-size-sm);
  line-height: 1.6;
  color: var(--el-text-color-primary);
}
.mono {
  font-family: var(--font-family-mono);
}
</style>
