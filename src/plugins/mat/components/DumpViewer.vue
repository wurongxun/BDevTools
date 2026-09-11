<template>
  <div class="dump-viewer">
    <el-tabs v-model="activeTab" class="viewer-tabs" @tab-change="onTabChange">
      <!-- 概览：文件元信息 -->
      <el-tab-pane name="overview" label="概览">
        <OverviewView :dump="dump" />
      </el-tab-pane>

      <!-- 直方图（Histogram）：对象数量 + 浅堆统计 -->
      <el-tab-pane name="histogram" label="类">
        <HistogramView :dump="dump" />
      </el-tab-pane>

      <!-- 内存泄漏检测（Leak Suspects）：支配树 Retained Heap + GC Root 路径 -->
      <el-tab-pane name="leak" label="泄漏检测">
        <LeakView :dump="dump" />
      </el-tab-pane>

      <!-- 支配树（Dominator Tree）：深堆统计占位 -->
      <el-tab-pane name="dominator" label="支配树">
        <PlaceholderView
          title="Dominator Tree（支配树）"
          desc="后续将基于对象图计算支配树，聚合 Retained Heap（深堆）并定位强持有者。"
        />
      </el-tab-pane>

      <!-- 对象引用追踪：outgoing / incoming / Path to GC Roots -->
      <el-tab-pane name="refs" label="引用追踪">
        <RefView :dump="dump" />
      </el-tab-pane>

      <!-- OQL 查询占位 -->
      <el-tab-pane name="oql" label="OQL">
        <PlaceholderView
          title="OQL（对象查询语言）"
          desc="后续将提供类 SQL 的对象查询，支持 select * from java.lang.String s where … 筛选堆内对象。"
        />
      </el-tab-pane>

      <!-- 预留 Tab（扩展位） -->
      <el-tab-pane name="reserve">
        <template #label>
          <el-icon class="reserve-icon" title="预留功能"><MoreFilled /></el-icon>
        </template>
        <PlaceholderView title="预留功能区" desc="线程分析 / 类加载器分析 / 堆 diff 等能力可在此扩展" />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { MoreFilled } from '@element-plus/icons-vue'
import { useMatStore } from '../store'
import type { MatDump, MatViewKey } from '../types'
import OverviewView from './views/OverviewView.vue'
import HistogramView from './views/HistogramView.vue'
import LeakView from './views/LeakView.vue'
import RefView from './views/RefView.vue'
import PlaceholderView from './views/PlaceholderView.vue'

/**
 * 单个堆转储文件的综合分析视图：
 * 概览 / 直方图 / 泄漏检测 / 支配树 / 引用追踪 / OQL / 预留
 * 子 tab 状态（当前选中）挂在 store（按 dumpId），KeepAlive 切换回来保持位置。
 */
const props = defineProps<{
  dump: MatDump
}>()

const store = useMatStore()

const activeTab = ref<MatViewKey>(store.viewOf(props.dump.id))

/** 用户切换子 tab 时同步 store（只记录用户主动切换，避免初始化覆盖） */
function onTabChange(name: string | number) {
  store.setActiveView(props.dump.id, name as MatViewKey)
}

// dump 变化（KeepAlive 实例复用于不同文件）时恢复该 dump 记住的子视图
watch(
  () => props.dump.id,
  (id) => {
    activeTab.value = store.viewOf(id)
  }
)

// store 侧切换子视图时同步本地 tab（直方图 / 泄漏检测的「追踪」按钮经 openRefs 跨视图跳转）
watch(
  () => store.viewOf(props.dump.id),
  (v) => {
    if (v !== activeTab.value) activeTab.value = v
  }
)
</script>

<style scoped lang="scss">
.dump-viewer {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.viewer-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.viewer-tabs :deep(.el-tabs__header) {
  margin: 0;
  padding: 0 12px;
  flex-shrink: 0;
}

.viewer-tabs :deep(.el-tabs__nav-wrap::after) {
  background-color: var(--el-border-color-light);
}

.viewer-tabs :deep(.el-tabs__content) {
  flex: 1;
  min-height: 0;
  padding: 0;
}

.viewer-tabs :deep(.el-tab-pane) {
  height: 100%;
}

.reserve-icon {
  font-size: 16px;
  color: var(--el-text-color-secondary);
}
</style>
