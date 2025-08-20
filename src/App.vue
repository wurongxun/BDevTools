<template>
  <div id="app">
    <router-view />
  </div>
</template>

<script setup lang="ts">
//import { invoke } from '@tauri-apps/api/tauri'
import { invoke } from '@tauri-apps/api/core';
import { load, Store } from '@tauri-apps/plugin-store';
import { onMounted } from 'vue';
import { useI18n } from "vue-i18n";
import { useDark, useToggle } from '@vueuse/core'
const isDark = useDark({
  storageKey: 'theme-appearance', //自定义localStorage存储名称
  valueDark: 'dark', //类名暗黑模式下值是dark
  valueLight: 'light', // 类名亮色模式下值是light
})
const toggleDark = useToggle(isDark)
let store: Store | undefined = undefined;
const { t, locale } = useI18n()
onMounted(async () => {
  store = await load('settings.json', { autoSave: false });
  if (store != undefined) {
    let language_val = await store.get<{ value: string }>('language');
    if (language_val !== undefined) {
      locale.value = language_val.value;
    }
    let theme_val = await store.get<{ value: string }>('theme');
    if (theme_val !== undefined) {
      let el_color_primary = isDark.value ? "black" : ""
      document.documentElement.style.setProperty('--el-color-primary', el_color_primary);
    }

    let dark_val = await store.get<{ value: boolean }>('dark');
    if (dark_val !== undefined) {
      if (dark_val.value && !isDark.value) {
        toggleDark();
      }
    }
  }

})
document.addEventListener('DOMContentLoaded', () => {
  //关闭loding窗口
  // setTimeout(() => {
  //   invoke('loding_window_close')
  // }, 3000)

})
//禁止F12
// document.onkeydown = function (event: KeyboardEvent) {
//   if (event.key === "F12") {
//     event.preventDefault()
//     return false
//   }
// }
//禁止右键和检查
// document.addEventListener("contextmenu", (event) => {
//   event.preventDefault();
// });
</script>

<style>
#app {
  height: 100vh;
  overflow: hidden;
  /* border-radius: 40px !important;; */
  /* background-color: #a10606; */
}
</style>
