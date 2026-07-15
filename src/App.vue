<template>
  <div id="app">
    <router-view v-slot="{ Component }">
      <component :is="Component" />
    </router-view>
  </div>
</template>
<script lang="ts" setup>
import { onMounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { load, Store } from '@tauri-apps/plugin-store'

// 监听从系统文件关联打开的事件
listen<string>('file-opened-from-os', (event) => {
  const filePath = event.payload
  console.log('从系统打开了文件:', filePath)
  // 在这里处理文件打开逻辑，比如加载文件内容到编辑器
})
//禁止F12
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

//禁止右键和检查
// document.addEventListener("contextmenu", (event) => {
//   event.preventDefault();
// });
let store: Store | undefined = undefined
onMounted(async () => {
  // 应用启动时获取全局用户信息
  store = await load('openfile.json', {
    autoSave: false,
    defaults: {}
  })
  if (store){
    let values = store.values()
    console.log("app store values", values)
  }
})
</script>