/**
 * 用**独立窗口**打开第三方插件页面（Web3D / 游戏等重资源插件用）。
 *
 * 与面板内沙箱 iframe 的分工（详见 `docs/plugin-dev/web3d.md`）：
 * - 顶层窗口没有 iframe 的 `sandbox` 限制 → `localStorage` 存档、全屏、指针锁定天然可用；
 *   游戏独占一个 webview，卡帧也不会跟主界面抢渲染线程。
 * - 窗口 label 用 `web3d-<id>`，刻意**不**匹配 `capabilities/default.json` 里任何 `windows`
 *   模式（`main` / `app-*` / `editor-*`）→ ACL 匹配不上即**全拒**，插件页拿不到任何宿主命令。
 *   这是本方案的安全立足点：插件页的权限不靠 iframe 撑，而靠「本来就没给它授权」。
 *
 * 代价是**有意为之**的：该窗口里没有宿主桥，插件页调不到 `plugin_call`。
 * 需要宿主能力（KV / 文件 / HTTP）的插件仍应走面板内的沙箱 iframe。
 * 正因为代价不可逆，**是否走独立窗口必须是插件清单里的显式声明**
 * （`standaloneWindow: true`），不能由「声明了 pageCapabilities」推断。
 *
 * 窗口必须保留**系统装饰**：插件页被 ACL 全拒 ⇒ 拿不到 Tauri API ⇒ 自绘标题栏、
 * 拖拽区（`startDragging`）都做不了。用无边框窗口只会得到一个既拖不动也关不掉的窗口。
 *
 * ## 主题：只在建窗时对齐
 * 窗口 `theme` 只在**建窗那一刻**写进 WebView2（`tauri-runtime-wry` 建 webview 时调
 * `with_theme` → wry `ICoreWebView2Profile::SetPreferredColorScheme`），运行期没有公开 API
 * 能再改：tauri 的 `set_theme` 只到 tao 窗口层（标题栏 / 菜单），系统主题变化也只 emit
 * `WINDOW_THEME_CHANGED` 事件。⇒ 宿主之后切主题时，**已打开**的独立窗口不会跟着变，
 * 关掉重开才对齐。
 * 这个窗口也收不到宿主的主题广播：`web3d-*` 不在任何 capability 的 `windows` 里，`listen`
 * 会被 ACL 拒；它和宿主还**不同源**（`plugin.localhost` vs `tauri.localhost`），
 * localStorage / `storage` 事件同样传不过去。要运行期同步只能由 Rust `eval()` 注入。
 * 所以取「创建时对齐」：主题由接线层注入（见 `setHostThemeProvider`），不写死 `dark`。
 */
import { ref } from 'vue'
import { WebviewWindow } from '@/core/runtime'
import type { Window } from '@/core/runtime'
import { invoke } from '@/api/rpc'
import NotificationUtil from '@/utils/NotificationUtil'
import { pluginAssetUrl } from './url'

/** 独立窗口 label 前缀（不要让它匹配任何 capability 的 windows 模式，见文件头注释） */
const WINDOW_PREFIX = 'web3d-'

/** 插件独立窗口的 label（同时也是「是否已打开」的判据） */
export function pluginWindowLabel(pluginId: string): string {
  return `${WINDOW_PREFIX}${pluginId}`
}

/**
 * 当前打开的插件独立窗口 label 集合。
 *
 * 为什么是在这里维护而不是每次去问系统：`WebviewWindow.getByLabel` 是异步的，
 * 而按钮文案 / 容器徽章需要在渲染期同步读到「是否已打开」。窗口由本模块创建，
 * 生命周期也只有本模块知道，所以这里就是唯一事实源。
 */
const openLabels = ref<Set<string>>(new Set())

/** 某插件的独立窗口是否已打开（响应式） */
export function isPluginWindowOpen(pluginId: string): boolean {
  return openLabels.value.has(pluginWindowLabel(pluginId))
}

/** 记录/清除「已打开」状态 */
function markOpen(label: string, open: boolean) {
  if (open) openLabels.value.add(label)
  else openLabels.value.delete(label)
}

/**
 * 正在创建中的窗口 label。
 *
 * 必要性：`WebviewWindow.getByLabel` 是异步的，而窗口创建要等 webview 就绪 ——
 * 在这段窗口期里再点一次，`getByLabel` 仍然查不到，就会拿同一个 label 创建第二次并报错
 * （表现为连点两下弹一个「打开失败」）。用同步的 in-flight 集合把这段窗口期盖住。
 */
const creating = new Set<string>()

/**
 * 宿主主题提供者。
 *
 * 为什么是注入而不是直接 import `@/composables/useDark`：`core/` 目前**不依赖**
 * `composables/` 与 `store/`（整个 core 目录 grep 可验证），而 `useDark` 自身 import 了
 * `core/runtime` 的 `load` —— 直接引会成环，也会破坏这一层的分层。做法照
 * `initPluginHost(BUILTIN_PLUGINS)` 的先例：从上层拿的信息一律当参数/回调交进来。
 */
let hostThemeProvider: (() => 'dark' | 'light') | null = null

/** 注入「当前宿主主题」取值函数（`main.ts` 在 `await initTheme()` 之后调用一次） */
export function setHostThemeProvider(provider: () => 'dark' | 'light'): void {
  hostThemeProvider = provider
}

/**
 * 建窗时用的窗口主题。
 *
 * 未注入时返回 `undefined`（＝交给系统主题），**刻意不回落成 `dark`**：漏接注入线时
 * 应表现为「跟随系统」，而不是静默回到旧的写死行为。
 */
function currentHostTheme(): 'dark' | 'light' | undefined {
  return hostThemeProvider ? hostThemeProvider() : undefined
}

/**
 * 打开插件的独立窗口；已打开则前置。创建失败会提示用户并写日志（不静默）。
 *
 * @param pluginId 插件 ID（同时是 `plugin://` 路径首段）
 * @param entry    包内入口页面相对路径
 * @param title    窗口标题（用插件名，便于在任务栏区分）
 */
export async function openPluginWindow(pluginId: string, entry: string, title: string): Promise<void> {
  const label = pluginWindowLabel(pluginId)

  // 已在创建中：什么都不做（第一次调用会 show + focus，这里再点只是别制造第二个窗口）
  if (creating.has(label)) return
  creating.add(label)

  // 同一 label 重复创建会直接报错，所以先查后建
  let existing: Awaited<ReturnType<typeof WebviewWindow.getByLabel>> = null
  try {
    existing = await WebviewWindow.getByLabel(label)
  } catch (e) {
    creating.delete(label)
    throw e
  }
  if (existing) {
    creating.delete(label)
    markOpen(label, true)
    // 已开着就前置（走 Rust 的 SetForegroundWindow，比 show() 可靠）
    await invoke('focus_window', { label })
    return
  }

  const win = new WebviewWindow(label, {
    url: pluginAssetUrl(pluginId, entry),
    title,
    width: 1100,
    height: 720,
    theme: currentHostTheme(),
    resizable: true,
    // 系统装饰，不可去掉（见文件头注释）
    decorations: true,
    visible: false,
  })

  win.once('tauri://created', async function () {
    creating.delete(label)
    markOpen(label, true)
    const w = win as unknown as Window
    await w.center()
    await w.show()
    await w.setFocus()
  })

  // 用户关掉窗口后按钮要变回「打开」，否则界面会一直说「已打开」
  win.once('tauri://destroyed', function () {
    creating.delete(label)
    markOpen(label, false)
  })

  win.once('tauri://error', function (event) {
    creating.delete(label)
    markOpen(label, false)
    console.error(`[PluginWindow] 创建插件窗口 ${label} 失败`, event)
    NotificationUtil.error(`打开「${title}」独立窗口失败，请查看日志`)
  })
}

/** 关闭插件独立窗口（没开着则什么都不做） */
export async function closePluginWindow(pluginId: string): Promise<void> {
  const label = pluginWindowLabel(pluginId)
  const existing = await WebviewWindow.getByLabel(label)
  if (existing) {
    await existing.close()
  }
  // 主动关闭时 `tauri://destroyed` 通常也会到，但别依赖它：这里同步清掉更稳
  creating.delete(label)
  markOpen(label, false)
}
