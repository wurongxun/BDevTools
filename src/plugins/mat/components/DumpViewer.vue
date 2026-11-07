<template>
  <div class="dump-viewer">
    <!--
      子视图切换用项目统一的 TabBar（约定见 `CODEBUDDY.md`：Tab 栏只认 `components/TabBar`）。
      这里**不用** `el-tabs`：它自带的 header 与全站 Tab 栏风格不一致，且 9 个子视图横向必然溢出。
      TabBarContainer 自带溢出滚动（两侧滚动按钮 + 滚轮转横向）与 ←/→/Home/End 键盘导航。
    -->
    <TabBarContainer class="viewer-tabs" aria-label="堆转储分析视图">
      <TabBarItem
        v-for="v in VIEWS"
        :key="v.key"
        :active="activeTab === v.key"
        :title="v.hint"
        @click="selectView(v.key)"
      >
        {{ v.label }}
      </TabBarItem>
    </TabBarContainer>

    <!--
      KeepAlive 解决两件事：
      ① **首次点开才挂载** —— `el-tabs` 默认把 9 个 pane 全部渲染（`lazy=false`），于是打开一份
         dump 会同时跑泄漏检测 / 支配树 / 引用追踪 / 报告四份重分析，互相抢会话锁还白算；
      ② **切走再切回不重建** —— 表格滚动位置、选中行、搜索词、已加载数据全部保留
         （与原 `el-tab-pane` 常驻语义一致，不会「切回来重新分析一遍」）。
      key 绑 `dump.id`：同一个 DumpViewer 实例会被复用于别的 dump（见下方 watch），
      不按 dump 分桶就会把上一份堆的视图状态带进新文件。
    -->
    <div class="viewer-body">
      <KeepAlive :key="dump.id">
        <component :is="currentView" :dump="dump" />
      </KeepAlive>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, type Component } from 'vue'
import { TabBarContainer, TabBarItem } from '@/components/TabBar'
import { useMatStore } from '../store'
import type { MatDump, MatViewKey } from '../types'
import OverviewView from './views/OverviewView.vue'
import HistogramView from './views/HistogramView.vue'
import LeakView from './views/LeakView.vue'
import RefView from './views/RefView.vue'
import DominatorView from './views/DominatorView.vue'
import TopConsumersView from './views/TopConsumersView.vue'
import ReportsView from './views/ReportsView.vue'
import CompareView from './views/CompareView.vue'
import OqlView from './views/OqlView.vue'

/**
 * 单个堆转储文件的综合分析视图。子视图状态（当前选中）挂在 store（按 dumpId）。
 *
 * **本数组即 Tab 顺序**：原 `el-tabs` 时期顺序写在模板里、与 `MatViewKey` 联合类型顺序还不一致，
 * 加视图要改两处易漏；现在只动这里。
 */
const VIEWS: { key: MatViewKey; label: string; hint: string; comp: Component }[] = [
  { key: 'overview', label: '概览', hint: '文件元信息、能力清单、缓存与索引状态', comp: OverviewView },
  { key: 'histogram', label: '类', hint: '按类 / 包统计对象数量与浅堆', comp: HistogramView },
  { key: 'leak', label: '泄漏检测', hint: 'Leak Suspects：支配树 Retained Heap + GC Roots 路径', comp: LeakView },
  { key: 'dominator', label: '支配树', hint: '对象级 Retained Heap 排序与子树展开', comp: DominatorView },
  { key: 'refs', label: '引用追踪', hint: 'Outgoing / Incoming / Path to GC Roots', comp: RefView },
  { key: 'top', label: '大对象排行', hint: '支配树顶层 retained 按类 / 包聚合（占比可加）', comp: TopConsumersView },
  { key: 'reports', label: '报告', hint: '线程 / 类加载器 / 内存浪费 / 不可达对象', comp: ReportsView },
  { key: 'compare', label: '对比', hint: '与另一份 dump 比 Δcount / Δshallow', comp: CompareView },
  { key: 'oql', label: 'OQL', hint: '安全子集查询（select … from … where …）', comp: OqlView },
]

const props = defineProps<{
  dump: MatDump
}>()

const store = useMatStore()

const activeTab = ref<MatViewKey>(store.viewOf(props.dump.id))

const currentView = computed(
  () => VIEWS.find((v) => v.key === activeTab.value)?.comp ?? OverviewView
)

/** 用户切换子视图时同步 store（只记录用户主动切换，避免初始化覆盖） */
function selectView(key: MatViewKey) {
  if (key === activeTab.value) return
  activeTab.value = key
  store.setActiveView(props.dump.id, key)
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
  // 9 个子视图必然溢出：TabBarItem 默认 min-width 100px，窄面板下光名字就放不下。
  // 这里只压缩**最小宽度**（标签很短，64px 足够），不改换行 ——
  // 换行会让内容区高度抖动，也与全站 Tab 栏的横向滚动语义不一致。
  :deep(.tab-bar-item) {
    min-width: 64px;
    max-width: 140px;
    justify-content: center;
  }
}

.viewer-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
