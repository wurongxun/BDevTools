<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import { Back, Minus, Close } from "@element-plus/icons-vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state';
import { emitTo } from '@tauri-apps/api/event';
import WindowMaximizationIcon from "@/components/Icon/WindowMaximizationIcon.vue";
import WindowRestoreIcon from "@/components/Icon/WindowRestoreIcon.vue";
import router from '@/router';
import { listen } from '@tauri-apps/api/event';

const route = useRoute();
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
  showWinBack: {
    type: Boolean,
    default: false,
  },
});

// 返回上一页
const back = async function () {
  router.back();
};

// 切换窗口最大化状态
const windowChange = async function () {
  const window = getCurrentWindow();
  const isCurrentlyMaximized = await window.isMaximized();

  await window.toggleMaximize();
  await saveWindowState(StateFlags.ALL);
  visible.value = !visible.value;

  // 如果从最大化恢复，则居中
  if (isCurrentlyMaximized) {
    setTimeout(async () => {
      await window.center();
      await saveWindowState(StateFlags.POSITION | StateFlags.SIZE);
    }, 150);
  }
};

// 最小化窗口
const handleWinMin = async () => {
  await getCurrentWindow().minimize();
};

// 隐藏窗口
const windowClose = async () => {
  let currentWindow = await getCurrentWindow();
  if (currentWindow) {
    console.log('windowClose', currentWindow.label);
    await emitTo(currentWindow.label, 'close', { loggedIn: true, token: 'authToken' });
  }
  //await getCurrentWindow().close();
};

const appWindow = getCurrentWindow();

onMounted(() => {
  document.getElementById('titlebar')?.addEventListener('mousedown', (e) => {
    if (e.buttons === 1) {
      e.detail === 2
        ? windowChange()
        : appWindow.startDragging();
    }
  });
});

// 计算左侧宽度
const leftWidth = computed(() => {
  return props.showWinBack ? 40 : 0;
});

// 计算右侧宽度
const rightWidth = computed(() => {
  let width = 0;
  if (props.showWinMin) width += 40;
  if (props.showWinChange) width += 40;
  if (props.showWinHide) width += 40;
  return width;
});

// 平衡左右宽度以保持居中对称
const balancedWidth = computed(() => {
  return Math.max(leftWidth.value, rightWidth.value);
});
</script>

<template>
  <div class="back_header">
    <div class="back_left" :style="{ width: rightWidth + 'px' }">
      <div v-if="showWinBack" class="control-button" @click="back">
        <el-icon size="20">
          <Back />
        </el-icon>
      </div>
      <!-- 占位元素 -->
      <div v-else style="width: 35px; height: 35px;"></div>
    </div>

    <div id="titlebar" class="back_center">
      {{ title }}
    </div>

    <div class="back_right" :style="{ width: rightWidth + 'px' }">
      <div v-if="showWinMin" class="control-button" @click="handleWinMin" :title="$t('minimize')">
        <el-icon size="16">
          <Minus />
        </el-icon>
      </div>
      <div v-if="showWinChange" class="control-button" @click="windowChange"
        :title="visible ? $t('restore') : $t('maximize')">
        <el-icon size="16">
          <WindowRestoreIcon v-if="visible" />
          <WindowMaximizationIcon v-else />
        </el-icon>
      </div>
      <div v-if="showWinHide" class="control-button close-button" @click="windowClose" :title="$t('close')">
        <el-icon size="16">
          <Close />
        </el-icon>
      </div>
      <!-- 占位元素 -->
      <div v-if="!showWinMin && !showWinChange && !showWinHide" style="width: 105px; height: 35px;"></div>
    </div>
  </div>
</template>
<style lang="scss" scoped>
.back_header {
  height: 25px;
  width: 100vw;
  display: flex;
  align-items: center;
  background-color: var(--main--header--bg-color);
  user-select: none;
  margin: 0 10px;
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
  width: 25px;
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

@media (max-width: 768px) {
  .back_center {
    font-size: 14px;
  }
}
</style>