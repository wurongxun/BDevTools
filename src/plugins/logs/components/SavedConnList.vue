<template>
  <div v-if="items.length" class="saved-section">
    <div class="saved-title">{{ title }}（{{ items.length }}）</div>
    <ul class="saved-list">
      <li
        v-for="c in items"
        :key="c.id"
        class="saved-item"
        :class="{ active: activeId === c.id }"
      >
        <!-- 整行是按钮：点击载入表单（不进入编辑态，避免误改原记录） -->
        <button type="button" class="saved-main" @click="emit('pick', c)">
          <span class="saved-name">{{ c.name }}</span>
          <span class="saved-sub">{{ c.subtitle }}</span>
        </button>
        <div class="saved-actions">
          <app-button
            bare
            :icon="EditPen"
            title="编辑"
            :aria-label="`编辑 ${c.name}`"
            @click="emit('edit', c)"
          />
          <app-button
            bare
            :icon="Delete"
            title="删除"
            :aria-label="`删除 ${c.name}`"
            @click="emit('remove', c)"
          />
        </div>
      </li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import { Delete, EditPen } from '@element-plus/icons-vue'

/** 列表条目：调用方把业务记录映射成这两个展示字段即可 */
export interface SavedConnItem {
  id: string
  name: string
  subtitle: string
}

withDefaults(
  defineProps<{
    items: SavedConnItem[]
    /** 标题，如「已保存的连接」 */
    title?: string
    /** 处于编辑态的条目 id（null 表示无） */
    activeId?: string | null
  }>(),
  { title: '已保存的连接', activeId: null },
)

/**
 * 事件用 pick/edit/remove 而非原生事件名（CODEBUDDY 自定义事件命名约定）：
 * pick = 点击列表正文（仅载入表单）；edit = 进入编辑态；remove = 请求删除
 */
const emit = defineEmits<{
  pick: [item: SavedConnItem]
  edit: [item: SavedConnItem]
  remove: [item: SavedConnItem]
}>()
</script>

<style scoped>
.saved-section {
  margin-bottom: 12px;
}

.saved-title {
  font-size: var(--font-size-base);
  color: var(--el-text-color-secondary);
  margin-bottom: 6px;
}

.saved-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 168px;
  overflow-y: auto;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: var(--radius-md);
}

.saved-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 6px;

  & + .saved-item {
    border-top: 1px solid var(--el-border-color-lighter);
  }

  /* 编辑中的条目高亮，提示表单已绑定该记录 */
  &.active {
    background: var(--el-fill-color-light);
  }
}

/* 条目正文作为按钮：占满剩余宽度，左对齐两行文本 */
.saved-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  padding: 2px 4px;
  border: none;
  background: transparent;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-family: inherit;
  text-align: left;

  &:hover {
    background: var(--el-fill-color-light);
  }
}

.saved-name {
  font-size: var(--font-size-base);
  color: var(--el-text-color-primary);
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.saved-sub {
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
  /* 路径 / URL 类文本用等宽字体：普惠体下划线等字形在小字号下不可见 */
  font-family: var(--font-family-mono);
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.saved-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex: none;
}
</style>
