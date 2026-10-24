<template>
  <div class="path-field">
    <el-input
      :model-value="modelValue"
      size="small"
      class="path-input"
      :placeholder="placeholder"
      :aria-label="placeholder || '文件路径'"
      @update:model-value="onInput"
    />
    <app-button
      bare
      :icon="FolderOpened"
      :title="browseLabel"
      :aria-label="browseLabel"
      @click="emit('browse')"
    ></app-button>
  </div>
</template>

<script setup lang="ts">
import { FolderOpened } from '@element-plus/icons-vue'

/**
 * 「路径输入框 + 浏览按钮」组合件（导入弹窗 / 导出弹窗共用）。
 *
 * 抽出来不是为了省几行，而是因为**两处各写一遍必然漂**：
 * 之前导出弹窗写的是 `<el-input small>`（`small` 根本不是 el-input 的属性，
 * 正确写法是 `size="small"`），于是两个弹窗的输入框高度不一样 —— 这就是「不协调」。
 *
 * 另有两条布局硬约束（踩过）：
 * - `el-input` 自带 `width: 100%`，在 flex 行里必须交出宽度控制权（flex:1 + min-width:0），
 *   否则占满整行、把浏览按钮挤到下一行（外层 `el-form-item__content` 是 flex-wrap: wrap）；
 * - 浏览按钮是固定 22×22 的图标按钮，必须 `flex: none`，否则会被压缩成一条。
 */
const props = withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
    browseLabel?: string
  }>(),
  {
    placeholder: '',
    browseLabel: '浏览'
  }
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
  browse: []
}>()

function onInput(v: string) {
  emit('update:modelValue', v)
}
</script>

<style scoped>
.path-field {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}
.path-field :deep(.el-input) {
  flex: 1;
  min-width: 0;
}
.path-field :deep(.el-button) {
  flex: none;
}
</style>
