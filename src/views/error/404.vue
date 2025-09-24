<template>
  <div class="container">
    <p>
      <img class="icon" :src="error" alt="404" />
    </p>
    <p class="desc">抱歉，页面找不到了，您可以返回首页或者与我们联系</p>
    <el-button type="primary" @click="onBack"> 返回首页 </el-button>
  </div>
</template>

<script setup lang="ts" name="error404">
import { useRouter } from 'vue-router'
import error from '@/assets/error/404.png'

const router = useRouter()
const onBack = () => {
  router.push('/')
}
</script>

<style scoped lang="scss">
.container {
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  text-align: center;

  .desc {
    font-size: 14px;
  }
  .icon {
    width: 40vw;
  }
}
</style>
