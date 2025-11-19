<template>
  <div id="app">
    <router-view v-slot="{ Component }">
      <keep-alive :include="cacheList">
        <component :is="Component" />
      </keep-alive>
    </router-view>
  </div>
</template>
<script lang="ts" setup>
import useStore from '@/store'
import { storeToRefs } from 'pinia'

const { tagview } = useStore()
const { cacheList } = storeToRefs(tagview)

// //禁止F12
// document.onkeydown = function (event: KeyboardEvent) {
//   // 使用 event.code 替代 keyCode
//   if (event.code === 'F12') {
//     event.preventDefault()
//     return false
//   }
//   // 对于 Enter 键，使用 event.key 或 event.code
//   if (event.key === 'Enter' || event.code === 'Enter') {
//     // 注意：这里修改keyCode在现代浏览器中可能无效
//     // 如果需要阻止默认行为，应直接使用 event.preventDefault()
//     // 或者根据具体需求处理
//   }
// }

// //禁止右键和检查
// document.addEventListener("contextmenu", (event) => {
//   event.preventDefault();
// });
</script>