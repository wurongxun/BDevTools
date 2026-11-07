<template>
  <div class="overview">
    <!-- 元信息卡片 -->
    <div class="meta-card">
      <div class="meta-row">
        <span class="label">文件</span>
        <span class="value mono">{{ dump.name }}</span>
      </div>
      <div class="meta-row">
        <span class="label">路径</span>
        <span class="value mono path" :title="dump.path">{{ dump.path }}</span>
      </div>
      <div class="meta-row">
        <span class="label">格式</span>
        <span class="value">
          <el-tag size="small" :type="dump.supported ? 'primary' : 'warning'" effect="plain">
            {{ dump.format || '识别中…' }}
          </el-tag>
        </span>
      </div>
      <div class="meta-row">
        <span class="label">大小</span>
        <span class="value">{{ formatBytes(dump.size) }}（{{ formatInt(dump.size) }} 字节）</span>
      </div>
      <div v-if="dump.compressed" class="meta-row">
        <span class="label">压缩包</span>
        <span class="value">
          gzip ·
          <template v-if="dump.sourceInfo">
            源 {{ formatBytes(dump.sourceInfo.source_size) }} → 解压后
            {{ formatBytes(dump.sourceInfo.hprof_size) }}
            <el-tag size="small" :type="dump.sourceInfo.cache_hit ? 'success' : 'info'" effect="plain" class="ml4">
              {{ dump.sourceInfo.cache_hit ? '命中已解压缓存' : '本次已解压' }}
            </el-tag>
          </template>
          <template v-else>解压中…</template>
        </span>
      </div>
      <template v-if="dump.supported && dump.idSize">
        <div class="meta-row">
          <span class="label">标识符宽</span>
          <span class="value">{{ dump.idSize }} 字节（{{ dump.idSize === 8 ? '64 位 JVM' : '32 位 JVM' }}）</span>
        </div>
        <div class="meta-row">
          <span class="label">Dump 时间</span>
          <span class="value mono">{{ dump.timestampText || '未知' }}</span>
        </div>
      </template>
      <div v-if="dump.detail" class="meta-row detail">
        <span class="label">说明</span>
        <span class="value">{{ dump.detail }}</span>
      </div>
      <div v-if="dump.error" class="meta-row detail error">
        <span class="label">错误</span>
        <span class="value">{{ dump.error }}</span>
      </div>
    </div>

    <!-- 支持的格式提示 -->
    <div class="cap-card">
      <div class="cap-title">支持格式</div>
      <ul class="cap-list">
        <li><b>JAVA PROFILE 1.0.x</b> —— HotSpot / OpenJDK（jcmd GC.heap_dump、jmap、OOM 自动转储）</li>
        <li><b>Android Studio .hprof</b> —— 同族 JAVA PROFILE 变体（自动识别，个别扩展记录可能需后续适配）</li>
        <li><b>IBM PHD</b> —— 可识别（暂不支持解析，见右侧路线图）</li>
      </ul>
    </div>

    <!-- 当前能力 -->
    <div class="cap-card">
      <div class="cap-title">当前能力</div>
      <ul class="cap-list">
        <li>✅ <b>类</b>：按类 / 包统计对象数量、浅堆与深堆（Retained Heap），支持导出</li>
        <li>✅ <b>Leak Suspects</b>：支配树 Retained Heap + GC Roots 引用路径</li>
        <li>✅ <b>支配树</b>：对象级 Retained Heap 排序、子树展开、保留集查看</li>
        <li>✅ <b>引用追踪</b>：Outgoing / Incoming / Path to GC Roots（多路径 / 弱引用开关）+ 对象字段值</li>
        <li>✅ <b>大对象排行</b>：按类 / 包聚合 retained（只算支配树顶层，占比可加）</li>
        <li>✅ <b>专用报告</b>：线程 / 类加载器 / 内存浪费 / 不可达对象</li>
        <li>✅ <b>OQL</b>：白名单安全子集（<code>select … from … where …</code>）</li>
        <li>✅ <b>多 dump 对比</b>：按类 / 包看 Δcount / Δshallow</li>
        <li>✅ <b>导出</b>：表格 CSV / 泄漏报告 HTML</li>
        <li>✅ <b>压缩包</b>：<code>.hprof.gz</code> 自动解压（缓存复用，见下方缓存卡片）</li>
        <li>
          ✅ <b>浅堆口径</b>：对象头 / 压缩指针 / 对齐按 JVM 推导（实例头、数组头、类对象大小，
          可经 <code>plugins/mat.json</code> 的 <code>heapLayout</code> 覆盖）；引用追踪页顶部会显示当前口径
        </li>
      </ul>
    </div>

    <!-- 解压缓存（.hprof.gz 用） -->
    <div class="cap-card">
      <div class="cap-title">
        解压缓存
        <el-button
          v-if="cache && cache.files"
          size="small"
          text
          type="danger"
          :loading="clearing"
          class="cache-clear"
          @click="clearCacheNow"
        >
          清理
        </el-button>
      </div>
      <div v-if="cacheErr" class="cache-line err">{{ cacheErr }}</div>
      <template v-else-if="cache">
        <div class="cache-line">
          <span class="mono">{{ cache.dir }}</span>
        </div>
        <div class="cache-line">
          {{ cache.files ? `${cache.files} 个已解压文件，共 ${formatBytes(cache.total_bytes)}` : '暂无缓存' }}
          <span v-if="cache.partial_files" class="cache-warn">· 残留 {{ cache.partial_files }} 个半成品</span>
        </div>
        <div class="cache-line hint">
          打开 <code>.hprof.gz</code> 时会解压到此处，下次打开同一文件直接复用；清理后需重新解压。
          仍被当前分析占用的文件会跳过，重启应用后可清掉。
        </div>
      </template>
      <div v-else class="cache-line hint">读取中…</div>
    </div>

    <!-- 本地索引（.bdtidx）：决定「二次打开是否秒开」 -->
    <div class="cap-card">
      <div class="cap-title">
        本地索引
        <el-button
          v-if="idx && (idx.graph_valid || idx.dom_valid)"
          size="small"
          text
          type="danger"
          :loading="idxClearing"
          class="cache-clear"
          @click="clearIdx"
        >
          删除
        </el-button>
        <el-button size="small" text :loading="idxLoading" class="cache-refresh" @click="loadIndex">
          刷新
        </el-button>
      </div>
      <div v-if="idxErr" class="cache-line err">{{ idxErr }}</div>
      <template v-else-if="idx">
        <div class="cache-line">
          图索引：
          <el-tag v-if="idx.graph_valid" size="small" type="success" effect="plain">
            已就绪 · {{ formatBytes(idx.graph_bytes) }}
          </el-tag>
          <el-tag v-else size="small" type="info" effect="plain">未生成</el-tag>
          <span class="idx-sep">支配树索引：</span>
          <el-tag v-if="idx.dom_valid" size="small" type="success" effect="plain">
            已就绪 · {{ formatBytes(idx.dom_bytes) }}
          </el-tag>
          <el-tag v-else size="small" type="info" effect="plain">未生成</el-tag>
        </div>
        <div v-if="idx.graph_path" class="cache-line">
          <span class="mono">{{ idx.graph_path }}</span>
        </div>
        <div class="cache-line hint">
          {{ idx.note }}<br />
          索引按源文件的路径 + 大小 + 修改时间校验，<b>源 dump 一旦改动自动失效</b>；
          超过 {{ idx.write_max_gb }} GB 的 dump 只读不写。删除后下次打开需重新解析。
        </div>
      </template>
      <div v-else class="cache-line hint">读取中…</div>
    </div>

    <!-- 后台分析（sidecar 独立进程）：主窗口全程可操作，可取消 -->
    <div class="cap-card">
      <div class="cap-title">
        后台分析
        <el-tag v-if="job" size="small" :type="jobTagType" effect="plain" class="ml4">
          {{ phaseText }}
        </el-tag>
        <el-button
          v-if="job && !job.finished"
          size="small"
          text
          type="danger"
          class="cache-clear"
          :loading="canceling"
          @click="onCancel"
        >
          取消
        </el-button>
        <el-button
          v-else
          size="small"
          text
          :loading="starting"
          class="cache-clear"
          @click="onStart"
        >
          {{ job ? '重新分析' : '开始分析' }}
        </el-button>
      </div>

      <template v-if="job">
        <div class="worker-bar">
          <el-progress
            :percentage="pct"
            :stroke-width="8"
            :status="progressStatus"
            :show-text="false"
          />
        </div>
        <div class="cache-line">
          {{ job.msg }}
          <span v-if="job.total" class="worker-count">
            · {{ formatInt(job.done) }} / {{ formatInt(job.total) }}
          </span>
          <span v-if="!job.finished && job.elapsed_ms" class="worker-count">
            · {{ (job.elapsed_ms / 1000).toFixed(1) }} s
          </span>
        </div>
        <div v-if="job.finished && job.ok" class="cache-line">
          对象图 <b>{{ formatInt(job.nodes || 0) }}</b> 个对象 /
          <b>{{ formatInt(job.edges || 0) }}</b> 条引用 · 耗时
          {{ (job.elapsed_ms / 1000).toFixed(1) }} s ·
          支配树 {{ domStateText }}
        </div>
        <div v-if="job.error" class="cache-line err">{{ job.error }}</div>
        <div class="cache-line hint">
          分析跑在<b>独立子进程</b>里：期间主窗口可以正常操作，点「取消」会直接结束该进程。
          产出的是本地索引，之后打开这份 dump 与各分析视图都直接命中索引（毫秒级）。
        </div>
      </template>
      <template v-else>
        <div class="cache-line hint">
          把「解析 + 建对象图{{ largeDump ? ' + 支配树' : '' }}」交给独立进程执行，
          主窗口不卡；进度实时显示在此，可随时取消。
        </div>
        <div v-if="largeDump" class="cache-line warn-line">
          这份堆转储有 {{ formatBytes(dumpSize) }}，体积已达 {{ formatBytes(threshold) }} 门槛：
          打开时<b>自动</b>在后台预热索引。当前未在分析（可能已有另一份 dump 在跑，避免内存翻倍），
          可手动点「开始分析」。
        </div>
        <div v-else class="cache-line hint">
          体积达到 {{ formatBytes(threshold) }} 会自动后台预热；这份较小，直接打开各视图即可（主进程几百毫秒）。
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { MatDump } from '../../types'
import {
  clearCache,
  clearIndex,
  fetchCacheInfo,
  fetchIndexInfo,
  formatBytes,
  formatInt,
} from '../../api/matApi'
import type { MatCacheInfo, MatIndexInfo, MatWorkerJob } from '../../api/matApi'
import { useMatStore } from '../../store'
import { workerPct, workerPhaseText } from '../../workerPhase'

/** 概览页：文件元信息 + 支持格式 + 能力清单 + 解压缓存 + 本地索引 + 后台分析 */
const props = defineProps<{
  dump: MatDump
}>()

const store = useMatStore()

const cache = ref<MatCacheInfo | null>(null)
const cacheErr = ref('')
const clearing = ref(false)

async function loadCache() {
  cacheErr.value = ''
  try {
    cache.value = await fetchCacheInfo()
  } catch (e: any) {
    cacheErr.value = e?.message || String(e)
  }
}

async function clearCacheNow() {
  try {
    await ElMessageBox.confirm(
      '将删除所有已解压的临时 HPROF。正在分析的 dump 若来自压缩包，其缓存文件会被占用而跳过，' +
        '下次打开该文件需要重新解压。确定清理？',
      '清理解压缓存',
      { type: 'warning', confirmButtonText: '清理', cancelButtonText: '取消' },
    )
  } catch {
    return
  }
  clearing.value = true
  try {
    const r = await clearCache()
    ElMessage.success(
      r.removed
        ? `已清理 ${r.removed} 个文件，释放 ${formatBytes(r.removed_bytes)}`
        : '没有可清理的文件',
    )
    if (r.skipped.length) {
      ElMessage.warning(`${r.skipped.length} 个文件仍被占用，重启应用后可清掉`)
    }
    await loadCache()
  } catch (e: any) {
    ElMessage.error(e?.message || String(e))
  } finally {
    clearing.value = false
  }
}

// ---- 本地索引 ----
const idx = ref<MatIndexInfo | null>(null)
const idxErr = ref('')
const idxLoading = ref(false)
const idxClearing = ref(false)

async function loadIndex() {
  idxErr.value = ''
  idxLoading.value = true
  try {
    idx.value = await fetchIndexInfo(props.dump.path)
  } catch (e: any) {
    idxErr.value = e?.message || String(e)
  } finally {
    idxLoading.value = false
  }
}

async function clearIdx() {
  try {
    await ElMessageBox.confirm(
      '删除后，下次打开该 dump 需要重新解析（大堆可能是分钟级）。确定删除本地索引？',
      '删除本地索引',
      { type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消' },
    )
  } catch {
    return
  }
  idxClearing.value = true
  try {
    const r = await clearIndex(props.dump.path)
    ElMessage.success(
      r.removed ? `已删除 ${r.removed} 个索引文件（${formatBytes(r.removed_bytes)}）` : '没有可删除的索引',
    )
    if (r.skipped.length) {
      ElMessage.warning(`${r.skipped.length} 个索引文件仍被占用，关闭该堆转储后可删除`)
    }
    await loadIndex()
  } catch (e: any) {
    ElMessage.error(e?.message || String(e))
  } finally {
    idxClearing.value = false
  }
}

// ---- 后台分析（sidecar） ----

/** 自动预热门槛（与 store 同源，别在两处写两份数字） */
const threshold = store.AUTO_WORKER_MIN_BYTES

const starting = ref(false)
const canceling = ref(false)

const job = computed<MatWorkerJob | null>(() => store.workerJobOf(props.dump.id))
/** 按**解压后**体积判断（`.gz` 看压缩包大小会漏判） */
const dumpSize = computed(() => store.dumpBytes(props.dump))
const largeDump = computed(() => dumpSize.value >= threshold)

/** 阶段文案与进度权重都在 `workerPhase.ts`（唯一真源，进度组件共用同一份） */
const phaseText = computed(() => workerPhaseText(job.value?.phase))
const pct = computed(() => workerPct(job.value))

const progressStatus = computed(() => {
  const j = job.value
  if (!j || !j.finished) return undefined
  if (j.ok) return 'success' as const
  if (j.canceled) return 'warning' as const
  return 'exception' as const
})

const jobTagType = computed(() => {
  const j = job.value
  if (!j) return 'info'
  if (!j.finished) return 'primary'
  if (j.ok) return 'success'
  if (j.canceled) return 'warning'
  return 'danger'
})

const domStateText = computed(() => {
  const st = job.value?.dom_state
  if (st === 'ready') return '已就绪'
  if (st === 'unavailable') return '已跳过（规模超闸或无 GC Roots，可在「分析预算」调高后重试）'
  if (st === 'skipped') return '未计算'
  return '—'
})

async function onStart() {
  starting.value = true
  try {
    const ok = await store.startWorker(props.dump.id, true)
    if (ok) ElMessage.success('后台分析已启动，期间可以正常操作')
  } finally {
    starting.value = false
  }
}

async function onCancel() {
  canceling.value = true
  try {
    await store.cancelWorkerOf(props.dump.id)
  } finally {
    canceling.value = false
  }
}

// 分析完成后索引刚落盘：刷新索引卡片，并把「二次打开秒开」这件事说明白
watch(
  () => job.value?.finished,
  (fin) => {
    if (fin && job.value?.ok) void loadIndex()
  },
)

onMounted(() => {
  void loadCache()
  void loadIndex()
  void store.restoreWorker(props.dump.id)
})
</script>

<style scoped lang="scss">
.ml4 { margin-left: 4px; }

.cache-clear { margin-left: auto; }
.cache-refresh { margin-left: 8px; }
.idx-sep { margin-left: 12px; }

.cache-line {
  font-size: 12px;
  line-height: 1.7;
  color: var(--el-text-color-regular);
  word-break: break-all;

  &.hint { color: var(--el-text-color-secondary); }
  &.err { color: var(--el-color-danger); }
}

.cache-warn { color: var(--el-color-warning); }

.worker-bar {
  margin: 2px 0 8px;
}

.worker-count {
  color: var(--el-text-color-secondary);
}

.warn-line {
  color: var(--el-color-warning);
}

.overview {
  height: 100%;
  overflow: auto;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.meta-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 8px;
  padding: 12px 16px;
  background: var(--el-fill-color-light);
  border-radius: var(--radius-md);
  align-content: start;
}

.meta-row {
  display: flex;
  align-items: baseline;
  gap: 12px;
  min-height: 24px;

  &.detail .value {
    word-break: break-all;
  }

  &.error .value {
    color: var(--stateErrorFg);
  }
}

.meta-row .label {
  flex-shrink: 0;
  width: 88px;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}

.meta-row .value {
  font-size: var(--font-size-base);
  color: var(--el-text-color-primary);
  min-width: 0;
}

.mono {
  font-family: var(--font-family-mono);
}

.path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cap-card {
  padding: 12px 16px;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: var(--radius-md);
}

.cap-title {
  display: flex;
  align-items: center;
  font-size: var(--font-size-md);
  font-weight: 600;
  color: var(--el-text-color-primary);
  margin-bottom: 8px;
}

.cap-list {
  margin: 0;
  padding-left: 20px;
  font-size: var(--font-size-base);
  line-height: 2;
  color: var(--el-text-color-secondary);
}
</style>
