<template>
  <div class="object-list-viewer">
    <div class="viewer-toolbar">
      <span class="viewer-title">{{ title }}</span>
      <el-input
        v-model="keyword"
        class="filter-input"
        size="small"
        clearable
        placeholder="过滤（支持名称 / 任意列模糊匹配）"
      />
      <app-button size="small" :disabled="loading" @click="load">刷新</app-button>
    </div>

    <div class="viewer-body" v-loading="loading" element-loading-text="加载中…">
      <template v-if="error">
        <div class="viewer-error">{{ error }}</div>
      </template>
      <el-table
        v-else
        :data="filteredRows"
        size="small"
        border
        stripe
        height="100%"
        :empty-text="loading ? '' : '暂无数据'"
      >
        <el-table-column type="index" label="#" width="48" />
        <el-table-column
          v-for="col in columns"
          :key="col.prop"
          :prop="col.prop"
          :label="col.label"
          :width="col.width"
          min-width="110"
          show-overflow-tooltip
        />
      </el-table>
    </div>

    <div class="viewer-footer">
      <span>共 {{ filteredRows.length }} 条{{ keyword ? `（原始 ${rows.length} 条）` : '' }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import {
  dbListUsers, dbListAdmins, dbListViews, dbListProcedures, dbListFunctions,
  dbListTriggers, dbListEvents, dbListDatabaseIndexes,
} from '../api/metadata'
import type { DatabaseIndexInfo, DatabaseObjectInfo, DbAdminInfo, DbUserInfo, ObjectListType } from '../types'

const props = defineProps<{
  configId?: string
  database?: string
  objectType?: ObjectListType
}>()

/** 表格列定义 */
interface ColumnDef {
  prop: string
  label: string
  width?: number
}

const loading = ref(false)
const error = ref('')
const keyword = ref('')
const rows = ref<Record<string, any>[]>([])

const TITLE_MAP: Record<ObjectListType, string> = {
  user: '用户',
  admin: '管理员',
  view: '视图',
  index: '索引',
  procedure: '存储过程',
  function: '函数',
  trigger: '触发器',
  event: '事件',
}

const COLUMN_MAP: Record<ObjectListType, ColumnDef[]> = {
  user: [
    { prop: 'user', label: '用户' },
    { prop: 'host', label: '主机' },
    { prop: 'plugin', label: '认证插件', width: 140 },
    { prop: 'privileges', label: '权限' },
  ],
  admin: [
    { prop: 'user', label: '用户' },
    { prop: 'host', label: '主机' },
    { prop: 'reason', label: '判定依据', width: 180 },
    { prop: 'privileges', label: '权限标记' },
  ],
  view: [
    { prop: 'name', label: '名称' },
    { prop: 'object_type', label: '类型', width: 90 },
    { prop: 'definer', label: '定义者', width: 140 },
    { prop: 'comment', label: '备注' },
  ],
  index: [
    { prop: 'name', label: '索引名' },
    { prop: 'table_name', label: '所属表' },
    { prop: 'column_name', label: '列' },
    { prop: 'is_unique', label: '唯一', width: 70 },
    { prop: 'is_primary', label: '主键', width: 70 },
    { prop: 'index_type', label: '索引类型', width: 100 },
  ],
  procedure: [
    { prop: 'name', label: '名称' },
    { prop: 'object_type', label: '类型', width: 100 },
    { prop: 'definer', label: '定义者', width: 140 },
    { prop: 'created', label: '创建时间', width: 160 },
    { prop: 'modified', label: '最后修改', width: 160 },
    { prop: 'status', label: '状态', width: 90 },
  ],
  function: [
    { prop: 'name', label: '名称' },
    { prop: 'object_type', label: '类型', width: 100 },
    { prop: 'definer', label: '定义者', width: 140 },
    { prop: 'created', label: '创建时间', width: 160 },
    { prop: 'modified', label: '最后修改', width: 160 },
    { prop: 'status', label: '状态', width: 90 },
  ],
  trigger: [
    { prop: 'name', label: '名称' },
    { prop: 'event', label: '触发事件', width: 100 },
    { prop: 'table_name', label: '所属表' },
    { prop: 'status', label: '状态', width: 90 },
  ],
  event: [
    { prop: 'name', label: '名称' },
    { prop: 'status', label: '状态', width: 90 },
    { prop: 'definer', label: '定义者', width: 140 },
    { prop: 'created', label: '创建时间', width: 160 },
    { prop: 'modified', label: '最后修改', width: 160 },
  ],
}

const title = computed(() => {
  const t = props.objectType ? TITLE_MAP[props.objectType] : '对象'
  return props.database ? `${t} - ${props.database}` : t
})

const columns = computed<ColumnDef[]>(() =>
  props.objectType ? COLUMN_MAP[props.objectType] : []
)

const filteredRows = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  if (!kw) return rows.value
  return rows.value.filter(r =>
    Object.values(r).some(v => String(v ?? '').toLowerCase().includes(kw))
  )
})

/** 布尔列转中文，避免前端直接显示 true/false */
function yesNo(v: boolean): string {
  return v ? '是' : '否'
}

/** 空值统一显示占位符，避免出现空白单元格 */
function blank(v: unknown): string {
  const s = String(v ?? '').trim()
  return s === '' ? '-' : s
}

async function load() {
  const objectType = props.objectType
  const configId = props.configId
  if (!objectType || !configId) {
    rows.value = []
    error.value = '缺少连接或对象类型'
    return
  }
  // 库级对象必须带 database
  if (objectType !== 'user' && objectType !== 'admin' && !props.database) {
    rows.value = []
    error.value = '缺少数据库名'
    return
  }

  loading.value = true
  error.value = ''
  try {
    let list: Record<string, any>[]
    if (objectType === 'user') {
      const res = await dbListUsers(configId)
      if (res.code !== 200) { error.value = res.msg || '查询失败'; return }
      list = (res.data as DbUserInfo[]).map(u => ({
        user: blank(u.user),
        host: blank(u.host),
        plugin: blank(u.plugin),
        privileges: blank(u.privileges),
      }))
    } else if (objectType === 'admin') {
      const res = await dbListAdmins(configId)
      if (res.code !== 200) { error.value = res.msg || '查询失败'; return }
      list = (res.data as DbAdminInfo[]).map(u => ({
        user: blank(u.user),
        host: blank(u.host),
        reason: blank(u.reason),
        privileges: blank(u.privileges),
      }))
    } else if (objectType === 'index') {
      const res = await dbListDatabaseIndexes(configId, props.database!)
      if (res.code !== 200) { error.value = res.msg || '查询失败'; return }
      list = (res.data as DatabaseIndexInfo[]).map(ix => ({
        name: blank(ix.name),
        table_name: blank(ix.table_name),
        column_name: blank(ix.column_name),
        is_unique: yesNo(ix.is_unique),
        is_primary: yesNo(ix.is_primary),
        index_type: blank(ix.index_type),
      }))
    } else {
      const fetcher = objectType === 'view' ? dbListViews
        : objectType === 'procedure' ? dbListProcedures
        : objectType === 'function' ? dbListFunctions
        : objectType === 'trigger' ? dbListTriggers
        : dbListEvents
      const res = await fetcher(configId, props.database!)
      if (res.code !== 200) { error.value = res.msg || '查询失败'; return }
      list = (res.data as DatabaseObjectInfo[]).map(o => ({
        name: blank(o.name),
        object_type: blank(o.object_type),
        definer: blank(o.definer),
        created: blank(o.created),
        modified: blank(o.modified),
        comment: blank(o.comment),
        status: blank(o.status),
        event: blank(o.event),
        table_name: blank(o.table_name),
      }))
    }
    rows.value = list
  } catch (e: any) {
    error.value = '查询失败: ' + (e?.message || e)
    rows.value = []
  } finally {
    loading.value = false
  }
}

onMounted(load)
watch(() => [props.configId, props.database, props.objectType], load)
</script>

<style scoped>
.object-list-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.viewer-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--borderColor);
}
.viewer-title {
  font-size: var(--font-size-md);
  color: var(--textColor);
  font-family: var(--font-family-mono);
  white-space: nowrap;
}
.filter-input {
  width: 260px;
}
.viewer-body {
  flex: 1;
  overflow: hidden;
}
.viewer-error {
  padding: 16px;
  color: var(--stateErrorFg);
  font-size: var(--font-size-md);
}
.viewer-footer {
  padding: 4px 10px;
  font-size: var(--font-size-xs);
  color: var(--textSecondary);
  border-top: 1px solid var(--borderColor);
}
</style>
