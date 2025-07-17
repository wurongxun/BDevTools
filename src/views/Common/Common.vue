<script setup lang="ts">
import { computed, inject, onMounted, ref, Ref, unref, watchEffect } from 'vue';
import { useI18n } from "vue-i18n";
import { SystemInfo } from "@/entity/entitys";
import { listen } from "@tauri-apps/api/event";
import { useRoute, useRouter } from 'vue-router';
import { UserFilled, Close, Minus, CopyDocument, Share, Search, More } from "@element-plus/icons-vue"
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from "@tauri-apps/api/window";
import BoxLogoIcon from "@/views/element/Icon/BoxLogoIcon.vue";
import BackHeader from "@/components/header/element/BackHeader.vue";
import { useHomeNavigation } from '@/events/navigation.ts';
const { pages, activePagePosition, isBackOtherPage, isInMainView } = useHomeNavigation()

import 'animate.css';
const { t, locale } = useI18n();
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


onMounted(() => {
  listen_to_greet()
})

</script>
<template>
  <!-- <div class="common-layout"> -->
  <el-container style="width: 100vw;">
    <el-header :height=header_height style="width: 100vw;padding: 0%;display: flex;">
      <template v-for="item in pages">
        <BackHeader v-if="$route.name === item.name" :showTitle=item.showTitle :title=item.title
          :showWinMin=item.showWinMin :showWinChange=item.showWinChange></BackHeader>
      </template>
    </el-header>
    <el-container style="height: calc(100vh - 35px);  ">
      <el-container style="width:100vw;">
        <el-main mode="out-in" enter-active-class="animate__animated animate__fadeIn"
          style="overflow: auto;scrollbar-width: none;" leave-active-class="animate__animated animate__fadeOut">
          <keep-alive>
            <router-view />
          </keep-alive>
        </el-main>
      </el-container>
    </el-container>
  </el-container>
</template>
<style lang="scss" scoped>
/* 针对 WebKit 内核的浏览器（如 Chrome 和 Safari） */
.element::-webkit-scrollbar {
  width: 0;
  height: 0;
}

/* 针对 Firefox */
.element {
  scrollbar-width: none;
  /* Firefox 64+ */
}
</style>