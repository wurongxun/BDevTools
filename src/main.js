/* eslint-disable */
// import { createApp } from 'vue'
// import App from './App.vue'

// createApp(App).mount('#app')


import { createApp } from 'vue'
import App from './App.vue'
import Antd from 'ant-design-vue';
import 'ant-design-vue/dist/reset.css';
import VMdEditor from '@kangc/v-md-editor';
import '@kangc/v-md-editor/lib/style/base-editor.css';
import githubTheme from '@kangc/v-md-editor/lib/theme/github.js';
import '@kangc/v-md-editor/lib/theme/style/github.css';
import router from "./router";
// highlightjs
import hljs from 'highlight.js';
import contextmenu from "v-contextmenu";
import "v-contextmenu/dist/themes/default.css";
/* eslint-disable */
import './assets/iconfont/iconfont.js';

VMdEditor.use(githubTheme, {
    Hljs: hljs,
});


const app = createApp(App);
app.config.productionTip = false;

app.use(Antd).use(router).use(VMdEditor).use(contextmenu).mount('#app')
