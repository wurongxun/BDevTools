<template>
  <div id="app">
    <Layout v-if="!isChildWindow">
      <router-view />
    </Layout>
    <router-view v-else />
  </div>
</template>
<script lang="ts" setup>import { onMounted, ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { load, Store } from '@tauri-apps/plugin-store'
import useStore from '@/store'
import Layout from '@/Layout/index.vue'

const isChildWindow = ref(false)

function checkWindow() {
  const hash = window.location.hash
  isChildWindow.value = hash.startsWith('#/App')
}
checkWindow()

const { routerNoteStore, useDocumentManagerStore } = useStore()
listen<string>('file-opened-from-os', (event) => {
  const filePath = event.payload
  console.log('从系统打开了文件:', filePath)
})

let store: Store | undefined = undefined
onMounted(async () => {
  store = await load('openfile.json', {
    autoSave: false,
    defaults: {}
  })
  if (store){
    var keys = store.keys()
    store.entries().then((entries) => {
      console.log("app store entries", entries)
    })
    console.log("app store keys", keys)
    let values:Promise<String[]> =store.values()
    console.log("app store values", values)
    await values.then((value) => {
      for (let i in value) {
        console.log(i, value[i])
        var s = value[i].toString()
        if (s){
          useDocumentManagerStore.openDocument(s)
        }
      }
    })
  }
  const routerNote = await routerNoteStore.setRoutes()
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
</script>
