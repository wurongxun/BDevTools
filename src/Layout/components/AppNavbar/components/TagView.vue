<template>
  <div class="idea-tab-bar">
    <div class="tab-scroll-container" ref="scrollContainer">
      <transition-group name="idea-tab" tag="div" class="tabs-wrapper">
        <div
          v-for="tag in tagViewList"
          :key="tag.path"
          class="idea-tab"
          :class="{
            'is-active': isActive(tag),
            'is-pinned': tag.affix
          }"
          @click="navigateToTab(tag)"
        >
          <span class="tab-icon">
            <component :is="getTabIcon(tag)" />
          </span>
          <span class="tab-text">{{ truncateText(tag.title) }}</span>
          <el-icon
            v-if="!tag.affix"
            class="close-icon"
            @click.stop="removeTab(tag)"
          >
            <Close />
          </el-icon>
        </div>
      </transition-group>
    </div>

    <!-- 滚动控制按钮 -->
    <div v-if="showScrollControls" class="scroll-controls">
      <div class="control-btn left" @click="scrollLeft" title="向左滚动">
        <el-icon><ArrowLeft /></el-icon>
      </div>
      <div class="control-btn right" @click="scrollRight" title="向右滚动">
        <el-icon><ArrowRight /></el-icon>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import useStore from '@/store/index'
import { computed, nextTick, watch, ref, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  Close,
  ArrowLeft,
  ArrowRight,
  Document,
  HomeFilled,
  Setting,
  InfoFilled,
  QuestionFilled
} from '@element-plus/icons-vue'
import ToolUtils from '@/utils/ToolUtils'
import type { ITagItem } from '@/Types/TagView'
import { ElMessageBox } from 'element-plus'
import NotificationUtil from '@/utils/NotificationUtil'
import { useResizeObserver } from '@vueuse/core'

const route = useRoute()
const router = useRouter()
const { tagview, noteTem } = useStore()
const scrollContainer = ref<HTMLElement | null>(null)
const showScrollControls = ref(false)
const tabBarController = ref<HTMLElement | null>(null)

watch(
  () => router.currentRoute.value,
  (newValue) => {
    if (route.meta.keepAlive && route.name) {
      tagview.addCacheList(route.name as string)
    }
    if (!route.meta.noTagView) {
      let path = newValue.path
      if (ToolUtils.notEmptyObject(newValue.params)) {
        path = newValue.matched[newValue.matched.length - 1].path
      }
      tagview.addTagView({
        name: newValue.name as string,
        path,
        title: newValue.meta.title as string,
        query: newValue.query,
        params: newValue.params,
        id: route.meta.id as string,
        filePath: route.meta.filePath as string
      })
    }
  },
  { immediate: true }
)

const tagViewList = computed<ITagItem[]>(() => tagview.tagViewList)

const navigateToTab = (tag: ITagItem) => {
  const toPath = getToPath(tag)
  router.push(toPath)
}

const removeTab = async (tag: ITagItem) => {
  const noteId = tag.name
  const noteData = noteTem.getNoteData(noteId)

  if (noteData && noteData.tempContent && noteData.tempContent !== noteData.content) {
    try {
      await ElMessageBox.confirm(
        `标签页 "${tag.title}" 有未保存的内容，是否保存更改?`,
        "保存提醒",
        {
          confirmButtonText: "保存",
          cancelButtonText: "不保存",
          type: "warning",
        }
      )
      NotificationUtil.info("请切换到对应标签页手动保存内容")
    } catch {
      console.log("用户选择不保存更改，继续关闭标签页")
    }
  }

  tagview.removeTagView(tag)
  if (isActive(tag)) {
    const endRoute = tagview.tagViewList[tagview.tagViewList.length - 1] as ITagItem
    if (endRoute) {
      router.push({ path: endRoute.path })
    } else {
      router.push({ path: '/' })
    }
  }
}

const isActive = (tag: ITagItem): boolean => {
  if (route.meta.noTagView && route.meta.activeMenu) {
    return route.meta.activeMenu === tag.path
  }
  if (ToolUtils.notEmptyObject(tag)) {
    let path = route.matched[route.matched.length - 1].path
    return path === tag.path
  }
  return route.path === tag.path
}

const getToPath = (tag: ITagItem) => {
  if (ToolUtils.notEmptyObject(tag.params)) {
    return {
      name: tag.name,
      params: tag.params
    }
  }
  return {
    path: tag.path,
    query: tag.query
  }
}

// 根据路径获取合适的图标
const getTabIcon = (tag: ITagItem) => {
  const path = tag.path.toLowerCase()

  if (path === '/home' || path === '/') {
    return HomeFilled
  } else if (path.includes('setting') || path.includes('config')) {
    return Setting
  } else if (path.includes('help') || path.includes('about')) {
    return InfoFilled
  } else if (path.includes('question') || path.includes('faq')) {
    return QuestionFilled
  }

  return Document
}

// 截断过长的文本
const truncateText = (text: string, maxLength = 20) => {
  if (text.length <= maxLength) return text
  return text.substring(0, maxLength - 3) + '...'
}

// 滚动控制
const scrollLeft = () => {
  if (scrollContainer.value) {
    scrollContainer.value.scrollBy({ left: -150, behavior: 'smooth' })
  }
}

const scrollRight = () => {
  if (scrollContainer.value) {
    scrollContainer.value.scrollBy({ left: 150, behavior: 'smooth' })
  }
}

// 检查是否需要显示滚动控件
const checkScrollControls = () => {
  nextTick(() => {
    if (scrollContainer.value) {
      showScrollControls.value = scrollContainer.value.scrollWidth > scrollContainer.value.clientWidth
    }
  })
}

// 自动滚动到激活的标签
const scrollToActiveTab = () => {
  nextTick(() => {
    const activeTab = document.querySelector('.idea-tab.is-active')
    if (activeTab && scrollContainer.value) {
      const container = scrollContainer.value
      const tab = activeTab as HTMLElement
      const containerRect = container.getBoundingClientRect()
      const tabRect = tab.getBoundingClientRect()

      // 只有当标签不完全可见时才滚动
      if (tabRect.left < containerRect.left || tabRect.right > containerRect.right) {
        tab.scrollIntoView({
          behavior: 'smooth',
          inline: 'center',
          block: 'nearest'
        })
      }
    }
  })
}

// 使用 ResizeObserver 监听容器大小变化（不影响其他组件）
useResizeObserver(tabBarController, (entries) => {
  for (const entry of entries) {
    if (entry.target === tabBarController.value) {
      setTimeout(() => {
        checkScrollControls()
        scrollToActiveTab()
      }, 100)
      break
    }
  }
})

onMounted(() => {
  checkScrollControls()
})

onUnmounted(() => {
  // ResizeObserver 会自动清理，无需手动操作
})

watch(
  () => route.path,
  () => {
    setTimeout(() => {
      scrollToActiveTab()
      checkScrollControls()
    }, 100)
  }
)

watch(
  () => tagViewList.value.length,
  () => {
    checkScrollControls()
  }
)
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss';

.idea-tab-bar {
  display: flex;
  align-items: center;
  height: 32px;
  position: relative;
  overflow: hidden;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-bottom: 1px solid theme.getVar('borderColor');
  }

  .tab-scroll-container {
    flex: 1;
    overflow-x: auto;
    overflow-y: hidden;
    white-space: nowrap;
    scrollbar-width: none;
    -ms-overflow-style: none;
    min-width: 0;

    &::-webkit-scrollbar {
      display: none;
    }

    .tabs-wrapper {
      display: inline-flex;
      height: 100%;
      align-items: center;
      padding-left: 4px;
    }
  }

  .idea-tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    height: 28px;
    margin: 0 1px;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s ease;
    user-select: none;
    min-width: 100px;
    max-width: 200px;
    flex-shrink: 0;

    @include theme.useTheme {
      color: theme.getVar('textColor');
      background-color: theme.getVar('bgColor');
      border: 1px solid transparent;
    }

    border-top-left-radius: 3px;
    border-top-right-radius: 3px;

    &:hover {
      @include theme.useTheme {
        background-color: theme.getVar('hoverColor');
        color: theme.getVar('textColor');
      }

      .close-icon {
        opacity: 1;
        visibility: visible;
      }
    }

    &.is-active {
      font-weight: 500;
      position: relative;
      z-index: 1;

      @include theme.useTheme {
        background-color: theme.getVar('appMainBgColor');
        color: $theme-color;
        border-color: theme.getVar('borderColor');
        border-bottom-color: transparent;
      }

      &::after {
        content: '';
        position: absolute;
        bottom: -1px;
        left: 0;
        right: 0;
        height: 2px;
        background: linear-gradient(90deg, $theme-color 0%, lighten($theme-color, 10%) 100%);
      }

      .close-icon {
        opacity: 1;
        visibility: visible;

        &:hover {
          @include theme.useTheme {
            background-color: rgba($theme-color, 0.2);
          }
          color: #ff6b6b;
        }
      }
    }

    &.is-pinned {
      cursor: default;

      .close-icon {
        display: none;
      }
    }

    .tab-icon {
      font-size: 13px;
      flex-shrink: 0;
      opacity: 0.8;

      svg {
        width: 13px;
        height: 13px;
      }
    }

    .tab-text {
      flex: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      line-height: 1;
      min-width: 0;
    }

    .close-icon {
      font-size: 11px;
      padding: 2px;
      border-radius: 3px;
      opacity: 0;
      visibility: hidden;
      transition: all 0.15s ease;
      flex-shrink: 0;

      &:hover {
        @include theme.useTheme {
          background-color: theme.getVar('shadowColor');
        }
      }
    }
  }

  .scroll-controls {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 4px;
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    z-index: 2;
    flex-shrink: 0;

    @include theme.useTheme {
      background: linear-gradient(to right,
        theme.getVar('bgColor') 0%,
        theme.getVar('bgColor') 80%,
        transparent 100%);
    }

    .control-btn {
      width: 24px;
      height: 24px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 3px;
      cursor: pointer;
      transition: all 0.15s ease;
      flex-shrink: 0;

      @include theme.useTheme {
        color: theme.getVar('textColor');
      }

      &:hover {
        @include theme.useTheme {
          background-color: $theme-color;
          color: #fff;
        }
      }

      &:active {
        transform: scale(0.95);
      }

      &.left {
        border-top-left-radius: 0;
        border-bottom-left-radius: 0;
      }

      &.right {
        border-top-right-radius: 0;
        border-bottom-right-radius: 0;
      }
    }
  }
}

// IDEA 风格的标签动画
.idea-tab-enter-active {
  transition: all 0.2s ease-out;
}

.idea-tab-leave-active {
  transition: all 0.2s ease-in;
  position: absolute;
}

.idea-tab-enter-from {
  opacity: 0;
  transform: translateX(-10px) scale(0.95);
}

.idea-tab-leave-to {
  opacity: 0;
  transform: translateX(10px) scale(0.95);
}

.idea-tab-move {
  transition: transform 0.2s ease;
}
</style>
