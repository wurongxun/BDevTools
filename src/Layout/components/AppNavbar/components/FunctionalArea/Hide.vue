<template>
    <!-- <el-icon class="svg-icon" @click="windowHide" >
        <Close  />
    </el-icon> -->

    <!-- <div class="control-group window-controls"> -->
    <el-tooltip content="最小化" placement="bottom">
        <!-- <div @click="handleWinMin" class="control-button "> -->
        <el-icon class="svg-icon" @click="handleWinMin">
            <Minus />
        </el-icon>
        <!-- </div> -->
    </el-tooltip>

    <el-tooltip :content="visible ? '还原' : '最大化'" placement="bottom">
        <!-- <div @click="windowChange" class="control-button "> -->
        <el-icon class="svg-icon" @click="windowChange">
            <WindowRestoreIcon v-if="visible" />
            <WindowMaximizationIcon v-else />
        </el-icon>
        <!-- </div> -->
    </el-tooltip>

    <el-tooltip content="关闭" placement="bottom">
        <!-- <div @click="windowHide" class="control-button close-button"> -->

        <el-icon class="svg-icon" @click="windowHide">
            <Close />
        </el-icon>
        <!-- </div> -->
    </el-tooltip>

</template>
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from "vue-i18n";
import { useRoute } from 'vue-router';
// import 'animate.css';
import { Close, Minus, Share, Search, More } from "@element-plus/icons-vue"
import { getCurrentWindow } from '@tauri-apps/api/window';
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state';

import WindowMaximizationIcon from "@/components//Icon/WindowMaximizationIcon.vue"
import WindowRestoreIcon from "@/components/Icon/WindowRestoreIcon.vue"
import BlackModeIcon from "@/components/Icon/BlackModeIcon.vue"
import WhiteModeIcon from "@/components/Icon/WhiteModeIcon.vue"
import BoxLogoIcon from "@/components/Icon/BoxLogoIcon.vue"
import logoImage from "@/assets/image/logo.png"

import { useDark, useToggle } from '@vueuse/core'
import router from "@/router";
const isDark = useDark({
    storageKey: 'theme-appearance', //自定义localStorage存储名称
    valueDark: 'dark', //类名暗黑模式下值是dark
    valueLight: 'light', // 类名亮色模式下值是light
})
const toggleDark = useToggle(isDark)
// const isDark = useDark()
const { t, locale } = useI18n();
const route = useRoute()
const visible = ref(false);
const dark_mode_icon = ref("black")
const dark = ref(false);
//路由名称
const routeName = computed(() => route.path.match('/[^/]+')?.[0] ?? '');
// 应用标题，动态切换不同标题
const input3 = ref('')
const navigateTo = async function (routeUrl: string) {
    console.log("navigateTo", routeUrl);
    router.push(routeUrl);
};
const darkModeChange = async function () {
    toggleDark();
    dark.value = isDark.value
    dark_mode_icon.value = isDark.value ? "black" : "white"
    let el_color_primary = isDark.value ? "black" : ""
    document.documentElement.style.setProperty('--el-color-primary', el_color_primary);
}
const windowChange = async function () {
    await getCurrentWindow().toggleMaximize();
    if (visible.value == false) {
        visible.value = true
    } else {
        visible.value = false
    }
};

// 最小化
const handleWinMin = async () => {
    await getCurrentWindow().minimize();
};
const windowHide = async () => {
    saveWindowState(StateFlags.ALL);
    await getCurrentWindow().hide();
};

onMounted(() => {
    //updateSettting()
})
</script>
<style lang="scss" scoped>
@use '@/styles/variables.scss' as variables;

.icon-box {
    cursor: pointer;
    margin-right: 12px;
    margin-bottom: 2px;

    &:hover {
        color: variables.$theme-color;
    }
}

.svg-icon {
    width: 20px;
    height: 20px;
    margin-right: 12px;
    cursor: pointer;

    &:hover {
        color: variables.$theme-color;
    }
}
</style>
