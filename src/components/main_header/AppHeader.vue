<template>
    <!-- <a-layout-header> -->
    <div class="header" data-tauri-drag-region>
        <div class="left">
            <span><a-image :width="24"  :preview="false" />
            </span>
            <span></span>
        </div>
        <div class="right">
            <span><icon-font :style="{ color: '#08c', fontSize: '16px', }" @click="openAboutWin"
                    type="icon-guanyu" /></span>
            <span><icon-font :style="{ color: '#08c', fontSize: '16px', }" @click="handleWinMin"
                    type="icon-window-min" /></span>
            <span><icon-font :style="{ color: '#08c', fontSize: '16px', }" @click="handleWinMax2Min"
                    type="icon-zuidahua" /></span>
            <span><icon-font :style="{ color: '#08c', fontSize: '16px', }" @click="showModal"
                    type="icon-guanbi" /></span>
        </div>

    </div>
    <!-- </a-layout-header> -->
    <!-- <a-modal v-model:visible="visible" :closable="true" :centered="true" width="500px" title="退出" @cancel="close_modal"
        @ok="handleOk">
        <template #footer>
            <a-button key="back" @click="handleCancel">后台运行</a-button>
            <a-button key="submit" type="primary" :loading="loading" @click="handleOk">退出应用</a-button>
        </template>
        <br />
        <br />
        <p style="text-align:center">确认退出应用程序吗？</p>
        <br />
        <br />
    </a-modal> -->
</template>
<script>
//import { createFromIconfontCN } from '@ant-design/icons-vue';
/* eslint-disable */
// const IconFont = createFromIconfontCN({
//     scriptUrl: '../../assets/iconfont/iconfont',
// });
/* eslint-disable */
import { invoke } from '@tauri-apps/api/core';
//import { appWindow, WebviewWindow, getAll } from '@tauri-apps/api/window'
import { defineComponent, ref } from 'vue';
import { exit } from '@tauri-apps/plugin-process';
export default defineComponent({
    components: {

    },
    setup() {
        //const logo = ref(require("../../assets/image/logo.png"))
        const a = ref("a");
        const visible = ref(false);
        const showModal = () => {
            visible.value = true;
        };
        const handleOk = async (e) => {
            visible.value = false;
            // if (appWindow.label.indexOf('main') > -1) {
            //     await exit();
            // }
        };
        // 最小化
        const handleWinMin = async () => {
            // await appWindow.minimize()
        };
        //后台运行
        const handleCancel = async () => {
            visible.value = false;
            // await appWindow.hide()
        };

        // 最大化/还原
        const handleWinMax2Min = async () => {
            // const resizable = await appWindow.isResizable()
            // if (!resizable) return
            // await appWindow.toggleMaximize()
        }
        const close_modal = async () => {
            visible.value = false;
        }
        //关于
        const openAboutWin = () => {
            let all_Label = getAll();
            for (let i = 0; i < all_Label.length; i++) {
                let window_ = all_Label[i];
                // if ("about-window" == window_.label) {
                //     //有窗口，重新创建,聚焦并显示在最顶层
                //     WebviewWindow.getByLabel("about-window").setFocus()
                //     return;
                // }
            }
            invoke('about').then((res) => {
                console.log(res)
            });

        }
        return {
            a,
            visible,
            handleOk,
            showModal,
            handleWinMin,
            // handleWinClose,
            handleWinMax2Min,
            handleCancel,
            openAboutWin,
            close_modal
        }
    }
})
</script>
<style scoped>
.header {
    height: 30px;
    display: flex;
    /*align-items 属性定义flex子项在flex容器的当前行的侧轴（纵轴）方向上的对齐方式。 */
    align-items: center;
    /* 左右边距 */
    padding: 0 0px;
    line-height: normal;
    max-height: 30px;
    background-color: red;

    justify-content: space-between;
    -webkit-app-region: drag;
}

.right span {
    margin-left: 10px;
    margin-right: 5px;
    -webkit-app-region: no-drag;
}

.left span {
    margin-left: 3px;
    -webkit-app-region: no-drag;
}
</style>