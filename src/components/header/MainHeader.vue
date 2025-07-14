<script setup lang="ts">
import { computed, inject, onMounted, ref, Ref, unref, watchEffect } from 'vue';
import { useI18n } from "vue-i18n";
import { SystemInfo } from "@/entity/entitys";
import { listen } from "@tauri-apps/api/event";
import { useRoute, useRouter } from 'vue-router';
import 'animate.css';
import { UserFilled, Close, Minus, CopyDocument, Share, Search, More } from "@element-plus/icons-vue"
import { invoke } from '@tauri-apps/api/core';
import router from '@/router';
import { getCurrentWindow, WebviewWindow, getAll } from "@tauri-apps/api/window";
import WindowMaximizationIcon from "@/components/Icon/WindowMaximizationIcon.vue"
import WindowRestoreIcon from "@/components/Icon/WindowRestoreIcon.vue"
import BlackModeIcon from "@/components/Icon/BlackModeIcon.vue"
import WhiteModeIcon from "@/components/Icon/WhiteModeIcon.vue"
import SettingsIcon from "@/components/Icon/SettingsIcon.vue"
import BasicFunctionIcon from "@/components/Icon/BasicFunctionIcon.vue"
import HelpIcon from "@/components/Icon/HelpIcon.vue"
import SSHIcon from "@/components/Icon/SSHIcon.vue"
import CopyIcon from "@/components/Icon/CopyIcon.vue"
import GitIcon from "@/components/Icon/GitIcon.vue"
import CommandIcon from "@/components/Icon/CommandIcon.vue"
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
const drawer = ref(true);
const rail = ref(true);
const visible = ref(false);
const value6 = ref(true)
const dark_mode_icon = ref("black")
import { appConfigInfo } from "@/store/appConfigInfo";
import { ResponseData, SettingMap } from '@/entity/entitys';
//路由名称
const routeName = computed(() => route.path.match('/[^/]+')?.[0] ?? '');
// 应用标题，动态切换不同标题
const appName = ref<string>(t('appName'));
const input3 = ref('')
let greetMsg = ref<SystemInfo>({ memory_total: 0, memory_used: 0, cpu_used: '0', hostname: '', cpus: 0 })
const userStore = appConfigInfo();
const darkModeChange = async function () {
    console.log("isDark: " + isDark)
    toggleDark()
    if (userStore.getIsDark == true) {
        invoke('updatate_setting', { id: userStore.getisDarkId, appSettingItemValue: String("false"), appSettingtemName: 'use_dark' }).then((res): void => {
            updateSettting()
        })
    } else {
        invoke('updatate_setting', { id: userStore.getisDarkId, appSettingItemValue: String("true"), appSettingtemName: 'use_dark' }).then((res): void => {
            updateSettting()
        })
    }






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
    // invoke('windows_minimize', { label: "main" }).then((res:any) => {
    //   console.log(res)
    // })
    await getCurrentWindow().minimize();
};
const windowHide = async () => {
    await getCurrentWindow().hide();
};

const updateSettting = function () {
   // if (userStore.appConfig.length < 1) {
        invoke('query_setting_list').then((res): void => {
            let res_ = res as ResponseData<Array<SettingMap>>
            let data = res_.data as Array<SettingMap>;
            for (let i = 0; i < data.length; i++) {
                const element = data[i];
                if (element.app_setting_item_name == "use_dark") {
                    element.app_setting_item_name;
                    element.id;
                    userStore.insetId(element.id)
                    let str = element.app_setting_item_value
                    let app_setting_item_value = (str === "true" ? true : false);
                    if (app_setting_item_value) {
                        userStore.changeTrue();
                        dark_mode_icon.value = "white"
                        document.documentElement.style.setProperty('--mouse--hover--button--bg-color', '#6b6d71');
                        document.documentElement.style.setProperty('--main--header--bg-color', '#525457');
                    } else {
                        userStore.changeFalse();
                        dark_mode_icon.value = "black"
                        document.documentElement.style.setProperty('--mouse--hover--button--bg-color', '#636466');
                        document.documentElement.style.setProperty('--main--header--bg-color', '#409EFF');
                    }
                }
            }
        })
   // }
}
onMounted(() => {
    updateSettting()
})

</script>
<template>
    <div class="main_header" data-tauri-drag-region>
        <div style="width: 250px;">
            <el-row style="height: 35px;text-align: center;" justify>
                <el-col :span="12">
                    <div class="header-logo" style="position: relative;height: 100%;">
                        <div>
                            <el-icon size="30">
                                <BoxLogoIcon />
                            </el-icon>
                        </div>
                        <div>
                            <el-icon size="30">
                                <el-avatar shape="square" size="small"
                                    src="https://cube.elemecdn.com/9/c2/f0ee8a3c7c9638a54940382568c9dpng.png" />
                            </el-icon>
                        </div>
                        <div>

                        </div>
                    </div>
                </el-col>
                <el-col :span="12"></el-col>
            </el-row>
        </div>
        <div
            style="width: calc(100vw - 500px);display: flex;justify-content: center;text-align: center;height: 35px; align-items: center; ">
            <el-input v-model="input3" style="min-width: 240px;max-width: 350px;" size="small" clearable="true"
                placeholder="Type something" :prefix-icon="Search">
            </el-input>
        </div>
        <div style="width: 250px;">
            <el-row style="height: 35px;text-align: center;" justify>
                <el-col :span="12">
                    <div class="demo-type" style="position: relative;height: 100%;">
                        <div>
                            <el-tooltip class="box-item" effect="dark" content="分享" placement="top-start">
                                <el-icon>
                                    <Share />
                                </el-icon>
                            </el-tooltip>
                        </div>
                        <div @click="darkModeChange">
                            <el-tooltip class="box-item" effect="dark" content="暗黑模式" placement="top-start">
                                <el-icon v-if="dark_mode_icon == 'black'">
                                    <BlackModeIcon />
                                </el-icon>
                                <el-icon v-else>
                                    <WhiteModeIcon />
                                </el-icon>
                            </el-tooltip>
                        </div>
                        <div>
                            <el-icon>
                                <More />
                            </el-icon>
                        </div>
                    </div>
                </el-col>
                <el-col :span="12">
                    <div class="demo-type" style="position: relative;height: 100%;">
                        <div @click="handleWinMin">
                            <el-tooltip class="box-item" effect="dark" content="最小化" placement="top-start">
                                <el-icon>
                                    <Minus />
                                </el-icon>
                            </el-tooltip>
                        </div>
                        <div @click="windowChange">
                            <template v-if="visible">
                                <el-tooltip class="box-item" effect="dark" content="还原" placement="top-start">
                                    <el-icon>
                                        <WindowRestoreIcon />
                                    </el-icon>
                                </el-tooltip>
                            </template>
                            <template v-else>
                                <el-tooltip class="box-item" effect="dark" content="最大化" placement="top-start">
                                    <el-icon>
                                        <WindowMaximizationIcon />
                                    </el-icon>
                                </el-tooltip>
                            </template>
                        </div>
                        <div @click="windowHide">
                            <el-tooltip class="box-item" effect="dark" content="关闭" placement="top-start">
                                <el-icon>
                                    <Close />
                                </el-icon>
                            </el-tooltip>
                        </div>

                    </div>
                </el-col>
            </el-row>
        </div>
    </div>
</template>
<style lang="scss" scoped>
.main_header {
    height: 35px;
    text-align: center;
    width: 100vw;
    display: flex;
    background-color: var(--main--header--bg-color);
}

.el-row:last-child {
    margin-bottom: 0;
}

//CopyDocument
// .el-col {
//   border-radius: 4px;
// }

.grid-content {
    border-radius: 4px;
    min-height: 36px;
}

.demo-type,
.header-logo {
    display: flex;
    justify-content: center;
    align-items: center;
}

.demo-type>div,
.header-logo>div {
    height: 100%;
    flex: 1;
    text-align: center;
    display: flex; //将子元素设置为弹性容器。
    justify-content: center; //使子元素在水平方向上居中对齐。
    align-items: center; //使子元素在垂直方向上居中对齐。
    //background-color: #c3d0da; /* 鼠标悬停时的背景色 */
    transition: background-color 0.3s;
    /* 平滑背景色过渡 */
    //padding-left: 3px;
    //padding-right: 3px;
    border: 1px solid transparent;
}



.demo-type>div:hover {
    //background-color: #afafaf; /* 鼠标悬停时的背景色 */
    background-color: rgba(124, 124, 124, 0.479);
    backdrop-filter: blur(10px);
    box-shadow: 0 0 10px rgba(0, 0, 0, 0.5);
    border-radius: 1px;
}

.header-logo>div:hover {}

.header-logo>div:not(:last-child) {
    //border-right: 1px solid var(--el-border-color);
}
</style>