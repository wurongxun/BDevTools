<template>
    <template v-for="item in menuList" :key="item.path">
        <el-sub-menu v-if="!item.meta?.hidden && hasShowChildren(item)" :index="item.redirect || item.path">
            <template #title>
                <el-icon v-if="isElIcon(item.meta?.icon as string)">
                    <component :is="item.meta?.icon" />
                </el-icon>
                <i v-else class="svg-box">
                    <svg-icon class-name="svg-icon" :icon-class="item.meta?.icon" />
                </i>
                <span>{{ item.meta?.title }}</span>
            </template>
            <sub-menu :menu-list="item.children || []" />
        </el-sub-menu>

        <el-menu-item v-else-if="!item.meta?.hidden" :index="item.redirect || item.path">
            <el-icon v-if="isElIcon(item.meta?.icon as string)">
                <component :is="item.meta?.icon" />
            </el-icon>
            <i v-else class="svg-box">
                <svg-icon class-name="svg-icon" :icon-class="item.meta?.icon" />
            </i>
            <template #title>
                <app-link :to="item.path" :redirect="item.redirect">
                    {{ item.meta?.title }}
                </app-link>
            </template>
        </el-menu-item>
    </template>
</template>

<script setup lang="ts">
import { defineAsyncComponent } from 'vue'
import type { RouteRecordRaw } from 'vue-router'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import AppLink from './AppLink.vue'

// 递归引用自身组件
const SubMenu = defineAsyncComponent(() => import('./SubMenu.vue'))

const props = defineProps<{
    menuList: RouteRecordRaw[]
}>()

const elIconNames = Object.keys(ElementPlusIconsVue)

// 判断是否有可见的子菜单
const hasShowChildren = (item: RouteRecordRaw) => {
    if (!item.children) return false
    const showChildren = item.children.filter(child => !child.meta?.hidden)
    return showChildren.length > 0
}

const isElIcon = (iconName: string | null): boolean => {
    if (iconName) {
        return elIconNames.includes(iconName)
    }
    return false
}
</script>

<style scoped lang="scss">
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