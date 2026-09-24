<template>
  <!-- 沉浸式专享：窗口顶部的「弹出条」。
       平时完全不可见，鼠标移到窗口顶部才滑下来；它同时承担被隐藏的标题栏的两项职责 ——
       **拖动窗口**（data-tauri-drag-region）与**窗口控制**（最小化/最大化/关闭）。
       窗口是无边框的（decorations:false），少了这两样用户既挪不动也关不掉窗口，所以不能只放一个
       「退出沉浸式」按钮了事。 -->
  <div v-if="immersive" class="immersive-layer">
    <!-- 顶部热区：跨源 iframe 会吞掉宿主 window 的 mousemove，光靠下面的 mousemove 监听
         在插件里永远唤不出退出条。这条透明的 6px 带钉在窗口最顶端、浮于 iframe 之上，
         指针一进来就能在**宿主页面**触发 mouseenter，从而唤出下面的退出条。 -->
    <div class="immersive-hotzone" @mouseenter="keepOpen" @mouseleave="scheduleHide"></div>
    <transition name="immersive-bar">
      <div
        v-show="open"
        class="immersive-bar"
        role="toolbar"
        aria-label="沉浸式退出条"
        data-tauri-drag-region
        @mouseenter="keepOpen"
        @mouseleave="scheduleHide"
      >
        <button
          type="button"
          class="exit-btn"
          aria-label="退出沉浸式"
          :title="`退出沉浸式（${IMMERSIVE_EXIT_LABEL}）`"
          @click="exit"
        >
          <el-icon class="exit-icon"><Close /></el-icon>
          <span>退出沉浸式</span>
        </button>
        <span class="bar-hint" data-tauri-drag-region>
          只显示插件界面 · 鼠标移开自动收起 · {{ IMMERSIVE_EXIT_LABEL }} 退出
        </span>
        <!-- 空白拖拽区：把可用拖拽面积铺满，点这里就能拖窗口 -->
        <div class="bar-spacer" data-tauri-drag-region></div>
        <WindowControls />
      </div>
    </transition>
  </div>
</template>

<script lang="ts" setup name="AppImmersiveBar">
import { onMounted, onUnmounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { Close } from '@element-plus/icons-vue'
import useStore from '@/store'
import WindowControls from '@/components/WindowControls.vue'
import { subscribe, type Unsubscribe } from '@/core/runtime'
import {
  IMMERSIVE_EXIT_EVENT,
  IMMERSIVE_EXIT_LABEL,
  IMMERSIVE_TOGGLE_EVENT,
} from '@/composables/useImmersiveShortcut'

const store = useStore()
const { immersiveMode: immersive } = storeToRefs(store.useLayoutStore)

/** 鼠标进入窗口顶部这条带内即唤出（px） */
const HOT_ZONE = 6
/** 移出后延迟收起：留一点容错，避免斜向划过时闪没（ms） */
const HIDE_DELAY = 320

const open = ref(false)
let hideTimer: number | undefined

function clearHideTimer() {
  if (hideTimer !== undefined) {
    window.clearTimeout(hideTimer)
    hideTimer = undefined
  }
}

function keepOpen() {
  clearHideTimer()
  open.value = true
}

function scheduleHide() {
  clearHideTimer()
  hideTimer = window.setTimeout(() => {
    hideTimer = undefined
    open.value = false
  }, HIDE_DELAY)
}

/**
 * 鼠标移到窗口顶部 → 唤出（负责「指针在宿主页面上」的情况）。
 *
 * 原本只用 window `mousemove`，理由是窗口无边框（`decorations(false)`），顶边兼作缩放边，
 * 铺常驻热区元素会把上边缘的拖拽缩放吃掉。但第三方插件是跨源 iframe，指针落在它上面时宿主
 * window **收不到** mousemove，在插件里就永远唤不出退出条 —— 于是补了模板里的
 * `.immersive-hotzone`：一条钉在窗口顶端、浮于 iframe 之上的透明热区，专门负责
 * 「指针在插件里」时的唤出。代价仅为「沉浸式下顶端 6px 不能拖拽缩放」，可接受
 * （窗口拖动由退出条自己的 data-tauri-drag-region 承担）。
 */
function onWindowMouseMove(e: MouseEvent) {
  if (open.value) return
  if (e.clientY <= HOT_ZONE) keepOpen()
}

function exit() {
  store.useLayoutStore.setImmersiveMode(false)
}

/**
 * 后端（OS 层全局热键）广播的「切换沉浸式」事件 —— F11 / macOS Control+Command+F。
 *
 * 为什么不直接监听 window 的 keydown：第三方插件是 `sandbox="allow-scripts"` 的跨源 iframe，
 * 其内部产生的 `keydown` 根本不会冒泡到宿主 window —— 所以在插件里编辑时按 F11 毫无反应。
 * 后端改用 Tauri global-shortcut 在系统层拦下按键再广播本事件，从而穿透 iframe 边界
 * （顺带也解决了 xterm / Monaco 抢按键的老问题：热键根本不会再进 WebView）。
 *
 * 广播会送到**每一个窗口**，故用 `document.hasFocus()` 兜一层：只让真正聚焦的那个窗口响应，
 * 否则后台窗口会被连带切换。
 */
function onToggleEvent() {
  if (!document.hasFocus()) return
  store.useLayoutStore.toggleImmersive()
}

/**
 * 后端广播的「退出沉浸式」事件 —— Esc。
 *
 * 正在开的弹窗优先吃掉 Esc（否则「关弹窗」会顺带退出沉浸式）—— 统一封装的 GlobalDialog
 * 带 role="dialog" + aria-modal="true"，与 EP 自带弹层用的是同一套语义选择器。
 */
function onExitEvent() {
  if (!document.hasFocus()) return
  if (document.querySelector('[role="dialog"][aria-modal="true"]')) return
  exit()
}

let unsubToggle: Unsubscribe | undefined
let unsubExit: Unsubscribe | undefined

onMounted(async () => {
  window.addEventListener('mousemove', onWindowMouseMove)
  unsubToggle = await subscribe(IMMERSIVE_TOGGLE_EVENT, onToggleEvent)
  unsubExit = await subscribe(IMMERSIVE_EXIT_EVENT, onExitEvent)
})

onUnmounted(() => {
  window.removeEventListener('mousemove', onWindowMouseMove)
  unsubToggle?.()
  unsubExit?.()
  clearHideTimer()
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.immersive-bar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: $header-height;
  display: flex;
  align-items: center;
  gap: 8px;
  padding-left: 8px;
  // 高于主区与侧栏（side-box 1000 / 分隔条 1001），低于 EP 弹层（2000+）——
  // 弹窗盖住它是期望行为，用户关掉弹窗就能重新唤出
  z-index: 2000;
  box-sizing: border-box;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-bottom: 1px solid theme.getVar('borderColor');
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.16);
    color: theme.getVar('textColor');
  }
}

// 顶部热区：把被 iframe 吃掉的那 6px 抢回来，让宿主能知道「指针到了窗口顶部」。
// 只在沉浸式下存在，代价是沉浸式时窗口最顶端这 6px 不能再用于拖拽缩放 —— 可接受，
// 因为退出条才是沉浸式的首要逃生口，且窗口拖动可由退出条的 data-tauri-drag-region 承担。
.immersive-hotzone {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 6px;
  // 高于退出条（2000）以免鼠标进出判定被压住；仍低于 EP 弹层的更高层级，
  // 弹窗打开时由弹窗接住指针（此时收不到热区事件是期望行为）
  z-index: 2001;
}

.exit-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  // 命中高度按项目约定 ≥24px（WASM/标题栏一套按钮规格都在 24 附近）
  height: 24px;
  padding: 0 8px;
  flex-shrink: 0;
  font-size: var(--font-size-sm);
  font-family: inherit;
  cursor: pointer;
  border-radius: var(--radius-md);
  transition: background-color 0.15s;
  // 语义化 <button> 需要抹平浏览器默认外观
  appearance: none;
  border: 1px solid transparent;
  background: transparent;

  @include theme.useTheme {
    color: theme.getVar('textColor');
    border-color: theme.getVar('borderStrong');
  }

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('activityBarHover');
      border-color: transparent;
    }
  }

  // 暗色下 activityBarHover 是饱和蓝（#4b6eaf），主文本色对比不足，hover 时转纯白
  html.dark &:hover {
    color: #fff;
  }

  .exit-icon {
    font-size: 13px;
    display: inline-flex;
    align-items: center;
  }
}

.bar-hint {
  font-size: var(--font-size-sm);
  flex: 0 1 auto;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  user-select: none;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.bar-spacer {
  flex: 1 1 auto;
  min-width: 16px;
  align-self: stretch;
}

// 下滑/上收动画（prefers-reduced-motion 由全局 index.scss 统一降级）
.immersive-bar-enter-active,
.immersive-bar-leave-active {
  transition: transform 0.18s ease, opacity 0.18s ease;
}

.immersive-bar-enter-from,
.immersive-bar-leave-to {
  transform: translateY(-100%);
  opacity: 0;
}
</style>
