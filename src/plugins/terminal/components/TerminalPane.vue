<template>
  <!-- 一个窗格 = 一个 PTY 会话 + 一个 xterm 实例 -->
  <div
    class="term-pane"
    :class="{ 'is-active': active, 'is-exited': pane?.exited, 'is-busy': pane?.busy }"
    :style="{ backgroundColor: paneBg }"
    @mousedown="onPaneMouseDown"
  >
    <!-- 宿主容器：position:absolute 纯受父容器驱动，避免 fit() → xterm 改内部尺寸 → 反过来触发 ResizeObserver 的抖动 -->
    <div ref="hostRef" class="term-host"></div>

    <!-- 会话结束覆盖层：不要直接卸载窗格，用户可能需要看最后的输出 -->
    <div v-if="pane?.exited" class="term-exit">
      <div class="term-exit-card">
        <p class="term-exit-title">会话已结束</p>
        <p class="term-exit-meta">
          退出码
          <code>{{ pane.exitCode === null ? '未知（被信号终止）' : pane.exitCode }}</code>
        </p>
        <div class="term-exit-actions">
          <app-button size="small" @click.stop="restart">重新启动</app-button>
          <app-button size="small" @click.stop="store.closePane(paneId)">关闭窗格</app-button>
        </div>
      </div>
    </div>

    <!-- 启动中 / 启动失败 -->
    <div v-else-if="!sessionId" class="term-exit">
      <div class="term-exit-card">
        <p class="term-exit-title">{{ bootError ? '无法启动终端' : '正在启动…' }}</p>
        <p v-if="bootError" class="term-exit-meta term-exit-meta--error">{{ bootError }}</p>
        <div v-if="bootError" class="term-exit-actions">
          <app-button size="small" @click.stop="restart">重试</app-button>
          <app-button size="small" @click.stop="store.closePane(paneId)">关闭窗格</app-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, watch } from 'vue'
import { useResizeObserver } from '@vueuse/core'
import { TerminalSession } from '../session'
import { useTerminalStore } from '../store'

const props = defineProps<{
  paneId: string
  active: boolean
}>()

const store = useTerminalStore()
const hostRef = ref<HTMLElement | null>(null)
const sessionId = ref('')
const bootError = ref('')
/** 每次 restart 递增，触发重新建会话 */
const epoch = ref(0)
/** Windows ConPTY 需要在 xterm 里显式声明后端（影响滚动回看重排） */
const isWindows = computed(() => store.defaults?.platform === 'windows')

/** 当前窗格的元数据（标题/cwd/退出码等） */
const pane = computed(() => store.panes[props.paneId] ?? null)

/**
 * 窗格底色 = 当前终端配色方案的 background。
 * xterm 的 canvas 只覆盖字符网格区域，网格之外（最后一行下方、右端）会露出宿主底色；
 * 不把宿主设成同一个颜色就会出现一道色差带（`.xterm-viewport` 已被置为 transparent）。
 */
const paneBg = computed(() => store.currentPalette.background)

function onPaneMouseDown() {
  if (!props.active) store.setActivePane(props.paneId)
}

/** 建立会话：创建 xterm + 打开 PTY + 挂回调 */
async function boot() {
  const host = hostRef.value
  if (!host) return
  const spec = store.takeLaunchSpec(props.paneId)

  const session = new TerminalSession({
    paneId: props.paneId,
    container: host,
    appearance: store.appearance,
    palette: store.currentPalette,
    windowsPty: isWindows.value,
    openLink: (uri) => store.openExternal(uri),
    hooks: {
      onTitle: (t) => store.onPaneTitle(props.paneId, t),
      onCwd: (cwd) => store.onPaneCwd(props.paneId, cwd),
      onMark: (mark, code) => store.onPaneMark(props.paneId, mark, code),
      onExit: (code) => {
        sessionId.value = ''
        store.onPaneExit(props.paneId, code)
      },
      onError: (msg) => {
        bootError.value = msg
      },
      onActivity: () => {
        /* 输出活动目前只用于 busy 态显示，不需要每块都刷 UI */
      },
      onRequestSearch: () => {
        store.searchOpen = true
      },
      onSearchResults: (info) => {
        // 只有激活窗格的搜索结果才写到状态栏（避免后台窗格的搜索覆盖提示）
        if (props.paneId !== store.activePaneId) return
        store.searchInfo =
          info.resultCount === 0
            ? '无匹配'
            : info.resultIndex < 0
              ? `${info.resultCount}+ 处匹配`
              : `${info.resultIndex + 1} / ${info.resultCount}`
      },
    },
  })
  store.registerSession(props.paneId, session)

  try {
    const res = await session.open({ shell: spec.shell, cwd: spec.cwd, args: spec.args })
    // 建会话是异步的，这期间窗格可能已被关掉（用户手快）：实例已被释放，
    // 后端 PTY 由 session.open() 自己回收，这里不要再往组件/store 里写状态
    if (session !== store.getSession(props.paneId)) return
    sessionId.value = res.sessionId
    bootError.value = ''
    store.onPaneOpened(props.paneId, {
      sessionId: res.sessionId,
      cwd: res.cwd,
      integration: res.integration,
      shellKind: res.shellKind,
    })
    session.fitNow()
    if (props.active) session.focus()
  } catch (e) {
    bootError.value = e instanceof Error ? e.message : String(e)
    sessionId.value = ''
  }
}

function restart() {
  bootError.value = ''
  store.restartPaneNode(props.paneId)
  epoch.value += 1
}

onMounted(() => {
  void boot()
})

// 尺寸变化（窗口 / 面板高度 / 分隔条 / 标签切换）统一由 ResizeObserver 驱动 fit，
// 比在各处手动 refit 更可靠：只要宿主的盒模型变了就会触发。
useResizeObserver(hostRef, () => {
  store.getSession(props.paneId)?.fit()
})

watch(epoch, () => {
  void boot()
})

// 激活态变化：切标签/切窗格后立即适配并聚焦（此时容器刚变为可见，尺寸才是有效的）
watch(
  () => props.active,
  async (isActive) => {
    if (!isActive) return
    const s = store.getSession(props.paneId)
    if (!s) return
    s.fitNow()
    s.focus()
  },
)

onUnmounted(() => {
  store.unregisterSession(props.paneId)
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;
@use '@/styles/mixin.scss' as mixin;

.term-pane {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  box-sizing: border-box;
  border: 1px solid transparent;
  // 实际的 background-color 由内联 style 给（= 当前配色方案的 background，随主题变化）；
  // 这里只留一个与面板根同源的兜底，防止内联样式失效时露出主区底色。
  @include theme.useTheme {
    background-color: theme.getVar('appMainBgColor');
  }

  // 激活窗格给一圈细描边（多分屏时用来分辨焦点在哪个窗格）
  &.is-active {
    @include theme.useTheme {
      border-color: theme.getVar('borderStrong');
    }
  }
}

.term-host {
  position: absolute;
  inset: 0;
  overflow: hidden;
  padding: 4px 6px;
  box-sizing: border-box;

  // xterm 的滚动容器不要自带背景/内边距，交给 .term-pane
  :deep(.xterm) {
    height: 100%;
    padding: 0;
  }

  // 滚动条：全局只给 `html::-webkit-scrollbar` 上过令牌，管不到这里，
  // 不显式声明的话 Windows 下会露出系统默认的浅色滚动条（会话区里的另一处「没适配」）
  :deep(.xterm-viewport) {
    background-color: transparent !important;
    @include mixin.scrollbar;
  }
}

.term-exit {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(1px);

  @include theme.useTheme {
    background-color: color-mix(in srgb, theme.getVar('bgColor') 78%, transparent);
  }
}

.term-exit-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderColor');
  }
}

.term-exit-title {
  margin: 0;
  font-size: var(--font-size-base);
  font-weight: 600;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.term-exit-meta {
  margin: 0;
  font-size: var(--font-size-sm);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  code {
    font-family: var(--font-family-mono);
    color: inherit;
  }

  &--error {
    max-width: 360px;
    text-align: center;
    word-break: break-all;
    color: var(--el-color-danger);
  }
}

.term-exit-actions {
  display: flex;
  gap: var(--space-2);
  margin-top: var(--space-1);
}
</style>
