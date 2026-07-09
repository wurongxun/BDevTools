import 'vue-router'

declare module 'vue-router' {
  interface RouteMeta {
    title?: string
    hidden?: boolean
    keepAlive?: boolean
    icon?: string
    activeMenu?: string
    noBreadCrumbs?: boolean
    noTagView?: boolean
    affix?: boolean
  }
}

declare module 'vue' {
  interface ComponentCustomProperties {
    $route: import('vue-router').RouteLocationNormalizedLoaded
    $router: import('vue-router').Router
  }
}
