/**
 * 插件面板的**按需加载**包装（懒激活的实现体那一层）。
 *
 * ## 三阶段加载里的哪一阶段
 *
 * 「一切皆是插件」的目标加载顺序是：清单（启动即就位）→ 贡献条目（启动即注册）
 * → **实现体（用到才加载）**。这里就是第三阶段：
 *
 * 1. 插件 `index.ts` 只同步 import 图标（活动栏要立刻画出来，不能等）；
 * 2. 面板组件换成 `lazyView(...)`，rollup 把它切成独立 chunk；
 * 3. 用户第一次切到该插件时 chunk 才被拉进来，耗时记进 `activation.ts`。
 *
 * 于是「装了很多插件」不再等于「首屏下载很多代码」——这正是装多了就慢的解药。
 *
 * ## 为什么必须有失败兜底
 *
 * 异步 chunk 的加载会失败（发版后旧页面还在、网络盘、文件损坏）。没有 `errorComponent`
 * 时 Vue 会把错误抛给上层，表现为整块面板区域空白且控制台一行报错，用户无从判断发生了什么。
 * 这里把失败就地显示成一块可读的错误面板：**故障半径只到这一块面板**。
 */
import { defineAsyncComponent, defineComponent, h, markRaw, type Component } from 'vue'
import { PART_LABELS, recordActivation } from './activation'

/** chunk 加载超时（毫秒）：超了就走失败兜底，不让面板永远转圈 */
const LOAD_TIMEOUT_MS = 30_000

/** 延迟多久才显示「加载中」（ms）：chunk 通常几十毫秒就回来，立刻显示会闪一下 */
const LOADING_DELAY_MS = 200

/** 加载中 / 加载失败的兜底面板（render 函数创建，样式走内联 CSS 变量，见下） */
function placeholder(pluginId: string, part: string, kind: 'loading' | 'error'): Component {
  const label = PART_LABELS[part] ?? part
  return defineComponent({
    name: `LazyView_${kind}_${pluginId}_${part}`,
    props: { error: { type: null, default: null } },
    setup(props: { error?: unknown }) {
      return () =>
        h('div', { class: 'lazy-view-placeholder', style: PLACEHOLDER_STYLE }, [
          h('div', { style: TITLE_STYLE }, kind === 'loading' ? `正在加载${label}…` : `${label}加载失败`),
          kind === 'error'
            ? h(
                'div',
                { style: DETAIL_STYLE },
                String((props.error as { message?: string })?.message ?? props.error ?? '未知错误'),
              )
            : null,
          h('div', { style: HINT_STYLE }, `插件：${pluginId}`),
        ])
    },
  })
}

// 样式内联而不是写进 CSS 文件：这些节点由 render 函数创建，**SFC 的 scoped 样式管不到**
// （scoped 靠编译期给模板节点打 data-v 属性，动态节点没有）。用设计令牌变量而不是硬编码值。
const PLACEHOLDER_STYLE = {
  display: 'flex',
  flexDirection: 'column',
  gap: 'var(--space-2)',
  padding: 'var(--space-4)',
  height: '100%',
  overflow: 'auto',
} as const
const TITLE_STYLE = { fontSize: 'var(--font-size-md)', color: 'var(--textColor)' } as const
const DETAIL_STYLE = {
  fontSize: 'var(--font-size-sm)',
  color: 'var(--dangerText)',
  fontFamily: 'var(--font-family-mono)',
  wordBreak: 'break-all',
} as const
const HINT_STYLE = { fontSize: 'var(--font-size-xs)', color: 'var(--textSecondary)' } as const

/** 面板组件的动态 import 形状（`import('./X.vue')` 的返回） */
export type PanelLoader = () => Promise<{ default: Component } | Component>

/**
 * 把一个动态 import 包装成可懒加载的组件。
 *
 * @param pluginId 归属插件（耗时记在它名下）
 * @param part 面板局部名（`sidebar` / `main` / `dock` / `tabBar`），只用于展示与分组
 * @param loader 动态 import，如 `() => import('./SidebarPanel.vue')`
 */
export function lazyView(pluginId: string, part: string, loader: PanelLoader): Component {
  const timedLoader = async (): Promise<Component> => {
    const t0 = performance.now()
    try {
      const mod = await loader()
      const comp = (mod as { default?: Component }).default ?? (mod as Component)
      recordActivation(pluginId, part, performance.now() - t0)
      return comp
    } catch (e) {
      // 失败不记耗时：那条记录会让人以为「加载过但很快」，实际是没加载成功
      console.error(`[lazyView] 插件 ${pluginId} 的${PART_LABELS[part] ?? part}加载失败`, e)
      throw e
    }
  }

  return markRaw(
    defineAsyncComponent({
      loader: timedLoader,
      loadingComponent: placeholder(pluginId, part, 'loading'),
      errorComponent: placeholder(pluginId, part, 'error'),
      delay: LOADING_DELAY_MS,
      timeout: LOAD_TIMEOUT_MS,
      // 面板区被 KeepAlive 包着：切走再切回不该重新走一遍加载流程（也避免重复记耗时）
    }),
  )
}
