<template>
  <div v-if="count > 0" class="pending-bar">
    <span class="pb-icon">✎</span>
    <span class="pb-text">
      <b>{{ count }}</b> 处改动未保存（{{ rowCount }} 行）
    </span>
    <span v-if="(errCount ?? 0) > 0" class="pb-err">{{ errCount }} 处上次保存失败</span>
    <span class="spacer" />
    <app-button size="small" text :disabled="saving" @click="emit('revert')">全部放弃</app-button>
    <app-button size="small" :disabled="saving" @click="emit('preview')">查看 SQL</app-button>
    <app-button size="small" type="primary" :loading="saving" @click="emit('save')">保存</app-button>
  </div>
</template>

<script setup lang="ts">
/**
 * 未保存改动提示条。
 *
 * 网格编辑刻意做成「先攒着、明确点保存才写库」：单元格失焦就 UPDATE 会让
 * 「改错一格」立刻落到数据库里，而且没法一次看到自己到底改了哪些地方。
 */
defineProps<{
  count: number
  rowCount: number
  errCount?: number
  saving?: boolean
}>()

const emit = defineEmits<{
  save: []
  revert: []
  preview: []
}>()
</script>

<style scoped>
.pending-bar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-shrink: 0;
  padding: 4px var(--space-3);
  font-size: var(--font-size-sm);
  /* 文字色必须用状态色族的 *Fg：--el-color-warning 是填充色，
     当文字用对比度不达标（同 dangerText 的教训） */
  color: var(--stateWarningFg);
  background: var(--stateWarningFill);
  border-top: 1px solid var(--stateWarningFg);
  border-bottom: 1px solid var(--stateWarningFg);
}
.pb-icon {
  font-size: 13px;
}
.pb-text b {
  font-family: var(--font-family-mono);
}
.pb-err {
  color: var(--stateErrorFg);
}
.spacer {
  flex: 1;
}
</style>
