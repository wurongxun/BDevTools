<template>
  <div class="log-sidebar">


    <!-- 日志源列表（内容保持原有结构） -->
    <div class="sb-header">
      <span class="sb-title">日志源</span>
      <div class="sb-actions">
        <el-tooltip content="打开本地日志文件" placement="top">
          <app-button bare :icon="FolderOpened" @click="pickFile" />
        </el-tooltip>
        <el-tooltip content="通过SSH拉取远程日志" placement="top">
          <app-button bare :icon="Connection" @click="remoteVisible = true" />
        </el-tooltip>
        <el-tooltip content="接入实时日志流（WebSocket / TCP）" placement="top">
          <app-button bare :icon="VideoPlay" @click="streamVisible = true" />
        </el-tooltip>
      </div>
    </div>

    <div class="sb-list">
      <div
        v-for="s in store.sources"
        :key="s.id"
        class="sb-item"
        :class="{ 'is-active': s.id === store.activeSourceId, 'dragging': s.id === dragId }"
        draggable="true"
        @click="store.setActiveSource(s.id)"
        @dragstart="onDragStart(s, $event)"
        @dragover.prevent="onDragOver"
        @drop="onDrop(s, $event)"
        @dragend="dragId = ''"
      >
        <span class="sb-dot" :style="{ background: s.color }"></span>
        <span class="sb-kind" :class="`k-${s.kind}`">{{ kindLabel(s.kind) }}</span>
        <div class="sb-main">
          <div class="sb-name" :title="s.path">{{ s.name }}</div>
          <div class="sb-path" :title="s.path">{{ s.path }}</div>
          <div v-if="s.kind === 'stream' && s.streamError" class="sb-err">{{ s.streamError }}</div>
        </div>
        <span class="sb-count">{{ s.loadedLines }}/{{ s.totalLines }}</span>
        <!-- 源内操作按钮：无文字、靠右排列 -->
        <div class="sb-actions-right">
          <app-button
            v-if="s.kind === 'stream'"
            size="small"
            text
            :title="s.streamState === 'live' ? '暂停' : '继续'"
            @click.stop="toggleStream(s)"
          >
            <el-icon><component :is="s.streamState === 'live' ? VideoPause : VideoPlay" /></el-icon>
          </app-button>
          <app-button size="small" text title="移除该源" @click.stop="store.closeSource(s.id)">
            <el-icon><Close /></el-icon>
          </app-button>
        </div>
      </div>

      <p v-if="!store.sources.length" class="sb-empty">还没有日志源，点击上方按钮打开本地 / 远程日志或接入实时流</p>
    </div>

    <div class="sb-divider"></div>
    <!-- 日志浏览器（类似 database 的 ScriptExplorer，默认打开工作区目录） -->
    <div class="explorer-wrap">
      <LogExplorer />
    </div>

    <RemoteSourceDialog v-model="remoteVisible" />
    <StreamSourceDialog v-model="streamVisible" />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { Close, Connection, FolderOpened, VideoPause, VideoPlay } from '@element-plus/icons-vue'
import { open } from '@/core/runtime'
import { useLogStore } from '../store/logs'
import type { LogSource } from '../types'
import RemoteSourceDialog from '../components/RemoteSourceDialog.vue'
import StreamSourceDialog from '../components/StreamSourceDialog.vue'
import LogExplorer from '../components/LogExplorer.vue'

const store = useLogStore()
const remoteVisible = ref(false)
const streamVisible = ref(false)
/** 当前正在拖拽的日志源 id（用于置灰 + 重排） */
const dragId = ref('')

function onDragStart(s: LogSource, e: DragEvent) {
  dragId.value = s.id
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move'
    e.dataTransfer.setData('text/plain', s.id)
  }
}

function onDragOver(e: DragEvent) {
  // 必须 preventDefault 才能触发 drop
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
}

function onDrop(s: LogSource, e: DragEvent) {
  e.preventDefault()
  if (dragId.value && dragId.value !== s.id) {
    store.moveSource(dragId.value, s.id)
  }
  dragId.value = ''
}

function kindLabel(kind: LogSource['kind']): string {
  if (kind === 'ssh') return 'SSH'
  if (kind === 'stream') return '流'
  return '本地'
}

function toggleStream(s: LogSource) {
  if (s.streamState === 'live') store.pauseStream(s.id)
  else store.resumeStream(s.id)
}

async function pickFile() {
  const selected = await open({
    multiple: false,
    filters: [{ name: '日志文件', extensions: ['*'] }],
  })
  if (typeof selected === 'string' && selected) {
    await store.openLocalFile(selected)
  }
}

onMounted(() => {
  void store.init()
})
</script>

<style scoped lang="scss">
.log-sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

/* 日志浏览器区域：占据上半部分，内部自带滚动 */
.explorer-wrap {
  flex: 1 1 50%;
  min-height: 0;
  overflow: hidden;
}

.sb-divider {
  height: 1px;
  flex-shrink: 0;
  background: var(--el-border-color-light);
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
  flex: 1 1 50%;
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

  &.dragging {
    opacity: 0.4;
  }
}

.sb-dot {
  flex: none;
  width: 6px;
  height: 6px;
  border-radius: 50%;
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

  &.k-local {
    color: var(--el-text-color-secondary);
    background: var(--el-fill-color-light);
  }

  &.k-ssh {
    color: #fff;
    background: var(--el-color-primary);
  }

  &.k-stream {
    color: #fff;
    background: var(--el-color-success);
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

/* 源内操作按钮：推到最右侧，无文字（仅图标），紧凑排列 */
.sb-actions-right {
  display: flex;
  align-items: center;
  gap: 2px;
  margin-left: auto;
  flex-shrink: 0;
}

.sb-empty {
  margin: 8px;
  font-size: var(--font-size-sm);
  line-height: 1.6;
  color: var(--el-text-color-placeholder);
}
</style>
