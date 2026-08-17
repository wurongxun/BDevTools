<!-- src/components/GlobalDialog.vue -->
<template>
  <Teleport to="body">
    <div
      v-drag-pop-up
      class="global-dialog-wrapper"
      :class="{ 'is-vertical-center': verticalCenter }"
      :style="dialogStyle"
      v-if="dialogVisible"
    >
      <div class="dialog-header">
        <span class="title">{{ title }}</span>
        <span class="close-btn" @click="handleClose">✕</span>
      </div>
      <div class="dialog-body">
        <slot />
      </div>
      <div class="dialog-footer" v-if="$slots.footer">
        <slot name="footer" />
      </div>
    </div>
    <div
      class="dialog-overlay"
      v-if="dialogVisible"
      @click="closeOnOverlay ? handleClose() : undefined"
    />
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import vDragPopUp from '@/directives/dragPopUp'

interface Props {
  modelValue: boolean
  title?: string
  width?: string
  /** 可选：固定高度；不传则由内容自适应（body 限高 60vh 滚动） */
  height?: string
  /** 是否垂直居中；默认 true，传 false 则靠上（15vh）显示 */
  verticalCenter?: boolean
  closeOnOverlay?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  title: '提示',
  width: '520px',
  height: '',
  verticalCenter: true,
  closeOnOverlay: false
})

interface Emits {
  (e: 'update:modelValue', value: boolean): void
  (e: 'close'): void
}

const emit = defineEmits<Emits>()

// 默认水平垂直居中；宽高均可通过 prop 定制，height 不传时内容自适应
const dialogStyle = computed(() => ({
  width: props.width,
  ...(props.height ? { height: props.height } : {})
}))

const dialogVisible = ref<boolean>(props.modelValue)

watch(() => props.modelValue, (val: boolean) => {
  dialogVisible.value = val
})

watch(dialogVisible, (val: boolean) => {
  emit('update:modelValue', val)
  if (!val) emit('close')
})

const handleClose = (): void => {
  dialogVisible.value = false
}
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as variables;
@use '@/styles/theme.scss';

.global-dialog-wrapper {
  position: fixed;
  top: 15vh;
  left: 50%;
  transform: translateX(-50%);
  width: 520px;
  border-radius: 4px;
  // 需高于布局层（侧边栏 z-index:1000、分割线 z-index:1001），
  // 但低于 Element Plus popper 基准层（2000+），保证弹窗内下拉菜单正常置顶
  z-index: 1501;
  display: flex;
  flex-direction: column;
  // 固定高度时 body 内部滚动，裁切超出圆角的内容
  overflow: hidden;

  @include theme.useTheme {
    background: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderColor');
  }

  // 阴影固定用深色半透明：弹窗悬浮在暗色遮罩上，亮色阴影会形成白边光晕
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.35);

  // 垂直居中模式；拖拽时指令会改写 left/top 并置 transform: none，不受影响
  &.is-vertical-center {
    top: 50%;
    transform: translate(-50%, -50%);
  }
}

.dialog-header {
  height: variables.$header-height;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 6px 0 12px;
  cursor: move;
  user-select: none;

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.dialog-header .title {
  font-weight: 600;

  @include theme.useTheme {
    font-size: theme.getVar('fontSizeSmall');
    color: theme.getVar('textColor');
  }
}

.dialog-header .close-btn {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  border-radius: 3px;
  transition: all 0.2s;

  @include theme.useTheme {
    font-size: theme.getVar('fontSizeSmall');
    color: theme.getVar('textSecondary');

    &:hover {
      color: theme.getVar('textColor');
      background: theme.getVar('hoverColor');
    }
  }
}

.dialog-body {
  flex: 1;
  min-height: 0;
  padding: 12px 16px;
  max-height: 60vh;
  overflow-y: auto;

  @include theme.useTheme {
    font-size: theme.getVar('fontSizeSmall');
    color: theme.getVar('textColor');
  }
}

.dialog-footer {
  padding: 1px 3px;
  text-align: right;
  align-content: center;
  height: variables.$header-height;
  @include theme.useTheme {
    border-top: 1px solid theme.getVar('borderColor');
  }
}

.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.4);
  z-index: 1500;
}
</style>
