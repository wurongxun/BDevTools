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

  // 先占好实例壳子：关闭是异步的（留 150ms 给收起动画），期间可能已经开了新菜单
  const instance: {
    app: ReturnType<typeof createApp> | null
    container: HTMLDivElement
    close: () => void
  } = { app: null, container, close: () => {} }

  let closing = false

  instance.close = () => {
    // 重复关闭直接忽略：原来的实现会排两个定时器，第二次清空引用更危险
    if (closing) return
    closing = true
    visible.value = false
    setTimeout(() => {
      instance.app?.unmount()
      if (container.parentNode) {
        container.parentNode.removeChild(container)
      }
      // 只有「自己仍是当前实例」时才清空：关闭期间新开的菜单不能被这次清空抹掉引用，
      // 否则下一次打开时检测不到旧菜单 ⇒ 两个菜单叠在一起、旧的永远关不掉
      if (contextMenuInstance === instance) {
        contextMenuInstance = null
      }
    }, 150)
  }

  const app = createApp({
    setup() {
      // 不要在这里调 item.onClick：ContextMenu 内部已经调过一次了，
      // 再调一次就是两遍（过滤条件会被加两条）。这里只负责关闭。
      const handleSelect = (_item: MenuItem) => {
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

  instance.app = app
  app.mount(container)
  contextMenuInstance = instance

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
