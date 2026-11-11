<template>
  <div class="mat-main-panel">
    <!-- 顶层 Tab：每个打开的堆转储文件一个 tab（复用通用 TabBar，与 database 一致） -->
    <TabBarContainer v-if="store.dumps.length > 0" aria-label="堆转储文件">
      <TabBarItem
        v-for="d in store.dumps"
        :key="d.id"
        :active="d.id === store.activeDumpId"
        @click="store.setActiveDump(d.id)"
        @close="store.closeDump(d.id)"
      >
        <span class="tab-title" :title="d.path">{{ d.name }}</span>
        <TabCloseButton @close="store.closeDump(d.id)" />
      </TabBarItem>
    </TabBarContainer>

    <!-- 主内容区：激活 dump 的视图（每个 dump 独立 KeepAlive，切 tab 保留子视图状态） -->
    <div class="main-content">
      <template v-if="store.dumps.length > 0">
        <template v-for="d in store.dumps" :key="d.id">
          <KeepAlive>
            <DumpViewer v-if="d.id === store.activeDumpId" :dump="d" />
          </KeepAlive>
        </template>
      </template>

      <!-- 空态欢迎页 -->
      <div v-else class="welcome">
        <el-empty description="Java 堆转储内存分析（Memory Analyzer Tool）">
          <template #image>
            <el-icon :size="64"><DataAnalysis /></el-icon>
          </template>
          <p class="welcome-hint">从左侧打开 .hprof（或 .hprof.gz）堆转储文件，定位内存泄漏、分析对象占用</p>
        </el-empty>
      </div>
    </div>

    <!-- 分析预算与规模上限设置（全局唯一实例，各视图经 store.openSettings 打开） -->
    <MatSettingsDialog />
  </div>
</template>

<script setup lang="ts">
import { DataAnalysis } from '@element-plus/icons-vue'
import { TabBarContainer, TabBarItem, TabCloseButton } from '@/components/TabBar'
import { useMatStore } from './store'
import DumpViewer from './components/DumpViewer.vue'
import MatSettingsDialog from './components/MatSettingsDialog.vue'

defineOptions({ name: 'MatMainPanel' })

const store = useMatStore()
</script>

<style scoped lang="scss">
.mat-main-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  min-height: 0;
  overflow: hidden;
}

.tab-title {
  /* 文件名标识符用等宽字体，避免小字号下 _ 不可见 */
  font-family: var(--font-family-mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 200px;
}

.main-content {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.welcome {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.welcome-hint {
  font-size: var(--font-size-lg);
  color: var(--el-text-color-secondary);
}
</style>
