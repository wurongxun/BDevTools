<template>
  <el-container class="app-layout">
    <el-header class="app-header" data-tauri-drag-region>
      <MainHeader />
    </el-header>
    <el-main class="app-main">
      <router-view />
    </el-main>
    <el-footer class="app-footer">
      <div class="footer-content">
        <span class="copyright"> </span>
      </div>
    </el-footer>
  </el-container>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import router from '@/router'
import MainHeader from '@/components/header/MainHeader.vue'

// 计算主内容区域高度
const mainHeight = computed(() => {
  return `calc(100vh - ${headerHeight.value}-${footerHeight.value})`
})

// 头部高度
const headerHeight = ref('30px')
const footerHeight = ref('20px')

// 路由跳转方法
const gotoRouter = (url) => {
  router.push(url)
}

onMounted(() => {
  // 可以在这里添加初始化逻辑
})
</script>

<script>
// 兼容选项式 API 的部分
export default {
  name: 'AppMainWindows',
  components: {
    MainHeader
  }
}
</script>

<style scoped lang="scss">
.app-layout {
  width: 100vw;
  height: 100vh;
  flex-direction: column;
  overflow: hidden;
}

.app-header {
  height: var(--header-height);
  padding: 0;
  display: flex;
  align-items: center;
  width: 100%;
  flex-shrink: 0;
  border-bottom: 1px solid var(--el-border-color);
}

.app-main {
  flex: 1;
  padding: 0;
  overflow: hidden;
  height: v-bind(mainHeight);
}

.app-footer {
  height: v-bind(footerHeight);
  padding: 0 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background-color: var(--el-bg-color-page);
  border-top: 1px solid var(--el-border-color);
  flex-shrink: 0;

  .footer-content {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: var(--el-text-color-secondary);
  }

  .copyright {
    margin: 0;
  }
}
</style>