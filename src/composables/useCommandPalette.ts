/**
 * 命令面板的开关状态（模块级单例）。
 *
 * 面板本身在 `App.vue` 里挂一次，但唤起它的地方有好几处（快捷键、菜单按钮），
 * 所以可见性不能放在组件内部 —— 那样每个调用方都得拿到组件实例。
 */
import { ref } from 'vue'

const visible = ref(false)

/** 打开命令面板 */
export function openCommandPalette(): void {
  visible.value = true
}

/** 关闭命令面板 */
export function closeCommandPalette(): void {
  visible.value = false
}

/** 命令面板可见状态（只读给组件用，写入请走 open/close） */
export function useCommandPalette() {
  return { visible, open: openCommandPalette, close: closeCommandPalette }
}
