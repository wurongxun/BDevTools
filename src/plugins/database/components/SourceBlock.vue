<template>
  <div class="source-block">
    <div class="source-head">
      <span class="source-title">{{ title }}</span>
      <span class="source-spacer"></span>
      <el-tooltip content="复制源码" placement="top">
        <app-button
          v-if="source"
          bare
          size="small"
          :icon="DocumentCopy"
          aria-label="复制源码"
          @click="handleCopy"
        ></app-button>
      </el-tooltip>
      <app-button size="small" :loading="loading" @click="emit('reload')">刷新</app-button>
    </div>

    <!--
      源码可能上千行，交给 Monaco 而不是 <pre>：语法高亮、行号、Ctrl+F 查找都有，
      长存储过程才看得下去。只读由 MonacoEditor 的 readonly 保证（不能被编辑）。
    -->
    <div class="source-body">
      <MonacoEditor
        v-if="source"
        :model-value="source"
        language="sql"
        readonly
        :options="{ wordWrap: 'on', scrollBeyondLastLine: false, folding: true }"
      />
      <div v-else class="source-placeholder">{{ placeholder }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { DocumentCopy } from '@element-plus/icons-vue'
import MonacoEditor from '@/components/MonacoEditor.vue'
import { copyText } from '../utils/clipboard'

/**
 * 对象源码展示块（视图定义 / 存储过程体 共用）。
 *
 * 只负责「把一段 SQL 源码显示出来 + 能复制 + 能刷新」，
 * 源码从哪来由父组件决定（各库接口不同），这里不掺和。
 */
const props = withDefaults(
  defineProps<{
    source: string
    loading?: boolean
    error?: string
    title?: string
  }>(),
  { loading: false, error: '', title: '源码' }
)

const emit = defineEmits<{
  reload: []
}>()

const placeholder = computed(() => {
  if (props.loading) return '加载中…'
  if (props.error) return props.error
  return '没有取到源码（对象可能不存在，或这个数据库不支持取源码）'
})

function handleCopy() {
  copyText(props.source, '源码已复制')
}
</script>

<style scoped>
.source-block {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}
.source-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  flex-shrink: 0;
}
.source-title {
  font-size: var(--font-size-md);
  color: var(--el-text-color-primary);
  white-space: nowrap;
}
.source-spacer {
  flex: 1;
}
.source-body {
  flex: 1;
  min-height: 0;
  position: relative;
}
.source-placeholder {
  padding: 16px;
  font-size: var(--font-size-md);
  color: var(--el-text-color-secondary);
  line-height: 1.6;
}
</style>
