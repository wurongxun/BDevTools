<script setup lang="ts">
import { Window } from "@tauri-apps/api/window"
import { Webview } from "@tauri-apps/api/webview"
import { invoke } from '@tauri-apps/api/core';
import { getAllWebviewWindows } from '@tauri-apps/api/webviewWindow';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
//import {  } from '@tauri-apps/api/window';
import { computed, ref } from 'vue';
import { useI18n } from "vue-i18n";
import { useRoute } from 'vue-router';
import 'animate.css';
import router from '@/router';
import Excel from "@/components/Icon/Excel.vue";
import Calendar from "@/components/Icon/Calendar.vue";
import HelpIcon from "@/components/Icon/HelpIcon.vue";
import SettingsIcon from "@/components/Icon/SettingsIcon.vue";
import About from "@/components/Icon/About.vue";
import MarkDown from "@/components/Icon/MarkDown.vue";

const { t } = useI18n();
const route = useRoute();

// 路由名称
const routeName = computed(() => route.path.match('/[^/]+')?.[0] ?? '');

// 应用标题
const appName = ref<string>(t('appName'));

const navigateTo = async function (routeUrl: string) {
  console.log("navigateTo", routeUrl);
  router.push(routeUrl);
};

const newWebViewNotebook = async function () {
  const win = await WebviewWindow.getByLabel("note_book");
  if (win) {
    //win.setShadow(true);
    console.log("isFocused", win.isFocused())
    await win.setFocus();
    await win.show();
    return;
  } else {
    invoke('note_book_window').then((res) => {
      console.log(res)
    });
  }

};
</script>

<template>
  <el-container class="content-wrapper">
    <el-aside class="sidebar">
      <div class="sidebar-main">
        <div class="icon-wrapper">
          <el-tooltip effect="dark" :content="$t('excel.name')" placement="right">
            <el-icon size="30">
              <Excel />
            </el-icon>
          </el-tooltip>
        </div>
        <div class="icon-wrapper" @click="newWebViewNotebook">
          <el-tooltip effect="dark" :content="$t('md.name')" placement="right">
            <el-icon size="30">
              <MarkDown />
            </el-icon>
          </el-tooltip>
        </div>
        <div class="icon-wrapper" @click="navigateTo('/MainWindows/Calendar')">
          <el-tooltip effect="dark" :content="$t('calendar.name')" placement="right">
            <el-icon size="30">
              <Calendar />
            </el-icon>
          </el-tooltip>
        </div>
      </div>

      <div class="sidebar-footer">
        <div class="wrapper about-icon" @click="navigateTo('/App/AppAbout')">
          <el-tooltip class="box-item" :content="$t('about.name')" placement="top-start">
            <el-icon size="30">
              <About />
            </el-icon>
          </el-tooltip>
        </div>
        <div class="icon-wrapper help-icon" @click="navigateTo('/App/AppHelp')">
          <el-tooltip :content="$t('help.name')" placement="right">
            <el-icon size="30">
              <HelpIcon />
            </el-icon>
          </el-tooltip>
        </div>
        <div class="icon-wrapper settings-icon" @click="navigateTo('/App/AppSetting')">
          <el-tooltip :content="$t('settings.name')" placement="right" :hide-after="100">
            <el-icon size="30">
              <SettingsIcon />
            </el-icon>
          </el-tooltip>
        </div>
      </div>
    </el-aside>
    <el-main class="main-content">
      <router-view />
    </el-main>
  </el-container>
</template>

<style lang="scss" scoped>
.main-layout {
  height: 100vh;
  width: 100vw;
}

.content-wrapper {
  width: 100%;
  height: 100%;
}

.main-content {
  overflow: auto;
  scrollbar-width: none;
  height: 100%;
  padding: 0;
}

.sidebar {
  width: 50px;
  height: 100%;
  display: flex;
  flex-direction: column;
  background-color: var(--el-bg-color);
  border-right: 1px solid var(--el-border-color);
}

.sidebar-header {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.sidebar-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 10px 0;
  gap: 10px;
}

.sidebar-footer {
  display: flex;
  flex-direction: column-reverse;
  align-items: center;
  padding: 10px 0;
  gap: 10px;
  flex-shrink: 0;
}

.icon-wrapper {
  display: flex;
  justify-content: center;
  align-items: center;
  border-radius: 5px;
  width: 40px;
  height: 40px;
  cursor: pointer;
  transition: all 0.3s ease;

  &:hover {
    box-shadow: 0 0 10px rgba(128, 126, 126, 0.5);
    transform: scale(1.1);
    background-color: var(--mouse--hover--button--bg-color);
  }
}

.help-icon,
.settings-icon {
  // 可以添加特殊样式
}
</style>