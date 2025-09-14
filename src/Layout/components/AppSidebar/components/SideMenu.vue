<template>
  <div style="overflow: auto;height: 100%;">
    <el-menu :collapse="isCollapse" :default-active="currentOpenMenu()" class="el-menu-vertical">
      <!-- <el-menu-item v-for="item in onlyChildRoute" :index="getOnlyChildRoutePath(item)"
        :key="getOnlyChildRoutePath(item)">
        <el-icon v-if="isElIcon(item.meta?.icon)" @click="handleToPath(item)">
          <component :is="item.meta?.icon" />
        </el-icon>
        <i v-else class="svg-box" @click="handleToPath(item)">
          <svg-icon class-name="svg-icon" :icon-class="item.meta?.icon" />
        </i>
        <template #title>
          <app-link :to="getOnlyChildRoutePath(item)" :redirect="getOnlyChildRouteRedirect(item)">
            {{ getOnlyChildRouteTitle(item) }}
          </app-link>
        </template>
      </el-menu-item> -->

      <SubMenu :menu-list="childRoute"></SubMenu>
      <!-- <SubMenu :menu-list="childRouteNote"></SubMenu> -->
      <!-- <el-sub-menu>
        <template #title>
          <el-icon >
           <document />
          </el-icon>
          <span>笔记</span>
        </template> -->
        <NoteResources :menu-list="childRouteNote"></NoteResources>
      <!-- </el-sub-menu> -->


      <!-- <el-sub-menu v-for="item in childRoute" :index="item.redirect || item.path" :key="item.path">
      <template #title>
        <el-icon v-if="isElIcon(item.meta?.icon)">
          <component :is="item.meta?.icon" />
        </el-icon>
        <i v-else class="svg-box">
          <svg-icon class-name="svg-icon" :icon-class="item.meta?.icon" />
        </i>

        <span>{{ item.meta?.title }}</span>
      </template>
      <el-menu-item
        v-for="childItem in item.children"
        :index="childItem.redirect || childItem.path"
        :key="childItem.path"
      >
        <app-link :to="childItem.path"> {{ getTitle(childItem) }} </app-link>
      </el-menu-item>
    </el-sub-menu> -->
    </el-menu>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useRoute, useRouter } from 'vue-router'
import useStore from '@/store'
import AppLink from './AppLink.vue'

import type { RouteRecordRaw, RouteRecordRedirectOption } from 'vue-router'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import SubMenu from './SubMenu.vue'
import NoteResources from './NoteResources.vue'

const { routeStore, system, routerNoteStore } = useStore()
const route = useRoute()
const router = useRouter()
const { isCollapse } = storeToRefs(system)
const elIconNames = Object.keys(ElementPlusIconsVue)

const onlyChildRoute: any = computed(() =>
  routeStore.routes.filter(
    (item) => !item.meta?.hidden && (!item.children || item.children?.length <= 1)
  )
)
const childRoute: any = computed(() =>
  routeStore.routes.filter(
    (item) => !item.meta?.hidden && item.children && item.children?.length > 1
  )
)

const childRouteNote: any = computed(() =>
  routerNoteStore.routes
)
onMounted(() => {
  console.log(routeStore.routes)
  console.log("=========routerNoteStore.routes",childRouteNote)
  
})
const currentOpenMenu = () => {
  if (route.meta?.activeMenu) {
    return route.meta?.activeMenu
  }
  return route.fullPath
}

const handleToPath = (item: RouteRecordRaw) => {
  router.push({
    path: item.path
  })
}

const getTitle = (item: RouteRecordRaw) => {
  if (item.children && item.children.length > 1) {
    let showItem = item.children.filter((i) => !i.meta!.hidden)[0]
    return showItem.meta!.title
  }
  return item.meta!.title
}

const isElIcon = (iconName: string | null): boolean => {
  if (iconName) {
    return elIconNames.includes(iconName)
  }
  return false
}

const getOnlyChildRoutePath = (item: RouteRecordRaw): string => {
  return item.children && item.children.length ? item.children[0].path : item.path
}

const getOnlyChildRouteRedirect = (item: RouteRecordRaw): RouteRecordRedirectOption | undefined => {
  return item.children && item.children.length ? item.children[0].redirect : item.redirect
}

const getOnlyChildRouteTitle = (item: RouteRecordRaw): string => {
  if (item.children && item.children.length) {
    if (
      item.children &&
      item.children.length &&
      item.children[0].meta &&
      item.children[0].meta.title
    ) {
      return item.children[0].meta.title as string
    } else {
      return ''
    }
  } else {
    if (item.meta && item.meta.title) {
      return item.meta?.title as string
    } else {
      return ''
    }
  }
}
</script>

<style scoped lang="scss">
.el-menu-vertical {
  height: 100%;
}

.el-menu--collapse .el-menu .el-submenu,
.el-menu--popup {
  min-width: 120px !important;
}

.svg-box {
  margin-right: 5px;
  width: 24px;
  vertical-align: middle;
  text-align: center;
  display: inline-flex;
  fill: currentColor;
  align-items: center;
  justify-content: center;

  .svg-icon {
    font-size: 18px;
  }
}
</style>
