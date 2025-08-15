<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from "vue-i18n";
import { useRoute } from 'vue-router';
import 'animate.css';
import { Close, Minus, Share, Search, More } from "@element-plus/icons-vue"
import { getCurrentWindow } from '@tauri-apps/api/window';

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
//路由名称
const routeName = computed(() => route.path.match('/[^/]+')?.[0] ?? '');
// 应用标题，动态切换不同标题
const input3 = ref('')
const darkModeChange = async function () {
    toggleDark();
    dark_mode_icon.value = isDark.value ? "black" : "white"
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
    await getCurrentWindow().hide();
};

onMounted(() => {
    //updateSettting()
})
</script>

<template>
    <div class="main_header" data-tauri-drag-region>
        <div class="header-section header-left">
            <div class="logo-container">
                <el-icon size="30" class="app-logo">
                    <BoxLogoIcon />
                </el-icon>
            </div>
        </div>

        <!-- <div class="header-section header-center">
            <el-input v-model="input3" class="search-input" size="small" placeholder="Type something"
                :prefix-icon="Search" clearable>
            </el-input>
        </div> -->

        <div class="header-section header-right">
            <div class="control-group action-controls">
                <el-tooltip content="分享" placement="bottom">
                    <el-icon class="control-icon">
                        <Share />
                    </el-icon>
                </el-tooltip>

                <el-tooltip content="暗黑模式" placement="bottom">
                    <el-icon class="control-icon" @click="darkModeChange">
                        <BlackModeIcon v-if="dark_mode_icon == 'black'" />
                        <WhiteModeIcon v-else />
                    </el-icon>
                </el-tooltip>

                <el-tooltip content="更多" placement="bottom">
                    <el-icon class="control-icon">
                        <More />
                    </el-icon>
                </el-tooltip>
            </div>

            <div class="control-group window-controls">
                <el-tooltip content="最小化" placement="bottom">
                    <el-icon class="control-icon" @click="handleWinMin">
                        <Minus />
                    </el-icon>
                </el-tooltip>

                <el-tooltip :content="visible ? '还原' : '最大化'" placement="bottom">
                    <el-icon class="control-icon" @click="windowChange">
                        <WindowRestoreIcon v-if="visible" />
                        <WindowMaximizationIcon v-else />
                    </el-icon>
                </el-tooltip>

                <el-tooltip content="关闭" placement="bottom">
                    <el-icon class="control-icon close-icon" @click="windowHide">
                        <Close />
                    </el-icon>
                </el-tooltip>
            </div>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.main_header {
    height: 35px;
    width: 100vw;
    display: flex;
    align-items: center;
    justify-content: space-between;
    background-color: var(--main--header--bg-color);
    padding: 0 10px;
    box-sizing: border-box;
    -webkit-app-region: drag;

    .header-section {
        display: flex;
        align-items: center;
        height: 100%;
        -webkit-app-region: no-drag;
    }

    .header-left {
        flex: 0 0 auto;
        min-width: 100px;
    }

    .header-center {
        flex: 1 1 auto;
        display: flex;
        justify-content: center;
        padding: 0 20px;

        .search-input {
            min-width: 240px;
            max-width: 500px;
            width: 100%;
        }
    }

    .header-right {
        flex: 0 0 auto;
        display: flex;
        min-width: 200px;
        justify-content: flex-end;
    }

    .control-group {
        display: flex;
        height: 100%;
        margin: 0 5px;

        .control-icon {
            width: 32px;
            height: 100%;
            display: flex;
            align-items: center;
            justify-content: center;
            border-radius: 4px;
            margin: 0 2px;
            cursor: pointer;
            transition: all 0.2s ease;
            color: var(--el-text-color-primary);

            &:hover {
                background-color: rgba(124, 124, 124, 0.479);
                backdrop-filter: blur(10px);
                box-shadow: 0 0 5px rgba(0, 0, 0, 0.3);
            }

            &.close-icon:hover {
                background-color: #ff5f56;
                color: white;
            }
        }
    }

    .logo-container {
        display: flex;
        align-items: center;
        height: 100%;

        .app-logo {
            padding: 2px;
        }
    }
}

// 响应式优化
@media (max-width: 768px) {
    .main_header {
        .header-center {
            padding: 0 10px;

            .search-input {
                min-width: 150px;
            }
        }

        .control-group {
            margin: 0 2px;

            .control-icon {
                width: 28px;
                margin: 0 1px;
            }
        }
    }
}
</style>