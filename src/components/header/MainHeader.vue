<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from "vue-i18n";
import { useRoute } from 'vue-router';
import 'animate.css';
import { Close, Minus, Share, Search, More } from "@element-plus/icons-vue"
import { getCurrentWindow } from '@tauri-apps/api/window';
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state';

import WindowMaximizationIcon from "@/components//Icon/WindowMaximizationIcon.vue"
import WindowRestoreIcon from "@/components/Icon/WindowRestoreIcon.vue"
import BlackModeIcon from "@/components/Icon/BlackModeIcon.vue"
import WhiteModeIcon from "@/components/Icon/WhiteModeIcon.vue"
import BoxLogoIcon from "@/components/Icon/BoxLogoIcon.vue"
import { useDark, useToggle } from '@vueuse/core'
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

<template>
    <div class="back_header" data-tauri-drag-region>
        <div class="back_left">
            <!-- <div class="logo-container">
                <el-icon size="30" class="app-logo">
                    <BoxLogoIcon />
                </el-icon>
            </div> -->
            {{ t('appName') }}
        </div>

        <!-- <div class="header-section header-center">
            <el-input v-model="input3" class="search-input" size="small" placeholder="Type something"
                :prefix-icon="Search" clearable>
            </el-input>
        </div> -->
        <div id="titlebar" class="back_center">

        </div>

        <div class="back_right">
            <!-- <div class="control-group action-controls"> -->
            <el-tooltip content="分享" placement="bottom">
                <div class="control-button ">
                    <el-icon class="control-icon">
                        <Share />
                    </el-icon>
                </div>
            </el-tooltip>

            <el-tooltip content="暗黑模式" placement="bottom">
                <div @click="darkModeChange()" class="control-button ">
                    <el-icon class="control-icon">
                        <BlackModeIcon v-if="dark" />
                        <WhiteModeIcon v-else />
                    </el-icon>
                </div>
            </el-tooltip>

            <el-tooltip content="更多" placement="bottom">
                <div class="control-button ">
                    <el-icon class="control-icon">
                        <More />
                    </el-icon>
                </div>
            </el-tooltip>
            <!-- </div>

            <div class="control-group window-controls"> -->
            <el-tooltip content="最小化" placement="bottom">
                <div @click="handleWinMin" class="control-button ">
                    <el-icon class="control-icon">
                        <Minus />
                    </el-icon>
                </div>
            </el-tooltip>

            <el-tooltip :content="visible ? '还原' : '最大化'" placement="bottom">
                <div @click="windowChange" class="control-button ">
                    <el-icon class="control-icon">
                        <WindowRestoreIcon v-if="visible" />
                        <WindowMaximizationIcon v-else />
                    </el-icon>
                </div>
            </el-tooltip>

            <el-tooltip content="关闭" placement="bottom">
                <div @click="windowHide" class="control-button close-button">

                    <el-icon class="control-icon close-icon">
                        <Close />
                    </el-icon>
                </div>
            </el-tooltip>

            <!-- </div> -->
        </div>
    </div>
</template>
<style lang="scss" scoped>
.control-button {
    width: 35px;
    height: 25px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    border-radius: 4px;
    margin-left: 5px;
    transition: background-color 0.2s;

    &:hover {
        background-color: rgba(128, 128, 128, 0.2);
    }

    &.close-button:hover {
        background-color: #e81123;
        color: white;
    }
}

.back_header {
    height: var(--header-height);
    width: 100vw;
    display: flex;
    align-items: center;
    background-color: var(--main--header--bg-color);
    user-select: none;
    margin: 0 5px;
}

.back_left,
.back_right {
    display: flex;
    align-items: center;
}

.back_center {
    flex: 1;
    display: flex;
    justify-content: center;
    align-items: center;
    font-weight: 500;
    padding: 0 10px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.logo-container {
    display: flex;
    align-items: center;
    height: 100%;

    .app-logo {
        padding: 2px;
    }
}

@media (max-width: 768px) {
    .back_center {
        font-size: 14px;
    }
}
</style>