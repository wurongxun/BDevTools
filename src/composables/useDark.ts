import { useDark, useToggle } from '@vueuse/core'

export const isDark = useDark({
   storageKey: 'theme-appearance', //自定义localStorage存储名称
  valueDark: 'dark', //类名暗黑模式下值是dark
  valueLight: 'light', // 类名亮色模式下值是light
})
export const toggleDark = useToggle(isDark)
