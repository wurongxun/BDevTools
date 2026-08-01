<script setup lang="ts">
import { computed } from 'vue'
import { ArrowRight, ArrowDown, Document } from '@element-plus/icons-vue'

interface BookmarkItem {
  id: string
  title: string
  dest: any
  level: number
  expanded?: boolean
  children?: BookmarkItem[]
}

const props = defineProps<{
  bookmarks: BookmarkItem[]
  currentPage: number
}>()

const emit = defineEmits<{
  (e: 'go-to-page', dest: any): void
  (e: 'toggle-expand', bookmark: BookmarkItem): void
}>()

// 处理书签点击
const handleBookmarkClick = (bookmark: BookmarkItem) => {
  if (bookmark.children && bookmark.children.length > 0) {
    emit('toggle-expand', bookmark)
  }

  if (bookmark.dest) {
    emit('go-to-page', bookmark.dest)
  }
}

// 判断是否有子项
const hasChildren = (bookmark: BookmarkItem): boolean => {
  return !!(bookmark.children && bookmark.children.length > 0)
}
</script>

<template>
  <div class="bookmark-tree-wrapper">
    <div
      v-for="bookmark in bookmarks"
      :key="bookmark.id"
      class="bookmark-item"
    >
      <div
        class="bookmark-node"
        :class="{ 'has-children': hasChildren(bookmark) }"
        :style="{ paddingLeft: `${12 + bookmark.level * 16}px` }"
        @click="handleBookmarkClick(bookmark)"
      >
        <!-- 展开/折叠图标 -->
        <span v-if="hasChildren(bookmark)" class="bookmark-toggle">
          <el-icon>
            <ArrowDown v-if="bookmark.expanded" />
            <ArrowRight v-else />
          </el-icon>
        </span>
        <span v-else class="bookmark-toggle-placeholder"></span>

        <!-- 书签图标 -->
        <el-icon class="bookmark-icon">
          <Document />
        </el-icon>

        <!-- 书签标题 -->
        <span class="bookmark-text" :title="bookmark.title">
          {{ bookmark.title }}
        </span>

        <!-- 页码（如果有） -->
        <span v-if="bookmark.dest && typeof bookmark.dest === 'number'" class="bookmark-page">
          {{ bookmark.dest }}
        </span>
      </div>

      <!-- 递归渲染子项 -->
      <div v-if="hasChildren(bookmark) && bookmark.expanded" class="bookmark-children">
        <BookmarkTree
          :bookmarks="bookmark.children!"
          :current-page="currentPage"
          @go-to-page="$emit('go-to-page', $event)"
          @toggle-expand="$emit('toggle-expand', $event)"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.bookmark-tree-wrapper {
  width: 100%;
}

.bookmark-item {
  margin-bottom: 2px;
}

.bookmark-node {
  display: flex;
  align-items: center;
  padding: 8px 12px;
  cursor: pointer;
  border-radius: 4px;
  transition: all 0.2s;
  gap: 8px;
  min-height: 36px;
}

.bookmark-node:hover {
  background-color: var(--el-fill-color-light);
}

.bookmark-node.active {
  background-color: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
}

.bookmark-toggle {
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
  cursor: pointer;
}

.bookmark-toggle:hover {
  color: var(--el-color-primary);
}

.bookmark-toggle-placeholder {
  width: 16px;
  flex-shrink: 0;
}

.bookmark-icon {
  color: var(--el-color-warning);
  flex-shrink: 0;
  font-size: 14px;
}

.bookmark-text {
  flex: 1;
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--el-text-color-primary);
}

.bookmark-page {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
  min-width: 30px;
  text-align: right;
}

.bookmark-children {
  margin-top: 2px;
}
</style>