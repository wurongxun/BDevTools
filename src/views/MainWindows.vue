<template>
  <a-layout :style="{ height: '100%' }">
    <AppHeader ref="element"></AppHeader>
    <a-layout ref="Conten" :style="{ height: `${height_content}px`, minHeight: 'hidden' }">
      <!-- <a-layout-sider width="55" style=" writing-mode: vertical-lr;align-items: center;display: flex;background-color: rgba(223, 223, 223, 0.822)"> -->
        <!-- <span><icon-font @click="goto_router('/Home/Note')" :style="{ color: '#08c', fontSize: '28px', }"
            type="icon-typora" /></span> -->
        <!-- <span><icon-font @click="goto_router('/Home/About')" :style="{ color: '#08c', fontSize: '28px', }"
            type="icon-typora" /></span>
        <span><icon-font @click="goto_router('/Home/Test')" :style="{ color: '#08c', fontSize: '28px', }"
            type="icon-typora" /></span> -->
<!-- 
            <a-menu-item key="1" style="margin-top: 0px;" @click="goto_router('/Home/Note')">
            <icon-font  type="icon-typora" />
            <span>Note</span>
          </a-menu-item> -->

      <!-- </a-layout-sider> -->

      <!-- <a-layout-sider >
        <a-menu v-model:selectedKeys="selectedKeys" theme="dark" mode="inline">
          <a-menu-item key="1" style="margin-top: 0px;" @click="goto_router('/Home/Note')">
            <icon-font  type="icon-typora" />
            <span>Note</span>
          </a-menu-item>
          <a-menu-item key="2" @click="goto_router('/Home/About')">
            <file-outlined />
            <span>File</span>
          </a-menu-item>
          <a-menu-item key="3" @click="goto_router('/Home/Test')">
            <file-outlined />
            <span>Test</span>
          </a-menu-item>
        </a-menu>
      </a-layout-sider> -->

      <a-layout style="padding:0 0px;height: 100%;overflow: hidden">
        <a-layout-content :style="{ background: '#fff', padding: '0 0px' }">
          <router-view />
        </a-layout-content>
      </a-layout>
    </a-layout>
  </a-layout>
</template>
<script>

import router from '@/router';
import AppHeader from '@/components/main_header/AppHeader.vue';
import { defineComponent, ref, onMounted, computed } from 'vue';
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
span{
 margin-top: 10px;
}
</style>

