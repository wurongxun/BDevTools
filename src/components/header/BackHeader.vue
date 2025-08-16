<script setup lang="ts">
import { computed, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Back, Minus, Close } from "@element-plus/icons-vue"
import { getCurrentWindow } from "@tauri-apps/api/window";
import WindowMaximizationIcon from "@/components/Icon/WindowMaximizationIcon.vue"
import WindowRestoreIcon from "@/components/Icon/WindowRestoreIcon.vue"
import router from '@/router';

const route = useRoute()
const visible = ref(false);

// Props定义
const props = defineProps({
    showWinMin: {
        type: Boolean,
        default: false,
    },
    showWinChange: {
        type: Boolean,
        default: false,
    },
    showWinHide: {
        type: Boolean,
        default: false,
    },
    title: {
        type: String,
        default: "",
    },
});

// 返回上一页
const back = async function () {
    router.back();
}

// 切换窗口最大化状态
const windowChange = async function () {
    await getCurrentWindow().toggleMaximize();
    visible.value = !visible.value;
};

// 最小化窗口
const handleWinMin = async () => {
    await getCurrentWindow().minimize();
};

// 隐藏窗口
const windowHide = async () => {
    await getCurrentWindow().hide();
};
</script>

<template>
    <div class="back_header" data-tauri-drag-region>
        <div class="back_left">
            <div class="nav-button" @click="back">
                <el-icon size="20">
                    <Back />
                </el-icon>
            </div>
        </div>
        
        <div class="back_center">
            {{ title }}
        </div>
        
        <div class="back_right">
            <div class="window-controls">
                <div 
                    v-if="showWinMin" 
                    class="control-button" 
                    @click="handleWinMin"
                    :title="$t('minimize')"
                >
                    <el-icon size="16">
                        <Minus />
                    </el-icon>
                </div>
                
                <div 
                    v-if="showWinChange" 
                    class="control-button" 
                    @click="windowChange"
                    :title="visible ? $t('restore') : $t('maximize')"
                >
                    <el-icon size="16">
                        <WindowRestoreIcon v-if="visible" />
                        <WindowMaximizationIcon v-else />
                    </el-icon>
                </div>
                
                <div 
                    v-if="showWinHide" 
                    class="control-button close-button" 
                    @click="windowHide"
                    :title="$t('close')"
                >
                    <el-icon size="16">
                        <Close />
                    </el-icon>
                </div>
            </div>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.back_header {
    height: 35px;
    width: 100vw;
    display: flex;
    align-items: center;
    background-color: var(--main--header--bg-color);
    user-select: none;
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

.nav-button {
    width: 35px;
    height: 35px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 0.2s;
    border-radius: 4px;
    margin-left: 5px;
    
    &:hover {
        background-color: rgba(128, 128, 128, 0.2);
    }
}

.window-controls {
    display: flex;
    height: 100%;
    align-items: center;
    justify-content: flex-end;
    padding-right: 5px;
}

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

// 响应式优化
@media (max-width: 768px) {
    .back_left,
    .back_right {
        width: 100px;
    }
    
    .back_center {
        font-size: 14px;
    }
}
</style>