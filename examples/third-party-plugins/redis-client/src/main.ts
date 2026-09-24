import { createApp } from 'vue'
import AppRoot from './components/AppRoot.vue'
import { bridge } from './bridge'
import './ui.css'

// 收宿主消息：主题推送 + 调用回包都走这里
window.addEventListener('message', (e) => {
  bridge.handleMessage(e.data)
  const d = e.data
  if (d && d.v === 1 && d.kind === 'theme') {
    document.documentElement.setAttribute('data-theme', d.dark ? 'dark' : 'light')
  }
})

// 完成握手（否则宿主 3 秒后提示「未在 3 秒内完成 ready 握手」）
bridge.ready()

createApp(AppRoot).mount('#app')
