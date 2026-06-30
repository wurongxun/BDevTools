<script lang="ts" setup name="TableManage">
import { storeToRefs } from 'pinia'
import { onMounted } from 'vue'
import useStore from '@/store'
onMounted(() => {

  
})
</script>

<template>
  <router-view v-slot="{ Component }">
    <keep-alive >
      <component :is="Component" />
    </keep-alive>
  </router-view>
</template>
