<template>
  <div class="settings-layout">
    <!-- 左侧分类导航（固定：不随右侧内容滚动） -->
    <SettingNav v-model="activeSettingTab" />

    <!-- 右侧内容区：顶部固定条（页签不随内容滚动）+ 可滚动正文 -->
    <div class="settings-main">
      <header class="settings-head">
        <!-- 插件分区的「管理 / 开发」固定在顶部；管理页滚动时页签保持不动 -->
        <div
          v-if="activeSettingTab === 'plugin'"
          class="plugin-subtabs"
          role="tablist"
          aria-label="插件分区"
        >
          <button
            type="button"
            class="plugin-subtab"
            role="tab"
            :aria-selected="pluginTab === 'manage'"
            :class="{ 'is-active': pluginTab === 'manage' }"
            @click="pluginTab = 'manage'"
          >
            插件管理
          </button>
          <button
            type="button"
            class="plugin-subtab"
            role="tab"
            :aria-selected="pluginTab === 'dev'"
            :class="{ 'is-active': pluginTab === 'dev' }"
            @click="pluginTab = 'dev'"
          >
            插件开发
          </button>
        </div>
        <h2 v-else class="settings-head-title">{{ headTitle }}</h2>
      </header>

      <!-- 唯一滚动容器：各分区共用（v-show 切换），切页签不叠加滚动条。
           填充型分区（帮助 / 插件开发）自己带吸顶条与内部滚动，外层不再滚。 -->
      <div class="settings-body" :class="{ 'is-fill': fillBody }">
        <div v-show="activeSettingTab === 'app'" class="settings-pane">
          <AppGeneralPane />
        </div>
        <div v-show="activeSettingTab === 'window'" class="settings-pane">
          <WindowPresetPane />
        </div>
        <div v-show="activeSettingTab === 'plugin'" class="settings-pane">
          <PluginPane :active="activeSettingTab === 'plugin'" :tab="pluginTab" />
        </div>
        <div v-show="activeSettingTab === 'help'" class="settings-pane">
          <AppHelp />
        </div>
        <div v-show="activeSettingTab === 'about'" class="settings-pane">
          <AppAbout />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import SettingNav from './setting/SettingNav.vue'
import AppGeneralPane from './setting/AppGeneralPane.vue'
import WindowPresetPane from './setting/WindowPresetPane.vue'
import PluginPane from './setting/PluginPane.vue'
import AppHelp from './AppHelp.vue'
import AppAbout from './AppAbout.vue'
import type { PluginTab, SettingTab } from './setting/types'

/** 当前激活的设置分区 */
const activeSettingTab = ref<SettingTab>('app')

/** 插件分区内部页签：管理（内置/第三方插件的安装与排序）/ 开发（第三方插件开发文档） */
const pluginTab = ref<PluginTab>('manage')

/** 顶部固定条上的分区标题（插件分区走页签，不用标题） */
const HEAD_TITLES: Record<SettingTab, string> = {
  app: '应用设置',
  window: '窗口设置',
  plugin: '插件',
  help: '帮助',
  about: '关于',
}
const headTitle = computed(() => HEAD_TITLES[activeSettingTab.value])

/**
 * 是否让正文区「填满高度、由内部自己滚动」。
 * 帮助页与插件开发页都自带吸顶条/内部滚动容器：交给外层滚会把吸顶条一起滚走。
 */
const fillBody = computed(
  () =>
    activeSettingTab.value === 'help' ||
    (activeSettingTab.value === 'plugin' && pluginTab.value === 'dev'),
)
</script>

<style lang="scss">
// 设置页共享基元（分组 / 设置行 / 标签 …）：三个分区面板都要用，
// 必须非 scoped —— scoped 属性选择器不穿透到子组件。详见文件头注释。
@use './setting/settings.scss';
</style>
