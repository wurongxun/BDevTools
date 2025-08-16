<script setup lang="ts">
import { onMounted, reactive } from 'vue'
import { Check, Close } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { ResponseData, SettingIdValueName, SettingMap } from '@/entity/entitys';
import { useI18n } from "vue-i18n";
import { appConfigInfo } from "@/store/appConfigInfo";

const { t, locale } = useI18n()
const userStore = appConfigInfo();

const autostart = reactive(new SettingIdValueName<Boolean>('', '', false))
const update_reminder = reactive(new SettingIdValueName<Boolean>('', '', false))
const language = reactive(new SettingIdValueName<string>('', '', ''))

onMounted(() => {
    if (userStore.appConfig.length < 1) {
        query_setting_list();
    }
    if (userStore.appConfig.length > 1) {
        show_setting()
    }
})

function show_setting() {
    let data = userStore.appConfig as Array<SettingMap>;
    for (let i = 0; i < data.length; i++) {
        const element = data[i];
        if (element.app_setting_item_name == "autostart") {
            autostart.app_setting_item_name = element.app_setting_item_name;
            autostart.id = element.id;
            let str = element.app_setting_item_value
            autostart.app_setting_item_value = (str === "true" ? true : false);
        } else if (element.app_setting_item_name == "update_reminder") {
            update_reminder.app_setting_item_name = element.app_setting_item_name;
            update_reminder.id = element.id;
            let str = element.app_setting_item_value
            update_reminder.app_setting_item_value = (str === "true" ? true : false);
        } else if (element.app_setting_item_name == "language") {
            language.app_setting_item_name = element.app_setting_item_name;
            language.id = element.id;
            language.app_setting_item_value = element.app_setting_item_value;
        }
    }
}

function query_setting_list() {
    invoke('query_setting_list').then((res): void => {
        let res_ = res as ResponseData<Array<SettingMap>>
        let data = res_.data as Array<SettingMap>;
        userStore.setAppConfig(data);
    })
}

function setting_change(val: string, id: string) {
    invoke('updatate_setting', { id: id, appSettingItemValue: String(val), appSettingtemName: '' }).then((res): void => {
        console.log(res);
        query_setting_list()
    })
}

function language_change() {
    locale.value = language.app_setting_item_value;
    invoke('updatate_setting', { id: language.id, appSettingItemValue: String(language.app_setting_item_value), appSettingtemName: 'language' }).then((res): void => {
        console.log(res);
        query_setting_list()
    })
}

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
        value: 'zh_HK',
        label: '繁体中文（中国香港）',
        disabled: false,
    },
    {
        value: 'en',
        label: 'English',
        disabled: false,
    }
]
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
            <el-switch 
              v-model="autostart.app_setting_item_value"
              @change="setting_change(String(autostart.app_setting_item_value), autostart.id)"
              inline-prompt 
              :active-icon="Check" 
              :inactive-icon="Close" 
            />
          </div>
        </div>
        
        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">软件更新提醒</span>
            <span class="setting-description">及时获取新功能和安全更新</span>
          </div>
          <div class="setting-control">
            <el-switch 
              v-model="update_reminder.app_setting_item_value"
              @change="setting_change(String(update_reminder.app_setting_item_value), update_reminder.id)"
              inline-prompt 
              :active-icon="Check" 
              :inactive-icon="Close" 
            />
          </div>
        </div>
        
        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">切换语言</span>
            <span class="setting-description">选择界面显示语言</span>
          </div>
          <div class="setting-control">
            <el-select 
              v-model="language.app_setting_item_value" 
              @change="language_change" 
              placeholder="选择语言"
              style="width: 180px"
            >
              <el-option 
                v-for="item in language_options" 
                :key="item.value" 
                :label="item.label"
                :value="item.value" 
                :disabled="item.disabled" 
              />
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
}

.setting-label {
  display: block;
  font-size: 1rem;
  color: var(--el-text-color-primary);
  margin-bottom: 5px;
  font-weight: 400;
}

.setting-description {
  display: block;
  font-size: 0.85rem;
  color: var(--el-text-color-secondary);
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