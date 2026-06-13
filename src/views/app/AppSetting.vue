<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { Check, Close, Folder, FolderChecked } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { ResponseData, SettingIdValueName, SettingMap } from '@/entity/entitys'
import { useI18n } from 'vue-i18n'
import useStore from '@/store/index'
import { load, Store } from '@tauri-apps/plugin-store'
import { enable, isEnabled, disable } from '@tauri-apps/plugin-autostart'
import { open } from '@tauri-apps/plugin-dialog'
import { isDark, toggleDark } from '@/composables/useDark'
import { ITagItem } from '@/Types/TagView'
import NotificationUtil from '@/utils/NotificationUtil'
import { tauri_request } from '@/utils/tauri_request'
import { pdf_store_conten_key_prefix, pdf_store_tem_key_prefix } from '@/setting'
import { TreeNode } from '@/Types/entitys'
import { useRouter, useRoute, LocationQueryRaw, RouteParamsRaw } from 'vue-router'
import { check as check_, Update, DownloadEvent } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

const { appConfig, routerNoteStore, tagview } = useStore()
const router = useRouter()
const route = useRoute()
const { t, locale } = useI18n()
const userStore = appConfig
let store: Store | undefined = undefined
let autostart = ref<boolean>(false)
let update_reminder = ref<boolean>(false)
let dark = ref<boolean>(false)
let theme = ref<string>('white')
let language = ref<string>('')
let md_working_directory = ref<string>('')
let has_update = ref<boolean>(false)
let update_info = ref<{
  version?: string;
  date?: string;
  body?: string;
} | null>(null)

const language_options = [
  {
    value: 'zh_CN',
    label: '简体中文',
    disabled: false
  },
  {
    value: 'zh_TW',
    label: '繁体中文（中国台湾）',
    disabled: false
  },
  {
    value: 'zh_HK',
    label: '繁体中文（中国香港）',
    disabled: false
  },
  {
    value: 'en',
    label: 'English',
    disabled: false
  }
]

const theme_options = [
  {
    value: 'white',
    label: '亮色',
    disabled: false
  },
  {
    value: 'black',
    label: '黑色',
    disabled: false
  }
]

const tagViewList = computed<ITagItem[]>(() => tagview.tagViewList)

onMounted(async () => {
  if (userStore.appConfig.length < 1) {
    query_setting_list()
  }
  if (userStore.appConfig.length > 1) {
    show_setting()
  }

  // 检查更新
  try {
    const update_result = await check_();
    if (update_result) {
      has_update.value = true;
      update_info.value = {
        version: update_result.version || '未知版本',
        date: update_result.date || '',
        body: update_result.body || ''
      };
      console.log('Update available:', update_info.value);
    } else {
      has_update.value = false;
      console.log('No updates available');
    }
  } catch (error) {
    console.error('Failed to check for updates:', error);
    has_update.value = false;
  }

  store = await load('settings.json', {
    autoSave: false,
    defaults: {}
  })

  let val_auto_start = await store.get<{ value: boolean }>('auto_start')
  if (val_auto_start !== undefined) {
    autostart.value = val_auto_start.value
    userStore.setAutoStart(val_auto_start.value)
  }
  let language_val = await store.get<{ value: string }>('language')
  if (language_val !== undefined) {
    language.value = language_val.value
    userStore.setLanguage(language_val.value)
  }
  let md_working_directory_val = await store.get<{ value: string }>('md_working_directory')
  if (md_working_directory_val !== undefined) {
    md_working_directory.value = md_working_directory_val.value
    userStore.setMdWorkingDirectory(md_working_directory_val.value)
  }
  let update_reminder_val = await store.get<{ value: boolean }>('update_reminder')
  if (update_reminder_val !== undefined) {
    update_reminder.value = update_reminder_val.value
  }
  let theme_val = await store.get<{ value: string }>('theme')
  if (theme_val !== undefined) {
    theme.value = theme_val.value
    userStore.setTheme(theme_val.value)
  }
  let dark_val = await store.get<{ value: boolean }>('dark')
  if (dark_val !== undefined) {
    dark.value = dark_val.value
    userStore.setDark(dark.value)
  }
})

function show_setting() {}

function query_setting_list() {
  // invoke('query_setting_list').then((res): void => {
  //   let res_ = res as ResponseData<Array<SettingMap>>
  //   let data = res_.data as Array<SettingMap>;
  //   userStore.setAppConfig(data);
  // })
}

function setting_change() {
  // invoke('updatate_setting', { id: id, appSettingItemValue: String(val), appSettingtemName: '' }).then((res): void => {
  //   console.log(res);
  //   query_setting_list()
  // })
}

async function language_change() {
  locale.value = language.value
  console.log('locale.value:=================', locale.value)
  if (store) {
    await store.set('language', { value: language.value })
    await store.save()
  }
}

async function autostart_change() {
  if (store) {
    await store.set('auto_start', { value: autostart.value })
    await store.save()
    if (autostart.value) {
      await enable()
    } else {
      await disable()
    }
  }
}

async function software_change() {
  if (store) {
    await store.set('update_reminder', { value: update_reminder.value })
    await store.save()
  }
}

async function theme_change() {
  if (store) {
    await store.set('theme', { value: theme.value })
    await store.save()
    let el_color_primary = isDark.value ? 'black' : ''
    document.documentElement.style.setProperty('--el-color-primary', el_color_primary)
  }
}

async function dark_change() {
  if (store) {
    await store.set('dark', { value: dark.value })
    await store.save()
    if (!isDark.value && dark.value) {
      toggleDark()
    } else if (isDark.value && !dark.value) {
      toggleDark()
    }
  }
}

async function md_working_directory_change() {
  let file_directory: string = await open({
    multiple: false,
    directory: true
  })

  if (file_directory) {
    md_working_directory.value = file_directory

    //清空tagView
    let tagviews: ITagItem[] = tagview.getTagviewList()
    console.log('tagviews:=================', tagviews)
    for (let tagview of tagviews) {
      let result: boolean = await processNodeCache({
        filePath: tagview.filePath,
        id: tagview.id,
        label: tagview.title,
        isfile: true,
        isFolder: false,
        routerPath: tagview.path,
        folder: ''
      })
      if (!result) {
        console.log('processNodeCache error:', result)
        return
      }
    }

    if (store) {
      await store.set('md_working_directory', { value: file_directory })
      await store.save()
    }

    tagview.deleteTagViewList()
    const routerNote = routerNoteStore.setRoutes()
    console.log('processNodeCache error:========2', routerNote)
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

// 提取公共的缓存处理函数
const processNodeCache = async (item: TreeNode) => {
  try {
    let cached_content: string | null = null
    let content: string | null = null
    const key_tem = pdf_store_tem_key_prefix + item.id
    const key_conten = pdf_store_conten_key_prefix + item.id
    console.log('processNodeCache key_tem key_conten:', key_tem, key_conten)
    const tem_val = await store.get<{ value: string }>(key_tem)
    console.log('processNodeCache tem_val:', tem_val)
    if (tem_val != undefined && tem_val.value != null && tem_val.value != '') {
      cached_content = tem_val.value
    } else {
      console.log('processNodeCache 无需处理:', key_tem, key_conten)
      return true
    }

    const content_val = await store.get<{ value: string }>(key_conten)
    if (content_val !== undefined) {
      content = content_val.value
    }

    // 简化条件判断
    console.log('processNodeCache 简化条件判断:', cached_content, content)
    if (cached_content && content != cached_content) {
      console.log('processNodeCache 简化条件判断:')
      //保存文件
      const save_file_re = await tauri_request<ResponseData<String[]>>('save_note', {
        path: item.filePath,
        mdText: cached_content
      })
      if (save_file_re && save_file_re.code != 200) {
        NotificationUtil.error('文件保存失败：' + save_file_re.msg + '请检查！')
        return false
      }
    }

    let delete_tem: boolean = await store.delete(key_tem)
    let delete_conten: boolean = await store.delete(key_conten)
    console.log('processNodeCache 删除缓存 删除结果:', delete_tem, delete_conten)
    if (!delete_tem || !delete_conten) {
      NotificationUtil.error('缓存删除失败！')
    }
    console.log('processNodeCache 删除缓存 删除结果:', item.routerPath)

    tagview.removeTagViewByPath(item.routerPath)
    return true
  } catch (error) {
    NotificationUtil.error('操作失败：' + error)
    return false
  }
}

const app_updates_check = async () => {
  if (!has_update.value || !update_info.value) {
    NotificationUtil.info('当前已是最新版本');
    return;
  }

  try {
    const version = update_info.value.version || '未知版本';
    const date = update_info.value.date || '';
    const body = update_info.value.body || '';

    console.log(`开始下载更新 ${version} (${date})`);

    let downloaded = 0;
    let contentLength = 0;

    // 重新检查更新以获取 Update 对象
    const update = await check_();
    if (!update) {
      NotificationUtil.error('无法获取更新信息');
      return;
    }

    await update.downloadAndInstall((event: DownloadEvent) => {
      switch (event.event) {
        case 'Started':
          contentLength = event.data.contentLength || 0;
          console.log(`开始下载 ${contentLength} bytes`);
          break;
        case 'Progress':
          downloaded += event.data.chunkLength || 0;
          const progress = contentLength > 0 ? Math.round((downloaded / contentLength) * 100) : 0;
          console.log(`下载进度: ${progress}% (${downloaded}/${contentLength})`);
          break;
        case 'Finished':
          console.log('下载完成');
          break;
      }
    });

    console.log('更新安装完成');
    NotificationUtil.success('更新成功，即将重启应用');
    await relaunch();
  } catch (error) {
    console.error('更新失败:', error);
    NotificationUtil.error('更新失败: ' + (error instanceof Error ? error.message : String(error)));
  }
}
</script>

<template>
  <div class="settings-container">
    <div class="settings-content">
      <!-- 通用设置 -->
      <div class="settings-section">
        <h2 class="section-title">{{ $t('settings.name') }}</h2>

        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">{{ $t('autostart.name') }}</span>
            <span class="setting-description">{{ $t('autostart.hint') }}</span>
          </div>
          <div class="setting-control">
            <el-switch
              v-model="autostart"
              @change="autostart_change()"
              inline-prompt
              :active-icon="Check"
              :inactive-icon="Close"
            />
          </div>
        </div>

        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">{{ $t('update.name') }}</span>
            <span class="setting-description">{{ $t('update.hint') }}</span>
          </div>
          <div class="setting-control">
            <el-switch
              v-model="update_reminder"
              @change="software_change()"
              inline-prompt
              :active-icon="Check"
              :inactive-icon="Close"
              disabled
            />
          </div>
        </div>

        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">{{ $t('settings.theme') }}</span>
            <span class="setting-description">切换主题</span>
          </div>
          <div class="setting-control">
            <el-select
              v-model="theme"
              @change="theme_change"
              placeholder="选择主题"
              style="width: 180px"
            >
              <el-option
                v-for="item in theme_options"
                :key="item.value"
                :label="item.label"
                :value="item.value"
                :disabled="item.disabled"
              />
            </el-select>
          </div>
        </div>

        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">{{ $t('darkMode.name') }}</span>
            <span class="setting-description"></span>
          </div>
          <div class="setting-control">
            <el-switch
              v-model="dark"
              @change="dark_change()"
              inline-prompt
              :active-icon="Check"
              :inactive-icon="Close"
            />
          </div>
        </div>

        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">{{ $t('locale.language') }}</span>
            <span class="setting-description">选择界面显示语言</span>
          </div>
          <div class="setting-control">
            <el-select
              v-model="language"
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
            <el-button type="primary" disabled plain>检测网络</el-button>
          </div>
        </div>
      </div>

      <!-- 更新设置 -->
      <div class="settings-section">
        <h2 class="section-title">更新</h2>
        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">应用更新</span>
            <span class="setting-description">手动检查新版本，获取最新功能和修复</span>
            <span v-if="has_update && update_info" class="update-info">
              发现新版本: {{ update_info.version }} ({{ update_info.date }})
            </span>
          </div>
          <div class="setting-control">
            <el-button
              type="primary"
              @click="app_updates_check"
              :loading="!has_update"
              plain
            >
              {{ has_update ? '立即更新' : '检查更新' }}
            </el-button>
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
              <span class="setting-working-directory">{{ md_working_directory || '未设置' }}</span>
            </div>
          </div>
          <div class="setting-control">
            <el-button type="primary" @click="md_working_directory_change" plain>
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
            <el-button type="danger" disabled plain>清除缓存</el-button>
          </div>
        </div>

        <div class="settings-item">
          <div class="setting-info">
            <span class="setting-label">清除浏览器缓存</span>
            <span class="setting-description">清理内置浏览器缓存数据</span>
          </div>
          <div class="setting-control">
            <el-button type="danger" disabled plain>清除浏览器缓存</el-button>
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

.update-info {
  display: block;
  font-size: 0.85rem;
  color: var(--el-color-success);
  margin-top: 4px;
  font-weight: 500;
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
