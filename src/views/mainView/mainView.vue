<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from "vue-i18n";
import { useRoute } from 'vue-router';
import 'animate.css';
import router from '@/router';
import CommandIcon from "@/components/Icon/CommandIcon.vue";
import SSHIcon from "@/components/Icon/SSHIcon.vue";
import HelpIcon from "@/components/Icon/HelpIcon.vue";
import SettingsIcon from "@/components/Icon/SettingsIcon.vue";
import GitIcon from "@/components/Icon/GitIcon.vue";
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
</script>

<template>

    <el-aside class="sidebar">
      
      <div class="sidebar-main">
        <div class="icon-wrapper">
          <el-tooltip effect="dark" :content="$t('git.name')" placement="right">
            <el-icon size="30">
              <GitIcon />
            </el-icon>
          </el-tooltip>
        </div>
        <div class="icon-wrapper" @click="navigateTo('/MainWindows/NoteBook')">
          <el-tooltip effect="dark" :content="$t('md.name')" placement="right">
            <el-icon size="30">
              <MarkDown />
            </el-icon>
          </el-tooltip>
        </div>
      </div>
      
      <div class="sidebar-footer">
        <div class="icon-wrapper help-icon" @click="navigateTo('/App/Help')">
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
    
    <!-- <el-container class="content-wrapper"> -->
      <el-main 
        mode="out-in" 
        enter-active-class="animate__animated animate__fadeIn"
        leave-active-class="animate__animated animate__fadeOut"
        class="main-content"
      >
        <router-view />
      </el-main>
    <!-- </el-container> -->
</template>

<style lang="scss" scoped>
.main-layout {
  height: 100vh;
  width: 100vw;
}

.content-wrapper {
  width: calc(100vw - 50px);
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