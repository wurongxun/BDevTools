<template>
  <div class="log-workspace" @dragover.prevent @drop.prevent="onDrop">
    <LogToolbar
      :timeline-visible="showTimeline"
      @goto="onGoto"
      @manage-rules="rulesVisible = true"
      @export="exportVisible = true"
      @toggle-timeline="showTimeline = !showTimeline"
    />

    <FilterBar />

    <Timeline v-if="showTimeline" @jump="onGoto" />

    <!-- 已打开的日志源 / 连接 / 实时流 以 tab 形式快速切换；重复打开同一文件会复用已有 tab -->
    <LogTabs />

    <div v-if="store.errorMsg" class="ws-error">
      <el-icon><WarningFilled /></el-icon>
      <span>{{ store.errorMsg }}</span>
      <app-button size="small" text @click="store.errorMsg = ''">
        <el-icon><Close /></el-icon>
      </app-button>
    </div>

    <LogView ref="viewRef" />

    <RuleManager v-model="rulesVisible" />
    <ExportDialog v-model="exportVisible" />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { ArrowDown, ArrowUp, Close, WarningFilled } from '@element-plus/icons-vue'
import { useLogStore } from '../store/logs'
import LogToolbar from './LogToolbar.vue'
import FilterBar from './FilterBar.vue'
import Timeline from './Timeline.vue'
import LogView from './LogView.vue'
import RuleManager from './RuleManager.vue'
import ExportDialog from './ExportDialog.vue'
import LogTabs from './LogTabs.vue'

const store = useLogStore()
const viewRef = ref<InstanceType<typeof LogView> | null>(null)
const rulesVisible = ref(false)
const exportVisible = ref(false)
const showTimeline = ref(true)

function onGoto(lineNo: number | null) {
  if (lineNo && lineNo > 0) viewRef.value?.scrollToLine(lineNo)
}

/**
 * 拖入文件打开为日志源。
 * Tauri 环境下拖拽项带真实路径；浏览器环境下拿不到路径，提示改用侧边栏按钮。
 */
function onDrop(e: DragEvent) {
  const files = e.dataTransfer?.files
  if (!files || !files.length) return
  const first = files[0] as File & { path?: string }
  const path = first?.path
  if (typeof path === 'string' && path) {
    void store.openLocalFile(path)
  } else {
    store.errorMsg = '当前环境无法从拖拽中读取文件路径，请通过侧边栏「打开文件」选择'
  }
}

onMounted(() => {
  void store.init()
})
</script>

<style scoped lang="scss">
.log-workspace {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  min-height: 0;
}

.ws-error {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  font-size: var(--font-size-sm);
  color: var(--el-color-danger);
  background: var(--el-color-danger-light-9);
  border-bottom: 1px solid var(--el-color-danger-light-7);

  span {
    flex: 1;
    min-width: 0;
  }
}
</style>
