<template>
  <div id="app">
    <router-view />
  </div>
</template>

<script>
//import { invoke } from '@tauri-apps/api/tauri'
import { invoke } from '@tauri-apps/api/core';
export default {
  name: 'App',
  components: {

  }
}

document.addEventListener('DOMContentLoaded', () => {
  //关闭loding窗口
  setTimeout(() => {
    invoke('loding_window_close')
  }, 3000)

})
//禁止F12
document.onkeydown = function (event) {
  var winEvent = window.event
  if (winEvent && winEvent.keyCode == 123) {
    event.keyCode = 0
    event.returnValue = false
  }
  if (winEvent && winEvent.keyCode == 13) {
    winEvent.keyCode = 505
  }
}
//禁止右键和检查
document.addEventListener("contextmenu", (event) => {
  event.preventDefault();
});
</script>

<style>
#app {
  height: 100vh;
  overflow: hidden;
  /* border-radius: 40px !important;; */
  background-color: #f3f3f3;
}
</style>
