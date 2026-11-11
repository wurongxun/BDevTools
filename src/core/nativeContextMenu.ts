/*! 原生右键菜单守卫
 *
 * 桌面应用里不该出现 WebView 的浏览器右键菜单（重新加载/后退/检查元素那一套）——
 * 它既不原生、还会把开发者选项暴露给最终用户。本模块在**宿主界面的全局**把它去掉。
 *
 * ## 覆盖范围（重要）
 *
 * 本项目所有窗口（主窗口 / editor-* / about / note_book）加载的都是同一个 index.html +
 * hash 路由，都会跑一遍 `main.ts` ⇒ 这里注册的监听对它们**全部生效**。
 *
 * 但**第三方插件页覆盖不到**：它们跑在 `sandbox="allow-scripts"` 的跨源 iframe 里
 * （见 `docs/plugin-dev/*.md`），宿主拿不到 `contentDocument`，那里是否禁用由插件自己决定。
 * 想连插件页一起管，得改 Rust 侧给每个 window 注入 `initialization_script`，成本大得多，
 * 且会破坏「插件页权限边界」的设计前提 —— 暂不做。
 *
 * ## 为什么留了三个口子
 *
 * 1. **dev 模式放行**：WebView2 的右键菜单里有 `Inspect`，是开发期唯一的 DevTools 入口
 *    （生产构建本来就没有），砍掉等于自断 Debug 能力。
 * 2. **可编辑区域放行**：输入框 / textarea / contenteditable 需要原生菜单做
 *    「粘贴 / 全选 / 拼写建议 / 撤销」。全禁会让用户连粘贴都找不到入口，得不偿失。
 * 3. **`data-native-contextmenu` 标记**：某块区域确实要原生菜单时，给它加这个属性
 *    （含其所有子元素）即可。这是显式转义阀，避免以后有人为了一个用例把全局开关改掉。
 *
 * ## 与自定义菜单的关系
 *
 * 监听挂在 **capture 阶段**，且只 `preventDefault` —— 它**不阻止**后续监听器，
 * 所以各组件自己的 `@contextmenu.prevent` + 弹出 `ContextMenu` 完全不受影响
 * （preventDefault 已经替它们做了，行为等价于多调一次同样的操作）。
 */

/** 需要保留原生菜单的可编辑标签名 */
const EDITABLE_TAGS = new Set(['INPUT', 'TEXTAREA'])

/** 显式保留原生菜单的标记（标在容器上，整个子树生效） */
export const NATIVE_CONTEXT_MENU_ATTR = 'data-native-contextmenu'

export interface NativeContextMenuGuardOptions {
  /**
   * dev 模式是否放行原生菜单（默认 true）。
   * 生产构建传什么都是禁用 —— 这个选项只影响 `import.meta.env.DEV`。
   */
  allowInDev?: boolean
  /** 输入框 / 可编辑区域是否保留原生菜单（默认 true，强烈建议不要关） */
  allowInEditable?: boolean
}

/** 判断这次右键是否该放行给浏览器 */
function shouldKeepNative(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false
  // 先看显式豁免标记：容器上一标，整棵子树都走原生菜单
  if (target.closest(`[${NATIVE_CONTEXT_MENU_ATTR}]`)) return true
  if (EDITABLE_TAGS.has(target.tagName)) return true
  // `isContentEditable` 只在 HTMLElement 上，SVG 等 Element 子类没有 —— 必须先收窄类型
  return target instanceof HTMLElement && target.isContentEditable
}

/**
 * 安装原生右键菜单守卫，返回卸载函数。
 *
 * 在 `main.ts` 里、**挂载之前**调用：越早注册越不会出现「首帧还能弹出原生菜单」的窗口期。
 */
export function installNativeContextMenuGuard(
  options: NativeContextMenuGuardOptions = {},
): () => void {
  const { allowInDev = true, allowInEditable = true } = options

  // dev 模式：放行 —— 否则 WebView2 右键的 Inspect（唯一的 DevTools 入口）会被一起砍掉。
  // 生产构建里 import.meta.env.DEV 为 false，这里的分支整段会被 tree-shake 掉。
  if (allowInDev && import.meta.env.DEV) {
    return () => {}
  }

  const handler = (event: MouseEvent) => {
    if (allowInEditable && shouldKeepNative(event.target)) return
    event.preventDefault()
  }

  // capture：抢在组件自己的 handler 之前，但**不 stopPropagation**，别打断自定义菜单
  document.addEventListener('contextmenu', handler, true)

  return () => document.removeEventListener('contextmenu', handler, true)
}
