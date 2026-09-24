<template>
  <!--
    终端停靠面板（底部）
    由 Layout 渲染在 editor-wrapper 下方，与主区并存；活动栏图标是显隐开关。
    面板自身的显隐/高度由 layout store 控制，这里只负责内容。
  -->
  <div class="term-panel">
    <!-- 工具栏：标签条 + 操作区 -->
    <div class="term-toolbar">
      <TabBarContainer v-if="store.tabs.length" class="term-tabs"  aria-label="终端会话">
        <TabBarItem
          v-for="t in store.tabs"
          :key="t.id"
          :active="t.id === store.activeTabId"
          :title="tabTooltip(t)"
          @click="store.setActiveTab(t.id)"
          @close="store.closeTab(t.id)"
        >
          <span class="term-tab-dot" :class="dotClass(t)" aria-hidden="true"></span>
          <span class="term-tab-title">{{ t.title }}</span>
          <TabCloseButton @close="store.closeTab(t.id)" />
        </TabBarItem>
      </TabBarContainer>
      <div v-else class="term-tabs term-tabs--empty">终端</div>

      <div class="term-actions">
        <!-- 当前工作目录（OSC 7 跟踪）；shell 集成不可用时为空 -->
        <span v-if="activeCwd" class="term-cwd" :title="activeCwd">{{ activeCwd }}</span>

        <!-- 上次命令结果（shell 集成可用时才有值） -->
        <span v-if="activePane?.lastCommandOk === false" class="term-badge term-badge--fail" title="上一条命令返回非 0">
          退出码 {{ activePane.exitCode ?? '非 0' }}
        </span>

        <el-dropdown trigger="click" placement="top-start" popper-class="term-shell-menu" @command="onNewTab">
          <app-button bare :icon="Plus" title="新建终端会话（选择 shell）" aria-label="新建终端会话" />
          <template #dropdown>
            <el-dropdown-menu>
              <!-- 只读提示：新建会话会落在哪儿（工作区目录 → 系统默认目录） -->
              <el-dropdown-item disabled>
                <span class="term-menu-label">工作目录</span>
                <span class="term-menu-hint">{{ store.workspaceCwd ? '工作区' : '系统默认' }}</span>
                <span class="term-menu-hint term-menu-hint--path" :title="store.newSessionCwd">
                  {{ store.newSessionCwd || '—' }}
                </span>
              </el-dropdown-item>
              <el-dropdown-item command="" divided>
                <span class="term-menu-label">默认 shell</span>
                <span class="term-menu-hint" title="自动探测到的系统默认 shell">{{ defaultShellHint }}</span>
              </el-dropdown-item>
              <el-dropdown-item v-for="s in shells" :key="s.path" :command="s.path" divided>
                <span class="term-menu-label">
                  {{ s.label }}
                  <em v-if="s.isDefault" class="term-menu-tag">系统默认</em>
                </span>
                <span class="term-menu-hint" :title="integrationTitle(s.integration)">
                  {{ integrationLabel(s.integration) }}
                </span>
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>

        <app-button bare :icon="Right" :disabled="!canSplit" title="向右分屏" aria-label="向右分屏" @click="store.splitPane(store.activePaneId, 'row')" />
        <app-button bare :icon="Bottom" :disabled="!canSplit" title="向下分屏" aria-label="向下分屏" @click="store.splitPane(store.activePaneId, 'column')" />

        <span class="term-sep" aria-hidden="true"></span>

        <app-button bare :icon="Search" :disabled="!activePaneId" :class="{ active: store.searchOpen }" title="查找（Ctrl+Shift+F）" aria-label="在终端中查找" @click="toggleSearch" />
        <app-button bare :icon="VideoPlay" :disabled="!activePaneId || activePane?.exited" title="快速执行命令（异步 + 超时）" aria-label="快速执行命令" @click="store.execOpen = true" />
        <app-button bare :icon="Delete" :disabled="!activePaneId" title="清屏" aria-label="清屏" @click="clearActive" />
        <app-button bare :icon="Setting" :class="{ active: store.settingsOpen }" title="终端外观设置" aria-label="终端外观设置" @click="store.settingsOpen = true" />
      </div>
    </div>

    <!-- 查找栏 -->
    <div v-if="store.searchOpen" class="term-search">
      <input
        ref="searchInputRef"
        v-model="store.searchQuery"
        class="term-search-input"
        type="text"
        placeholder="在终端缓冲区中查找…"
        aria-label="查找内容"
        spellcheck="false"
        @keydown.enter.prevent="runSearch($event.shiftKey ? 'prev' : 'next')"
        @keydown.esc.prevent="closeSearch"
      />
      <label class="term-search-toggle" title="区分大小写">
        <input v-model="store.searchCaseSensitive" type="checkbox" @change="runSearch('next')" />
        Aa
      </label>
      <label class="term-search-toggle" title="全词匹配">
        <input v-model="store.searchWholeWord" type="checkbox" @change="runSearch('next')" />
        ab
      </label>
      <label class="term-search-toggle" title="正则表达式">
        <input v-model="store.searchRegex" type="checkbox" @change="runSearch('next')" />
        .*
      </label>
      <span class="term-search-info">{{ store.searchInfo }}</span>
      <app-button size="small" text @click="runSearch('prev')">上一个</app-button>
      <app-button size="small" text @click="runSearch('next')">下一个</app-button>
      <app-button bare :icon="Close" title="关闭查找（Esc）" aria-label="关闭查找" @click="closeSearch" />
    </div>

    <!-- 会话区：所有标签都保持挂载（PTY 会话与 xterm 实例都不重建），只切换显示 -->
    <div class="term-body">
      <div
        v-for="t in store.tabs"
        v-show="t.id === store.activeTabId"
        :key="t.id"
        class="term-tab-pane"
      >
        <PaneLayout :node="t.root" :tab-id="t.id" />
      </div>

      <div v-if="!store.tabs.length" class="term-empty">
        <p class="term-empty-title">尚未打开终端</p>
        <p class="term-empty-hint">
          {{ initError || '新建一个本地终端会话（跨平台 shell，走系统 PTY）' }}
        </p>
        <app-button size="small" type="primary" :icon="Plus" @click="store.newTab()">新建会话</app-button>
      </div>
    </div>

    <TerminalSettingsDialog />
    <TerminalExecDialog />
  </div>
</template>

<script setup lang="ts" name="TerminalPanel">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { Bottom, Close, Delete, Plus, Right, Search, Setting, VideoPlay } from '@element-plus/icons-vue'
import { TabBarContainer, TabBarItem, TabCloseButton } from '@/components/TabBar'
import '@xterm/xterm/css/xterm.css'
import PaneLayout from './PaneLayout.vue'
import TerminalSettingsDialog from './TerminalSettingsDialog.vue'
import TerminalExecDialog from './TerminalExecDialog.vue'
import { useTerminalStore } from '../store'
import { leafCount } from '../tree'
import type { IntegrationLevel, TabState } from '../types'

const store = useTerminalStore()

const searchInputRef = ref<HTMLInputElement | null>(null)

const activePaneId = computed(() => store.activePaneId)
const activePane = computed(() => store.activePane)
const activeCwd = computed(() => activePane.value?.cwd ?? '')
const shells = computed(() => store.shells)
const defaults = computed(() => store.defaults)
const initError = computed(() => store.initError)
const canSplit = computed(() => Boolean(activePaneId.value) && !!store.activeTab && leafCount(store.activeTab.root) < 8)

/** 菜单里「默认 shell」项右侧说明：系统默认是哪个 + 它能达到的集成程度 */
const defaultShellHint = computed(() => {
  const info = shells.value.find((s) => s.isDefault) ?? null
  if (!info) return defaults.value?.shellKind || '—'
  const level = integrationLabel(info.integration)
  return level ? `${info.label} · ${level}` : info.label
})

/**
 * 集成程度 → 菜单文案。
 * `none` 必须说成「无集成」：早先后端只给布尔量 `fullIntegration`、前端按
 * 「非 full 即部分」渲染，把完全没有注入的 WSL 也说成了「部分集成」。
 * 字段缺失（后端二进制还没重新编译）时返回空串——宁可不显示，也不要一律显示成「无集成」。
 */
function integrationLabel(level: IntegrationLevel | undefined): string {
  if (level === 'full') return '完整集成'
  if (level === 'partial') return '部分集成'
  if (level === 'none') return '无集成'
  return ''
}

/** 集成程度的具体含义（菜单项悬停查看） */
function integrationTitle(level: IntegrationLevel | undefined): string {
  if (level === 'full') return '命令边界 + 退出码 + 工作目录跟踪'
  if (level === 'partial') return '能跟踪命令边界与工作目录，但拿不到退出码'
  if (level === 'none') return '未注入 shell 集成：拿不到退出码与工作目录，快速执行只能靠超时兜底'
  return ''
}

/** 标签上的状态点：运行中 / 已退出 / 命令失败 */
function dotClass(t: TabState): string {
  const pane = store.panes[t.activePaneId]
  if (!pane) return ''
  if (pane.exited) return 'is-exited'
  if (pane.busy) return 'is-busy'
  if (pane.lastCommandOk === false) return 'is-fail'
  return ''
}

function tabTooltip(t: TabState): string {
  const pane = store.panes[t.activePaneId]
  if (!pane) return t.title
  const parts = [pane.title]
  if (pane.cwd) parts.push(pane.cwd)
  if (pane.exited) parts.push(`已退出（${pane.exitCode ?? '未知'}）`)
  else if (pane.integration === 'none') parts.push('无 shell 集成：拿不到退出码/cwd')
  return parts.join('\n')
}

function onNewTab(shell: string) {
  store.newTab(shell ? { shell } : {})
}

function toggleSearch() {
  store.searchOpen = !store.searchOpen
}

function closeSearch() {
  store.searchOpen = false
  store.searchInfo = ''
  const s = store.getSession(store.activePaneId)
  s?.clearSearch()
  s?.focus()
}

/** 在激活窗格里执行查找 */
function runSearch(direction: 'next' | 'prev') {
  const s = store.getSession(store.activePaneId)
  if (!s) return
  const q = store.searchQuery
  if (!q) {
    s.clearSearch()
    store.searchInfo = ''
    return
  }
  const opts = {
    caseSensitive: store.searchCaseSensitive,
    regex: store.searchRegex,
    wholeWord: store.searchWholeWord,
  }
  const hit = direction === 'next' ? s.findNext(q, opts) : s.findPrevious(q, opts)
  if (!hit) store.searchInfo = '无匹配'
}

function clearActive() {
  store.getSession(store.activePaneId)?.clear()
}

/** 面板内的全局快捷键（不经过 xterm：面板获得焦点但未落到终端时也能用） */
function onKeydown(e: KeyboardEvent) {
  const mod = e.ctrlKey || e.metaKey
  if (mod && e.shiftKey && e.code === 'KeyF') {
    e.preventDefault()
    store.searchOpen = true
  } else if (mod && e.shiftKey && e.code === 'KeyT') {
    e.preventDefault()
    store.newTab()
  }
}

// 搜索栏打开后聚焦输入框
watch(
  () => store.searchOpen,
  (open) => {
    if (open) void Promise.resolve().then(() => searchInputRef.value?.focus())
  },
)

// 切换标签后让该标签下所有窗格重新适配（隐藏期间容器尺寸为 0，fit 会被跳过）
watch(
  () => store.activeTabId,
  () => {
    void store.refitActiveTab()
  },
)

onMounted(async () => {
  window.addEventListener('keydown', onKeydown)
  // 面板常驻挂载（Layout 用 v-show 控制显隐），这里完成一次性初始化：
  // 读配置 → 恢复面板高度/显隐 → 探测本机 shell。
  // 首个会话由 store 的「可见性变化」监听创建，所以应用启动时不会白拉一个 shell。
  await store.ensureInit()
  if (store.panelVisible) void store.ensureFirstTab()
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKeydown)
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.term-panel {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;
  box-sizing: border-box;

  // 面板根色板与其它主面板同源（ssh TerminalView / LogView / database MainPanel），
  // 共享 TabBar 的激活标签也用 appMainBgColor 「与下方内容连体」——两者必须一致，
  // 否则标签与面板之间会出现色差。终端画布（内容面）自己另有底色，见 TerminalPane.vue。
  @include theme.useTheme {
    background-color: theme.getVar('appMainBgColor');
  }
}

// ---------- 工具栏 ----------
.term-toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  flex-shrink: 0;
  padding: 0 var(--space-1);
  height: 28px;
  box-sizing: border-box;

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.term-tabs {
  flex: 1;
  min-width: 0;
}

.term-tabs--empty {
  font-size: var(--font-size-sm);
  padding-left: var(--space-2);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.term-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}

.term-sep {
  width: 1px;
  height: 14px;
  margin: 0 3px;

  @include theme.useTheme {
    background-color: theme.getVar('borderColor');
  }
}

.term-cwd {
  max-width: 320px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  direction: rtl; // 左侧超长时优先保留尾部（路径末端信息量更大）
  text-align: left;
  padding: 0 var(--space-1);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.term-badge {
  padding: 0 6px;
  border-radius: var(--radius-xs);
  font-size: var(--font-size-xs);
  line-height: 16px;
  flex-shrink: 0;

  &--fail {
    color: var(--el-color-danger);
    background-color: var(--el-color-danger-light-9);
  }
}

.term-tab-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
  margin-right: 5px;
  background-color: var(--el-color-success);
  opacity: 0.65;

  &.is-exited {
    background-color: var(--el-text-color-placeholder);
  }

  &.is-busy {
    background-color: var(--el-color-warning);
  }

  &.is-fail {
    background-color: var(--el-color-danger);
  }
}

.term-tab-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-size-sm);
  max-width: 180px;
}

:deep(.el-tab-bar-item),
.term-tab-title {
  min-width: 0;
}

// ---------- 查找栏 ----------
.term-search {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  flex-shrink: 0;
  height: 28px;
  padding: 0 var(--space-2);

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
    background-color: theme.getVar('sidebarBg');
  }
}

.term-search-input {
  width: 220px;
  height: 20px;
  padding: 0 6px;
  border-radius: var(--radius-xs);
  font-size: var(--font-size-sm);
  box-sizing: border-box;

  @include theme.useTheme {
    color: theme.getVar('textColor');
    background-color: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderStrong');
  }

  &:focus-visible {
    outline: 1px solid var(--focus-ring-color);
    outline-offset: 0;
  }
}

.term-search-toggle {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: var(--font-size-xs);
  font-family: var(--font-family-mono);
  cursor: pointer;
  user-select: none;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  input {
    margin: 0;
    cursor: pointer;
  }
}

.term-search-info {
  min-width: 70px;
  font-size: var(--font-size-xs);
  font-family: var(--font-family-mono);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

// ---------- 会话区 ----------
.term-body {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.term-tab-pane {
  position: absolute;
  inset: 0;
  display: flex;
  overflow: hidden;
}

.term-empty {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-1);
}

.term-empty-title {
  margin: 0;
  font-size: var(--font-size-base);

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.term-empty-hint {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-sm);
  max-width: 420px;
  text-align: center;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

// 下拉菜单里的 shell 说明样式见文件末尾的非 scoped 块（菜单被 teleport 到 body）
.term-menu-label {
  font-size: var(--font-size-base);
}

.term-menu-hint {
  font-size: var(--font-size-xs);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}
</style>

<style lang="scss">
// 非 scoped：el-dropdown 的菜单节点挂在 body 下，需要靠 popper-class 命名空间限定
.term-shell-menu {
  .term-menu-label {
    font-size: var(--font-size-base);
  }

  .term-menu-hint {
    font-size: var(--font-size-xs);
    color: var(--textSecondary);
  }

  // 「系统默认」角标
  .term-menu-tag {
    margin-left: var(--space-2);
    padding: 0 var(--space-1);
    font-size: var(--font-size-xs);
    font-style: normal;
    line-height: 16px;
    border: 1px solid var(--borderColor);
    border-radius: var(--radius-sm);
    color: var(--textSecondary);
  }

  // 工作目录提示行：路径靠右、过长省略（完整路径看 title）
  .term-menu-hint--path {
    margin-left: auto;
    padding-left: var(--space-4);
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-family-mono);
  }
}
</style>
