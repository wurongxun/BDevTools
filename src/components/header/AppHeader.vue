<template>
    <!-- <a-layout-header> -->
    <div class="header" data-tauri-drag-region>
        <div class="left">
            <span v-if="WinIcon"><icon-font :style="{ color: '#08c', fontSize: '20px', }" type="iconType" /></span>
        </div>
        <div class="center" data-tauri-drag-region>
            <span>{{ title }}</span>
        </div>
        <div class="right" data-tauri-drag-region>
            <span v-if="WinMin"><icon-font :style="{ color: '#08c', fontSize: '16px', }" @click="handleWinMin"
                    type="icon-window-min" /></span>
            <span v-if="WinMax"><icon-font :style="{ color: '#08c', fontSize: '16px', }" @click="handleWinMax2Min"
                    type="icon-zuidahua" /></span>
            <span v-if="WinClose"><icon-font :style="{ color: '#08c', fontSize: '16px', }" @click="handleWinClose"
                    type="icon-guanbi" /></span>
        </div>

    </div>
</template>
<script>
// import { createFromIconfontCN } from '@ant-design/icons-vue';
// const IconFont = createFromIconfontCN({
//     scriptUrl: '../../assets/iconfont/iconfont',
// });
import { defineComponent } from 'vue';
export default defineComponent({
    components: {

    },
    props: {
        title: {
            type: String,
            default: "",
        },
        WinMin: {
            type: Boolean,
            default: false,
        },
        WinClose: {
            type: Boolean,
            default: true,
        },
        WinMax: {
            type: Boolean,
            default: false,
        },
        WinIcon: {
            type: Boolean,
            default: false,
        },
        iconType: {
            type: String,
            default: "",
        }

    },
    setup(props, { emit }) {
        // 最小化
        const handleWinMin = async () => {
            emit('handleWinMin')
        };
        //后台运行
        const handleCancel = async () => {
            emit('handleCancel')
        };

        // 最大化/还原
        const handleWinMax2Min = async () => {
            emit('handleWinMax2Min')
        }
        //关闭
        const handleWinClose = async () => {
            emit('handleWinClose')
        }


        return {
            handleWinClose,
            handleWinMin,
            handleWinMax2Min,
            handleCancel,

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
    padding: 0 3px;
    line-height: normal;
    max-height: 30px;
    background-color: rgb(223, 223, 223);

    justify-content: space-between;
    -webkit-app-region: drag;
}

.right span {
    margin-left: 10px;
    margin-right: 5px;
    -webkit-app-region: no-drag;
}

.left {
    width: 30%;
    display: flex;
    justify-content: left;
}

.center {
    width: 40%;
    display: flex;
    justify-content: center;
}

.right {
    width: 30%;
    display: flex;
    justify-content: right;
}
</style>