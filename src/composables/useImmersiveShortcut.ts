/**
 * 沉浸式的键盘快捷键 —— 按钮提示（tooltip / aria-keyshortcuts）与全局按键处理**共用这一份事实源**，
 * 避免「提示写着 F11、实际响应的是别的键」这类只有盲测才发现的偏差。
 *
 * 键位选择：
 * - Windows / Linux 用 `F11`：浏览器、播放器等绝大多数软件的全屏/沉浸默认键，学习成本为零。
 * - macOS 用 `Control+Command+F`：与系统原生「进入全屏」完全一致，无需额外记忆。
 * - 退出**始终保留 `Esc`**：进入沉浸式后标题栏随之隐藏，这个按钮本身不可见，Esc 是唯一零成本退出口。
 *
 * 本模块与 `useDark.ts` 一样是「无响应式状态的模块级导出」，不是需要调用的 composable 工厂。
 */

/**
 * 是否 macOS。
 * 优先 `userAgentData.platform`（`navigator.platform` 已被标记废弃），再退回 `platform` / `userAgent`。
 * 只在 Tauri 的 WebView 里运行（Windows WebView2 / macOS WKWebView / Linux WebKitGTK），
 * 三者对这三个字段都有实现，所以不需要再往后端要一次平台。
 */
const platform =
  (typeof navigator !== 'undefined' &&
    ((navigator as unknown as { userAgentData?: { platform?: string } }).userAgentData?.platform ||
      navigator.platform ||
      navigator.userAgent)) ||
  ''

export const IS_MAC = /mac|iphone|ipad|ipod/i.test(platform)

/** 给人看的键名（tooltip、提示文案） */
export const IMMERSIVE_TOGGLE_LABEL = IS_MAC ? 'Control+Command+F' : 'F11'

/** `aria-keyshortcuts` 用的键名（ARIA 规范里 Command 写作 `Meta`） */
export const IMMERSIVE_TOGGLE_ARIA = IS_MAC ? 'Control+Meta+F' : 'F11'

/** 退出提示里并列的两个退出口 */
export const IMMERSIVE_EXIT_LABEL = `Esc / ${IMMERSIVE_TOGGLE_LABEL}`

/**
 * 前端上报「本窗口是否处于沉浸式」的事件名（发给后端）。
 *
 * 后端据此决定是否在系统层拦截 Esc：Esc 一旦被 global-shortcut 拦下就**彻底离开 WebView**，
 * 所以必须只在真的处于沉浸式时才让它注册，否则常态下按 Esc 会毫无反应（连弹窗都关不掉）。
 * 详见 `src-tauri/src/immersive_shortcut.rs`。
 */
export const IMMERSIVE_STATE_EVENT = 'immersive-state'

/**
 * 后端拦住热键后**广播给前端**的两个事件名。
 *
 * 与上面 `IMMERSIVE_STATE_EVENT` 方向相反（那条是前端 → 后端），但同属沉浸式的接线协议，
 * 所以放在一起：调用点一律 import 这两个常量，**不要写裸字符串**（裸字符串是跨窗口同步
 * 约定的老毛病，改了名字两边不会一起改）。
 */
export const IMMERSIVE_TOGGLE_EVENT = 'immersive-shortcut-toggle'
export const IMMERSIVE_EXIT_EVENT = 'immersive-shortcut-exit'

/**
 * 「这一刻按下的键是否命中沉浸式切换」**不再由前端判定**。
 *
 * 这里原先有个 `isImmersiveToggleKey(e)`，配 window 上的 keydown 监听使用。但它对第三方插件
 * 无效：插件是跨源沙箱 iframe，其内部按键根本不会冒泡到宿主 window（详见 AppImmersiveBar 注释）。
 * 现在改由后端 Tauri global-shortcut 在系统层拦下按键后广播事件给前端，见
 * `src-tauri/src/immersive_shortcut.rs`。
 *
 * 上面的键位（Windows/Linux `F11`、macOS `Control+Command+F`）与 Rust 侧的 `Shortcut::new(...)`
 * 是同一套约定，**改动任一侧都要同步另一侧**；本文件仍负责把键名呈现给用户
 * （tooltip / `aria-keyshortcuts`），是「给人看的键名」的唯一事实源。
 */
