<template>
  <div class="oql-view">
    <!-- 查询编辑区 -->
    <div class="oql-editor">
      <div class="oql-editor-head">
        <span class="oql-title">OQL 安全子集</span>
        <span class="oql-hint">
          支持 SELECT / FROM INSTANCEOF / WHERE（数值、null、@class 文本）/ LIMIT；禁用方法调用与反射
        </span>
      </div>
      <el-input
        v-model="query"
        type="textarea"
        :rows="3"
        resize="none"
        spellcheck="false"
        class="oql-input"
        placeholder="select @id, @shallow from INSTANCEOF java.util.HashMap where size > 1000 limit 100"
        @keydown.ctrl.enter.prevent="run"
        @keydown.meta.enter.prevent="run"
      />
      <div class="oql-actions">
        <app-button size="small" :disabled="result.loading" @click="run">
          {{ result.loading ? '执行中…' : '执行' }}
        </app-button>
        <app-button size="small" text :disabled="result.loading" @click="checkSyntax">仅校验语法</app-button>
        <span class="oql-kbd">Ctrl + Enter</span>
        <span v-if="checkMsg" class="oql-check" :class="{ bad: !checkOk }">{{ checkMsg }}</span>
      </div>
      <div class="oql-samples">
        <span class="oql-samples-label">示例：</span>
        <button
          v-for="(s, i) in SAMPLES"
          :key="i"
          type="button"
          class="oql-sample"
          :title="s.q"
          @click="useSample(s.q)"
        >{{ s.label }}</button>
      </div>
    </div>

    <!-- 结果区 -->
    <div class="oql-result">
      <div v-if="result.error" class="oql-error">
        <el-icon><WarningFilled /></el-icon>
        <span>{{ result.error }}</span>
      </div>
      <div v-else-if="!result.data" class="oql-empty">
        <el-icon class="oql-empty-ic"><Search /></el-icon>
        <div>
          <b>还没有结果</b>
          <span>写一条查询或用上面的示例，执行后按对象逐行列出；命中数超出行上限时会明确提示。</span>
        </div>
      </div>
      <template v-else>
        <div class="oql-summary">
          <span>命中 <b class="mono">{{ formatInt(result.data.matched) }}</b> 个对象</span>
          <span class="oql-sep" />
          <span>返回 <b class="mono">{{ formatInt(result.data.rows.length) }}</b> 行</span>
          <span class="oql-sep" />
          <span>扫描 <b class="mono">{{ formatInt(result.data.scanned) }}</b> 个候选</span>
          <span class="oql-sep" />
          <span>{{ result.data.elapsed_ms }} ms</span>
          <span class="oql-grow" />
          <ExportButton
            :rows="flatRows"
            :columns="exportColumns"
            :filename="`${dump.name}-oql.csv`"
            label="导出 CSV"
          />
        </div>
        <div v-if="result.data.note" class="oql-note">{{ result.data.note }}</div>
        <el-table
          v-if="result.data.rows.length"
          :data="flatRows"
          height="100%"
          size="small"
          class="oql-table"
          border
        >
          <el-table-column prop="__id" label="对象 ID" width="150" class-name="mono" />
          <el-table-column prop="__class" label="类" min-width="240" show-overflow-tooltip />
          <el-table-column
            v-for="c in result.data.columns"
            :key="c"
            :prop="c"
            :label="c"
            min-width="140"
            show-overflow-tooltip
          />
        </el-table>
        <div v-else class="oql-empty small">没有对象匹配该查询</div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { Search, WarningFilled } from '@element-plus/icons-vue'
import { fetchOqlCheck, fetchOqlQuery, formatInt } from '../../api/matApi'
import type { OqlResult } from '../../api/matApi'
import type { MatDump } from '../../types'
import ExportButton from '../ExportButton.vue'
import type { ExportColumn } from '../../utils/export'

/**
 * OQL 视图：**只**执行后端安全子集（`mat_oql_query`）。
 * 前端不做语法解析 —— 语法真源只有一份（Rust 解析器），避免两侧对「什么合法」理解不一致。
 * 「仅校验语法」走 `mat_oql_check`，不读 dump，输入阶段就能拿到明确报错。
 */
const props = defineProps<{ dump: MatDump }>()

const SAMPLES = [
  { label: '大 HashMap', q: 'select @id, @shallow, size from INSTANCEOF java.util.HashMap where size > 1000 limit 100' },
  { label: '长 String', q: 'select @id, @shallow from INSTANCEOF java.lang.String limit 50' },
  { label: '所有线程', q: 'select @id, @class, priority from INSTANCEOF java.lang.Thread' },
  { label: '自定义类通配', q: 'select * from INSTANCEOF *Demo* limit 100' },
]

const query = ref(SAMPLES[0].q)
const result = reactive<{ loading: boolean; error: string; data: OqlResult | null }>({
  loading: false,
  error: '',
  data: null,
})
const checkMsg = ref('')
const checkOk = ref(true)

const flatRows = computed(() => {
  const rows = result.data?.rows ?? []
  return rows.map((r) => {
    const flat: Record<string, string> = { __id: r.obj_id, __class: r.class_name }
    for (const c of r.cells) flat[c.name] = c.value
    return flat
  })
})

const exportColumns = computed<ExportColumn[]>(() => {
  const cols: ExportColumn[] = [
    { key: '__id', label: 'Object ID' },
    { key: '__class', label: 'Class' },
  ]
  for (const c of result.data?.columns ?? []) cols.push({ key: c, label: c })
  return cols
})

function useSample(q: string) {
  query.value = q
  checkMsg.value = ''
}

async function run() {
  const q = query.value.trim()
  if (!q) {
    result.error = '请输入查询语句'
    return
  }
  result.loading = true
  result.error = ''
  checkMsg.value = ''
  try {
    result.data = await fetchOqlQuery(props.dump.path, q)
  } catch (e: any) {
    result.data = null
    result.error = e?.message || String(e)
  } finally {
    result.loading = false
  }
}

/** 只校验语法：不读文件，几毫秒返回；失败给出解析器的原始提示 */
async function checkSyntax() {
  const q = query.value.trim()
  if (!q) {
    checkOk.value = false
    checkMsg.value = '请输入查询语句'
    return
  }
  try {
    const c = await fetchOqlCheck(q)
    checkOk.value = true
    checkMsg.value = `语法通过 · 类 ${c.class_pattern} · 列 ${c.columns.join(', ')}${c.has_filter ? ' · 带过滤' : ''} · LIMIT ${c.limit}`
  } catch (e: any) {
    checkOk.value = false
    checkMsg.value = e?.message || String(e)
  }
}
</script>

<style scoped lang="scss">
.oql-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 10px 12px;
  gap: 10px;
  box-sizing: border-box;
}

.oql-editor {
  flex-shrink: 0;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
  background: var(--el-fill-color-blank);
}

.oql-editor-head {
  display: flex;
  align-items: baseline;
  gap: 10px;
  margin-bottom: 6px;
  flex-wrap: wrap;
}

.oql-title {
  font-weight: 600;
  font-size: var(--font-size-sm);
}

.oql-hint,
.oql-kbd {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
}

.oql-kbd {
  font-family: var(--font-family-mono);
}

.oql-input :deep(textarea) {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  line-height: 1.5;
}

.oql-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 6px;
  flex-wrap: wrap;
}

.oql-check {
  font-size: var(--font-size-xs);
  color: var(--el-color-success);

  &.bad {
    color: var(--stateErrorFg);
  }
}

.oql-samples {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 6px;
  flex-wrap: wrap;
  font-size: var(--font-size-xs);
}

.oql-samples-label {
  color: var(--el-text-color-secondary);
}

.oql-sample {
  border: 1px solid var(--el-border-color-lighter);
  background: transparent;
  color: var(--el-text-color-regular);
  border-radius: 999px;
  padding: 1px 8px;
  font-size: var(--font-size-xs);
  cursor: pointer;

  &:hover {
    border-color: var(--el-color-primary);
    color: var(--el-color-primary);
  }
}

.oql-result {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.oql-summary {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  flex-wrap: wrap;
}

.oql-sep {
  width: 1px;
  height: 12px;
  background: var(--el-border-color-lighter);
}

.oql-grow {
  flex: 1;
}

.oql-note {
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
  border-radius: var(--radius-xs);
  padding: 4px 8px;
}

.oql-table {
  flex: 1;
  min-height: 0;
}

.oql-error {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--font-size-sm);
  color: var(--stateErrorFg);
}

.oql-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--el-text-color-secondary);
  font-size: var(--font-size-sm);

  &.small {
    padding: 24px 0;
  }

  div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  span {
    font-size: var(--font-size-xs);
  }
}

.oql-empty-ic {
  font-size: 28px;
  opacity: 0.5;
}
</style>
