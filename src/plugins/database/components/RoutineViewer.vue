<template>
  <div class="routine-viewer">
    <el-tabs v-model="activeTab" class="viewer-tabs">
      <el-tab-pane name="source" label="源码">
        <SourceBlock
          :title="`${name}（${kindLabel}）`"
          :source="source"
          :loading="loadingSource"
          :error="sourceError"
          @reload="loadSource"
        />
      </el-tab-pane>

      <el-tab-pane name="props" label="属性">
        <div class="pane-body" v-loading="loadingProps">
          <div class="meta-grid">
            <div class="meta-row">
              <span class="label">名称</span>
              <span class="value mono">{{ name }}</span>
            </div>
            <div class="meta-row">
              <span class="label">类型</span>
              <span class="value">{{ info?.object_type || kindLabel }}</span>
            </div>
            <div class="meta-row">
              <span class="label">所属库</span>
              <span class="value mono">{{ database || '-' }}</span>
            </div>
            <div class="meta-row" v-if="info?.table_name">
              <span class="label">所属表</span>
              <span class="value mono">{{ info.table_name }}</span>
            </div>
            <div class="meta-row" v-if="info?.status">
              <span class="label">状态</span>
              <span class="value">{{ info.status }}</span>
            </div>
            <div class="meta-row" v-if="info?.event">
              <span class="label">触发事件</span>
              <span class="value">{{ info.event }}</span>
            </div>
            <div class="meta-row" v-if="info?.definer">
              <span class="label">定义者</span>
              <span class="value mono">{{ info.definer }}</span>
            </div>
            <div class="meta-row" v-if="info?.created">
              <span class="label">创建时间</span>
              <span class="value">{{ info.created }}</span>
            </div>
            <div class="meta-row" v-if="info?.modified">
              <span class="label">最后修改</span>
              <span class="value">{{ info.modified }}</span>
            </div>
            <div class="meta-row meta-row-wide" v-if="info?.comment">
              <span class="label">备注</span>
              <span class="value">{{ info.comment }}</span>
            </div>
          </div>
          <div v-if="!info && !loadingProps" class="pane-hint">
            没有读到该对象的目录信息（可能已被删除，或当前账号看不到）
          </div>
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
/**
 * 例程查看器：存储过程 / 函数 / 触发器共用。
 *
 * 三者差异只有「从哪张目录表里查属性」，源码统一由 `db_object_source` 取，
 * 所以合并成一个组件、`kind` 决定查哪张表 —— 比写三个几乎一样的组件好维护。
 */
import { ref, computed, watch } from 'vue'
import SourceBlock from './SourceBlock.vue'
import {
  dbListProcedures,
  dbListFunctions,
  dbListTriggers,
  dbObjectSource,
} from '../api/metadata'
import { ensureConnection } from '../utils/connection'
import type { DatabaseObjectInfo, RoutineKind } from '../types'

const props = defineProps<{
  configId: string
  database: string
  name: string
  /** 例程类型：决定读哪张目录表、取哪种源码 */
  kind: RoutineKind
}>()

const KIND_LABEL: Record<string, string> = {
  procedure: '存储过程',
  function: '函数',
  trigger: '触发器',
  view: '视图',
}

const kindLabel = computed(() => KIND_LABEL[props.kind] ?? props.kind)

/** 源码优先：打开一个存储过程，绝大多数时候就是为了看它写了什么 */
const activeTab = ref<'source' | 'props'>('source')

const source = ref('')
const loadingSource = ref(false)
const sourceError = ref('')

const info = ref<DatabaseObjectInfo | null>(null)
const loadingProps = ref(false)
const propsLoaded = ref(false)

async function loadSource() {
  loadingSource.value = true
  sourceError.value = ''
  if (await ensureConnection(props.configId)) {
    const res = await dbObjectSource(props.configId, props.database, props.kind, props.name).catch(
      () => null
    )
    if (res?.code === 200) source.value = res.data
    else sourceError.value = res?.msg || '取源码失败'
  }
  loadingSource.value = false
}

/**
 * 属性：没有「按名字查单个对象」的接口，只能在所属列表里捞出来。
 *
 * 一次列表请求换一个对象的属性，比为此在四种数据库里各写一条单对象查询便宜得多；
 * 用名字严格匹配 + 大小写兜底（Oracle / PG 的对象名大小写政策不同）。
 */
async function loadProps() {
  loadingProps.value = true
  if (await ensureConnection(props.configId)) {
    const fetcher =
      props.kind === 'function'
        ? dbListFunctions
        : props.kind === 'trigger'
          ? dbListTriggers
          : dbListProcedures
    const res = await fetcher(props.configId, props.database).catch(() => null)
    if (res?.code === 200) {
      const lower = props.name.toLowerCase()
      info.value = res.data.find(o => o.name === props.name || o.name?.toLowerCase() === lower) ?? null
    }
  }
  loadingProps.value = false
}

watch(activeTab, tab => {
  if (tab === 'props' && !propsLoaded.value) {
    propsLoaded.value = true
    loadProps()
  }
})

watch(
  () => [props.configId, props.database, props.name, props.kind],
  () => {
    propsLoaded.value = false
    info.value = null
    source.value = ''
    sourceError.value = ''
    loadSource()
    if (activeTab.value === 'props') {
      propsLoaded.value = true
      loadProps()
    }
  },
  { immediate: true }
)
</script>

<style scoped>
.routine-viewer {
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
.viewer-tabs :deep(.el-tabs__content) {
  flex: 1;
  min-height: 0;
  padding: 0;
}
.viewer-tabs :deep(.el-tab-pane) {
  height: 100%;
}
.pane-body {
  height: 100%;
  overflow: auto;
  padding: 16px;
}
.meta-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(280px, 1fr));
  gap: 8px 24px;
  padding: 12px 16px;
  background: var(--el-fill-color-light);
  border-radius: var(--radius-md);
}
.meta-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 24px;
}
.meta-row-wide {
  grid-column: 1 / -1;
}
.meta-row .label {
  flex-shrink: 0;
  width: 64px;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}
.meta-row .value {
  font-size: var(--font-size-md);
  color: var(--el-text-color-primary);
  word-break: break-all;
}
.mono {
  font-family: var(--font-family-mono);
}
.pane-hint {
  padding: 12px;
  font-size: var(--font-size-md);
  color: var(--el-text-color-secondary);
}
</style>
