<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { Check, Close } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { ResponseData, SettingIdValueName, SettingMap } from '@/entity/entitys';
import { useI18n } from "vue-i18n";
import { appConfigInfo } from "@/store/appConfigInfo";
import { load, Store } from '@tauri-apps/plugin-store';
import { open } from '@tauri-apps/plugin-dialog';
import { useDark, useToggle } from '@vueuse/core'
const isDark = useDark({
  storageKey: 'theme-appearance', //自定义localStorage存储名称
  valueDark: 'dark', //类名暗黑模式下值是dark
  valueLight: 'light', // 类名亮色模式下值是light
})
const toggleDark = useToggle(isDark)

const { t, locale } = useI18n()
const userStore = appConfigInfo();
let store: Store | undefined = undefined;
let autostart = ref<boolean>(false)
let update_reminder = ref<boolean>(false)
let dark = ref<boolean>(false)
let theme = ref<string>('white')
let language = ref<string>('')
let md_working_directory = ref<string>('')
const language_options = [
  {
    value: 'zh_CN',
    label: '简体中文',
    disabled: false,
  },
  {
    value: 'zh_TW',
    label: '繁体中文（中国台湾）',
    disabled: false,
  },
  {
    value: 'zh_TW',
    label: '繁体中文（中国香港）',
    disabled: false,
  },
  {
    value: 'en',
    label: 'English',
    disabled: false,
  }
]
const theme_options = [
  {
    value: 'white',
    label: '亮色',
    disabled: false,
  },
  {
    value: 'black',
    label: '黑色',
    disabled: false,
  }
]

onMounted(async () => {
  if (userStore.appConfig.length < 1) {
    query_setting_list();
  }
  if (userStore.appConfig.length > 1) {
    show_setting()
  }
  store = await load('settings.json', { autoSave: false });

  let val_auto_start = await store.get<{ value: boolean }>('auto_start');
  if (val_auto_start !== undefined) {
    autostart.value = val_auto_start.value;
  }
  let language_val = await store.get<{ value: string }>('language');
  if (language_val !== undefined) {
    language.value = language_val.value;
  }
  let md_working_directory_val = await store.get<{ value: string }>('md_working_directory');
  if (md_working_directory_val !== undefined) {
    md_working_directory.value = md_working_directory_val.value;
  }
  let update_reminder_val = await store.get<{ value: boolean }>('update_reminder');
  if (update_reminder_val !== undefined) {
    update_reminder.value = update_reminder_val.value;
  }
  let theme_val = await store.get<{ value: string }>('theme');
  if (theme_val !== undefined) {
    theme.value = theme_val.value;
  }
  let dark_val = await store.get<{ value: boolean }>('dark');
  if (dark_val !== undefined) {
    dark.value = dark_val.value;
  }
})

function show_setting() {

}

function query_setting_list() {
  invoke('query_setting_list').then((res): void => {
    let res_ = res as ResponseData<Array<SettingMap>>
    let data = res_.data as Array<SettingMap>;
    userStore.setAppConfig(data);
  })
}

function setting_change() {
  // invoke('updatate_setting', { id: id, appSettingItemValue: String(val), appSettingtemName: '' }).then((res): void => {
  //   console.log(res);
  //   query_setting_list()
  // })
}

async function language_change() {
  locale.value = language.value;
  if (store) {
    await store.set('language', { value: language.value });
    await store.save();
  }
}
async function autostart_change() {
  if (store) {
    await store.set('auto_start', { value: autostart.value });
    await store.save();
  }
}
async function software_change() {
  if (store) {
    await store.set('update_reminder', { value: update_reminder.value });
    await store.save();
  }
}
async function theme_change() {
  if (store) {
    await store.set('theme', { value: theme.value });
    await store.save();
    let el_color_primary = isDark.value ? "black" : ""
    document.documentElement.style.setProperty('--el-color-primary', el_color_primary);
  }
}
async function dark_change() {
  if (store) {
    await store.set('dark', { value: dark.value });
    await store.save();
    if (!isDark.value && dark.value) {
      toggleDark()
    } else if (isDark.value && !dark.value) {
      toggleDark()
    }
  }
}
async function md_working_directory_change() {
  const file = await open({
    multiple: false,
    directory: true,
  });
  if (file) {
    md_working_directory.value = file;
    if (store) {
      store.set('md_working_directory', { value: file });
      store.save();
    }
  }
}
// 添加清除缓存和浏览器缓存的函数
const clear_cache = () => {
  // 实现清除缓存的逻辑
  console.log('清除缓存')
}

const clear_browser_cache = () => {
  // 实现清除浏览器缓存的逻辑
  console.log('清除浏览器缓存')
}

</script>

<template>
  <div class="settings-container">
    <div class="settings-content">
      <!-- 通用设置 -->
      <div class="settings-section">
        <h2 class="section-title">通用设置</h2>

        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">开机自启动</span>
            <span class="setting-description">可能影响开机速度</span>
          </div>
          <div class="setting-control">
            <el-switch v-model="autostart" @change="autostart_change()" inline-prompt :active-icon="Check"
              :inactive-icon="Close" />
          </div>
        </div>

        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">软件更新提醒</span>
            <span class="setting-description">及时获取新功能和安全更新</span>
          </div>
          <div class="setting-control">
            <el-switch v-model="update_reminder" @change="software_change()" inline-prompt :active-icon="Check"
              :inactive-icon="Close" />
          </div>
        </div>
        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">主题</span>
            <span class="setting-description">切换主题</span>
          </div>
          <div class="setting-control">
            <el-select v-model="theme" @change="theme_change" placeholder="选择主题" style="width: 180px">
              <el-option v-for="item in theme_options" :key="item.value" :label="item.label" :value="item.value"
                :disabled="item.disabled" />
            </el-select>
          </div>
        </div>
        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">暗黑模式</span>
            <span class="setting-description"></span>
          </div>
          <div class="setting-control">
            <el-switch v-model="dark" @change="dark_change()" inline-prompt :active-icon="Check"
              :inactive-icon="Close" />
          </div>
        </div>
        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">切换语言</span>
            <span class="setting-description">选择界面显示语言</span>
          </div>
          <div class="setting-control">
            <el-select v-model="language" @change="language_change" placeholder="选择语言" style="width: 180px">
              <el-option v-for="item in language_options" :key="item.value" :label="item.label" :value="item.value"
                :disabled="item.disabled" />
            </el-select>
          </div>
        </div>
      </div>

      <!-- 网络设置 -->
      <div class="settings-section">
        <h2 class="section-title">网络</h2>

        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">网络检测</span>
            <span class="setting-description">检测当前网络连接状态</span>
          </div>
          <div class="setting-control">
            <el-button type="primary" plain>检测网络</el-button>
          </div>
        </div>
      </div>

      <!-- 其他设置 -->
      <div class="settings-section">
        <h2 class="section-title">其他设置</h2>

        <div class="settings-item">
          <div class="setting-info">
            <div class="setting-header">
              <span class="setting-label">MarkDown编辑器</span>
              <span class="setting-description">MarkDown编辑器工作目录</span>
            </div>
            <div class="directory-display">
              <el-icon class="folder-icon">
                <Folder />
              </el-icon>
              <span class="setting-working-directory">{{ md_working_directory }}</span>
            </div>
          </div>
          <div class="setting-control">
            <el-button type="primary" @click="md_working_directory_change()" plain>
              <el-icon>
                <FolderChecked />
              </el-icon>
              更换工作目录
            </el-button>
          </div>
        </div>

        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">清除缓存</span>
            <span class="setting-description">清理应用缓存文件</span>
          </div>
          <div class="setting-control">
            <el-button type="danger" plain>清除缓存</el-button>
          </div>
        </div>

        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">清除浏览器缓存</span>
            <span class="setting-description">清理内置浏览器缓存数据</span>
          </div>
          <div class="setting-control">
            <el-button type="danger" plain>清除浏览器缓存</el-button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
<style lang="scss" scoped>
.settings-container {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  padding: 20px;
  box-sizing: border-box;

  // 隐藏滚动条
  scrollbar-width: none;
  -ms-overflow-style: none;

  &::-webkit-scrollbar {
    width: 0;
    height: 0;
    background: transparent;
  }
}

.settings-content {
  max-width: 800px;
  margin: 0 auto;
}

.settings-section {
  background: var(--el-bg-color);
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 20px;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.05);
}

.section-title {
  font-size: 1.3rem;
  color: var(--el-text-color-primary);
  margin: 0 0 20px 0;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--el-border-color-light);
  font-weight: 500;
}

.settings-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 15px 0;
  border-bottom: 1px solid var(--el-border-color-lighter);

  &:last-child {
    border-bottom: none;
  }
}

.setting-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.setting-header {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.setting-label {
  display: block;
  font-size: 1rem;
  color: var(--el-text-color-primary);
  font-weight: 500;
}

.setting-description {
  display: block;
  font-size: 0.85rem;
  color: var(--el-text-color-secondary);
}

.directory-display {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
  padding: 10px;
  background-color: var(--el-fill-color-light);
  border-radius: 4px;
  font-family: monospace;
  font-size: 0.9rem;
}

.folder-icon {
  color: var(--el-color-primary);
}

.setting-working-directory {
  color: var(--el-text-color-primary);
  word-break: break-all;
}

.setting-control {
  flex-shrink: 0;
  margin-left: 20px;
}

.el-button {
  transition: all 0.3s;

  &:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }
}

@media (max-width: 768px) {
  .settings-container {
    padding: 15px;
  }

  .settings-item {
    flex-direction: column;
    align-items: flex-start;
  }

  .setting-control {
    margin-left: 0;
    margin-top: 15px;
    width: 100%;
    display: flex;
    justify-content: flex-end;
  }

  .section-title {
    font-size: 1.2rem;
  }
}
</style>