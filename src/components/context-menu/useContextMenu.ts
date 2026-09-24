/**
 * hooks
 */
import { createApp, h, ref } from 'vue'
import ContextMenu from './ContextMenu.vue'
import type { MenuItem } from './ContextMenu.vue'

interface ContextMenuOptions {
  x: number
  y: number
  items: MenuItem[]
}

let contextMenuInstance: any = null

/**
 * 命令式显示右键菜单
 */
export function showContextMenu(options: ContextMenuOptions) {
  // 如果已存在实例，先销毁
  if (contextMenuInstance) {
    closeContextMenu()
  }

  const container = document.createElement('div')
  document.body.appendChild(container)

  const visible = ref(true)
  const position = ref({ x: options.x, y: options.y })

  const app = createApp({
    setup() {
      const handleSelect = (item: MenuItem) => {
        if (item.onClick) {
          item.onClick()
        }
        closeContextMenu()
      }

      const handleClose = () => {
        closeContextMenu()
      }

      return () =>
        h(ContextMenu, {
          visible: visible.value,
          menuItems: options.items,
          position: position.value,
          onSelect: handleSelect,
          onClose: handleClose
        })
    }
  })

  const instance = app.mount(container)

  contextMenuInstance = {
    app,
    container,
    close: () => {
      visible.value = false
      setTimeout(() => {
        app.unmount()
        if (container.parentNode) {
          container.parentNode.removeChild(container)
        }
        contextMenuInstance = null
      }, 150)
    }
  }

  return contextMenuInstance
}

/**
 * 关闭右键菜单
 */
export function closeContextMenu() {
  if (contextMenuInstance) {
    contextMenuInstance.close()
  }
}

/**
 * 组合式API方式使用
 */
export function useContextMenu() {
  const menuVisible = ref(false)
  const menuPosition = ref({ x: 0, y: 0 })
  const menuItems = ref<MenuItem[]>([])

  const show = (event: MouseEvent, items: MenuItem[]) => {
    event.preventDefault()
    event.stopPropagation()

    menuItems.value = items
    menuPosition.value = {
      x: event.clientX,
      y: event.clientY
    }
    menuVisible.value = true
  }

  const hide = () => {
    menuVisible.value = false
  }

  return {
    menuVisible,
    menuPosition,
    menuItems,
    show,
    hide
  }
}
