<template>
  <div class="common-layout">
    <el-container>
      <!-- <el-header style="padding: 0%;"> <AppHeader ref="element"></AppHeader></el-header> -->
      <el-header :height=header_height style="width: 100vw;padding: 0%;display: flex;">
        <MainHeader></MainHeader>
      </el-header>
      <el-container>
        <el-main><router-view /></el-main>
      </el-container>
    </el-container>
  </div>
</template>
<script>

import router from '@/router';
import AppHeader from '@/components/main_header/AppHeader.vue';
import { defineComponent, ref, onMounted, computed } from 'vue';
import MainHeader from '@/components/header/MainHeader.vue';
export default defineComponent({
  components: {
    AppHeader,
    //IconFont,
    //LikeOutlined,
    //CommentOutlined,
    //FileOutlined,
    //EditFilled
  },
  setup() {

    const height_content = computed(() => {
      // 回调函数必须return，结果就是计算的结果
      // 如果计算属性依赖的数据发生变化，那么会重新计算
      return document.documentElement.clientHeight - 30
    })
    const goto_router = (url) => {
      router.push(url)
    };
    onMounted(() => {


    });
    const headerStyle = {
      textAlign: 'center',
      color: '#fff',
      height: 64,
      paddingInline: 50,
      lineHeight: '64px',
      backgroundColor: '#7dbcea',
    };
    return {
      height_content,
      goto_router,
      selectedKeys1: ref(['2']),
      selectedKeys2: ref(['1']),
      collapsed: ref(false),
      openKeys: ref(['sub1']),
      headerStyle,
    };
  },
});
</script>
<style scoped>
a-layout-sider::before {
  content: "";
  display: block;
}

a-layout-sider::after {
  content: "";
  display: block;
}

span {
  /* margin-top: 10px; */
}
</style>
