/* eslint-disable */
// import { createApp } from 'vue'
// import App from './App.vue'

// createApp(App).mount('#app')

import { createApp } from "vue";
import App from "./App.vue";
// highlightjs
//import hljs from "highlight.js";
import "v-contextmenu/dist/themes/default.css";
/* eslint-disable */
import "./assets/iconfont/iconfont.js";
import { registerPlugins } from "@/plugins";
const app = createApp(App);
// app.config.productionTip = false;
registerPlugins(app);
app.mount("#app");
