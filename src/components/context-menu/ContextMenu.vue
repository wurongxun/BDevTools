<template>
  <Teleport to="body">
    <Transition name="context-menu">
      <ul
        v-if="visible"
        ref="menuRef"
        class="context-menu"
        :style="menuStyle"
        @contextmenu.prevent
        @click.stop
      >
        <template v-for="(item, index) in menuItems" :key="index">
          <!-- 分隔线 -->
          <li v-if="item.divider" class="context-menu-divider"></li>
          <!-- 子菜单 -->
          <li
            v-else-if="item.children && item.children.length"
            class="context-menu-item context-menu-submenu"
            :class="{ 'is-disabled': item.disabled }"
            @mouseenter="openSubmenu(index, $event)"
            @mouseleave="closeSubmenu(index)"
            @click.stop
          >
            <span class="menu-item-icon">
              <template v-if="item.icon">
                <component :is="item.icon" v-if="!isStringIcon(item.icon)" />
                <template v-else>{{ item.icon }}</template>
              </template>
            </span>
            <span class="menu-item-label">{{ item.label }}</span>
            <span class="menu-item-arrow">▶</span>
            <!-- 子菜单容器 -->
            <ContextMenu
              :visible="activeSubmenu === index"
              :menu-items="item.children"
              :position="submenuPosition"
              :level="level + 1"
              @select="handleSelect"
              @close="closeAll"
            />
          </li>
          <!-- 普通菜单项 -->
          <li
            v-else
            class="context-menu-item"
            :class="{ 'is-disabled': item.disabled, 'is-danger': item.danger }"
            @click="handleItemClick(item)"
            @mouseenter="closeAllSubmenus"
          >
            <span class="menu-item-icon">
              <template v-if="item.icon">
                <component :is="item.icon" v-if="!isStringIcon(item.icon)" />
                <template v-else>{{ item.icon }}</template>
              </template>
            </span>
            <span class="menu-item-label">{{ item.label }}</span>
            <span v-if="item.shortcut" class="menu-item-shortcut">{{ item.shortcut }}</span>
          </li>
        </template>
      </ul>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'

export interface MenuItem {
  label?: string
  icon?: string | object
  shortcut?: string
  disabled?: boolean
  danger?: boolean
  divider?: boolean
  children?: MenuItem[]
  onClick?: () => void
  [key: string]: any
}

interface Props {
  visible: boolean
  menuItems: MenuItem[]
  position?: { x: number; y: number }
  level?: number
}

const props = withDefaults(defineProps<Props>(), {
  position: () => ({ x: 0, y: 0 }),
  level: 0
})

const emit = defineEmits<{
  (e: 'select', item: MenuItem): void
  (e: 'close'): void
  (e: 'update:position', pos: { x: number; y: number }): void
}>()

const menuRef = ref<HTMLElement | null>(null)
// 判断 icon 是否为「字符串图标」（emoji / 文本）：字符串直接作为文本渲染，组件型图标走 <component :is>
const isStringIcon = (icon: MenuItem['icon']): boolean => typeof icon === 'string'
const activeSubmenu = ref<number | null>(null)
const submenuPosition = ref({ x: 0, y: 0 })
const actualPosition = ref({ x: props.position.x, y: props.position.y })

// 计算菜单位置，防止超出视口
const menuStyle = computed(() => ({
  left: `${actualPosition.value.x}px`,
  top: `${actualPosition.value.y}px`
}))

// 调整位置确保菜单不超出屏幕边界
const adjustPosition = () => {
  if (!menuRef.value) return

  const menuRect = menuRef.value.getBoundingClientRect()
  const viewportWidth = window.innerWidth
  const viewportHeight = window.innerHeight

  let { x, y } = props.position

  // 一级菜单处理右/下边界
  if (props.level === 0) {
    if (x + menuRect.width > viewportWidth - 10) {
      x = viewportWidth - menuRect.width - 10
    }
    if (y + menuRect.height > viewportHeight - 10) {
      y = viewportHeight - menuRect.height - 10
    }
    if (x < 10) x = 10
    if (y < 10) y = 10
  }

  actualPosition.value = { x, y }
}

// 打开子菜单
const openSubmenu = (index: number, event: MouseEvent) => {
  const item = props.menuItems[index]
  if (item.disabled) return

  const target = event.currentTarget as HTMLElement
  const rect = target.getBoundingClientRect()

  // 计算子菜单位置（右侧）
  let subX = rect.right - 2
  let subY = rect.top

  // 检查是否超出右边界，如果超出则显示在左侧
  const submenuWidth = 180 // 预估宽度
  if (subX + submenuWidth > window.innerWidth - 10) {
    subX = rect.left - submenuWidth + 2
  }

  // 检查是否超出下边界
  const submenuHeight = (item.children?.length || 0) * 36
  if (subY + submenuHeight > window.innerHeight - 10) {
    subY = window.innerHeight - submenuHeight - 10
  }
  if (subY < 10) subY = 10

  submenuPosition.value = { x: subX, y: subY }
  activeSubmenu.value = index
}

// 关闭子菜单
const closeSubmenu = (index: number) => {
  if (activeSubmenu.value === index) {
    activeSubmenu.value = null
  }
}

// 关闭所有子菜单
const closeAllSubmenus = () => {
  activeSubmenu.value = null
}

// 关闭所有菜单
const closeAll = () => {
  activeSubmenu.value = null
  emit('close')
}

// 点击菜单项
const handleItemClick = (item: MenuItem) => {
  if (item.disabled) return
  if (item.divider) return

  emit('select', item)

  if (item.onClick) {
    item.onClick()
  }

  closeAll()
}

// 处理子菜单选择
const handleSelect = (item: MenuItem) => {
  emit('select', item)
}

// 点击外部关闭菜单
const handleClickOutside = (event: MouseEvent) => {
  if (!menuRef.value) return
  if (!menuRef.value.contains(event.target as Node)) {
    closeAll()
  }
}

// 滚动时关闭菜单
const handleScroll = () => {
  closeAll()
}

// 窗口大小改变时关闭
const handleResize = () => {
  closeAll()
}

// ESC键关闭
const handleKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape') {
    closeAll()
  }
}

watch(
  () => props.visible,
  (val) => {
    if (val) {
      actualPosition.value = { ...props.position }
      nextTick(() => {
        adjustPosition()
      })
      document.addEventListener('mousedown', handleClickOutside)
      document.addEventListener('scroll', handleScroll, true)
      window.addEventListener('resize', handleResize)
      window.addEventListener('keydown', handleKeydown)
    } else {
      activeSubmenu.value = null
      document.removeEventListener('mousedown', handleClickOutside)
      document.removeEventListener('scroll', handleScroll, true)
      window.removeEventListener('resize', handleResize)
      window.removeEventListener('keydown', handleKeydown)
    }
  }
)

watch(
  () => props.position,
  () => {
    if (props.visible) {
      actualPosition.value = { ...props.position }
      nextTick(() => {
        adjustPosition()
      })
    }
  }
)

onMounted(() => {
  if (props.visible) {
    nextTick(() => {
      adjustPosition()
    })
  }
})

onUnmounted(() => {
  document.removeEventListener('mousedown', handleClickOutside)
  document.removeEventListener('scroll', handleScroll, true)
  window.removeEventListener('resize', handleResize)
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<style scoped lang="scss">
.context-menu {
  position: fixed;
  min-width: 100px;
  padding: 3px 0;
  margin: 0;
  list-style: none;
  background-color: var(--bgColor);
  border: 1px solid var(--borderColor);
  border-radius: 2px;
  box-shadow: 0 6px 24px var(--shadowColor), 0 0 1px var(--borderStrong);
  z-index: 9999;
  user-select: none;
  font-size: var(--fontSize);
  color: var(--textColor);
}

.context-menu-item {
  display: flex;
  align-items: center;
  padding: 6px 12px;
  cursor: pointer;
  transition: background-color 0.15s ease;
  position: relative;
  gap: 10px;
}

.context-menu-item:hover:not(.is-disabled) {
  background-color: var(--hoverColor);
  color: var(--activityBarActive);
}

.context-menu-item.is-disabled {
  color: var(--textDisabled);
  cursor: not-allowed;
}

.context-menu-item.is-danger:not(.is-disabled) {
  color: var(--dangerText);
}

.context-menu-item.is-danger:not(.is-disabled):hover {
  background-color: var(--dangerText);
  color: var(--bgColor);
}

.menu-item-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  font-size: 14px;
  line-height: 1;
  flex-shrink: 0;
}

.menu-item-label {
  flex: 1;
  white-space: nowrap;
}

.menu-item-shortcut {
  color: var(--textSecondary);
  font-size: var(--fontSize);
  margin-left: 20px;
}

.menu-item-arrow {
  font-size: 10px;
  color: var(--textSecondary);
  margin-left: 8px;
}

.context-menu-submenu:hover > .menu-item-arrow {
  color: currentColor;
}

.context-menu-divider {
  height: 1px;
  margin: 6px 8px;
  background-color: var(--borderColor);
  list-style: none;
}

/* 过渡动画 */
.context-menu-enter-active {
  transition: all 0.15s ease-out;
}

.context-menu-leave-active {
  transition: all 0.1s ease-in;
}

.context-menu-enter-from {
  opacity: 0;
  transform: scale(0.95);
  transform-origin: top left;
}

.context-menu-leave-to {
  opacity: 0;
  transform: scale(0.98);
}
</style>
