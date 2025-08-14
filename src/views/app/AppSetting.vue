<script setup lang="ts">
import { onMounted, reactive } from 'vue'
import { Check, Close } from '@element-plus/icons-vue'
// When using the Tauri API npm package:
import { invoke } from '@tauri-apps/api/core'
import { ResponseData, SettingIdValueName, SettingMap } from '@/entity/entitys';
import { useI18n } from "vue-i18n";
const { t, locale } = useI18n()
import { appConfigInfo } from "@/store/appConfigInfo";
const userStore = appConfigInfo();

const autostart = reactive(new SettingIdValueName<Boolean>('', '', false))
const update_reminder = reactive(new SettingIdValueName<Boolean>('', '', false))
const language = reactive(new SettingIdValueName<string>('', '', ''))

onMounted(() => {
    if (userStore.appConfig.length < 1) {
        query_setting_list();
    }
    if (userStore.appConfig.length > 1) {
        show_setting()
    }
})
function show_setting() {
    let data = userStore.appConfig as Array<SettingMap>;
    for (let i = 0; i < data.length; i++) {
        const element = data[i];
        if (element.app_setting_item_name == "autostart") {
            autostart.app_setting_item_name = element.app_setting_item_name;
            autostart.id = element.id;
            let str = element.app_setting_item_value
            autostart.app_setting_item_value = (str === "true" ? true : false);
            console.log("autostart.app_setting_item_value", autostart.app_setting_item_value)
        } else if (element.app_setting_item_name == "update_reminder") {
            update_reminder.app_setting_item_name = element.app_setting_item_name;
            update_reminder.id = element.id;
            let str = element.app_setting_item_value
            update_reminder.app_setting_item_value = (str === "true" ? true : false);
        } else if (element.app_setting_item_name == "language") {
            language.app_setting_item_name = element.app_setting_item_name;
            language.id = element.id;
            language.app_setting_item_value = element.app_setting_item_value;
        }
    }
}
function query_setting_list() {
    invoke('query_setting_list').then((res): void => {
        let res_ = res as ResponseData<Array<SettingMap>>
        let data = res_.data as Array<SettingMap>;
        userStore.setAppConfig(data);

    })
}
function setting_change(val: string, id: string) {
    invoke('updatate_setting', { id: id, appSettingItemValue: String(val), appSettingtemName: '' }).then((res): void => {
        console.log(res);
        query_setting_list()
    })
}

function language_change() {
    locale.value = language.app_setting_item_value;
    invoke('updatate_setting', { id: language.id, appSettingItemValue: String(language.app_setting_item_value), appSettingtemName: 'language' }).then((res): void => {
        console.log(res);
        query_setting_list()
    })

}
const language_options = [
    {
        value: 'zh_CN',
        label: '简体中文',
        disabled: false,
    },
    {
        value: 'zh_TW',
        label: '繁体中文（中国台湾）',
        disabled: false,
    },
    {
        value: 'zh_HK',
        label: '繁体中文（中国香港）',
        disabled: false,
    },
    {
        value: 'en',
        label: 'English',
        disabled: false,
    }
]
</script>
<template>
    <div class="app-setting">
        <div class="app-setting-title">
            <el-row :gutter="20">
                <el-col :span="24">
                    通用设置
                </el-col>
            </el-row>
        </div>
        <div class="app-setting-item">
            <el-row :gutter="20">
                <el-col :span="8">
                    开机自启动
                </el-col>
                <el-col :span="4">
                    <div class="grid-content ep-bg-purple" />
                </el-col>
                <el-col :span="4">
                    <div class="grid-content ep-bg-purple" />
                </el-col>
                <el-col :span="8">
                    <el-switch v-model="autostart.app_setting_item_value"
                        @click="setting_change(String(autostart.app_setting_item_value), autostart.id)"
                        style="margin-left: 24px" inline-prompt :active-icon="Check" :inactive-icon="Close" />
                </el-col>
            </el-row>
            <el-row :gutter="20">
                <el-col :span="24">
                    <el-text class="mx-1" type="info">可能影响开机速度</el-text>
                </el-col>
            </el-row>
        </div>
        <div class="app-setting-item">
            <el-row :gutter="20">
                <el-col :span="8">
                    软件更新提醒
                </el-col>
                <el-col :span="4">
                    <div class="grid-content ep-bg-purple" />
                </el-col>
                <el-col :span="4">
                    <div class="grid-content ep-bg-purple" />
                </el-col>
                <el-col :span="8">
                    <el-switch v-model="update_reminder.app_setting_item_value"
                        @click="setting_change(String(update_reminder.app_setting_item_value), update_reminder.id)"
                        style="margin-left: 24px" inline-prompt :active-icon="Check" :inactive-icon="Close" />
                </el-col>
            </el-row>
            <el-row :gutter="20">
                <el-col :span="24">
                    <el-text class="mx-1" type="info">
                    </el-text>
                </el-col>
            </el-row>
        </div>
        <div class="app-setting-item">
            <el-row :gutter="20">
                <el-col :span="8">
                    切换语言
                </el-col>
                <el-col :span="4">
                    <div class="grid-content ep-bg-purple" />
                </el-col>
                <el-col :span="4">
                    <div class="grid-content ep-bg-purple" />
                </el-col>
                <el-col :span="8">
                    <el-select v-model="language.app_setting_item_value" @change="language_change" placeholder="Select"
                        style="width: 100px">
                        <el-option v-for="item in language_options" :key="item.value" :label="item.label"
                            :value="item.value" :disabled="item.disabled" />
                    </el-select>
                </el-col>
            </el-row>
            <el-row :gutter="20">
                <el-col :span="24">
                    <el-text class="mx-1" type="info">
                    </el-text>
                </el-col>
            </el-row>
        </div>
        <div class="app-setting-title">
            <el-row :gutter="20">
                <el-col :span="24">
                    网络
                </el-col>
            </el-row>
        </div>
        <div class="app-setting-item">
            <el-row :gutter="20">
                <el-col :span="8">
                    <el-button key="clear" type="info" text>
                        网络检测
                    </el-button>
                </el-col>
                <el-col :span="4">
                    <div class="grid-content ep-bg-purple" />
                </el-col>
                <el-col :span="4">
                    <div class="grid-content ep-bg-purple" />
                </el-col>
                <el-col :span="8">

                </el-col>
            </el-row>
            <el-row :gutter="20">
                <el-col :span="24">

                </el-col>
            </el-row>
        </div>
        <div class="app-setting-title">
            <el-row :gutter="20">
                <el-col :span="24">
                    其他设置
                </el-col>
            </el-row>
        </div>
        <div class="app-setting-item">
            <el-row :gutter="20">
                <el-col :span="8">
                    <el-button key="clear1" type="info" text>
                        清除缓存
                    </el-button>
                </el-col>
                <el-col :span="4">
                    <div class="grid-content ep-bg-purple" />
                </el-col>
                <el-col :span="4">
                    <div class="grid-content ep-bg-purple" />
                </el-col>
                <el-col :span="8">

                </el-col>
            </el-row>
            <el-row :gutter="20">
                <el-col :span="24">

                </el-col>
            </el-row>
        </div>
        <div class="app-setting-item">
            <el-row :gutter="20">
                <el-col :span="8">
                    <el-button key="clear" type="info" text>
                        清除浏览器缓存
                    </el-button>
                </el-col>
                <el-col :span="4">
                    <div class="grid-content ep-bg-purple" />
                </el-col>
                <el-col :span="4">
                    <div class="grid-content ep-bg-purple" />
                </el-col>
                <el-col :span="8">

                </el-col>
            </el-row>
            <el-row :gutter="20">
                <el-col :span="24">

                </el-col>
            </el-row>
        </div>
    </div>
</template>
  
<style lang="scss" scoped>
.app-setting {
    width: 100%;
    -webkit-scrollbar: none;
}

.app-setting>div>el-row {
    width: 100%;
    text-align: center;
    display: flex; //将子元素设置为弹性容器。
    justify-content: center; //使子元素在水平方向上居中对齐。
    align-items: center; //使子元素在垂直方向上居中对齐。
}

.app-setting>div {
    //display: flex;
    //justify-content: center; //使子元素在水平方向上居中对齐。
    //align-items: center; //使子元素在垂直方向上居中对齐。
    margin-bottom: 12px;
    border-radius: 5px;
    width: 100%;
    height: 100%;
}

.app-setting>div:not(.app-setting-title) {
    margin-left: 10px;
}

.app-setting>div:not(.app-setting-title):hover {
    // background-color: rgba(231, 231, 231, 0.479);
    // backdrop-filter: blur(20px);
    // box-shadow: 0 0 10px rgba(32, 32, 32, 0.5);
    // border-radius: 2px;
}


.app-setting>div:not(.app-setting-title)>el-row>el-col {
    width: 100%;
    text-align: center;
    display: flex; //将子元素设置为弹性容器。
    justify-content: center; //使子元素在水平方向上居中对齐。
    align-items: center; //使子元素在垂直方向上居中对齐。
}


.el-row:last-child {
    margin-bottom: 0;
}

// .el-col {
//     border-radius: 4px;
// }

// .grid-content {
//     border-radius: 4px;
//     min-height: 36px;
// }
.el-button {
    box-shadow: 0 0px 0px rgba(0, 0, 0, 0.2);
}

.el-button:hover {
    background-color: rgba(231, 231, 231, 0.479);
    backdrop-filter: blur(20px);
    box-shadow: 0 0 10px rgba(32, 32, 32, 0.5);
    border-radius: 2px;
}
</style>
  