<script setup lang="ts">
/**
 * files 跨文件搜索 / 批量替换面板（挂在 SidebarPanel 的「搜索」视图内）。
 *
 * 设计取舍：宿主 dock 是单槽位且已被 terminal 占用，新增 ViewSlot 要动 4 处宿主代码，
 * 因此 M2 复用 files 自己的 sidebar，用 v-show 切换「文件树 / 搜索」两个视图。
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Search, FolderOpened } from '@element-plus/icons-vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import NotificationUtil from '@/utils/NotificationUtil'
import useStore from '@/store'
import { useDocumentManagerStore } from '@/store/modules/documents'
import { useFileSearch } from './useFileSearch'
import type { FileHits, SearchHit } from './fileSearchApi'
import { revealLine } from '../navigation/editorEvents'
import { suggestRegex, describeReplaceImpact } from './regexWorkshop'
import type { RegexSuggestion } from './regexWorkshop'
import {
  clearUndoHistory,
  discardUndoBatch,
  formatBatchTime,
  restoreUndoBatch,
  undoBatches,
  undoHistoryVisible
} from './useUndoHistory'

const { fileTreeStore } = useStore()
const docStore = useDocumentManagerStore()
const s = useFileSearch()

// el-autocomplete 组件实例（暴露 focus()），不用具体 DOM 类型以免与 EP 版本签名打架
const keywordInput = ref<any>(null)

/** 查找历史下拉（Element Plus 要求 cb 收到 { value } 数组）。 */
function querySearchHistory(query: string, cb: (items: { value: string }[]) => void) {
  const q = query.toLowerCase()
  const items = s.history.value
    .filter((v) => !q || v.toLowerCase().includes(q))
    .slice(0, 20)
    .map((value) => ({ value }))
  cb(items)
}
const advancedOpen = ref(false)
const showPreviewDialog = ref(false)
const expandedFiles = ref<Set<string>>(new Set())

// ===== M4 离线正则工坊 + 替换影响面 =====
const helperOpen = ref(false)
const helperQuery = ref('')
const helperSuggestions = computed(() => suggestRegex(helperQuery.value))

function applySuggestion(item: RegexSuggestion) {
  s.keyword.value = item.pattern
  s.useRegex.value = true // 模板是正则，自动切到 regex 模式，避免被当字面量
  helperOpen.value = false
  NotificationUtil.info(`已填入「${item.label}」正则，可直接查找或继续改`)
}

const impactLines = computed(() =>
  describeReplaceImpact(s.preview.value, {
    dirtyOpenPaths: (s.preview.value?.items || [])
      .filter((i) => docStore.documents.get(i.path)?.isDirty)
      .map((i) => i.path),
    selectedOnly: s.selectedPaths.value.size > 0
  })
)

const summary = computed(() => {
  if (s.searching.value) return '搜索中…'
  if (s.errorMsg.value) return s.errorMsg.value
  if (!s.hasResults.value) return '无结果'
  return `${s.results.value.length} 个文件 / ${s.totalMatches.value} 处 · 扫描 ${s.filesScanned.value} · 跳过 ${s.filesSkipped.value}`
})

function syncRoot() {
  if (fileTreeStore.workingDirectory) s.setRoot(fileTreeStore.workingDirectory)
}

/** 用户显式选根：搜任意目录，不再被工作区切换覆盖（M4 收口 M3 遗留）。 */
async function pickRoot() {
  await s.chooseRoot()
}

async function restoreWorkspaceRoot() {
  await s.resetRoot(fileTreeStore.workingDirectory || '')
}

watch(() => fileTreeStore.workingDirectory, syncRoot)
onMounted(() => {
  syncRoot()
  document.addEventListener('keydown', onGlobalKeydown)
})
onBeforeUnmount(() => {
  document.removeEventListener('keydown', onGlobalKeydown)
  void s.cancel()
})

/** Ctrl/Cmd+Shift+F 拉起面板。终端聚焦时交给 terminal 自己的查找栏，避免双触发。 */
function onGlobalKeydown(e: KeyboardEvent) {
  if (!(e.ctrlKey || e.metaKey) || !e.shiftKey || e.key.toLowerCase() !== 'f') return
  const target = e.target as HTMLElement | null
  if (target?.closest?.('.xterm')) return
  e.preventDefault()
  s.openSearchPanel()
  void nextTick(() => keywordInput.value?.focus())
}

function onKeywordEnter() {
  void s.search()
}

function escapeHtml(v: string): string {
  return v.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

function escapeRegex(v: string): string {
  return v.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/**
 * 命中高亮在前端按同一份关键词规则重算，不用后端返回的 start/end：
 * 后端给的是 UTF-8 字节偏移，中文场景下直接当 JS 字符下标会错列。
 */
function highlight(line: string): string {
  if (!s.keyword.value) return escapeHtml(line)
  try {
    const pattern = s.mode.value === 'regex' ? s.keyword.value : escapeRegex(s.keyword.value)
    const re = new RegExp(pattern, s.ignoreCase.value ? 'gi' : 'g')
    let out = ''
    let last = 0
    for (const m of line.matchAll(re)) {
      const idx = m.index ?? 0
      out += escapeHtml(line.slice(last, idx))
      out += `<mark class="fs-hit">${escapeHtml(m[0])}</mark>`
      last = idx + m[0].length
      if (m[0].length === 0) break // 零宽正则保护
    }
    out += escapeHtml(line.slice(last))
    return out
  } catch {
    return escapeHtml(line)
  }
}

function toggleFile(path: string) {
  const next = new Set(expandedFiles.value)
  if (next.has(path)) next.delete(path)
  else next.add(path)
  expandedFiles.value = next
}

function isExpanded(path: string): boolean {
  // 默认全部展开，只有用户主动收起过的才折叠
  return !expandedFiles.value.has(path)
}

/** 点击结果行：打开文档并把光标放到命中行。 */
async function jump(file: FileHits, hit: SearchHit) {
  try {
    await docStore.openDocument(file.path)
    const line = hit.line
    const column = (hit.start || 0) + 1
    docStore.updateViewState(file.path, { cursor: { line, column } })
    await nextTick()
    // CodeEditor 与 >1MB 的 TextView 都订阅这个事件；后者支持跨块定位（M3）。
    // 首次打开时编辑器可能尚未挂载，补一次延时投递兜底，重复定位无副作用。
    revealLine(file.path, line, column)
    window.setTimeout(() => revealLine(file.path, line, column), 120)
  } catch (e: any) {
    console.error('[files/search] 打开命中文件失败', e)
    NotificationUtil.error(e?.message || '打开文件失败')
  }
}

async function openPreview() {
  if (!s.replacement.value) {
    NotificationUtil.warn('请先填写「替换为」内容')
    return
  }
  const ok = await s.previewReplace()
  if (ok) showPreviewDialog.value = true
}

async function confirmApply() {
  showPreviewDialog.value = false
  await s.applyReplace()
}

/** 应用替换后搜索结果会清空（mtime 基线已失效），此处保留一份结果报告与撤销入口。 */
const appliedReport = computed(() => {
  const p = s.preview.value
  return !!p && !p.dry_run
})

async function restoreBatch(id: string) {
  const ok = await restoreUndoBatch(id)
  if (ok) s.clearResults()
}

async function dropBatch(id: string) {
  await discardUndoBatch(id)
}

async function clearAll() {
  await clearUndoHistory()
}

const previewItems = computed(() =>
  (s.preview.value?.items || []).filter((i) => i.replacements > 0)
)
const skippedItems = computed(() =>
  (s.preview.value?.items || []).filter((i) => i.code !== 200 && i.msg !== '无匹配')
)
</script>

<template>
  <div class="fs-panel">
    <!-- 查询条件 -->
    <div class="fs-form">
      <el-autocomplete
        ref="keywordInput"
        v-model="s.keyword.value"
        :fetch-suggestions="querySearchHistory"
        size="small"
        clearable
        placeholder="在文件中查找（可复用历史）"
        @keydown.enter="onKeywordEnter"
        @select="onKeywordEnter"
      >
        <template #prefix><el-icon><Search /></el-icon></template>
      </el-autocomplete>

      <el-input
        v-model="s.replacement.value"
        size="small"
        clearable
        placeholder="替换为（留空＝删除匹配）"
      />

      <div class="fs-options">
        <el-checkbox v-model="s.useRegex.value" size="small" title="正则表达式">.*</el-checkbox>
        <el-checkbox v-model="s.ignoreCase.value" size="small" title="忽略大小写">Aa</el-checkbox>
        <el-button
          size="small"
          link
          :type="advancedOpen ? 'primary' : 'default'"
          @click="advancedOpen = !advancedOpen"
        >过滤</el-button>
        <el-button
          size="small"
          link
          :type="helperOpen ? 'primary' : 'default'"
          title="用自然语言描述要找的内容，选一条本地正则模板"
          @click="helperOpen = !helperOpen"
        >正则助手</el-button>
      </div>

      <div v-if="advancedOpen" class="fs-advanced">
        <el-input v-model="s.include.value" size="small" placeholder="包含：*.ts, *.vue" />
        <el-input v-model="s.exclude.value" size="small" placeholder="排除：*.min.js, dist/**" />
        <div class="fs-root" :title="s.rootPath.value || '未选择搜索根目录'">
          <el-icon><FolderOpened /></el-icon>
          <span class="fs-root-path">{{ s.rootPath.value || '未选择搜索根目录' }}</span>
          <span class="fs-root-source">{{ s.rootSource.value === 'custom' ? '自定义' : '工作区' }}</span>
        </div>
        <div class="fs-root-actions">
          <app-button size="small" @click="pickRoot">选目录…</app-button>
          <app-button v-if="s.rootSource.value === 'custom'" size="small" text @click="restoreWorkspaceRoot">
            跟随工作区
          </app-button>
        </div>
      </div>

      <!-- M4：离线正则工坊。没有 LLM 通道，用本地规则模板，UI 契约与将来接模型时一致 -->
      <div v-if="helperOpen" class="fs-helper">
        <div class="fs-helper-head">
          <el-input
            v-model="helperQuery"
            size="small"
            clearable
            placeholder="描述要找什么：邮箱 / IP / 日期 / 空行 / 引号"
          />
          <span class="fs-helper-hint">离线规则模板 · 点击填入</span>
        </div>
        <div class="fs-helper-list">
          <div
            v-for="item in helperSuggestions"
            :key="item.id"
            class="fs-helper-item"
            :title="item.explanation"
            @click="applySuggestion(item)"
          >
            <div class="fs-helper-item-top">
              <b>{{ item.label }}</b><code>{{ item.pattern }}</code>
            </div>
            <div class="fs-helper-item-desc">{{ item.explanation }}</div>
          </div>
        </div>
      </div>

      <div class="fs-actions">
        <app-button size="small" type="primary" :loading="s.searching.value" @click="s.search()">
          查找
        </app-button>
        <app-button v-if="s.busy.value" size="small" @click="s.cancel()">取消</app-button>
        <app-button
          v-if="s.hasResults.value"
          size="small"
          :disabled="s.busy.value"
          @click="openPreview"
        >替换预览</app-button>
        <app-button v-if="s.canUndo.value" size="small" @click="s.undo()">撤销最近一次</app-button>
        <app-button v-if="undoBatches.length" size="small" text @click="undoHistoryVisible = true">
          撤销历史 ({{ undoBatches.length }})
        </app-button>
      </div>
      <div class="fs-summary" :title="s.errorMsg.value">{{ summary }}</div>
    </div>

    <!-- 结果列表 -->
    <div class="fs-results">
      <div v-if="!s.hasResults.value && !appliedReport" class="fs-empty">
        {{ s.searching.value ? '正在扫描文件…' : '输入关键词并回车开始搜索' }}
      </div>
      <div v-else-if="!s.hasResults.value" class="fs-apply-report">
        <p>
          已替换 <b>{{ s.preview.value?.total_replacements ?? 0 }}</b> 处 /
          <b>{{ s.preview.value?.files_changed ?? 0 }}</b> 个文件
        </p>
        <div v-if="skippedItems.length" class="fs-skipped-list">
          <div>跳过 {{ skippedItems.length }} 个文件（未覆盖）：</div>
          <div
            v-for="item in skippedItems"
            :key="item.path"
            class="fs-skipped-item"
            :title="item.msg"
          >{{ item.rel_path }} · {{ item.msg }}</div>
        </div>
        <app-button v-if="s.canUndo.value" size="small" @click="s.undo()">撤销最近一次写盘</app-button>
      </div>
      <template v-else>
        <div
          v-for="file in s.results.value"
          :key="file.path"
          class="fs-file"
        >
          <div class="fs-file-head" :title="file.path" @click="toggleFile(file.path)">
            <el-checkbox
              size="small"
              :model-value="s.selectedPaths.value.has(file.path)"
              :title="s.selectedPaths.value.size ? '勾选后仅替换选中文件' : '未勾选＝全部文件'"
              @click.stop
              @change="s.toggleSelected(file.path)"
            />
            <span class="fs-file-name">{{ file.rel_path }}</span>
            <span class="fs-file-count">{{ file.hits.length }}</span>
          </div>
          <div v-if="isExpanded(file.path)" class="fs-hits">
            <div
              v-for="hit in file.hits"
              :key="`${file.path}:${hit.line}:${hit.start}`"
              class="fs-hit-row"
              :title="hit.text"
              @click="jump(file, hit)"
            >
              <span class="fs-line-no">{{ hit.line }}</span>
              <span class="fs-line-text" v-html="highlight(hit.text)" />
            </div>
          </div>
        </div>
      </template>
    </div>

    <!-- 替换预览：必须显式确认才落盘 -->
    <GlobalDialog v-model="showPreviewDialog" title="批量替换预览" width="640px">
      <div class="fs-preview">
        <p class="fs-preview-summary">
          将替换 <b>{{ s.preview.value?.total_replacements ?? 0 }}</b> 处，影响
          <b>{{ previewItems.length }}</b> 个文件（{{ s.selectedPaths.value.size ? '仅勾选文件' : '全部命中文件' }}）。
          写回前会为每个文件生成 <code>.bdt-replace-*.bak</code> 备份，可用「撤销替换」还原。
        </p>
        <p class="fs-preview-tip">
          文件按打开时的原编码 / 换行符 / BOM 回写；若文件已被外部修改（409）会跳过而不覆盖。
          已打开且<b>干净</b>的文档会自动重新加载，<b>有未保存改动</b>的文档不会覆盖，保存时会弹出冲突处理。
        </p>
        <div class="fs-impact">
          <div class="fs-impact-title">影响面解读</div>
          <div
            v-for="(line, idx) in impactLines"
            :key="idx"
            class="fs-impact-line"
            :class="`is-${line.level}`"
          >
            <span class="fs-impact-dot" />{{ line.text }}
          </div>
        </div>
        <div v-for="item in previewItems" :key="item.path" class="fs-preview-file">
          <div class="fs-preview-file-head">{{ item.rel_path }} · {{ item.replacements }} 处 · {{ item.encoding }} / {{ item.eol }}</div>
          <div v-for="sample in item.samples" :key="sample.line" class="fs-preview-sample">
            <div class="fs-diff before"><span>- {{ sample.line }}</span>{{ sample.before }}</div>
            <div class="fs-diff after"><span>+ {{ sample.line }}</span>{{ sample.after }}</div>
          </div>
        </div>
        <div v-if="skippedItems.length" class="fs-preview-skipped">
          跳过 {{ skippedItems.length }} 个文件：{{ skippedItems[0].rel_path }}（{{ skippedItems[0].msg }}）
        </div>
      </div>
      <template #footer>
        <app-button size="small" @click="showPreviewDialog = false">取消</app-button>
        <app-button size="small" type="primary" :loading="s.replacing.value" @click="confirmApply">
          应用替换
        </app-button>
      </template>
    </GlobalDialog>

    <!-- M4：落盘的撤销清单。批量替换与编码转换共用，重启后备份仍存在的条目才可恢复 -->
    <GlobalDialog v-model="undoHistoryVisible" title="写盘撤销历史" width="560px">
      <div class="fs-undo">
        <p class="fs-undo-hint">
          清单保存在应用配置中，重启后自动恢复；启动时会自动过滤掉备份文件已不存在的失效条目。
          恢复会按备份原始字节覆盖当前文件，请先确认目标文件没有新的未保存更改。
        </p>
        <div v-for="b in [...undoBatches].reverse()" :key="b.id" class="fs-undo-batch">
          <div class="fs-undo-head">
            <span class="fs-undo-label">{{ b.label }}</span>
            <span class="fs-undo-meta">{{ formatBatchTime(b.createdAt) }} · {{ b.entries.length }} 个文件</span>
          </div>
          <div class="fs-undo-actions">
            <app-button size="small" type="primary" @click="restoreBatch(b.id)">恢复</app-button>
            <app-button size="small" text @click="dropBatch(b.id)">丢弃备份</app-button>
          </div>
        </div>
        <div v-if="!undoBatches.length" class="fs-undo-empty">暂无可撤销的写盘操作。</div>
      </div>
      <template #footer>
        <app-button v-if="undoBatches.length" size="small" text @click="clearAll">清空全部</app-button>
        <app-button size="small" @click="undoHistoryVisible = false">关闭</app-button>
      </template>
    </GlobalDialog>
  </div>
</template>

<style scoped lang="scss">
.fs-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  font-family: var(--font-family-base);
  font-size: var(--font-size-xs);
}

.fs-form {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px;
  flex: none;
  border-bottom: 1px solid var(--el-border-color-lighter);
}

.fs-options {
  display: flex;
  align-items: center;
  gap: 4px;

  :deep(.el-checkbox) {
    margin-right: 4px;
  }
}

.fs-advanced {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.fs-root {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--el-text-color-secondary);
  overflow: hidden;

  .fs-root-path {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fs-root-source {
    flex: none;
    padding: 0 4px;
    border: 1px solid var(--borderColor, var(--el-border-color));
    border-radius: 3px;
    font-size: var(--fontSizeXs);
  }
}

.fs-root-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
}

.fs-undo {
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-size: var(--fontSizeSmall);
}
.fs-undo-hint {
  margin: 0;
  line-height: 1.6;
  color: var(--textSecondary);
}
.fs-undo-batch {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border: 1px solid var(--borderColor, var(--el-border-color));
  border-radius: 4px;
}
.fs-undo-head {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1 1 auto;
  min-width: 0;
}
.fs-undo-label {
  font-weight: 600;
  color: var(--textColor);
}
.fs-undo-meta {
  color: var(--textSecondary);
  font-size: var(--fontSizeXs);
}
.fs-undo-actions {
  flex: none;
  display: flex;
  gap: 4px;
}
.fs-undo-empty {
  color: var(--textSecondary);
  padding: 8px 0;
}

.fs-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.fs-summary {
  color: var(--el-text-color-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.fs-results {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.fs-empty {
  padding: 16px 10px;
  color: var(--el-text-color-secondary);
  text-align: center;
}

.fs-apply-report {
  padding: 10px;
  line-height: 1.6;

  p {
    margin: 0 0 8px;
  }
}

.fs-skipped-list {
  margin-bottom: 8px;
  color: var(--stateWarningFg, var(--el-color-warning));
  word-break: break-all;
}

.fs-skipped-item {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.fs-file-head {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 6px;
  cursor: pointer;
  background: var(--el-fill-color-light);
  border-bottom: 1px solid var(--el-border-color-lighter);

  :deep(.el-checkbox) {
    margin-right: 0;
  }
}

.fs-file-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--el-text-color-primary);
}

.fs-file-count {
  flex: none;
  padding: 0 4px;
  border-radius: 8px;
  background: var(--el-fill-color);
  color: var(--el-text-color-secondary);
}

.fs-hit-row {
  display: flex;
  gap: 6px;
  padding: 2px 6px 2px 10px;
  cursor: pointer;
  overflow: hidden;
  white-space: nowrap;

  &:hover {
    background: var(--el-fill-color);
  }
}

.fs-line-no {
  flex: none;
  min-width: 28px;
  text-align: right;
  color: var(--el-text-color-secondary);
  font-family: var(--font-family-mono);
}

.fs-line-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: var(--font-family-mono);

  :deep(.fs-hit) {
    background: var(--el-color-warning-light-5);
    color: var(--el-text-color-primary);
  }
}

.fs-preview {
  font-size: var(--font-size-base);
  line-height: 1.6;
}

.fs-preview-summary {
  margin: 0 0 6px;
}

.fs-preview-tip {
  margin: 0 0 10px;
  color: var(--el-text-color-secondary);
  font-size: var(--fontSizeSmall);
}

.fs-preview-file {
  margin-bottom: 10px;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.fs-preview-file-head {
  padding: 4px 8px;
  background: var(--el-fill-color-light);
  font-family: var(--font-family-mono);
  font-size: var(--fontSizeSmall);
}

.fs-preview-sample {
  padding: 2px 8px;
  font-family: var(--font-family-mono);
  font-size: var(--fontSizeSmall);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.fs-diff {
  span {
    display: inline-block;
    min-width: 34px;
    color: var(--el-text-color-secondary);
  }
}

.fs-diff.before {
  color: var(--el-color-danger);
}

.fs-diff.after {
  color: var(--el-color-success);
}

.fs-preview-skipped {
  color: var(--stateWarningFg, var(--el-color-warning));
  font-size: var(--fontSizeSmall);
}

/* ===== M4 离线正则工坊 / 影响面 ===== */
.fs-helper {
  margin-top: var(--space-2);
  border: 1px solid var(--borderColor);
  border-radius: var(--radius-sm, 4px);
  padding: var(--space-2);
}

.fs-helper-head {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.fs-helper-hint {
  flex: none;
  font-size: var(--fontSizeSmall);
  color: var(--el-text-color-secondary);
  white-space: nowrap;
}

.fs-helper-list {
  margin-top: var(--space-2);
  max-height: 190px;
  overflow-y: auto;
}

.fs-helper-item {
  padding: var(--space-1) var(--space-2);
  border-bottom: 1px solid var(--borderColor);
  cursor: pointer;

  &:hover {
    background: var(--el-fill-color-light);
  }
}

.fs-helper-item-top {
  display: flex;
  align-items: baseline;
  gap: var(--space-2);
  min-width: 0;
  font-size: var(--fontSizeSmall);

  b {
    flex: none;
    color: var(--el-text-color-primary);
  }

  code {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--stateInfoFg, var(--el-color-primary));
  }
}

.fs-helper-item-desc {
  font-size: var(--fontSizeSmall);
  color: var(--el-text-color-secondary);
  line-height: 1.5;
}

.fs-impact {
  margin: var(--space-2) 0 var(--space-3) 0;
  padding: var(--space-2);
  border: 1px solid var(--borderColor);
  border-radius: var(--radius-sm, 4px);
}

.fs-impact-title {
  margin-bottom: var(--space-1);
  font-size: var(--fontSizeSmall);
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.fs-impact-line {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  padding: 2px 0;
  font-size: var(--fontSizeSmall);
  line-height: 1.5;
  color: var(--el-text-color-regular);

  &.is-warn {
    color: var(--stateWarningFg, var(--el-color-warning));
  }

  &.is-danger {
    color: var(--el-color-danger);
  }
}

.fs-impact-dot {
  flex: none;
  width: 6px;
  height: 6px;
  margin-top: 6px;
  border-radius: 50%;
  background: currentColor;
}
</style>
