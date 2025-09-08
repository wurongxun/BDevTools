declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<{}, {}, any>
  export default component
}

declare module '*.svg' {
  const content: any
  export default content
}

declare module 'vue-i18n' {
  import type { DefineComponent } from 'vue'
  const useI18n: any
  export { useI18n }
}
