<template>
  <div class="mat-sidebar">
    <!-- 头部：标题 + 「打开 Java 堆转储文件」按钮 -->
    <div class="sb-header">
      <span class="sb-title">堆转储文件</span>
      <div class="sb-actions">
        <el-tooltip content="打开 Java 堆转储文件（HPROF / PHD）" placement="top">
          <app-button bare :icon="FolderOpened" :loading="picking" @click="pickFile" />
        </el-tooltip>
      </div>
    </div>

    <!-- 已打开 dump 列表 -->
    <div v-if="store.dumps.length" class="sb-list">
      <div
        v-for="d in store.dumps"
        :key="d.id"
        class="sb-item"
        :class="{ 'is-active': d.id === store.activeDumpId }"
        @click="store.setActiveDump(d.id)"
      >
        <span class="sb-dot" :class="{ 'is-ok': d.supported, 'is-bad': !d.supported }"></span>
        <span class="sb-kind" :class="kindClass(d)">{{ kindLabel(d) }}</span>
        <div class="sb-main">
          <div class="sb-name" :title="d.path">{{ d.name }}</div>
          <div class="sb-path" :title="d.path">{{ d.path }}</div>
          <div v-if="d.error" class="sb-err">{{ d.error }}</div>
        </div>
        <span class="sb-count">{{ formatBytes(d.size) }}</span>
        <div class="sb-actions-right">
          <app-button size="small" text title="移除该文件" @click.stop="store.closeDump(d.id)">
            <el-icon><Close /></el-icon>
          </app-button>
        </div>
      </div>
    </div>

    <!-- 空态：提示从按钮或拖拽打开 -->
    <div v-if="!store.dumps.length" class="sb-drop" @dragover.prevent @drop.prevent="onDrop">
      <p class="sb-empty">还没有打开堆转储文件</p>
      <p class="sb-hint">点击上方按钮选择 .hprof / .phd 文件，或直接拖拽到此处</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { Close, FolderOpened } from '@element-plus/icons-vue'
import { open } from '@/core/runtime'
import { useMatStore } from './store'
import { formatBytes } from './api/matApi'
import type { MatDump } from './types'

const store = useMatStore()
const picking = ref(false)

/** 打开文件对话框（支持 HPROF / Android .hprof / IBM PHD） */
async function pickFile() {
  picking.value = true
  try {
    const selected = await open({
      multiple: true,
      filters: [
        { name: '堆转储文件', extensions: ['hprof', 'phd'] },
        { name: '所有文件', extensions: ['*'] },
      ],
    })
    const paths = Array.isArray(selected) ? selected : selected ? [selected] : []
    for (const p of paths) {
      if (typeof p === 'string' && p) await store.openDump(p)
    }
  } catch (e: any) {
    console.error('[mat] 打开失败:', e)
  } finally {
    picking.value = false
  }
}

/** Tauri 拖拽项带真实路径；浏览器环境拿不到路径时提示改用按钮 */
function onDrop(e: DragEvent) {
  const files = e.dataTransfer?.files
  if (!files || !files.length) return
  const first = files[0] as File & { path?: string }
  const path = first?.path
  if (typeof path === 'string' && path) {
    void store.openDump(path)
  }
}

function kindLabel(d: MatDump): string {
  if (d.format.startsWith('JAVA PROFILE')) return 'HPROF'
  if (d.format && !d.supported) return '其他'
  return '…'
}

function kindClass(d: MatDump): string {
  if (d.format.startsWith('JAVA PROFILE')) return 'k-hprof'
  if (d.error) return 'k-err'
  return 'k-other'
}
</script>

<style scoped lang="scss">
.mat-sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.sb-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}

.sb-title {
  font-size: var(--font-size-base);
  font-weight: 500;
  color: var(--el-text-color-primary);
}

.sb-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 2px;
}

.sb-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 4px;
}

.sb-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border-radius: var(--radius-md);
  cursor: pointer;

  &:hover {
    background: var(--el-fill-color-light);
  }

  &.is-active {
    background: var(--el-color-primary-light-9);
  }
}

.sb-dot {
  flex: none;
  width: 6px;
  height: 6px;
  border-radius: 50%;

  &.is-ok {
    background: var(--el-color-success);
  }

  &.is-bad {
    background: var(--el-color-warning);
  }
}

.sb-kind {
  flex: none;
  padding: 0 5px;
  height: 16px;
  line-height: 16px;
  border-radius: var(--radius-sm);
  font-size: var(--font-size-xs);
  font-weight: 600;
  letter-spacing: 0.3px;

  &.k-hprof {
    color: #fff;
    background: var(--el-color-primary);
  }

  &.k-err {
    color: #fff;
    background: var(--el-color-danger);
  }

  &.k-other {
    color: var(--el-text-color-secondary);
    background: var(--el-fill-color-light);
  }
}

.sb-main {
  flex: 1;
  min-width: 0;
}

.sb-name {
  font-size: var(--font-size-base);
  color: var(--el-text-color-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sb-path {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sb-err {
  font-size: var(--font-size-xs);
  color: var(--el-color-danger);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sb-count {
  flex: none;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  font-variant-numeric: tabular-nums;
  margin-right: 2px;
}

.sb-actions-right {
  display: flex;
  align-items: center;
  gap: 2px;
  margin-left: auto;
  flex-shrink: 0;
}

.sb-drop {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 16px;
  border: 1px dashed var(--el-border-color);
  border-radius: var(--radius-md);
  margin: 8px;
  cursor: pointer;
}

.sb-empty {
  margin: 0;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}

.sb-hint {
  margin: 0;
  font-size: var(--font-size-xs);
  text-align: center;
  line-height: 1.5;
  color: var(--el-text-color-placeholder);
}
</style>
