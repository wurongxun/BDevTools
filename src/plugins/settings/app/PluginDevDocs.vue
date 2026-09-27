<script setup lang="ts">
/**
 * 插件开发文档（设置 → 插件 → 插件开发）
 *
 * 文档本体是仓库里的 `docs/plugin-dev/*.md`：**唯一的真源**，应用内直接渲染，
 * 不再维护一份「应用里的文档」副本。构建期用 `import.meta.glob(..., '?raw')` 把原文
 * 打进包里（10 篇、约 100KB；比再发一次网络请求更省事，也让文档离线可用）。
 *
 * 三栏：左 = 文档导航 + 跨文档搜索，中 = markdown 正文，右 = 本文章节目录。
 *
 * 两个细节值得记一笔：
 *
 * 1. **标题定位不用 id，用「DOM 里的第 N 个 h*」**。markdown 标题与渲染出的 DOM 顺序
 *    一一对应，所以解析出的标题下标可以直接拿去
 *    `querySelectorAll('h1,h2,h3,h4,h5,h6')[index]`，不依赖任何 slug/锚点生成规则。
 *    跨文档锚点用「去掉所有非字母数字汉字后比较」来容错匹配（`#api-参考` ↔ `## API 参考`），
 *    精确匹配不到再退化成包含匹配（`#测试` ↔ `## ⑤ 测试`）。
 * 2. **所有链接点击都被拦下**。`.md` 走内部切换、`#xxx` 走同文档滚动，其余一律
 *    `preventDefault` —— 让 webview 去导航一个不存在的相对路径（比如 `../../examples/x.sh`）
 *    会把整个应用界面顶掉。
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { MdPreview } from 'md-editor-v3'
import 'md-editor-v3/lib/preview.css'
import { isDark } from '@/composables/useDark'

// ==================== 文档装载 ====================

/** 文档原文：`/docs/plugin-dev/<名>.md` → 内容 */
const RAW = import.meta.glob('/docs/plugin-dev/*.md', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>

/** 一篇文章的导航信息 */
interface DocMeta {
  /** 文件名（不含 .md），与 docs/plugin-dev/ 下的文件同名 */
  file: string
  /** 导航里显示的名字 */
  title: string
  /** 分组标题 */
  group: string
}

/** 导航清单：顺序即阅读顺序。新增文档要在这里登记（见 docs/plugin-dev/checklist.md） */
const DOCS: DocMeta[] = [
  { file: 'README', title: '总览与阅读路径', group: '开始' },
  { file: 'quickstart', title: '快速开始（15 分钟）', group: '开始' },
  { file: 'examples', title: '最小示例仓库', group: '开始' },
  { file: 'manifest', title: '清单字段表 plugin.json', group: '参考' },
  { file: 'capabilities', title: '能力边界与 API 参考', group: '参考' },
  { file: 'web3d', title: '3D 与游戏（WebGL / WASM 引擎）', group: '参考' },
  { file: 'bridge', title: '桥协议与生命周期', group: '参考' },
  { file: 'errors', title: '错误码与 FAQ', group: '参考' },
  { file: 'debugging', title: '调试指南', group: '工程' },
  { file: 'packaging', title: '打包与发布', group: '工程' },
  { file: 'checklist', title: '发布前核对清单', group: '工程' },
]

const GROUPS = ['开始', '参考', '工程']

/** 搜索命中上限（避免一篇文档把结果刷屏） */
const MAX_HITS = 80

/** 取某篇文档的正文（文件不存在时返回空串） */
function textOf(file: string): string {
  return RAW[`/docs/plugin-dev/${file}.md`] ?? ''
}

/** 导航分组，只保留实际存在的文档（防止 DOCS 里写了还没建的文件） */
const navGroups = computed(() =>
  GROUPS.map((group) => ({
    group,
    items: DOCS.filter((d) => d.group === group && textOf(d.file)),
  })).filter((g) => g.items.length > 0),
)

// ==================== 标题解析（目录 / 搜索 / 锚点共用） ====================

interface Heading {
  /** 在全文标题里的下标 —— 也是 DOM 里的下标 */
  index: number
  /** 标题所在的行号（从 0 起），搜索时用来判断某一行归属哪个标题 */
  line: number
  level: number
  text: string
}

/** 去掉标题里的行内标记，留下可读文本 */
function plainText(s: string): string {
  return s
    .replace(/`([^`]*)`/g, '$1')
    .replace(/\*\*([^*]*)\*\*/g, '$1')
    .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
    .trim()
}

/**
 * 扫描 markdown 标题与围栏区间。
 * 返回标题表 + 每行是否在 ``` 围栏内（搜索时要跳过代码块里的内容）。
 */
function scan(md: string): { headings: Heading[]; fenced: boolean[] } {
  const headings: Heading[] = []
  const fenced: boolean[] = []
  const lines = md.split('\n')
  let fence: string | null = null
  lines.forEach((raw, i) => {
    const line = raw.trimEnd()
    const fenceMatch = /^ {0,3}(`{3,}|~{3,})/.exec(line)
    if (fenceMatch) {
      const marker = fenceMatch[1][0]
      if (fence === null) fence = marker
      else if (fence === marker) fence = null
      fenced[i] = true
      return
    }
    fenced[i] = fence !== null
    if (fence !== null) return
    const m = /^(#{1,6})\s+(.*)$/.exec(line)
    if (!m) return
    headings.push({ index: headings.length, line: i, level: m[1].length, text: plainText(m[2]) })
  })
  return { headings, fenced }
}

/** 容错匹配用的归一键：只留字母/数字/汉字，去掉所有标点空格 */
function norm(s: string): string {
  return s.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, '')
}

/** decodeURIComponent 的安全版本（malformed 输入不抛异常） */
function safeDecode(s: string): string {
  try {
    return decodeURIComponent(s)
  } catch {
    return s
  }
}

/** 在标题表里找锚点对应的下标：先精确、再包含（容忍「⑤ 测试」这类前缀符号） */
function findByAnchor(headings: Heading[], anchor: string): number {
  const want = norm(safeDecode(anchor))
  if (!want) return -1
  const exact = headings.find((h) => norm(h.text) === want)
  if (exact) return exact.index
  const loose = headings.find((h) => norm(h.text).includes(want) || want.includes(norm(h.text)))
  return loose ? loose.index : -1
}

// ==================== 状态 ====================

const current = ref(DOCS[0].file)
const keyword = ref('')
const scrollRef = ref<HTMLElement | null>(null)
const activeHeading = ref(0)

const content = computed(() => textOf(current.value))
const headings = computed(() => scan(content.value).headings)

/** 右侧目录：跳过 h1（标题已在导航里），只列 h2~h4 */
const outline = computed(() => headings.value.filter((h) => h.level >= 2 && h.level <= 4))

const currentMeta = computed(() => DOCS.find((d) => d.file === current.value))
const isSearching = computed(() => keyword.value.trim().length > 0)

// ==================== 跨文档搜索 ====================

interface Hit {
  file: string
  docTitle: string
  /** 命中行归属的标题下标；-1 表示在第一个标题之前 */
  headingIndex: number
  headingText: string
  /** 命中的那一行（去掉行首标记、截断） */
  line: string
}

const hits = computed<Hit[]>(() => {
  const kw = keyword.value.trim().toLowerCase()
  if (!kw) return []
  const out: Hit[] = []
  for (const doc of DOCS) {
    const md = textOf(doc.file)
    if (!md) continue
    const { headings: hs, fenced } = scan(md)
    md.split('\n').forEach((raw, i) => {
      if (out.length >= MAX_HITS || fenced[i]) return
      const line = raw.trim()
      if (!line || !line.toLowerCase().includes(kw)) return
      // 命中行归属「行号不超过它的最后一个标题」
      let owner: Heading | undefined
      for (const h of hs) {
        if (h.line <= i) owner = h
        else break
      }
      out.push({
        file: doc.file,
        docTitle: doc.title,
        headingIndex: owner ? owner.index : -1,
        headingText: owner ? owner.text : doc.title,
        line: line.replace(/^[-*>+]\s+/, '').slice(0, 160),
      })
    })
    if (out.length >= MAX_HITS) break
  }
  return out
})

const hitCount = computed(() => hits.value.length)

// ==================== 跳转 ====================

/**
 * 滚到正文里的第 index 个标题。
 * 手算偏移而不用 scrollIntoView：后者会连带滚动外层容器（设置页、插件主面板），
 * 表现为「点目录后整页跳了一下」。
 */
async function scrollToHeading(index: number) {
  await nextTick()
  const box = scrollRef.value
  if (!box || index < 0) return
  const el = box.querySelectorAll('h1, h2, h3, h4, h5, h6')[index]
  if (!el) return
  const delta = el.getBoundingClientRect().top - box.getBoundingClientRect().top
  box.scrollTop += delta - 8
  activeHeading.value = index
}

/** 打开某篇文档；anchor 为跨文档锚点（不含 #） */
async function openDoc(file: string, anchor?: string) {
  keyword.value = ''
  const changed = file !== current.value
  if (changed) current.value = file
  await nextTick()
  if (anchor) {
    const idx = findByAnchor(headings.value, anchor)
    if (idx >= 0) {
      await scrollToHeading(idx)
      return
    }
  }
  if (changed && scrollRef.value) scrollRef.value.scrollTop = 0
}

/** 点搜索结果：切到那篇文档并滚到命中所在的章节 */
async function onHitClick(hit: Hit) {
  keyword.value = ''
  const changed = hit.file !== current.value
  if (changed) current.value = hit.file
  await nextTick()
  if (hit.headingIndex >= 0) {
    await scrollToHeading(hit.headingIndex)
  } else if (scrollRef.value) {
    scrollRef.value.scrollTop = 0
  }
}

// ==================== 链接拦截 ====================

/** 从链接 href 解析「文档名 + 锚点」；不是站内文档链接则返回 null */
function parseDocLink(href: string): { file: string; anchor?: string } | null {
  const clean = href.replace(/^\.\//, '').replace(/^\/+/, '')
  const [path, anchor] = clean.split('#')
  if (!path || !path.endsWith('.md')) return null
  const file = safeDecode(path.slice(0, -3))
  return DOCS.some((d) => d.file === file) ? { file, anchor } : null
}

function onPreviewClick(e: MouseEvent) {
  const a = (e.target as HTMLElement | null)?.closest?.('a') as HTMLAnchorElement | null
  if (!a) return
  const href = a.getAttribute('href') ?? ''
  // 一律拦下：webview 导航到不存在的相对路径会把应用界面顶掉
  e.preventDefault()
  e.stopPropagation()

  if (href.startsWith('#')) {
    const idx = findByAnchor(headings.value, href.slice(1))
    if (idx >= 0) scrollToHeading(idx)
    return
  }
  const link = parseDocLink(href)
  if (link) {
    openDoc(link.file, link.anchor)
    return
  }
  // 指向仓库文件的路径（如 examples/…/verify.sh）：应用内打不开，留个线索给开发者
  console.warn('[插件开发文档] 站内无法跳转的链接：', href)
}

// ==================== 滚动时高亮当前章节 ====================

function onScroll() {
  const box = scrollRef.value
  if (!box) return
  const els = box.querySelectorAll('h1, h2, h3, h4, h5, h6')
  const line = box.getBoundingClientRect().top + 24
  let active = 0
  els.forEach((el, i) => {
    if (el.getBoundingClientRect().top <= line) active = i
  })
  activeHeading.value = active
}

onMounted(() => {
  scrollRef.value?.addEventListener('scroll', onScroll, { passive: true })
  nextTick(onScroll)
})

onBeforeUnmount(() => {
  scrollRef.value?.removeEventListener('scroll', onScroll)
})

// 切文档后重算当前章节
watch(current, () => nextTick(onScroll))
</script>

<template>
  <div class="pdd">
    <!-- 左：文档导航 + 搜索 -->
    <aside class="pdd-nav">
      <div class="pdd-search">
        <input
          v-model="keyword"
          class="pdd-search-input"
          type="search"
          placeholder="搜索全部文档…"
          aria-label="搜索插件开发文档"
        />
        <span v-if="isSearching" class="pdd-search-count">
          {{ hitCount }} 条{{ hitCount >= MAX_HITS ? '（已截断）' : '' }}
        </span>
      </div>

      <!-- 搜索结果 -->
      <div v-if="isSearching" class="pdd-nav-list">
        <button
          v-for="(hit, i) in hits"
          :key="`${hit.file}-${i}`"
          type="button"
          class="pdd-hit"
          @click="onHitClick(hit)"
        >
          <span class="pdd-hit-where">
            {{ hit.docTitle }}
            <template v-if="hit.headingText !== hit.docTitle"> › {{ hit.headingText }}</template>
          </span>
          <span class="pdd-hit-line">{{ hit.line }}</span>
        </button>
        <div v-if="!hits.length" class="pdd-empty">没有匹配「{{ keyword }}」的内容</div>
      </div>

      <!-- 文档列表 -->
      <div v-else class="pdd-nav-list">
        <template v-for="g in navGroups" :key="g.group">
          <div class="pdd-nav-group">{{ g.group }}</div>
          <button
            v-for="doc in g.items"
            :key="doc.file"
            type="button"
            class="pdd-nav-item"
            :class="{ 'is-active': doc.file === current }"
            @click="openDoc(doc.file)"
          >
            {{ doc.title }}
          </button>
        </template>
      </div>
    </aside>

    <!-- 中：正文 -->
    <div class="pdd-main">
      <div class="pdd-head">
        <span class="pdd-head-title">{{ currentMeta?.title }}</span>
        <span class="pdd-head-file">docs/plugin-dev/{{ current }}.md</span>
      </div>
      <!-- 在容器上统一拦链接；MdPreview 只负责渲染 -->
      <div ref="scrollRef" class="pdd-body" @click="onPreviewClick">
        <MdPreview
          id="plugin-dev-doc"
          :model-value="content"
          :theme="isDark ? 'dark' : 'light'"
          preview-theme="github"
          :code-theme="isDark ? 'atom' : 'github'"
          :no-mermaid="true"
          :no-katex="true"
        />
      </div>
    </div>

    <!-- 右：本文章节目录 -->
    <nav class="pdd-outline" aria-label="本页目录">
      <div class="pdd-outline-title">本页目录</div>
      <div class="pdd-outline-list">
        <button
          v-for="h in outline"
          :key="h.index"
          type="button"
          class="pdd-outline-item"
          :class="[`lv${h.level}`, { 'is-active': h.index === activeHeading }]"
          :title="h.text"
          @click="scrollToHeading(h.index)"
        >
          {{ h.text }}
        </button>
        <div v-if="!outline.length" class="pdd-empty">（本文没有分节标题）</div>
      </div>
    </nav>
  </div>
</template>

<style lang="scss" scoped>
@use '@/styles/theme' as theme;

.pdd {
  display: flex;
  width: 100%;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: var(--radius-md);

  @include theme.useTheme {
    background: theme.getVar('bgColor');
  }
}

/* ---------- 左：导航 ---------- */
.pdd-nav {
  flex: none;
  width: 232px;
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-right: 1px solid var(--el-border-color-lighter);
}

.pdd-search {
  flex: none;
  padding: var(--space-2);
}

.pdd-search-input {
  width: 100%;
  height: 28px;
  padding: 0 var(--space-2);
  border: 1px solid var(--el-border-color);
  border-radius: var(--radius-sm);
  font-family: inherit;
  font-size: var(--font-size-base);
  outline: none;

  @include theme.useTheme {
    background: theme.getVar('bgColor');
    color: theme.getVar('textColor');
  }

  &:focus {
    @include theme.useTheme {
      border-color: theme.getVar('focusRing');
    }
  }
}

.pdd-search-count {
  display: block;
  padding-top: var(--space-1);
  font-size: var(--font-size-xs);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.pdd-nav-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 0 var(--space-2) var(--space-3);
}

.pdd-nav-group {
  padding: var(--space-2) var(--space-1) var(--space-1);
  font-size: var(--font-size-xs);
  font-weight: 600;
  letter-spacing: 0.04em;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.pdd-nav-item {
  display: block;
  width: 100%;
  padding: var(--space-1) var(--space-2);
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  font-family: inherit;
  font-size: var(--font-size-base);
  text-align: left;
  cursor: pointer;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }

  &:hover {
    @include theme.useTheme {
      background: theme.getVar('hoverColor');
    }
  }

  &.is-active {
    font-weight: 600;

    @include theme.useTheme {
      background: theme.getVar('hoverColor');
      color: theme.getVar('activityBarActive');
    }
  }
}

.pdd-hit {
  display: block;
  width: 100%;
  padding: var(--space-2);
  margin-bottom: var(--space-1);
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  font-family: inherit;
  text-align: left;
  cursor: pointer;

  &:hover {
    @include theme.useTheme {
      background: theme.getVar('hoverColor');
    }
  }
}

.pdd-hit-where {
  display: block;
  font-size: var(--font-size-xs);
  font-weight: 600;

  @include theme.useTheme {
    color: theme.getVar('activityBarActive');
  }
}

.pdd-hit-line {
  display: block;
  margin-top: 2px;
  font-size: var(--font-size-sm);
  line-height: 1.5;
  word-break: break-all;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.pdd-empty {
  padding: var(--space-3) var(--space-2);
  font-size: var(--font-size-sm);

  @include theme.useTheme {
    color: theme.getVar('textDisabled');
  }
}

/* ---------- 中：正文 ---------- */
.pdd-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.pdd-head {
  flex: none;
  display: flex;
  align-items: baseline;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--el-border-color-lighter);
}

.pdd-head-title {
  font-size: var(--font-size-md);
  font-weight: 600;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.pdd-head-file {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.pdd-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 0 var(--space-4);
  /* 超长单词/路径不要撑破布局 */
  overflow-wrap: break-word;
}

/* ---------- 右：本页目录 ---------- */
.pdd-outline {
  flex: none;
  width: 190px;
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-left: 1px solid var(--el-border-color-lighter);
}

.pdd-outline-title {
  flex: none;
  padding: var(--space-2) var(--space-2) var(--space-1);
  font-size: var(--font-size-xs);
  font-weight: 600;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.pdd-outline-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 0 var(--space-1) var(--space-3);
}

.pdd-outline-item {
  display: -webkit-box;
  width: 100%;
  padding: 3px var(--space-1);
  border: 0;
  border-left: 2px solid transparent;
  background: transparent;
  font-family: inherit;
  font-size: var(--font-size-sm);
  line-height: 1.5;
  text-align: left;
  cursor: pointer;
  /* 标题可能很长：两行截断 */
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &.lv3 {
    padding-left: var(--space-3);
  }

  &.lv4 {
    padding-left: var(--space-5);
  }

  &:hover {
    @include theme.useTheme {
      color: theme.getVar('textColor');
    }
  }

  &.is-active {
    font-weight: 600;

    @include theme.useTheme {
      border-left-color: theme.getVar('focusRing');
      color: theme.getVar('textColor');
    }
  }
}
</style>

<!--
  非 scoped：md-editor 的内部节点上没有本组件的 data-v，scoped 规则到不了。
  统一用 .pdd-body 作业务前缀限定作用范围（项目约定）。
-->
<style lang="scss">
.pdd-body {
  /* md-editor 自身是「填满父容器 + 内部滚动」的编辑器布局；这里改成「自适应高度」，
     让滚动统一由 .pdd-body 承担，避免出现嵌套滚动条、目录定位偏移 */
  .md-editor {
    height: auto;
    overflow: visible;
    background: transparent;
    border: 0;
  }

  .md-editor-preview-wrapper {
    padding: 0;
    overflow: visible;
  }

  .md-editor-preview {
    padding: var(--space-4) 0 40px;
    font-size: var(--font-size-base);
    line-height: 1.7;
  }

  /* 标题留出滚动定位余量，避免被容器顶边裁掉 */
  .md-editor-preview h1,
  .md-editor-preview h2,
  .md-editor-preview h3,
  .md-editor-preview h4 {
    scroll-margin-top: var(--space-2);
  }

  .md-editor-preview h1 {
    margin: 0 0 var(--space-3);
    padding-bottom: var(--space-2);
    font-size: var(--font-size-xl);
    border-bottom: 1px solid var(--el-border-color-lighter);
  }

  .md-editor-preview h2 {
    margin: var(--space-6) 0 var(--space-2);
    font-size: var(--font-size-lg);
  }

  .md-editor-preview h3 {
    margin: var(--space-4) 0 var(--space-2);
    font-size: var(--font-size-md);
  }

  /* 表格是第一屏信息量最大的东西：给它横向滚动，别撑破三栏布局 */
  .md-editor-preview table {
    display: block;
    width: max-content;
    max-width: 100%;
    overflow-x: auto;
  }

  .md-editor-preview pre,
  .md-editor-preview code {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-sm);
  }

  .md-editor-preview pre {
    overflow-x: auto;
  }

  .md-editor-preview blockquote {
    margin-left: 0;
    padding-left: var(--space-3);
    border-left: 3px solid var(--el-color-primary);
  }
}
</style>
