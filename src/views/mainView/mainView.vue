<script setup lang="ts">
import { computed, inject, onMounted, ref, Ref, unref, watchEffect } from 'vue';
import { useI18n } from "vue-i18n";
import { SystemInfo } from "@/entity/entitys";
import { listen } from "@tauri-apps/api/event";
import { useRoute, useRouter } from 'vue-router';
import 'animate.css';
// import { UserFilled, Close, Minus, CopyDocument, Share, Search, More } from "@element-plus/icons-vue"
import { invoke } from '@tauri-apps/api/core';
import router from '@/router';
import { getCurrentWindow } from "@tauri-apps/api/window";
import WindowMaximizationIcon from "@/components/Icon/WindowMaximizationIcon.vue"
import WindowRestoreIcon from "@/components/Icon/WindowRestoreIcon.vue"
import BlackModeIcon from "@/components/Icon/BlackModeIcon.vue"
import WhiteModeIcon from "@/components/Icon/WhiteModeIcon.vue"
import SettingsIcon from "@/components/Icon/SettingsIcon.vue";
import BasicFunctionIcon from "@/components/Icon/BasicFunctionIcon.vue";
import HelpIcon from "@/components/Icon/HelpIcon.vue";
import SSHIcon from "@/components/Icon/SSHIcon.vue";
import CopyIcon from "@/components/Icon/CopyIcon.vue";
import GitIcon from "@/components/Icon/GitIcon.vue";
import CommandIcon from "@/components/Icon/CommandIcon.vue";
import Calendar from "@/components/Icon/Calendar.vue";
import BoxLogoIcon from "@/components/Icon/BoxLogoIcon.vue"
import MainHeader from '@/components/header/MainHeader.vue';
const { t, locale } = useI18n()
const route = useRoute()
const drawer = ref(true);
const rail = ref(true);
const visible = ref(false);
const value6 = ref(true)
const dark_mode_icon = ref("black")
//路由名称
const routeName = computed(() => route.path.match('/[^/]+')?.[0] ?? '');
const header_height = '35px';

// 应用标题，动态切换不同标题
const appName = ref<string>(t('appName'));
const input3 = ref('')
let greetMsg = ref<SystemInfo>({ memory_total: 0, memory_used: 0, cpu_used: '0', hostname: '', cpus: 0 })
async function listen_to_greet() {
  const unlisten = await listen('hello', (event: any) => {
    // event.payload 才是实际的结构体
    greetMsg.value = event.payload;
  });
}


const navigateTo = async function (roter_url: string) {
  //@click="goto_router('/Home/About')"
  console.log("navigateTo", roter_url);
  router.push(roter_url);//({ name: 'AppSetting' });
};

onMounted(() => {
  // listen_to_greet()
})

</script>
<template>
  <el-container style="width: 100vw;">
    <!--  <el-container style="height: calc(100vh - 35px);"> -->
    <el-aside class="aside" style="width: 50px;">
      <div class="aside_header">
        <div>
          <el-tooltip class="box-item" effect="dark" content="资源管理" placement="top-start">
            <el-icon size="30">
              <CopyIcon />
            </el-icon>
          </el-tooltip>
        </div>
        <div>
          <el-tooltip class="box-item" effect="dark" content="搜索" placement="top-start">
            <el-icon size="30">
              <Search />
            </el-icon>
          </el-tooltip>
        </div>
        <div>
          <el-tooltip class="box-item" effect="dark" :content="$t('command.name')" placement="top-start">
            <el-icon size="30">
              <CommandIcon />
            </el-icon>
          </el-tooltip>
        </div>
      </div>
      <div class="aside_main">
        <div>
          <el-tooltip class="box-item" effect="dark" :content="$t('git.name')" placement="top-start">
            <el-icon size="30">
              <GitIcon />
            </el-icon>
          </el-tooltip>
        </div>
        <div @click="navigateTo('/Business/Ssh')">
          <el-tooltip class="box-item" effect="dark" :content="$t('ssh.name')" placement="top-start">
            <el-icon size="30">
              <SSHIcon />
            </el-icon>
          </el-tooltip>
        </div>
        <div @click="navigateTo('/index/BasicFunction')">
          <el-tooltip class="box-item" effect="dark" :content="$t('tools.name')" placement="top-start">
            <el-icon size="30">
              <BasicFunctionIcon />
            </el-icon>
          </el-tooltip>
        </div>
      </div>
      <div class="aside_footer">
        <div class="HelpIcon" @click="navigateTo('/App/Calendar')">
          <el-tooltip class="box-item" effect="dark" :content="$t('calendar.name')" placement="top-start">
            <el-icon size="30">
              <Calendar />
            </el-icon>
          </el-tooltip>
        </div>
        <div class="HelpIcon" @click="navigateTo('/App/Help')">
          <el-tooltip class="box-item" effect="dark" :content="$t('help.name')" placement="top-start">
            <el-icon size="30">
              <HelpIcon />
            </el-icon>
          </el-tooltip>
        </div>
        <div class="SettingsIcon" @click="navigateTo('/App/AppSetting')">
          <el-tooltip class="box-item" effect="dark" :content="$t('settings.name')" placement="top-start"
            hide-after="100">
            <el-icon size="30">
              <SettingsIcon />
            </el-icon>
          </el-tooltip>
        </div>
      </div>
    </el-aside>
    <el-container style="width: calc(100vw - 50px);">
      <el-main mode="out-in" enter-active-class="animate__animated animate__fadeIn"
        style="overflow: auto;scrollbar-width: none;" leave-active-class="animate__animated animate__fadeOut">
        <router-view />
      </el-main>
    </el-container>
    <!-- </el-container> -->
  </el-container>
</template>
<style lang="scss" scoped>
.aside_header {
  grid-area: header;
}

.sidebar {
  grid-area: sidebar;
}

.aside_main {
  grid-area: main;
}

.aside_footer {
  grid-area: footer;
}

.el-row:last-child {
  margin-bottom: 0;
}

//CopyDocument
// .el-col {
//   border-radius: 4px;
// }

.grid-content {
  border-radius: 4px;
  min-height: 36px;
}

.demo-type,
.header-logo {
  display: flex;
  justify-content: center;
  align-items: center;
}

.demo-type>div,
.header-logo>div {
  height: 100%;
  flex: 1;
  text-align: center;
  display: flex; //将子元素设置为弹性容器。
  justify-content: center; //使子元素在水平方向上居中对齐。
  align-items: center; //使子元素在垂直方向上居中对齐。
  //background-color: #c3d0da; /* 鼠标悬停时的背景色 */
  transition: background-color 0.3s;
  /* 平滑背景色过渡 */
  //padding-left: 3px;
  //padding-right: 3px;
  border: 1px solid transparent;
}

.aside .aside_header>div,
.aside .aside_main>div,
.aside .aside_footer>div {
  display: flex; //将子元素设置为弹性容器。
  justify-content: center; //使子元素在水平方向上居中对齐。
  align-items: center; //使子元素在垂直方向上居中对齐。
  transition: background-color 0.3s;
  //border: 2px solid red;
  border-radius: 5px;
  margin-top: 5px;
  margin-bottom: 5px;
  margin-left: 5px;
  margin-right: 5px;
  height: 40px;

}

.aside .aside_header {
  height: 200px;
  /* 根据实际需求设置高度 */
}

.aside .aside_main {
  height: calc(100vh - 435px);
}

.aside .aside_footer {
  //display: flex;/* 保证父容器的高度足够容纳子元素 */
  //flex-direction: column; /* 子元素垂直排列 */
  //align-items: flex-end; /* 子元素在交叉轴（垂直方向）上靠底对齐 */
  display: grid;
  /* 保证父容器的高度足够容纳子元素 */
  height: 200px;
  /* 根据实际需求设置高度 */
  flex-direction: column;
  /* 子元素垂直排列 */
  align-items: end;
  /* 子元素在交叉轴（垂直方向）上靠底对齐 */
  grid-template-columns: repeat(auto-fit, 1fr);
  /* 或固定数量：repeat(3, 1fr) */
}

.aside .aside_footer .SettingsIcon {
  grid-area: 4 / 1 / span 0 / span 0;
}

.aside .aside_footer .HelpIcon {
  grid-area: 3 / 1 / span 0 / span 0;
}

.aside {
  display: grid;
  grid-template-columns: 1fr;
  //grid-template-rows: auto 1fr;/* 第一行自适应内容高度，第二行占据剩余空间 */
  /* 使用网格布局 */
  /* 保证父容器的高度足够容纳子元素位于底部 */
  height: 100%;
  /* 根据实际需求设置高度 */
  //align-items: end;
  grid-template-rows: [row-start] auto [row-mid] auto [row-end];
  grid-template-areas:
    "header header"
    "main main"
    "footer footer";
}

.aside>div:not(:last-child) {

  //border-bottom: 1px solid var(--el-border-color);
}

.aside .aside_header>div:hover,
.aside .aside_main>div:hover,
.aside .aside_footer>div:hover {
  //background-color: hsl(0, 17%, 92%); //鼠标悬停时的背景色
  box-shadow: 0 0 10px rgba(128, 126, 126, 0.5);
  transform: scale(1.1);
  background-color: var(--mouse--hover--button--bg-color); //鼠标悬停时的背景色
}

.aside .aside_header,
.aside .aside_main,
.aside .aside_footer {
  transition: transform 0.5s;

}

.demo-type>div:hover {
  //background-color: #afafaf; /* 鼠标悬停时的背景色 */
  background-color: rgba(124, 124, 124, 0.479);
  backdrop-filter: blur(10px);
  box-shadow: 0 0 10px rgba(0, 0, 0, 0.5);
  border-radius: 1px;
}

.header-logo>div:hover {}

.header-logo>div:not(:last-child) {
  //border-right: 1px solid var(--el-border-color);
}
</style>