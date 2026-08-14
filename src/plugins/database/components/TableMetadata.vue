<template>
  <div class="table-metadata">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="字段信息" name="columns">
        <el-table :data="columns" border size="small" style="width: 100%">
          <el-table-column prop="name" label="字段名" min-width="120" />
          <el-table-column prop="data_type" label="类型" width="100" />
          <el-table-column label="主键" width="60">
            <template #default="{ row }">
              <el-tag v-if="row.is_primary_key" type="success" size="small">PK</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="可空" width="60">
            <template #default="{ row }">
              <span>{{ row.is_nullable ? 'YES' : 'NO' }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="default_value" label="默认值" width="120" show-overflow-tooltip />
          <el-table-column prop="comment" label="注释" min-width="150" show-overflow-tooltip />
        </el-table>
      </el-tab-pane>
      <el-tab-pane label="索引信息" name="indexes">
        <el-table :data="indexes" border size="small" style="width: 100%">
          <el-table-column prop="name" label="索引名" min-width="120" />
          <el-table-column prop="column_name" label="字段" width="120" />
          <el-table-column label="唯一" width="60">
            <template #default="{ row }">
              <el-tag v-if="row.is_unique" type="info" size="small">YES</el-tag>
              <span v-else>NO</span>
            </template>
          </el-table-column>
          <el-table-column label="主键" width="60">
            <template #default="{ row }">
              <el-tag v-if="row.is_primary" type="success" size="small">PK</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="index_type" label="类型" width="80" />
        </el-table>
      </el-tab-pane>
      <el-tab-pane label="DDL" name="ddl">
        <pre class="ddl-content">{{ ddl }}</pre>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { dbTableColumns, dbTableIndexes, dbTableSchema } from '../api/metadata'
import type { ColumnInfo, IndexInfo } from '../types'

const props = defineProps<{
  configId: string
  database: string
  table: string
}>()

const activeTab = ref('columns')
const columns = ref<ColumnInfo[]>([])
const indexes = ref<IndexInfo[]>([])
const ddl = ref('')

onMounted(() => loadAll())
watch(() => [props.database, props.table], () => loadAll())

async function loadAll() {
  loadColumns()
  loadIndexes()
  loadDdl()
}

async function loadColumns() {
  try {
    const res = await dbTableColumns(props.configId, props.database, props.table)
    if (res.code === 200) columns.value = res.data
  } catch (e) { console.error(e) }
}

async function loadIndexes() {
  try {
    const res = await dbTableIndexes(props.configId, props.database, props.table)
    if (res.code === 200) indexes.value = res.data
  } catch (e) { console.error(e) }
}

async function loadDdl() {
  try {
    const res = await dbTableSchema(props.configId, props.database, props.table)
    if (res.code === 200) ddl.value = res.data
  } catch (e) { console.error(e) }
}
</script>

<style scoped>
.table-metadata {
  padding: 8px;
}
.ddl-content {
  background: #f5f7fa;
  padding: 12px;
  border-radius: 4px;
  font-family: 'Consolas', monospace;
  font-size: 13px;
  white-space: pre-wrap;
  overflow: auto;
  max-height: 500px;
}
</style>
