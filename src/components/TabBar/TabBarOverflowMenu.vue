<template>
  <div ref="rootRef" class="tab-bar-overflow">
    <button
      type="button"
      class="tab-bar-overflow-btn"
      :title="buttonTitle"
      :aria-label="buttonTitle"
      :aria-expanded="visible"
      aria-haspopup="menu"
      @click.stop="toggle"
    >
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
        <path d="M4 6h8M4 8h8M4 10h8" stroke="currentColor" stroke-width="1.5" fill="none" />
      </svg>
    </button>
  </div>

  <!--
    Teleport 到 body：TabBar 所在的面板层层套着 overflow:hidden（结果区尤其深），
    fixed 定位虽然一般不受影响，但祖先一旦出现 transform/filter/contain 就会被裁切。
    菜单不是 hover 子菜单（关闭靠点击外部 / Esc），断掉 DOM 后代关系没有副作用。
  -->
  <Teleport to="body">
    <div
      v-if="visible && items.length > 0"
      ref="menuRef"
      class="tab-bar-overflow-menu"
      role="menu"
      :style="menuStyle"
    >
      <div class="menu-header">
        <span>{{ headerLabel }} ({{ items.length }})</span>
        <button
          v-if="items.length > 1"
          type="button"
          class="menu-close-all"
          title="关闭全部"
          @click.stop="onCloseAll"
        >
          关闭全部
        </button>
      </div>
      <div class="menu-list">
        <div
          v-for="item in items"
          :key="item.id"
          class="menu-item"
          :class="{ 'is-active': item.id === activeId }"
          role="menuitem"
          @click="onSelect(item.id)"
        >
          <span class="menu-name" :title="item.title || item.label">{{ item.label }}</span>
          <span v-if="item.dirty" class="menu-dirty" title="未保存">●</span>
          <button
            type="button"
            class="menu-close"
            :title="`关闭 ${item.label}`"
            :aria-label="`关闭 ${item.label}`"
            @click.stop="emit('close', item.id)"
          >
            <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
              <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.5" fill="none" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, type CSSProperties } from 'vue'

/**
 * Tab 栏右侧「收纳菜单」：列出所有标签 + 一键关闭全部。
 *
 * 存在的理由：标签横向溢出后，靠两端的滚动按钮一个个找很低效（尤其「关掉最后那几个」）。
 * 菜单里每一项既能跳转也能单独关闭，右上角是「关闭全部」（>=2 个才给，1 个时点它就是误导）。
 *
 * 组件只做**展示与意图上报**：数据由父组件给（items），点完之后怎么关由父组件决定
 * （各插件 store 不同，这里不碰）。
 */
interface OverflowMenuItem {
  /** 稳定标识。不要传数组下标作 id —— 关掉中间一项下标会平移 */
  id: string
  /** 菜单里显示的一行文本（父组件负责截断） */
  label: string
  /** 悬浮提示用的全文（可选，缺省用 label） */
  title?: string
  /** 是否有未保存修改（显示圆点） */
  dirty?: boolean
}

withDefaults(
  defineProps<{
    items: OverflowMenuItem[]
    /** 当前激活项的 id（高亮用） */
    activeId: string
    /** 触发按钮的悬浮/无障碍文案 */
    buttonTitle?: string
    /** 菜单头部文案，组件自行补上 (n) */
    headerLabel?: string
  }>(),
  { buttonTitle: '显示所有标签', headerLabel: '打开的标签' }
)

const emit = defineEmits<{
  select: [id: string]
  close: [id: string]
  'close-all': []
}>()

const visible = ref(false)
const rootRef = ref<HTMLElement | null>(null)
const menuRef = ref<HTMLElement | null>(null)
const menuStyle = ref<CSSProperties>({})

function updatePosition() {
  const root = rootRef.value
  if (!root) return
  const rect = root.getBoundingClientRect()
  // 宽度上限受视口约束，左边界再夹一次，保证小窗口下菜单不被裁到屏幕外
  const width = Math.max(200, Math.min(360, window.innerWidth - 16))
  const left = Math.max(8, Math.min(rect.right - width, window.innerWidth - width - 8))
  menuStyle.value = {
    left: `${left}px`,
    top: `${rect.bottom + 4}px`,
    width: `${width}px`,
    maxHeight: `${Math.max(120, window.innerHeight - rect.bottom - 16)}px`,
  }
}

/** 菜单打开后把激活项滚进可视区（十几项以上就得往下翻） */
function revealActiveItem() {
  const list = menuRef.value?.querySelector<HTMLElement>('.menu-list')
  list?.querySelector<HTMLElement>('.menu-item.is-active')?.scrollIntoView({ block: 'nearest' })
}

function onDocumentClick(event: MouseEvent) {
  const target = event.target as Node | null
  if (!target) return
  // 菜单被 Teleport 到 body 了，按钮与菜单是两棵 DOM 子树，两边都要判包含关系
  if (rootRef.value?.contains(target) || menuRef.value?.contains(target)) return
  close()
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') close()
}

function addListeners() {
  document.addEventListener('click', onDocumentClick)
  document.addEventListener('keydown', onKeydown)
  // capture：滚动容器不一定是 window/body 的直接祖先
  window.addEventListener('scroll', updatePosition, true)
  window.addEventListener('resize', updatePosition)
}

function removeListeners() {
  document.removeEventListener('click', onDocumentClick)
  document.removeEventListener('keydown', onKeydown)
  window.removeEventListener('scroll', updatePosition, true)
  window.removeEventListener('resize', updatePosition)
}

function open() {
  updatePosition()
  visible.value = true
  // 同步注册本次点击的后续监听：本次 click 还会冒泡到 document，
  // 但 target 落在按钮内（rootRef.contains 命中），不会被误判成「点击外部」
  addListeners()
  nextTick(revealActiveItem)
}

function close() {
  if (!visible.value) return
  visible.value = false
  removeListeners()
}

function toggle() {
  if (visible.value) close()
  else open()
}

function onSelect(id: string) {
  emit('select', id)
  close()
}

function onCloseAll() {
  emit('close-all')
  close()
}

onBeforeUnmount(removeListeners)
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.tab-bar-overflow {
  flex-shrink: 0;
  display: flex;
}

.tab-bar-overflow-btn {
  width: 24px;
  height: 100%;
  min-height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  appearance: none;
  border: none;
  padding: 0;
  background: transparent;
  color: inherit;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &:hover {
    @include theme.useTheme {
      color: theme.getVar('textColor');
      background-color: theme.getVar('activityBarHover');
    }
  }
}

.tab-bar-overflow-menu {
  position: fixed;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-radius: var(--radius-sm);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  // 高于布局层与 GlobalDialog(1501)，低于 Element Plus popper(2000+) 会挡住菜单里的 tooltip —— 取中间值
  z-index: 1800;
  user-select: none;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderColor');
    color: theme.getVar('textColor');
  }
}

.menu-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 6px 8px 6px 10px;
  flex-shrink: 0;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    border-bottom: 1px solid theme.getVar('borderColor');
    font-size: theme.getVar('fontSizeSmall');
    color: theme.getVar('textSecondary');
  }
}

.menu-close-all {
  flex-shrink: 0;
  padding: 2px 8px;
  cursor: pointer;
  border-radius: var(--radius-sm);
  appearance: none;
  background: transparent;
  font-family: inherit;

  @include theme.useTheme {
    border: 1px solid theme.getVar('borderStrong');
    color: theme.getVar('textColor');
    font-size: theme.getVar('fontSizeSm');
  }

  &:hover {
    border-color: transparent;
    background-color: #c42b1c;
    color: #fff;
  }
}

.menu-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 6px 5px 10px;
  cursor: pointer;

  @include theme.useTheme {
    font-size: theme.getVar('fontSizeSmall');

    &:hover {
      background-color: theme.getVar('activityBarHover');
    }
  }

  &.is-active {
    @include theme.useTheme {
      background-color: theme.getVar('hoverColor');
    }
  }
}

.menu-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.menu-dirty {
  flex-shrink: 0;
  line-height: 1;
  color: #e2b714;
}

.menu-close {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  appearance: none;
  border: none;
  padding: 0;
  border-radius: var(--radius-sm);
  background: transparent;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &:hover {
    background-color: rgba(255, 255, 255, 0.15);

    @include theme.useTheme {
      color: theme.getVar('textColor');
    }
  }
}
</style>
