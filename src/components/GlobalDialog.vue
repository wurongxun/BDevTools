<!-- src/components/GlobalDialog.vue -->
<template>
  <Teleport to="body">
    <div
      v-drag-pop-up
      ref="dialogRef"
      class="global-dialog-wrapper"
      :class="{ 'is-vertical-center': verticalCenter }"
      :style="dialogStyle"
      v-if="dialogVisible"
      role="dialog"
      aria-modal="true"
      :aria-labelledby="titleId"
      tabindex="-1"
    >
      <div class="dialog-header">
        <span :id="titleId" class="title">{{ title }}</span>
        <button type="button" class="close-btn" :aria-label="`关闭 ${title}`" @click="handleClose">
          ✕
        </button>
      </div>
      <div class="dialog-body" :style="{ maxHeight: props.bodyMaxHeight }">
        <slot />
      </div>
      <div class="dialog-footer" v-if="$slots.footer">
        <slot name="footer" />
      </div>
    </div>
    <div
      class="dialog-overlay"
      v-if="dialogVisible"
      aria-hidden="true"
      @click="closeOnOverlay ? handleClose() : undefined"
    />
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, computed, nextTick, onBeforeUnmount } from 'vue'
import vDragPopUp from '@/directives/dragPopUp'

interface Props {
  modelValue: boolean
  title?: string
  width?: string
  /** 可选：固定高度；不传则由内容自适应（body 限高 60vh 滚动） */
  height?: string
  /** 是否垂直居中；默认 true，传 false 则靠上（15vh）显示 */
  verticalCenter?: boolean
  closeOnOverlay?: boolean
  /** 内容区最大高度；默认 60vh，编辑器类弹窗可放宽 */
  bodyMaxHeight?: string
  /** 关闭前钩子：返回 false 可拦截关闭（例如保存失败要用户确认） */
  beforeClose?: () => boolean | Promise<boolean>
}

const props = withDefaults(defineProps<Props>(), {
  title: '提示',
  width: '520px',
  height: '',
  verticalCenter: true,
  closeOnOverlay: false,
  bodyMaxHeight: '60vh'
})

interface Emits {
  (e: 'update:modelValue', value: boolean): void
  (e: 'close'): void
}

const emit = defineEmits<Emits>()

// 默认水平垂直居中；宽高均可通过 prop 定制，height 不传时内容自适应。
// max-width/max-height 兜底：小窗口下固定宽度会被裁切（审查报告 #30）
const dialogStyle = computed(() => ({
  width: props.width,
  maxWidth: 'calc(100vw - 32px)',
  maxHeight: 'calc(100vh - 32px)',
  ...(props.height ? { height: props.height } : {})
}))

const dialogVisible = ref<boolean>(props.modelValue)

// 弹窗标题 id，供 aria-labelledby 关联
const titleId = `global-dialog-title-${Math.random().toString(36).slice(2, 9)}`
const dialogRef = ref<HTMLElement | null>(null)
/** 打开前的焦点元素，关闭后归还焦点（WCAG 2.4.3 Focus Order） */
let lastFocusedElement: HTMLElement | null = null

/** 可聚焦元素选择器（排除 disabled / 隐藏元素） */
const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])'
].join(',')

function getFocusableElements(): HTMLElement[] {
  if (!dialogRef.value) return []
  return Array.from(dialogRef.value.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (el) => el.offsetWidth > 0 || el.offsetHeight > 0 || el === document.activeElement
  )
}

/**
 * 弹窗内的键盘处理（捕获阶段，位于 document 上）：
 * - Esc 关闭弹窗
 * - Tab / Shift+Tab 在弹窗内循环，避免焦点跑到背景控件（WCAG 2.1.2）
 */
function onKeydownCapture(e: KeyboardEvent) {
  if (!dialogVisible.value) return
  // 弹窗内又打开了更高层的 ElMessageBox（例如「保存失败，仍要关闭吗」）时，
  // 键盘完全交给它：否则这里的捕获监听会吞掉 Esc/Tab，确认框既关不掉也切不了焦点。
  if (document.querySelector('.el-overlay-message-box')) return

  if (e.key === 'Escape') {
    e.stopPropagation()
    e.preventDefault()
    void handleClose()
    return
  }

  if (e.key !== 'Tab') return

  const focusable = getFocusableElements()
  if (focusable.length === 0) {
    e.preventDefault()
    dialogRef.value?.focus()
    return
  }

  const first = focusable[0]
  const last = focusable[focusable.length - 1]
  const active = document.activeElement as HTMLElement | null

  // 焦点已在弹窗外（例如误入背景控件），拉回弹窗内
  if (!active || !dialogRef.value?.contains(active)) {
    e.preventDefault()
    first.focus()
    return
  }

  if (e.shiftKey && active === first) {
    e.preventDefault()
    last.focus()
  } else if (!e.shiftKey && active === last) {
    e.preventDefault()
    first.focus()
  }
}

watch(() => props.modelValue, (val: boolean) => {
  dialogVisible.value = val
})

watch(dialogVisible, async (val: boolean) => {
  emit('update:modelValue', val)
  if (val) {
    lastFocusedElement = document.activeElement as HTMLElement | null
    await nextTick()
    // 焦点移入弹窗：优先聚焦首个可聚焦控件，否则回落到弹窗容器本身
    const focusable = getFocusableElements()
    const target = focusable.find((el) => !el.classList.contains('close-btn')) ?? focusable[0]
    ;(target ?? dialogRef.value)?.focus()
    document.addEventListener('keydown', onKeydownCapture, true)
  } else {
    document.removeEventListener('keydown', onKeydownCapture, true)
    lastFocusedElement?.focus?.()
    lastFocusedElement = null
    emit('close')
  }
})

// 组件卸载时兜底移除监听，避免监听器泄漏
onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydownCapture, true)
})

const handleClose = async (): Promise<void> => {
  if (props.beforeClose) {
    const allow = await props.beforeClose()
    if (!allow) return
  }
  dialogVisible.value = false
}
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as variables;
@use '@/styles/theme.scss';

.global-dialog-wrapper {
  position: fixed;
  top: 15vh;
  left: 50%;
  transform: translateX(-50%);
  width: 520px;
  border-radius: var(--radius-md);
  // 需高于布局层（侧边栏 z-index:1000、分割线 z-index:1001），
  // 但低于 Element Plus popper 基准层（2000+），保证弹窗内下拉菜单正常置顶
  z-index: 1501;
  display: flex;
  flex-direction: column;
  // 固定高度时 body 内部滚动，裁切超出圆角的内容
  overflow: hidden;

  @include theme.useTheme {
    background: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderColor');
  }

  // 阴影固定用深色半透明：弹窗悬浮在暗色遮罩上，亮色阴影会形成白边光晕
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.35);

  // 垂直居中模式；拖拽时指令会改写 left/top 并置 transform: none，不受影响
  &.is-vertical-center {
    top: 50%;
    transform: translate(-50%, -50%);
  }
}

.dialog-header {
  height: variables.$header-height;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 6px 0 12px;
  cursor: move;
  user-select: none;

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.dialog-header .title {
  font-weight: 600;

  @include theme.useTheme {
    font-size: theme.getVar('fontSizeSmall');
    color: theme.getVar('textColor');
  }
}

.dialog-header .close-btn {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: all 0.2s;
  // 语义化 <button> 需要抹平浏览器默认外观
  appearance: none;
  border: none;
  padding: 0;
  background: transparent;
  font-family: inherit;
  flex-shrink: 0;

  @include theme.useTheme {
    font-size: theme.getVar('fontSizeSmall');
    color: theme.getVar('textSecondary');

    &:hover {
      color: theme.getVar('textColor');
      background: theme.getVar('hoverColor');
    }
  }
}

.dialog-body {
  flex: 1;
  min-height: 0;
  padding: 12px 16px;
  max-height: 60vh;
  overflow-y: auto;

  @include theme.useTheme {
    font-size: theme.getVar('fontSizeSmall');
    color: theme.getVar('textColor');
  }
}

.dialog-footer {
  padding: 1px 3px;
  text-align: right;
  align-content: center;
  height: variables.$header-height;
  @include theme.useTheme {
    border-top: 1px solid theme.getVar('borderColor');
  }
}

.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.4);
  z-index: 1500;
}
</style>
