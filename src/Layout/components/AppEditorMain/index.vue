<template>
  <div class="app-main" :style="{ height: appMainHeight }">
    <el-header class="app-header"><EditorTab></EditorTab></el-header>
    <el-main class="app-main-content">
      <file-editor></file-editor>
      <!-- 返回顶部按钮 -->
      <el-backtop
        :right="30"
        :bottom="50"
        :visibility-height="300"
        target=".app-main-content"
      >
        <div class="back-to-top">
          <el-icon><ArrowUp /></el-icon>
        </div>
      </el-backtop>
    </el-main>
  </div>
</template>


<script lang="ts" setup name="AppEditorMain">
import useStore from '@/store'
import { storeToRefs } from 'pinia'
import { ArrowUp } from '@element-plus/icons-vue'
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import TagView from '../AppNavbar/components/TagView.vue'
import FileEditor from '../../../components/FileEditor/FileEditor.vue'
import EditorTab from '@/Layout/components/AppEditorMain/EditorTab.vue'

const { tagview } = useStore()
const { cacheList } = storeToRefs(tagview)

// 动态计算主内容区域高度
const appMainHeight = ref('calc(100vh - var(--header-height) - var(--tage-view-height))')
const appMainContent = ref('calc(100vh - var(--header-height) - var(--tage-view-height))')

const updateHeight = () => {
  const navbar = document.querySelector('.topbar-box')
  //const tagView = document.querySelector('.idea-tab-bar');
  //tab-bar-container
  const tagView = document.querySelector('.tab-bar-container');
  let headerHeight = 32
  let tagHeight = 32

  // 类型守卫：确保元素是 HTMLElement
  if (navbar instanceof HTMLElement) {
    headerHeight = navbar.offsetHeight
  }
  if (tagView instanceof HTMLElement) {
    tagHeight = tagView.offsetHeight
  }

  //appMainHeight.value = `calc(100vh - ${headerHeight}px - ${tagHeight}px)`
  appMainHeight.value = `calc(100vh - ${headerHeight}px)`

  appMainContent.value= `calc(100vh - ${headerHeight}px - ${tagHeight}px)`

  console.log('[AppMain] Height updated:', appMainHeight.value, '| Header:', headerHeight, 'TagView:', tagHeight)
}

// 窗口 resize 处理函数（带防抖）
let resizeTimer: ReturnType<typeof setTimeout> | null = null

const handleResize = async () => {
  if (resizeTimer) {
    clearTimeout(resizeTimer)
  }

  // 使用防抖，避免频繁触发
  resizeTimer = setTimeout(async () => {
    // 更新高度
    updateHeight()

    // 等待 DOM 更新
    await nextTick()

    // 触发自定义事件通知子组件重新渲染
    const event = new CustomEvent('window-resize', {
      detail: {
        timestamp: Date.now(),
        width: window.innerWidth,
        height: window.innerHeight,
        reason: 'window-resize'
      }
    })
    window.dispatchEvent(event)

    console.log('[AppMain] Window resized, height updated to:', appMainHeight.value)
  }, 200)
}
//处理侧边栏切换事件（重命名为 layout-update）
const handleLayoutUpdate = async (event: CustomEvent) => {
  console.log('[AppMain] Layout update detected:', event.detail)

  // 布局变化时重新计算高度
  await nextTick()
  updateHeight()

  // 触发子组件重新渲染
  const resizeEvent = new CustomEvent('window-resize', {
    detail: {
      timestamp: Date.now(),
      reason: event.detail.reason || 'layout-update',
      isCollapse: event.detail.isCollapse,
      sidebarWidth: event.detail.sidebarWidth,
      mainWidth: event.detail.mainWidth
    }
  })
  window.dispatchEvent(resizeEvent)
}

onMounted(() => {
  // 初始化时计算高度
  updateHeight()

  // 监听窗口 resize 事件
  window.addEventListener('resize', handleResize)

  // 也监听 orientationchange 事件（移动端旋转屏幕）
  window.addEventListener('orientationchange', handleResize)

  // 监听布局更新事件（替代原来的 sidebar-toggle）
  window.addEventListener('layout-update', handleLayoutUpdate as EventListener)

  console.log('[AppMain] Mounted, initial height:', appMainHeight.value)
})

onUnmounted(() => {
  // 清理事件监听器
  window.removeEventListener('resize', handleResize)
  window.removeEventListener('orientationchange', handleResize)
  window.removeEventListener('layout-update', handleLayoutUpdate as EventListener)

  if (resizeTimer) {
    clearTimeout(resizeTimer)
  }

  console.log('[AppMain] Unmounted, event listeners cleaned up')
})
</script>

<style scoped lang="scss">@use '@/styles/variables' as *;
@use '@/styles/theme';

.app-main {
  width: 100%;
  height: 100%;
  margin: 0;
  padding: 0;
  overflow: hidden;
  box-sizing: border-box;
  position: relative;
  display: flex;
  flex-direction: column;

  @include theme.useTheme {
    background-color: theme.getVar('appMainBgColor');
  }
}

.app-header {
  margin: 0;
  padding: 0;
  height: auto !important;
  min-height: unset;
  flex-shrink: 0;
}

.app-main-content {
  margin: 0;
  padding: 0;
  overflow: visible;
  flex: 1;
  min-height: 0;
  // 移除滚动条相关样式，因为滚动由外层处理
}


// 深色模式下的滚动条样式
//html.dark .app-main-content {
//  scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
//
//  &::-webkit-scrollbar-thumb {
//    background: rgba(255, 255, 255, 0.2);
//
//    &:hover {
//      background: rgba(255, 255, 255, 0.3);
//    }
//
//    &:active {
//      background: rgba(255, 255, 255, 0.4);
//    }
//  }
//}

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
//@media screen and (max-width: 768px) {
//  .app-main {
//    &::-webkit-scrollbar {
//      width: 4px;
//    }
//  }
//
//  .back-to-top {
//    width: 36px;
//    height: 36px;
//    right: 15px !important;
//    bottom: 30px !important;
//
//    .el-icon {
//      font-size: 16px;
//    }
//  }
//}
</style>
