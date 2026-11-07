<template>
  <div class="table-viewer">
    <el-tabs v-model="activeTab" class="viewer-tabs">
      <!-- 数据 Tab（默认）：传表名，由 QueryResultView 自行构造 SELECT（表数据浏览） -->
      <el-tab-pane name="data" label="数据">
        <QueryResultView :config-id="configId" :database="database" :table="table" />
      </el-tab-pane>

      <!-- 属性 Tab：表元数据 + 字段 + DDL -->
      <el-tab-pane name="props" label="属性">
        <div class="props-panel" v-loading="loadingProps">
          <!-- 表信息 -->
          <div class="meta-grid">
            <div class="meta-row">
              <span class="label">表名</span>
              <span class="value mono">{{ table }}</span>
            </div>
            <div class="meta-row">
              <span class="label">引擎</span>
              <span class="value">{{ tableInfo?.engine || '-' }}</span>
            </div>
            <div class="meta-row">
              <span class="label">行数</span>
              <span class="value">{{ tableInfo?.row_count ?? '-' }}</span>
            </div>
            <div class="meta-row">
              <span class="label">大小</span>
              <span class="value">{{ formatSize(tableInfo?.data_length) }}</span>
            </div>
            <div class="meta-row meta-row-wide">
              <span class="label">注释</span>
              <span class="value">{{ tableInfo?.comment || '-' }}</span>
            </div>
          </div>

          <!-- 字段 -->
          <div class="props-section">
            <div class="section-title">字段</div>
            <el-table :data="columns" size="small" border max-height="240" class="cols-table">
              <el-table-column prop="name" label="字段名" min-width="120" />
              <el-table-column prop="data_type" label="类型" min-width="100" />
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
              <el-table-column prop="default_value" label="默认值" min-width="100" show-overflow-tooltip />
              <el-table-column prop="comment" label="注释" min-width="120" show-overflow-tooltip />
            </el-table>
          </div>

          <!-- DDL -->
          <div class="props-section">
            <div class="section-title">DDL</div>
            <!--
              复制按钮放在**外层容器**的右上角，不放在 <pre> 里：
              pre 自己会滚动（max-height + overflow:auto），按钮跟着内容滚走就没法点了。
            -->
            <div class="ddl-wrap">
              <pre class="ddl-box">{{ loadingProps ? '加载中...' : (ddl || '暂无数据') }}</pre>
              <el-tooltip content="复制 DDL" placement="top">
                <app-button
                  v-if="ddl && !loadingProps"
                  class="ddl-copy"
                  bare
                  size="small"
                  :icon="DocumentCopy"
                  aria-label="复制 DDL"
                  @click="copyDdl"
                ></app-button>
              </el-tooltip>
            </div>
          </div>
        </div>
      </el-tab-pane>

      <!-- 预留 Tab（图标占位，扩展用） -->
      <el-tab-pane name="reserve">
        <template #label>
          <el-icon class="reserve-icon" title="预留功能"><MoreFilled /></el-icon>
        </template>
        <div class="reserve-panel">
          <p style="color: var(--textSecondary); font-size: var(--font-size-md);">预留功能区（图表 / ER 图 / 统计 等后续可加）</p>
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { MoreFilled, DocumentCopy } from '@element-plus/icons-vue'
import QueryResultView from './QueryResultView.vue'
import { useConnectionStore } from '../store'
import { dbTableColumns, dbTableSchema, dbListTables } from '../api/metadata'
import { copyText } from '../utils/clipboard'
import type { ColumnInfo, TableInfo } from '../types'

/**
 * 表查看器：双击表名时打开的综合视图
 * - 数据 Tab（默认）：用 SELECT 语句驱动 QueryResultView（表数据浏览）
 * - 属性 Tab：表元数据 + 字段 + DDL
 * - 预留 Tab：扩展位（图表/ER 图等）
 */
const props = defineProps<{
  configId: string
  database: string
  table: string
}>()

const activeTab = ref<'data' | 'props' | 'reserve'>('data')
const columns = ref<ColumnInfo[]>([])
const ddl = ref('')
const tableInfo = ref<TableInfo | null>(null)
const loadingProps = ref(false)
const propsLoaded = ref(false)
const connectionStore = useConnectionStore()

/** 确保连接可用（关闭 tab 断开后重新打开时自动重连），返回是否就绪 */
async function ensureConnection(): Promise<boolean> {
  if (connectionStore.isConnected(props.configId)) return true
  const res = await connectionStore.connect(props.configId)
  if (res.code !== 200) {
    ElMessage.error('连接失败：' + (res.msg || '未知错误'))
    return false
  }
  return true
}

function copyDdl() {
  copyText(ddl.value, 'DDL 已复制')
}

function formatSize(bytes?: number | null): string {
  if (!bytes || bytes <= 0) return '-'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`
}

/** 属性 Tab 首次切换到时才加载（懒加载）；每个请求独立超时，一个卡住不影响其他 */
async function loadProps() {
  loadingProps.value = true
  // 先确保连接可用（关闭 tab 断开后重新打开时自动重连）
  if (!(await ensureConnection())) {
    loadingProps.value = false
    return
  }
  // 请求级超时：单个请求最多等 8s，超时按失败处理（返回 null），不拖累其他请求
  const withTimeout = <T>(p: Promise<T>, ms = 8000): Promise<T | null> =>
    Promise.race([
      p.catch(() => null),
      new Promise<null>((resolve) => setTimeout(() => resolve(null), ms))
    ])

  const [colsRes, ddlRes, tablesRes] = await Promise.all([
    withTimeout(dbTableColumns(props.configId, props.database, props.table)),
    withTimeout(dbTableSchema(props.configId, props.database, props.table)),
    withTimeout(dbListTables(props.configId, props.database))
  ])

  if (colsRes?.code === 200) columns.value = colsRes.data
  if (ddlRes?.code === 200) ddl.value = ddlRes.data
  if (tablesRes?.code === 200) {
    tableInfo.value = tablesRes.data.find(t => t.name === props.table) ?? null
  }
  loadingProps.value = false

  // 全部失败（连接未建立/已断开/超时）时给出提示
  if (!colsRes && !ddlRes && !tablesRes) {
    ElMessage.warning('属性数据加载失败：连接不可用或查询超时，请确认连接后重试')
  }
}

// 属性 Tab 首次激活时才加载（懒加载，避免打开表数据时无谓请求属性数据）
watch(activeTab, (tab) => {
  if (tab === 'props' && !propsLoaded.value) {
    propsLoaded.value = true
    loadProps()
  }
})

onMounted(() => {
  // 若初始就停在属性 Tab（KeepAlive 恢复等场景），补一次加载
  if (activeTab.value === 'props' && !propsLoaded.value) {
    propsLoaded.value = true
    loadProps()
  }
})
// KeepAlive 缓存下，props 变化（不同表）时重置懒加载标记并重新加载
watch(
  () => [props.configId, props.database, props.table],
  () => {
    propsLoaded.value = false
    if (activeTab.value === 'props') {
      propsLoaded.value = true
      loadProps()
    }
  }
)
</script>

<style scoped>
.table-viewer {
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
.reserve-icon {
  font-size: 16px;
  color: var(--el-text-color-secondary);
}

.props-panel {
  height: 100%;
  overflow: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
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
  width: 56px;
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
.props-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.section-title {
  font-size: var(--font-size-md);
  font-weight: 600;
  color: var(--el-text-color-primary);
}
.cols-table {
  font-family: var(--font-family-mono);
}
.ddl-wrap {
  position: relative;
}
.ddl-copy {
  position: absolute;
  top: 6px;
  right: 6px;
  /* 长行会把文字顶到右上角，按钮得垫一层底才看得清 */
  background: var(--el-bg-color-overlay);
  border-radius: var(--radius-sm);
  box-shadow: var(--el-box-shadow-light);
}
.ddl-box {
  /* 给右上角按钮留出位置，首行文字不会被压住 */
  padding-right: 40px;
  margin: 0;
  padding: 12px;
  background: var(--el-fill-color-light);
  border-radius: var(--radius-md);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 360px;
  overflow: auto;
  color: var(--el-text-color-primary);
}
.reserve-panel {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-height: 200px;
}
</style>
