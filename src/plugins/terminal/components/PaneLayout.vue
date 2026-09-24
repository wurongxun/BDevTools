<template>
  <!-- 叶子：一个终端窗格 -->
  <TerminalPane
    v-if="node.kind === 'leaf'"
    :pane-id="node.paneId"
    :active="isActivePane(node.paneId)"
  />

  <!-- 分屏：a / 分隔条 / b，比例由 flex-grow 表达（divider 宽度不参与计算，不会累积误差） -->
  <div v-else ref="boxRef" class="pane-split" :class="`is-${node.direction}`">
    <div class="pane-slot" :style="{ flexGrow: node.ratio }">
      <PaneLayout :node="node.a" :tab-id="tabId" />
    </div>

    <div
      class="pane-divider"
      :class="[`is-${node.direction}`, { 'is-dragging': dragging }]"
      role="separator"
      tabindex="0"
      :aria-orientation="node.direction === 'row' ? 'vertical' : 'horizontal'"
      :aria-label="node.direction === 'row' ? '调整左右窗格宽度' : '调整上下窗格高度'"
      @mousedown="onDividerDown"
      @keydown="onDividerKeydown"
    >
      <div class="pane-divider-line"></div>
    </div>

    <div class="pane-slot" :style="{ flexGrow: 1 - node.ratio }">
      <PaneLayout :node="node.b" :tab-id="tabId" />
    </div>
  </div>
</template>

<script setup lang="ts" name="PaneLayout">
import { ref } from 'vue'
import TerminalPane from './TerminalPane.vue'
import { useTerminalStore } from '../store'
import { pathToLeaf } from '../tree'
import type { PaneNode } from '../types'

const props = defineProps<{
  node: PaneNode
  tabId: string
}>()

const store = useTerminalStore()
const boxRef = ref<HTMLElement | null>(null)
const dragging = ref(false)

const MIN_RATIO = 0.15
const MAX_RATIO = 0.85

function isActivePane(paneId: string): boolean {
  return store.activeTabId === props.tabId && store.activePaneId === paneId
}

/** 当前节点在整棵树里的路径（比例改的是这个节点，不是某个叶子） */
function myPath(): number[] | null {
  const tab = store.tabs.find((t) => t.id === props.tabId)
  if (!tab) return null
  // 用「a 侧第一个叶子」作为锚点定位该 split 节点
  const anchor = props.node.kind === 'split'
    ? firstLeafId(props.node.a)
    : props.node.paneId
  const leafPath = pathToLeaf(tab.root, anchor)
  if (!leafPath) return null
  // split 节点的路径 = 锚点叶子路径去掉最后一段（锚点在 a 侧，末段必为 0）
  return leafPath.slice(0, -1)
}

function firstLeafId(n: PaneNode): string {
  return n.kind === 'leaf' ? n.paneId : firstLeafId(n.a)
}

function ratioFromEvent(e: MouseEvent): number {
  const box = boxRef.value
  if (!box) return 0.5
  const rect = box.getBoundingClientRect()
  const raw =
    props.node.kind === 'split' && props.node.direction === 'row'
      ? (e.clientX - rect.left) / Math.max(1, rect.width)
      : (e.clientY - rect.top) / Math.max(1, rect.height)
  return Math.min(MAX_RATIO, Math.max(MIN_RATIO, raw))
}

function applyRatio(e: MouseEvent) {
  const path = myPath()
  if (!path) return
  store.resizeSplit(props.tabId, path, ratioFromEvent(e))
}

function onDividerDown(e: MouseEvent) {
  if (props.node.kind !== 'split') return
  e.preventDefault()
  dragging.value = true
  const isRow = props.node.direction === 'row'
  document.body.style.cursor = isRow ? 'col-resize' : 'row-resize'
  document.body.style.userSelect = 'none'

  const onMove = (ev: MouseEvent) => applyRatio(ev)
  const onUp = () => {
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    dragging.value = false
  }
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}

/** 键盘调整（WCAG 2.1.1）：←→ / ↑↓ 各 3%，Home/End 打到两端 */
function onDividerKeydown(e: KeyboardEvent) {
  if (props.node.kind !== 'split') return
  const isRow = props.node.direction === 'row'
  const dec = isRow ? 'ArrowLeft' : 'ArrowUp'
  const inc = isRow ? 'ArrowRight' : 'ArrowDown'
  let next = props.node.ratio
  if (e.key === dec) next -= 0.03
  else if (e.key === inc) next += 0.03
  else if (e.key === 'Home') next = MIN_RATIO
  else if (e.key === 'End') next = MAX_RATIO
  else return
  e.preventDefault()
  const path = myPath()
  if (!path) return
  store.resizeSplit(props.tabId, path, next)
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.pane-split {
  display: flex;
  width: 100%;
  height: 100%;
  overflow: hidden;
  min-width: 0;
  min-height: 0;

  &.is-row {
    flex-direction: row;
  }

  &.is-column {
    flex-direction: column;
  }
}

.pane-slot {
  flex: 1 1 0;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  display: flex;
}

.pane-divider {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;

  @include theme.useTheme {
    background-color: theme.getVar('borderColor');
  }

  &.is-row {
    width: var(--resize-handle-v-line-width);
    cursor: col-resize;

    // 扩大命中热区（纵向条向左右各扩，不占布局）
    &::before {
      content: '';
      position: absolute;
      top: 0;
      bottom: 0;
      left: calc(-1 * var(--resize-handle-hit-area));
      right: calc(-1 * var(--resize-handle-hit-area));
    }
  }

  &.is-column {
    height: var(--resize-handle-h-line-height);
    cursor: row-resize;

    &::before {
      content: '';
      position: absolute;
      left: 0;
      right: 0;
      top: calc(-1 * var(--resize-handle-hit-area));
      bottom: calc(-1 * var(--resize-handle-hit-area));
    }
  }

  &:hover,
  &.is-dragging {
    background-color: var(--resize-handle-color-active);
  }
}

.pane-divider-line {
  background-color: var(--resize-handle-color-active);
  border-radius: var(--radius-xs);
  opacity: 0;
  transition: opacity 0.15s ease, width 0.15s ease, height 0.15s ease;
}

.pane-divider.is-row .pane-divider-line {
  width: var(--resize-handle-v-line-width);
  height: 30px;
}

.pane-divider.is-column .pane-divider-line {
  width: 30px;
  height: var(--resize-handle-h-line-height);
}

.pane-divider:hover .pane-divider-line,
.pane-divider.is-dragging .pane-divider-line {
  opacity: 1;
}

.pane-divider.is-row:hover .pane-divider-line,
.pane-divider.is-row.is-dragging .pane-divider-line {
  width: var(--resize-handle-v-line-width-active);
}

.pane-divider.is-column:hover .pane-divider-line,
.pane-divider.is-column.is-dragging .pane-divider-line {
  height: var(--resize-handle-h-line-height-active);
}
</style>
