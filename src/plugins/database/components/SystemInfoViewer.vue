<template>
  <div class="system-info-viewer">
    <div class="viewer-toolbar">
      <span class="viewer-title">{{ title }}</span>
      <el-input
        v-model="keyword"
        class="filter-input"
        size="small"
        clearable
        placeholder="过滤变量名"
      />
      <app-button size="small" :disabled="loading" @click="load">刷新</app-button>
    </div>

    <div class="viewer-body" v-loading="loading" element-loading-text="加载中…">
      <template v-if="error">
        <div class="viewer-error">{{ error }}</div>
      </template>
      <el-table
        v-else
        :data="pagedRows"
        size="small"
        border
        stripe
        height="100%"
        :empty-text="loading ? '' : '暂无数据'"
      >
        <el-table-column type="index" label="#" width="48" />
        <el-table-column prop="name" label="名称" min-width="220" show-overflow-tooltip />
        <el-table-column prop="value" label="值" min-width="220" show-overflow-tooltip />
      </el-table>
    </div>

    <div class="viewer-footer">
      <span>共 {{ filteredRows.length }} 条{{ keyword ? `（原始 ${rows.length} 条）` : '' }}</span>
      <el-pagination
        v-if="filteredRows.length > pageSize"
        small
        layout="prev, pager, next"
        :total="filteredRows.length"
        :page-size="pageSize"
        :current-page="page"
        @current-change="onPageChange"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { dbShowStatus, dbShowVariables } from '../api/metadata'
import type { KeyValueRow, SystemInfoSubType } from '../types'

const props = defineProps<{
  configId?: string
  subType?: SystemInfoSubType
}>()

/** 状态/变量通常 300~600 行，前端分页避免一次性渲染过多行 */
const pageSize = 100

const loading = ref(false)
const error = ref('')
const keyword = ref('')
const page = ref(1)
const rows = ref<KeyValueRow[]>([])

const TITLE_MAP: Record<SystemInfoSubType, string> = {
  'session-status': '会话状态',
  'global-status': '全局状态',
  'session-variables': '会话变量',
  'global-variables': '全局变量',
}

const title = computed(() => (props.subType ? TITLE_MAP[props.subType] : '系统信息'))

const filteredRows = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  if (!kw) return rows.value
  return rows.value.filter(r => r.name.toLowerCase().includes(kw))
})

const pagedRows = computed(() => {
  const start = (page.value - 1) * pageSize
  return filteredRows.value.slice(start, start + pageSize)
})

async function load() {
  const configId = props.configId
  const subType = props.subType
  if (!configId || !subType) {
    rows.value = []
    error.value = '缺少连接或系统信息类型'
    return
  }

  loading.value = true
  error.value = ''
  page.value = 1
  try {
    const isStatus = subType === 'session-status' || subType === 'global-status'
    const scope = subType.startsWith('session-') ? 'session' : 'global'
    const res = isStatus
      ? await dbShowStatus(configId, scope)
      : await dbShowVariables(configId, scope)
    if (res.code !== 200) {
      error.value = res.msg || '查询失败'
      rows.value = []
      return
    }
    rows.value = res.data ?? []
  } catch (e: any) {
    error.value = '查询失败: ' + (e?.message || e)
    rows.value = []
  } finally {
    loading.value = false
  }
}

// 过滤条件变化时回到第一页，避免停留在越界页
watch(keyword, () => { page.value = 1 })

function onPageChange(p: number) {
  page.value = p
}

onMounted(load)
watch(() => [props.configId, props.subType], load)
</script>

<style scoped>
.system-info-viewer {
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
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 4px 10px;
  font-size: var(--font-size-xs);
  color: var(--textSecondary);
  border-top: 1px solid var(--borderColor);
}
</style>
