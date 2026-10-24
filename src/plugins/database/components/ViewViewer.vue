<template>
  <div class="view-viewer">
    <el-tabs v-model="activeTab" class="viewer-tabs">
      <el-tab-pane name="data" label="数据">
        <!--
          视图数据一律 **read-only**：绝大多数视图不可更新（聚合 / GROUP BY / UNION），
          而它的 SELECT 又会被判定成「单表」从而放开编辑 —— 那等于给了一个
          「界面允许改、点保存必失败」的入口。这里显式关掉。
        -->
        <QueryResultView :config-id="configId" :database="database" :table="name" read-only />
      </el-tab-pane>

      <el-tab-pane name="columns" label="字段">
        <div class="pane-body" v-loading="loadingColumns">
          <el-table :data="columns" size="small" border stripe height="100%" class="cols-table">
            <el-table-column prop="name" label="字段名" min-width="120" />
            <el-table-column prop="data_type" label="类型" min-width="100" />
            <el-table-column label="可空" width="60">
              <template #default="{ row }">
                <span>{{ row.is_nullable ? 'YES' : 'NO' }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="default_value" label="默认值" min-width="100" show-overflow-tooltip />
            <el-table-column prop="comment" label="注释" min-width="120" show-overflow-tooltip />
          </el-table>
        </div>
      </el-tab-pane>

      <el-tab-pane name="source" label="源码">
        <SourceBlock
          :title="`${name}（创建语句）`"
          :source="source"
          :loading="loadingSource"
          :error="sourceError"
          @reload="loadSource"
        />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
/**
 * 视图查看器：双击树上的视图叶子时打开。
 *
 * 三个页签对应三件不同的事：
 * - 数据：`SELECT * FROM 视图`，只读（见模板里的注释）
 * - 字段：视图的列元数据（各库的 `${table}_columns` 对视图同样有效，无需新接口）
 * - 源码：**这里刻意不用 `dbTableSchema`** —— 那条路在 PG / SQL Server / Oracle 上
 *   是按列信息拼出的 `CREATE TABLE`，用在视图上会给出一句不存在的语句。
 */
import { ref, watch } from 'vue'
import QueryResultView from './QueryResultView.vue'
import SourceBlock from './SourceBlock.vue'
import { dbTableColumns, dbObjectSource } from '../api/metadata'
import { ensureConnection } from '../utils/connection'
import type { ColumnInfo } from '../types'

const props = defineProps<{
  configId: string
  database: string
  name: string
}>()

const activeTab = ref<'data' | 'columns' | 'source'>('data')

const columns = ref<ColumnInfo[]>([])
const loadingColumns = ref(false)
const columnsLoaded = ref(false)

const source = ref('')
const loadingSource = ref(false)
const sourceError = ref('')
const sourceLoaded = ref(false)

async function loadColumns() {
  loadingColumns.value = true
  if (await ensureConnection(props.configId)) {
    const res = await dbTableColumns(props.configId, props.database, props.name).catch(() => null)
    if (res?.code === 200) columns.value = res.data
  }
  loadingColumns.value = false
}

async function loadSource() {
  loadingSource.value = true
  sourceError.value = ''
  if (await ensureConnection(props.configId)) {
    const res = await dbObjectSource(props.configId, props.database, 'view', props.name).catch(
      () => null
    )
    if (res?.code === 200) source.value = res.data
    else sourceError.value = res?.msg || '取源码失败'
  }
  loadingSource.value = false
}

// 两个副页签都是**首次切到才加载**：打开视图多半是先看数据，没必要一开始就把三份都拉回来
watch(activeTab, tab => {
  if (tab === 'columns' && !columnsLoaded.value) {
    columnsLoaded.value = true
    loadColumns()
  }
  if (tab === 'source' && !sourceLoaded.value) {
    sourceLoaded.value = true
    loadSource()
  }
})

// KeepAlive 下 props 变化（切到别的视图）要重置，否则会留着上一个对象的内容
watch(
  () => [props.configId, props.database, props.name],
  () => {
    columnsLoaded.value = false
    sourceLoaded.value = false
    columns.value = []
    source.value = ''
    sourceError.value = ''
    if (activeTab.value === 'columns') {
      columnsLoaded.value = true
      loadColumns()
    } else if (activeTab.value === 'source') {
      sourceLoaded.value = true
      loadSource()
    }
  }
)
</script>

<style scoped>
.view-viewer {
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
  padding: 8px;
  box-sizing: border-box;
}
.cols-table {
  font-family: var(--font-family-mono);
}
</style>
