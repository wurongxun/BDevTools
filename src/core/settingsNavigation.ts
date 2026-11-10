import { ref } from 'vue'

/**
 * 宿主菜单 → 设置插件的「直达某分区」意图。
 *
 * 设置插件主面板是 lazyView：点菜单时它可能还没挂载（一次性事件会丢），
 * 也可能已被 KeepAlive 缓存（不会重新挂载）。用响应式值 + immediate watch：
 * 首次挂载时补读，已挂载实例则直接响应，两种时序都覆盖。
 * 放在 core 而非插件目录：宿主不必 import 插件实现，也不会把设置插件 chunk 拉进主包。
 *
 * 取值与 `plugins/settings/components/types.ts` 的 SettingTab 一致（core 不依赖插件类型）。
 */
export const settingsRequestedTab = ref<'app' | 'window' | 'plugin' | 'help' | 'about' | null>(null)
