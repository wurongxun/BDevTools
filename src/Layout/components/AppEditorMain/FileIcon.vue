<template>
  <svg
    :class="['file-icon', `file-icon-${type}`]"
    viewBox="0 0 16 16"
    fill="currentColor"
    :title="getFileTypeName(type)"
  >
    <!-- Markdown 文件 -->
    <g v-if="type === 'markdown'">
      <path d="M3 2h7l3 3v9H3V2z" fill="none" stroke="currentColor" stroke-width="1"/>
      <text x="4.5" y="11" font-size="6" font-weight="bold" fill="currentColor">M</text>
    </g>

    <!-- PDF 文件 -->
    <g v-else-if="type === 'pdf'">
      <path d="M3 2h7l3 3v9H3V2z" fill="#c42b1c" stroke="currentColor" stroke-width="0.5"/>
      <text x="4" y="11" font-size="5" font-weight="bold" fill="white">PDF</text>
    </g>

    <!-- 代码文件 -->
    <g v-else-if="type === 'code'">
      <path d="M3 2h7l3 3v9H3V2z" fill="none" stroke="currentColor" stroke-width="1"/>
      <polyline points="5,6 7,8 5,10" fill="none" stroke="currentColor" stroke-width="1.2"/>
      <polyline points="11,6 9,8 11,10" fill="none" stroke="currentColor" stroke-width="1.2"/>
    </g>

    <!-- 文本文件 -->
    <g v-else-if="type === 'text'">
      <path d="M3 2h7l3 3v9H3V2z" fill="none" stroke="currentColor" stroke-width="1"/>
      <line x1="5" y1="5" x2="11" y2="5" stroke="currentColor" stroke-width="1"/>
      <line x1="5" y1="7" x2="11" y2="7" stroke="currentColor" stroke-width="1"/>
      <line x1="5" y1="9" x2="9" y2="9" stroke="currentColor" stroke-width="1"/>
    </g>

    <!-- 图片文件 -->
    <g v-else-if="type === 'image'">
      <path d="M3 2h7l3 3v9H3V2z" fill="none" stroke="currentColor" stroke-width="1"/>
      <circle cx="6" cy="6" r="1.5" fill="currentColor"/>
      <polyline points="3,11 6,8 8,10 11,7 13,9 13,11 3,11" fill="none" stroke="currentColor" stroke-width="1"/>
    </g>

    <!-- 二进制文件 -->
    <g v-else-if="type === 'binary'">
      <path d="M3 2h7l3 3v9H3V2z" fill="none" stroke="currentColor" stroke-width="1"/>
      <text x="4" y="10" font-size="4" font-family="monospace" fill="currentColor">01</text>
    </g>

    <!-- 默认文件 -->
    <g v-else>
      <path d="M3 2h7l3 3v9H3V2z" fill="none" stroke="currentColor" stroke-width="1"/>
      <line x1="5" y1="6" x2="11" y2="6" stroke="currentColor" stroke-width="1"/>
      <line x1="5" y1="8" x2="11" y2="8" stroke="currentColor" stroke-width="1"/>
      <line x1="5" y1="10" x2="9" y2="10" stroke="currentColor" stroke-width="1"/>
    </g>
  </svg>
</template>

<script setup lang="ts">
import { FileType } from '@/store/modules/documents'

defineProps<{
  type: FileType | string
}>()

function getFileTypeName(type: FileType | string): string {
  const typeNames: Record<string, string> = {
    [FileType.MARKDOWN]: 'Markdown 文件',
    [FileType.PDF]: 'PDF 文档',
    [FileType.CODE]: '代码文件',
    [FileType.TEXT]: '文本文件',
    [FileType.IMAGE]: '图片文件',
    [FileType.BINARY]: '二进制文件'
  }
  return typeNames[type] || '文件'
}
</script>

<style scoped>
.file-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

/* Markdown 文件 - 蓝色 */
.file-icon-markdown {
  color: #519aba;
}

/* PDF 文件 - 红色 */
.file-icon-pdf {
  color: #c42b1c;
}

/* 代码文件 - 绿色 */
.file-icon-code {
  color: #4ec9b0;
}

/* 文本文件 - 灰色 */
.file-icon-text {
  color: #cccccc;
}

/* 图片文件 - 紫色 */
.file-icon-image {
  color: #ce9178;
}

/* 二进制文件 - 橙色 */
.file-icon-binary {
  color: #dcdcaa;
}
</style>
