// src/directives/dragPopUp.ts
import type { Directive, DirectiveBinding } from 'vue'

interface DragElement extends HTMLElement {
  _dragCleanup?: () => void
}

const vDragPopUp: Directive<DragElement> = {
  mounted(el: DragElement, binding: DirectiveBinding) {
    // 幂等保护：HMR 等场景下重复挂载时先清理旧监听，避免旧闭包操作已脱离文档的 DOM
    el._dragCleanup?.()

    // 拖拽手柄选择器，默认 .dialog-header；通过事件委托判断，避免缓存子节点引用失效
    const handleSelector = (binding.value as string | undefined) || '.dialog-header'

    let dragging = false
    let startX = 0
    let startY = 0
    let initialLeft = 0
    let initialTop = 0

    const onMouseDown = (e: MouseEvent) => {
      if (e.button !== 0) return
      // 仅按在手柄上时才启动拖拽（委托判断，不依赖缓存的 DOM 引用）
      const target = e.target as HTMLElement | null
      if (!target || !target.closest(handleSelector)) return

      dragging = true
      startX = e.clientX
      startY = e.clientY
      const rect = el.getBoundingClientRect()
      initialLeft = rect.left
      initialTop = rect.top

      document.addEventListener('mousemove', onMouseMove)
      document.addEventListener('mouseup', onMouseUp)
      // 阻止默认行为（文本选中等）干扰拖拽
      e.preventDefault()
    }

    const onMouseMove = (e: MouseEvent) => {
      if (!dragging || !el.isConnected) return

      const dx = e.clientX - startX
      const dy = e.clientY - startY

      let newLeft = initialLeft + dx
      let newTop = initialTop + dy

      const maxLeft = window.innerWidth - el.offsetWidth
      const maxTop = window.innerHeight - el.offsetHeight
      newLeft = Math.max(0, Math.min(newLeft, maxLeft))
      newTop = Math.max(0, Math.min(newTop, maxTop))

      el.style.position = 'fixed'
      el.style.left = `${newLeft}px`
      el.style.top = `${newTop}px`
      el.style.transform = 'none'
    }

    const onMouseUp = () => {
      dragging = false
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mouseup', onMouseUp)
    }

    el.addEventListener('mousedown', onMouseDown)

    el._dragCleanup = () => {
      el.removeEventListener('mousedown', onMouseDown)
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mouseup', onMouseUp)
    }
  },

  unmounted(el: DragElement) {
    el._dragCleanup?.()
    delete el._dragCleanup
  }
}

export default vDragPopUp
