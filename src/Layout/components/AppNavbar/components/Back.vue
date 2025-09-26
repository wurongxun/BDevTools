<template>
  <div class="scale-icon" @click="back">
    <el-icon size="20px">
      <Back />
    </el-icon>
  </div>
</template>

<script lang="ts" setup>
import useStore from '@/store'
import { storeToRefs } from 'pinia'
import { Back, Minus, Close } from "@element-plus/icons-vue";
//import router from '@/router';
import { ITagItem } from '@/Types/TagView';
import { useRouter, useRoute } from 'vue-router'

const { routerNoteStore, tagview, noteTem } = useStore()
const { system } = useStore()
const { isCollapse } = storeToRefs(system)
const router = useRouter()
const route = useRoute()
const back = async function () {
  //router.back();
  activeRouteRedirect();
};
// 改进的实现：找到活动的 TagView
const activeRouteRedirect = async () => {
    // 根据当前路由路径找到对应的 tagView
    const activeRoute = tagview.tagViewList.find(tag => tag.path === route.path)
    
    if (activeRoute) {
        // 如果找到了活动的 tagView，保持在当前页面
        return
    }
    
    // 如果没有活动的 tagView，则跳转到最后一个 tagView 或默认路由
    const endRoute = tagview.tagViewList[tagview.tagViewList.length - 1] as ITagItem
    console.log("===========endRoute===========",endRoute)
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
</script>

<style scoped lang="scss">
@import '@/styles/variables.scss';

.scale-icon {
  cursor: pointer;
  width: 60px;
  height: 100%;
  font-size: 0;
  display: flex;
  align-items: center;
  padding-left: 20px;
  margin-right: 10px;

  &:hover {
    @include useTheme {
      background-color: getVar('hoverColor');
    }
  }
}
</style>
