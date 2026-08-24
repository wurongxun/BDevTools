<!-- src/components/GlobalPromptDialog.vue -->
<!-- 公共输入弹窗：基于 GlobalDialog 的 ElMessageBox.prompt 替代品，
     布局/拖拽/主题与 GlobalDialog 完全一致 -->
<template>
  <GlobalDialog
    v-model="visible"
    :title="title"
    :width="width"
    :height="height"
    :vertical-center="verticalCenter"
  >
    <div class="prompt-content">
      <div v-if="message" class="prompt-message">{{ message }}</div>
      <el-input
        ref="inputRef"
        v-model="inputValue"
        class="prompt-input"
        :class="{ 'is-error': !!error }"
        :placeholder="placeholder"
        @keyup.enter="handleConfirm"
        @input="error = ''"
      />
      <div v-if="error" class="prompt-error">{{ error }}</div>
    </div>
    <template #footer>
      <el-button size="small" @click="visible = false">{{ cancelButtonText }}</el-button>
      <el-button size="small" type="primary" @click="handleConfirm">{{ confirmButtonText }}</el-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import type { InputInstance } from 'element-plus'
import GlobalDialog from './GlobalDialog.vue'

interface Props {
  modelValue: boolean
  title?: string
  /** 输入框上方的提示文案 */
  message?: string
  placeholder?: string
  /** 打开时预填的值（重命名场景） */
  initialValue?: string
  width?: string
  /** 可选：固定高度，透传给 GlobalDialog；不传则内容自适应 */
  height?: string
  /** 是否垂直居中，透传给 GlobalDialog；默认 true */
  verticalCenter?: boolean
  confirmButtonText?: string
  cancelButtonText?: string
  /** 校验函数：返回错误文案则阻止确认并展示错误，返回 true 通过 */
  validator?: (value: string) => string | true
}

const props = withDefaults(defineProps<Props>(), {
  title: '请输入',
  message: '',
  placeholder: '',
  initialValue: '',
  width: '360px',
  height: '200px',
  verticalCenter: true,
  confirmButtonText: '确定',
  cancelButtonText: '取消'
})

interface Emits {
  (e: 'update:modelValue', value: boolean): void
  (e: 'confirm', value: string): void
}

const emit = defineEmits<Emits>()

const visible = ref<boolean>(props.modelValue)
const inputValue = ref<string>(props.initialValue)
const error = ref<string>('')
const inputRef = ref<InputInstance>()

watch(() => props.modelValue, (val: boolean) => {
  visible.value = val
})

watch(visible, (val: boolean) => {
  emit('update:modelValue', val)
  if (val) {
    // 打开时重置状态并聚焦全选，对齐 ElMessageBox.prompt 的交互
    inputValue.value = props.initialValue
    error.value = ''
    nextTick(() => {
      inputRef.value?.focus()
      inputRef.value?.select()
    })
  }
})

const handleConfirm = (): void => {
  const value = inputValue.value.trim()
  if (props.validator) {
    const result = props.validator(value)
    if (result !== true) {
      error.value = result
      return
    }
  }
  visible.value = false
  emit('confirm', value)
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.prompt-message {
  margin-bottom: 8px;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.prompt-input.is-error {
  :deep(.el-input__wrapper) {
    box-shadow: 0 0 0 1px var(--el-color-danger) inset;
  }
}

.prompt-error {
  margin-top: 4px;
  font-size: 12px;
  color: var(--el-color-danger);
}
</style>
