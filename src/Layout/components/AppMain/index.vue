<template>
  <div class="app-main">
    <router-view v-slot="{ Component, route }">
      <transition name="fade-transform" mode="out-in">
        <keep-alive :include="cacheList" :max="20">
          <component :is="Component" :key="route.fullPath" />
        </keep-alive>
      </transition>
    </router-view>

    <!-- 返回顶部按钮 -->
    <el-backtop
      :right="30"
      :bottom="50"
      :visibility-height="300"
      target=".app-main"
    >
      <div class="back-to-top">
        <el-icon><ArrowUp /></el-icon>
      </div>
    </el-backtop>
  </div>
</template>

<script lang="ts" setup name="AppMain">
import useStore from '@/store'
import { storeToRefs } from 'pinia'
import { ArrowUp } from '@element-plus/icons-vue'

const { tagview } = useStore()
const { cacheList } = storeToRefs(tagview)
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss';

.app-main {
  width: 100%;
  height: calc(100vh - $header-height - $tage-view-height);
  overflow-y: auto;
  overflow-x: hidden;
  box-sizing: border-box;
  position: relative;

  @include theme.useTheme {
    background-color: theme.getVar('appMainBgColor');
  }

  // 美化滚动条样式
  scrollbar-width: thin;
  scrollbar-color: rgba(155, 155, 155, 0.4) transparent;

  &::-webkit-scrollbar {
    width: 8px;
    height: 8px;
  }

  &::-webkit-scrollbar-track {
    background: transparent;
    border-radius: 4px;
  }

  &::-webkit-scrollbar-thumb {
    background: rgba(155, 155, 155, 0.4);
    border-radius: 4px;
    transition: all 0.3s ease;

    &:hover {
      background: rgba(155, 155, 155, 0.6);
    }

    &:active {
      background: rgba(155, 155, 155, 0.8);
    }
  }

  &::-webkit-scrollbar-corner {
    background: transparent;
  }
}

// 深色模式下的滚动条样式
html.dark .app-main {
  scrollbar-color: rgba(255, 255, 255, 0.2) transparent;

  &::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.2);

    &:hover {
      background: rgba(255, 255, 255, 0.3);
    }

    &:active {
      background: rgba(255, 255, 255, 0.4);
    }
  }
}

// 返回顶部按钮样式
.back-to-top {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  cursor: pointer;
  transition: all 0.3s ease;

  @include theme.useTheme {
    background-color: $theme-color;
    color: #fff;
    box-shadow: 0 2px 12px 0 rgba($theme-color, 0.3);
  }

  &:hover {
    transform: translateY(-3px);
    box-shadow: 0 4px 16px 0 rgba($theme-color, 0.4);
  }

  .el-icon {
    font-size: 18px;
  }
}

// 页面过渡动画
.fade-transform-enter-active,
.fade-transform-leave-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.fade-transform-enter-from {
  opacity: 0;
  transform: translateX(-10px);
}

.fade-transform-leave-to {
  opacity: 0;
  transform: translateX(10px);
}

// 响应式优化
@media screen and (max-width: 768px) {
  .app-main {
    // 移动端使用固定高度计算，避免未定义的变量
    height: calc(100vh - 50px - 32px);

    &::-webkit-scrollbar {
      width: 4px;
    }
  }

  .back-to-top {
    width: 36px;
    height: 36px;
    right: 15px !important;
    bottom: 30px !important;

    .el-icon {
      font-size: 16px;
    }
  }
}
</style>
