<script setup lang="ts">
import { computed, inject, onMounted, defineProps, ref, Ref, unref, watchEffect } from 'vue';
import { useI18n } from "vue-i18n";
import { SystemInfo } from "@/entity/entitys";
import { listen } from "@tauri-apps/api/event";
import { useRoute, useRouter } from 'vue-router';
import { Back } from "@element-plus/icons-vue"
import { invoke } from '@tauri-apps/api/core'
import { UserFilled, Close, Minus, CopyDocument, Share, Search, More } from "@element-plus/icons-vue"
import { getCurrentWindow } from "@tauri-apps/api/window";
import WindowMaximizationIcon from "@/components/Icon/WindowMaximizationIcon.vue"
import WindowRestoreIcon from "@/components/Icon/WindowRestoreIcon.vue"
import BlackModeIcon from "@/components/Icon/BlackModeIcon.vue"
import WhiteModeIcon from "@/components/Icon/WhiteModeIcon.vue"
import BoxLogoIcon from "@/components/Icon/BoxLogoIcon.vue"
import router from '@/router';
const { t, locale } = useI18n();
const route = useRoute()
const drawer = ref(true);
const rail = ref(true);
const visible = ref(false);
const value6 = ref(true)
const dark_mode_icon = ref("black")
//路由名称
const routeName = computed(() => route.path.match('/[^/]+')?.[0] ?? '');
const header_height = '35px';
// 应用标题，动态切换不同标题
const appName = ref<string>(t('appName'));
const input3 = ref('')
const props = defineProps({
    showWinMin: {
        type: Boolean,
        default: false, // 默认值
    },
    showWinChange: {
        type: Boolean,
        default: false, // 默认值
    },
    showWinHide: {
        type: Boolean,
        default: false, // 默认值
    },
    showTitle: {
        type: Boolean,
        default: false, // 默认值
    },
    title: {
        type: String,
        default: "", // 默认值
    },
});


let greetMsg = ref<SystemInfo>({ memory_total: 0, memory_used: 0, cpu_used: '0', hostname: '', cpus: 0 });
const back = async function () {
    router.back();
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
</script>
<template>
    <div class="back_header" data-tauri-drag-region>
        <div class="back_left">
            <el-row style="height: 35px;text-align: center;" justify>
                <el-col :span="12">
                    <div class="demo-type" style="position: relative;height: 100%;">
                        <div @click="back">
                            <el-icon size="30">
                                <Back />
                            </el-icon>
                        </div>

                        <div>

                        </div>
                        <div>

                        </div>
                    </div>
                </el-col>
                <el-col :span="12">

                </el-col>
            </el-row>
        </div>
        <div class="back_center" >
            {{ title }}
        </div>
        <div class="back_right" >
            <el-row style="height: 35px;text-align: center;" justify>
                <el-col :span="12">
                </el-col>
                <el-col :span="12">
                    <div class="demo-type-1">
                        <div @click="handleWinMin" v-show="showWinMin">
                            <el-tooltip class="box-item" effect="dark" content="最小化" placement="top-start">
                                <el-icon>
                                    <Minus />
                                </el-icon>
                            </el-tooltip>
                        </div>
                        <div @click="windowChange" v-show="showWinChange">
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
                        <div @click="windowHide" v-show="showWinHide">
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
.back_left,.back_right {
    width: 250px;
}
.back_center {
    width: calc(100vw - 500px);
    display: flex;
    justify-content: center;
    text-align: center;
    height: 35px; 
    align-items: center; 
}
.back_header {
    height: 35px;
    text-align: center;
    width: 100vw;
    display: flex;
    background-color: var(--main--header--bg-color);
}
.demo-type-1 {
    position: relative;
    height: 100%;
    display: flex;
    flex-wrap: wrap-reverse;
    justify-content: flex-end; /* 让子元素靠右对齐 */
    align-items: center;
}
.demo-type-1>div{
    flex-basis: calc(33.33%); 
    justify-content: center;
    align-items: center;
    height: 100%;
    text-align: center;
    display: flex; //将子元素设置为弹性容器。
    justify-content: center; //使子元素在水平方向上居中对齐。
    align-items: center; //使子元素在垂直方向上居中对齐。
    //background-color: #3a3d3f; /* 鼠标悬停时的背景色 */
    transition: background-color 0.3s;
    /* 平滑背景色过渡 */
    //padding-left: 3px;
    //padding-right: 3px;
    //border: 1px solid transparent;
}

.demo-type {
    display: flex;
    justify-content: center;
    align-items: center;
}
.demo-type>div{
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
    //border: 1px solid transparent;
}





// .demo-type>div:hover {
//     //background-color: #afafaf; /* 鼠标悬停时的背景色 */
//     background-color: rgba(124, 124, 124, 0.479);
//     backdrop-filter: blur(10px);
//     box-shadow: 0 0 10px rgba(0, 0, 0, 0.5);
//     border-radius: 1px;
// }

</style>