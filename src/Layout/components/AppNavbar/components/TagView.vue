<template>
  <div class="tab-control">
    <div class="tab-box">
      <router-link class="tag-item suspension" :class="{ active: isActive(tag) }" v-for="tag in tagViewList"
        :key="tag.path" :to="getToPath(tag)">
        <span class="tag--title">{{ tag.title }}</span>
        <el-icon v-if="!tag.affix" class="close--icon" @click.prevent.stop="removeTab(tag)">
          <Close />
        </el-icon>
      </router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import useStore from '@/store/index'
import { computed, nextTick, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Close } from '@element-plus/icons-vue'
import ToolUtils from '@/utils/ToolUtils'

import type { ITagItem } from '@/Types/TagView'
import { ElMessageBox } from 'element-plus'
import NotificationUtil from '@/utils/NotificationUtil'

const route = useRoute()
const router = useRouter()
const { tagview, noteTem } = useStore()

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
      console.log("tageview ", path)
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

const removeTab = async (tag: ITagItem) => {

  // 检查该标签页是否有未保存的笔记内容
  const noteId = tag.name; // 使用标签名作为笔记ID
  const noteData = noteTem.getNoteData(noteId);

  // 如果有未保存的更改，询问用户是否保存
  if (noteData && noteData.tempContent && noteData.tempContent !== noteData.content) {
    try {
      await ElMessageBox.confirm(
        `标签页 "${tag.title}" 有未保存的内容，是否保存更改?`,
        "保存提醒", {
        confirmButtonText: "保存",
        cancelButtonText: "不保存",
        type: "warning",
      }
      );
      // 用户选择保存，这里可以调用保存方法

      // 调用保存方法

      // 由于我们无法直接访问编辑器组件，可以提示用户切换到该标签页进行保存
      NotificationUtil.info("请切换到对应标签页手动保存内容");
    } catch {
      // 用户选择不保存，继续关闭标签页
      console.log("用户选择不保存更改，继续关闭标签页");
    }
  }


  tagview.removeTagView(tag)
  if (isActive(tag)) {
    const endRoute = tagview.tagViewList[tagview.tagViewList.length - 1] as ITagItem
    if (endRoute) {
      router.push({
        path: endRoute.path
      })
    } else {
      router.push({
        path: '/'
      })
    }
  }
}
/**
 * 如果当前路由设置了noTagView且有activeMenu，则比较activeMenu与标签路径
 * 如果标签对象不为空，则使用路由匹配路径进行比较
 * 否则直接比较路由路径与标签路径
 */
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

// 添加自动滚动到激活标签的函数
const scrollToActiveTab = () => {
  nextTick(() => {
    const activeTab = document.querySelector('.tag-item.active');
    if (activeTab) {
      activeTab.scrollIntoView({
        behavior: 'smooth',
        inline: 'center'
      });
    }
  });
};
// 监听路由变化并滚动到激活标签
watch(
  () => route.path,
  () => {
    setTimeout(() => {
      scrollToActiveTab();
    }, 100);
  }
)
// 在添加标签页后也调用滚动
watch(
  () => router.currentRoute.value,
  (newValue) => {
    // ... existing code ...

    // 在添加标签后滚动到激活项
    setTimeout(() => {
      scrollToActiveTab();
    }, 100);
  },
  { immediate: true }
);
</script>

<!-- <style scoped lang="scss">
@use '@/styles/variables.scss';

.suspension {
  box-shadow: 0 0 5px #ccc;

  &:hover {
    box-shadow: inset 0 0 3px 0px #ddd;
  }
}

.tab-control {
  display: flex;
  align-items: center;
  padding: 2px 2px;
  border-radius: 3px;
  height: 46px;

  @include useTheme {
    border-bottom: 1px solid getVar('borderColor');
  }

  .tab-box {
    white-space: nowrap;
    overflow-x: auto;

    @include scrollbar;

    .tag-item {
      display: inline-block;
      padding: 6px 15px;
      font-size: $font-size;
      margin: 3px 6px;
      border-radius: 5px;
      color: #666;

      &:not(.active):hover {
        color: $theme-color;
      }

      &:hover {
        .close--icon {
          display: inline-block;
          width: 14px;
          margin-left: 8px;
        }
      }

      &.active {
        box-shadow: inset 0 0 3px 0px #ddd;
        color: $theme-color;
      }

      .tag--title {
        vertical-align: middle;
      }

      .close--icon {
        width: 0;
        vertical-align: middle;
        transition: all 0.5s;

        &:hover {
          color: #fff;
          background: #ccc;
          border-radius: 50%;
        }
      }
    }
  }
}

:deep(.el-tabs__header) {
  margin-bottom: 0;
}
</style> -->
<style scoped lang="scss">
@use '@/styles/variables.scss' as variables;
@use '@/styles/theme.scss';

.suspension {
  box-shadow: 0 0 5px #ccc;

  &:hover {
    box-shadow: inset 0 0 3px 0px #ddd;
  }
}

.tab-control {
  display: flex;
  align-items: center;
  padding: 2px 2px;
  border-radius: 3px;
  height: $tage-view-height;

  @include useTheme {
    border-bottom: 1px solid getVar('borderColor');
  }

  .tab-box {
    white-space: nowrap;
    overflow-x: auto;
    overflow-y: hidden; // 确保只允许横向滚动
    scroll-behavior: smooth;
    width: 100%;
    // 改善滚动体验
    -webkit-overflow-scrolling: touch;

    @include scrollbar;

    .tag-item {
      display: inline-block;
      padding: 0px 3px;
      font-size: $font-size;
      margin: 0px 3px;
      border-radius: 1px;
      color: #666;

      &:not(.active):hover {
        color: variables.$theme-color;
      }

      &:hover {
        .close--icon {
          display: inline-block;
          width: 14px;
          margin-left: 8px;
        }
      }

      &.active {
        box-shadow: inset 0 0 3px 0px #ddd;
        color: variables.$theme-color;
      }

      .tag--title {
        vertical-align: middle;
      }

      .close--icon {
        width: 0;
        vertical-align: middle;
        transition: all 0.5s;

        &:hover {
          color: #fff;
          background: #ccc;
          border-radius: 50%;
        }
      }
    }
  }
}

:deep(.el-tabs__header) {
  margin-bottom: 0;
}
</style>