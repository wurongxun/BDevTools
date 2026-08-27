<script setup lang="ts">
import { onMounted, reactive, ref, computed } from 'vue'
import { Check, Close, Plus, Delete, Edit } from '@element-plus/icons-vue'
import { invoke } from '@/api/rpc'
import { ResponseData, SettingIdValueName, SettingMap } from '@/entity/entitys'
import { useI18n } from 'vue-i18n'
import useStore from '@/store/index'
import { load, Store } from '@/core/runtime'
import { autostartEnable, autostartDisable } from '@/core/runtime'
import { open } from '@/core/runtime'
import { isDark, toggleDark } from '@/composables/useDark'
import NotificationUtil from '@/utils/NotificationUtil'
import { tauri_request } from '@/utils/tauri_request'
import { pdf_store_conten_key_prefix, pdf_store_tem_key_prefix } from '@/setting'
import { TreeNode } from '@/Types/entitys'
import { checkUpdate, relaunch } from '@/core/runtime'
import type { DownloadEvent } from '@/core/runtime'
import { pluginManager } from '@/core/pluginManager'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { workspaceService } from '@/services/workspaceService'
import type { WorkspaceSummary } from '@/Types/workspace'
import { emit } from '@/core/runtime'

const { appConfig } = useStore()
const { t, locale } = useI18n()

/** 当前激活的设置页签 */
const activeSettingTab = ref<'app' | 'window'>('app')
const userStore = appConfig
let store: Store | undefined = undefined
let autostart = ref<boolean>(false)
let update_reminder = ref<boolean>(false)
let dark = ref<boolean>(false)
let theme = ref<string>('white')
let language = ref<string>('')
let read_lines = ref<number>(50000)
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



onMounted(async () => {
  if (userStore.appConfig.length < 1) {
    query_setting_list()
  }
  if (userStore.appConfig.length > 1) {
    show_setting()
  }

  // 检查更新
  try {
    const update_result = await checkUpdate();
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
  let read_lines_val = await store.get<{ value: number }>('read_lines')
  if (read_lines_val !== undefined) {
    read_lines.value = Number(read_lines_val.value)
    userStore.setReadLines(Number(read_lines_val.value))
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

  // 加载窗口预设
  await loadWindowPresets()
  await loadWorkspaceOptions()
})

function show_setting() { }

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
      await autostartEnable()
    } else {
      await autostartDisable()
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
// 步骤1：注册应用到"打开方式"列表
async function handleRegisterAssociation() {
  const res = await invoke<ResponseData<any>>('register_file_association')
  if (res.code === 200) {
    // 步骤2：打开 Windows 默认应用设置页面
    await invoke('open_default_apps_settings')
  }
}

async function onReadLines() {
  if (store) {
    await store.set('read_lines', { value: read_lines.value })
    await store.save();
    userStore.setReadLines(read_lines.value)
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

// ==================== 窗口预设管理 ====================

/** 窗口预设配置 */
interface WindowPresetConfig {
  id: string
  name: string
  enabledPlugins: string[]
  width: number
  height: number
  openOnStartup: boolean
  focusMode: boolean
  /** 绑定的工作区 ID（可选；多个预设可绑同一工作区，但同时只能打开一个） */
  workspaceId?: string
}

const windowPresets = ref<WindowPresetConfig[]>([])
const showWindowEditDialog = ref(false)
const editingPreset = ref<WindowPresetConfig | null>(null)
const isCreatingPreset = ref(false)

/** 主窗口预设（不可删除，始终启动） */
const mainWindowPreset = ref<WindowPresetConfig>({
  id: 'main',
  name: '主窗口',
  enabledPlugins: [],
  width: 960,
  height: 640,
  openOnStartup: true,
  focusMode: false
})

/** 可选的窗口尺寸 */
const sizeOptions = [
  { label: '小 (800×600)', width: 800, height: 600 },
  { label: '中 (1024×768)', width: 1024, height: 768 },
  { label: '大 (1280×900)', width: 1280, height: 900 },
  { label: '超大 (1920×1080)', width: 1920, height: 1080 },
  { label: '自定义', width: 0, height: 0 }
]
const selectedSizeLabel = ref('中 (1024×768)')

/** 所有已注册的插件 */
const registeredPlugins = computed(() => pluginManager.plugins)

/** 可绑定的工作区列表（排除临时工作区） */
const workspaceOptions = ref<WorkspaceSummary[]>([])

/** 加载工作区列表（供预设绑定选择） */
async function loadWorkspaceOptions() {
  try {
    const list = await workspaceService.list()
    const detail = await Promise.all(list.map(ws => workspaceService.get(ws.id).catch(() => null)))
    workspaceOptions.value = list.filter((_, i) => detail[i]?.settings?.isTemporary !== true)
  } catch (e) {
    console.warn('加载工作区列表失败:', e)
  }
}

/** 预设绑定的工作区名称（卡片展示用） */
function presetWorkspaceName(preset: WindowPresetConfig): string {
  if (!preset.workspaceId) return ''
  return workspaceOptions.value.find(ws => ws.id === preset.workspaceId)?.name ?? '(工作区已删除)'
}

/** 预设变更后广播通知，其他窗口的菜单栏监听后重新加载预设列表 */
async function notifyPresetsChanged() {
  try {
    await emit('window-presets-changed')
  } catch (e) {
    console.warn('广播窗口预设变更失败:', e)
  }
}

/** 当前编辑的预设是否为新建 */
const editingIsNew = computed(() => isCreatingPreset.value)

/** 当前编辑的是否为主窗口预设 */
const isEditingMain = computed(() => editingPreset.value?.id === 'main')

/** 所有预设列表（主窗口 + 其他窗口） */
const allPresets = computed(() => [mainWindowPreset.value, ...windowPresets.value])

/** 当前尺寸是否为自定义 */
const isCustomSize = computed(() => {
  if (!editingPreset.value) return false
  return !sizeOptions.some(s => s.width === editingPreset.value!.width && s.height === editingPreset.value!.height && s.width > 0)
})

/** 已保存的预设中开启了"启动时打开"的数量 */
const savedStartupCount = computed(() => windowPresets.value.filter(p => p.openOnStartup).length)

/** 加载窗口预设 */
async function loadWindowPresets() {
  if (!store) return
  const val = await store.get<WindowPresetConfig[]>('window_presets')
  if (val && Array.isArray(val)) {
    windowPresets.value = val
  }
  // 加载主窗口预设
  const mainVal = await store.get<WindowPresetConfig>('main_window_preset')
  if (mainVal) {
    mainWindowPreset.value = { ...mainVal, id: 'main', name: mainVal.name || '主窗口' }
  }
}

/** 保存窗口预设 */
async function saveWindowPresets() {
  if (!store) return
  await store.set('window_presets', windowPresets.value)
  await store.save()
}

/** 添加新窗口预设 */
function addWindowPreset() {
  isCreatingPreset.value = true
  editingPreset.value = {
    id: 'window-' + Date.now(),
    name: '新窗口',
    enabledPlugins: registeredPlugins.value.map(p => p.id),
    width: 1024,
    height: 768,
    openOnStartup: false,
    focusMode: false,
    workspaceId: ''
  }
  selectedSizeLabel.value = '中 (1024×768)'
  showWindowEditDialog.value = true
}

/** 编辑窗口预设 */
function editWindowPreset(preset: WindowPresetConfig) {
  isCreatingPreset.value = false
  editingPreset.value = { ...preset, enabledPlugins: [...preset.enabledPlugins] }
  const matched = sizeOptions.find(s => s.width === preset.width && s.height === preset.height && s.width > 0)
  selectedSizeLabel.value = matched?.label || '自定义'
  showWindowEditDialog.value = true
}

/** 删除窗口预设 */
async function removeWindowPreset(id: string) {
  if (id === 'main') return // 主窗口不可删除
  const preset = windowPresets.value.find(p => p.id === id)
  if (preset?.openOnStartup && windowPresets.value.filter(p => p.openOnStartup).length <= 1) {
    NotificationUtil.error('至少保留一个启动时打开的窗口')
    return
  }
  windowPresets.value = windowPresets.value.filter(p => p.id !== id)
  await saveWindowPresets()
  await notifyPresetsChanged()
}

/** 保存编辑的窗口预设 */
async function saveEditingPreset() {
  if (!editingPreset.value) return
  if (!editingPreset.value.name.trim()) {
    NotificationUtil.error('请输入窗口名称')
    return
  }
  if (editingPreset.value.focusMode && editingPreset.value.enabledPlugins.length !== 1) {
    NotificationUtil.error('专注模式下只能选择一个插件')
    return
  }
  // 主窗口可以不选插件（主窗口侧边栏可为空），其他窗口至少选一个
  if (editingPreset.value.id !== 'main' && editingPreset.value.enabledPlugins.length === 0) {
    NotificationUtil.error('至少选择一个插件')
    return
  }
  // 检查窗口名称是否重复（跨主窗口和其他预设）
  const currentName = editingPreset.value.name.trim()
  const nameConflict = allPresets.value.some(
    p => p.id !== editingPreset.value!.id && p.name.trim() === currentName
  )
  if (nameConflict) {
    NotificationUtil.error('窗口名称不能重复')
    return
  }

  // 主窗口预设：单独存储（主窗口始终使用默认工作区，不保存绑定）
  if (editingPreset.value.id === 'main') {
    const config = { ...editingPreset.value }
    delete config.workspaceId
    mainWindowPreset.value = config
    if (store) {
      await store.set('main_window_preset', config)
      // 同时写入 window_layout_main 供主窗口启动时读取
      await store.set('window_layout_main', {
        enabledPlugins: config.enabledPlugins,
        focusMode: config.focusMode,
        presetId: 'main'
      })
      await store.save()
    }
    showWindowEditDialog.value = false
    editingPreset.value = null
    await notifyPresetsChanged()
    return
  }

  if (isCreatingPreset.value) {
    windowPresets.value.push({ ...editingPreset.value })
  } else {
    const idx = windowPresets.value.findIndex(p => p.id === editingPreset.value!.id)
    if (idx !== -1) {
      windowPresets.value[idx] = { ...editingPreset.value }
    }
  }
  await saveWindowPresets()
  showWindowEditDialog.value = false
  editingPreset.value = null
  await notifyPresetsChanged()
}

/** 尺寸选择变化 */
function onSizeChange(label: string) {
  const opt = sizeOptions.find(s => s.label === label)
  if (opt && opt.width > 0 && editingPreset.value) {
    editingPreset.value.width = opt.width
    editingPreset.value.height = opt.height
  }
}

/** 切换插件选中状态（专注模式下为单选：点击即替换） */
function togglePlugin(pluginId: string) {
  if (!editingPreset.value) return
  const idx = editingPreset.value.enabledPlugins.indexOf(pluginId)
  if (idx === -1) {
    if (editingPreset.value.focusMode) {
      // 专注模式：单选替换，只保留当前点击的插件
      editingPreset.value.enabledPlugins = [pluginId]
    } else {
      editingPreset.value.enabledPlugins.push(pluginId)
    }
  } else {
    // 已选中的插件：专注模式下不允许取消（至少保留一个），普通模式可取消
    if (!editingPreset.value.focusMode) {
      editingPreset.value.enabledPlugins.splice(idx, 1)
    }
  }
}

/** 切换专注模式 */
function toggleFocusMode(val: boolean) {
  if (!editingPreset.value) return
  editingPreset.value.focusMode = val
  if (val && editingPreset.value.enabledPlugins.length > 1) {
    // 专注模式开启时只保留第一个插件
    editingPreset.value.enabledPlugins = [editingPreset.value.enabledPlugins[0]]
  }
}

// 提取公共的缓存处理函数
const processNodeCache = async (item: TreeNode) => {
  if (!store) {
    NotificationUtil.error('Store 未初始化')
    return false
  }
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
    const update = await checkUpdate();
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

function handleClose() {
  showWindowEditDialog.value = false
}
</script>

<template>
  <div class="settings-layout">
    <!-- 左侧导航 -->
    <div class="settings-nav">
      <div class="settings-nav-item" :class="{ active: activeSettingTab === 'app' }" @click="activeSettingTab = 'app'">
        应用设置
      </div>
      <div class="settings-nav-item" :class="{ active: activeSettingTab === 'window' }"
        @click="activeSettingTab = 'window'">
        窗口设置
      </div>
    </div>

    <!-- 右侧内容区 -->
    <div class="settings-main">
      <!-- 应用设置 -->
      <div v-show="activeSettingTab === 'app'" class="settings-content">
        <!-- 通用设置 -->
        <div class="settings-section">
          <h2 class="section-title">{{ $t('settings.name') }}</h2>

          <div class="settings-item">
            <div class="setting-info">
              <span class="setting-label">{{ $t('autostart.name') }}</span>
              <span class="setting-description">{{ $t('autostart.hint') }}</span>
            </div>
            <div class="setting-control">
              <el-switch v-model="autostart" @change="autostart_change()" inline-prompt :active-icon="Check"
                :inactive-icon="Close" />
            </div>
          </div>



          <div class="settings-item">
            <div class="setting-info">
              <span class="setting-label">{{ $t('darkMode.name') }}</span>
              <span class="setting-description"></span>
            </div>
            <div class="setting-control">
              <el-switch v-model="dark" @change="dark_change()" inline-prompt :active-icon="Check"
                :inactive-icon="Close" />
            </div>
          </div>

          <div class="settings-item">
            <div class="setting-info">
              <span class="setting-label">{{ $t('locale.language') }}</span>
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
              <el-button type="primary" @click="app_updates_check" :loading="!has_update" plain>
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
              <span class="setting-label">文本默认读取行数</span>
              <span class="setting-description">编辑器打开文件时默认读取的行数，建议最大值不超过20万行，行数直接影响内存占用，请谨慎设置</span>
            </div>
            <div class="setting-control">
              <div class="read-lines-control">
                <el-input v-model="read_lines" placeholder="默认读取行数" @keyup.enter="onReadLines" style="width: 180px" />
                <el-button type="primary" @click="onReadLines" plain>
                  确认
                </el-button>
              </div>
            </div>
          </div>
          <div class="settings-item">
            <div class="setting-info">
              <span class="setting-label">注册 </span>
              <span class="setting-description">注册应用到windows"打开方式"列表 </span>
            </div>
            <div class="setting-control">
              <el-button type="primary" @click="handleRegisterAssociation" plain>
                注册
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

      <!-- 窗口设置 -->
      <div v-show="activeSettingTab === 'window'" class="settings-content">
        <div class="settings-section">
          <h2 class="section-title">窗口</h2>
          <p class="section-desc">配置应用启动时自动打开的窗口，以及每个窗口的功能和布局。</p>

          <!-- 窗口预设列表 -->
          <div class="window-preset-list">
            <div v-for="preset in allPresets" :key="preset.id" class="window-preset-card">
              <div class="preset-info">
                <div class="preset-name-row">
                  <span class="preset-name">{{ preset.name }}</span>
                  <el-tag v-if="preset.id === 'main'" size="small" type="primary">主窗口</el-tag>
                  <el-tag v-if="preset.openOnStartup && preset.id !== 'main'" size="small" type="success">启动时打开</el-tag>
                  <el-tag v-if="preset.focusMode" size="small" type="warning">专注模式</el-tag>
                </div>
                <div class="preset-meta">
                  <span>插件: {{ preset.enabledPlugins.length > 0 ? preset.enabledPlugins.join(', ') : '全部' }}</span>
                  <span class="preset-sep">|</span>
                  <span>{{ preset.width }} × {{ preset.height }}</span>
                  <template v-if="preset.workspaceId">
                    <span class="preset-sep">|</span>
                    <span>工作区: {{ presetWorkspaceName(preset) }}</span>
                  </template>
                </div>
              </div>
              <div class="preset-actions">
                <el-button size="small" @click="editWindowPreset(preset)" :icon="Edit" circle />
                <el-button v-if="preset.id !== 'main'" size="small" @click="removeWindowPreset(preset.id)"
                  :icon="Delete" circle type="danger" />
              </div>
            </div>
          </div>

          <el-button type="primary" @click="addWindowPreset" plain>
            <el-icon>
              <Plus />
            </el-icon>
            添加窗口
          </el-button>
        </div>
      </div>
    </div>

    <!-- 窗口编辑弹窗 -->

    <GlobalDialog v-model="showWindowEditDialog" :title="editingIsNew ? '添加窗口' : '编辑窗口'" width="60%"
      @close="handleClose">
      <div v-if="editingPreset" class="window-edit-form">
        <!-- 窗口名称 -->
        <div class="form-item">
          <label class="form-label">窗口名称</label>
          <el-input v-model="editingPreset.name" placeholder="输入窗口名称" />
        </div>

        <!-- 绑定工作区（主窗口始终使用默认工作区，不提供绑定） -->
        <div class="form-item" v-if="!isEditingMain">
          <label class="form-label">绑定工作区</label>
          <el-select v-model="editingPreset.workspaceId" placeholder="不绑定（打开时选择）" clearable
            style="width: 100%">
            <el-option v-for="ws in workspaceOptions" :key="ws.id" :label="ws.name" :value="ws.id" />
          </el-select>
          <span class="form-hint">多个窗口可绑定同一工作区，但同一工作区同时只能在一个窗口中打开</span>
        </div>

        <!-- 启用插件 -->
        <div class="form-item">
          <label class="form-label">启用插件</label>
          <div class="plugin-checkbox-list">
            <el-checkbox v-for="plugin in registeredPlugins" :key="plugin.id"
              :model-value="editingPreset.enabledPlugins.includes(plugin.id)" @change="togglePlugin(plugin.id)">
              {{ plugin.label }}
            </el-checkbox>
          </div>
        </div>

        <!-- 窗口尺寸 -->
        <div class="form-item">
          <label class="form-label">窗口尺寸</label>
          <el-select v-model="selectedSizeLabel" @change="onSizeChange" style="width: 100%">
            <el-option v-for="opt in sizeOptions" :key="opt.label" :label="opt.label" :value="opt.label" />
          </el-select>
          <div v-if="isCustomSize" class="custom-size-inputs">
            <el-input-number v-model="editingPreset.width" :min="400" :max="3840" placeholder="宽" />
            <span class="size-sep">×</span>
            <el-input-number v-model="editingPreset.height" :min="300" :max="2160" placeholder="高" />
          </div>
        </div>

        <!-- 启动时打开 -->
        <div class="form-item form-item-row">
          <div class="form-label-group">
            <label class="form-label">启动时打开</label>
            <span class="form-hint">{{ isEditingMain ? '主窗口默认随应用启动，可手动关闭' : '应用启动时自动打开此窗口，至少保留一个' }}</span>
          </div>
          <el-switch v-model="editingPreset.openOnStartup"
            :disabled="!isEditingMain && editingPreset.openOnStartup && !editingIsNew && savedStartupCount <= 1"
            inline-prompt :active-icon="Check" :inactive-icon="Close" />
        </div>

        <!-- 专注模式 -->
        <div class="form-item form-item-row">
          <div class="form-label-group">
            <label class="form-label">专注模式</label>
            <span class="form-hint">只保留一个插件，左侧边栏默认隐藏</span>
          </div>
          <el-switch :model-value="editingPreset.focusMode" @change="toggleFocusMode" inline-prompt :active-icon="Check"
            :inactive-icon="Close" />
        </div>
      </div>
      <template #footer>
        <el-button @click="showWindowEditDialog = false">取消</el-button>
        <el-button type="primary" @click="saveEditingPreset">保存</el-button>
      </template>
    </GlobalDialog>
  </div>
</template>

<style lang="scss" scoped>
@use '@/styles/theme.scss' as theme;

.settings-layout {
  width: 100%;
  height: 100%;
  display: flex;
  overflow: hidden;

  @include theme.useTheme {
    background-color: theme.getVar('appMainBgColor');
  }
}

// ---- 左侧导航 ----
.settings-nav {
  width: 160px;
  flex-shrink: 0;
  padding: 12px 6px;
  display: flex;
  flex-direction: column;
  gap: 2px;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    border-right: 1px solid theme.getVar('borderColor');
  }
}

.settings-nav-item {
  padding: 8px 14px;
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
  transition: background-color 0.15s;
  user-select: none;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('activityBarHover');
      color: theme.getVar('textColor');
    }
  }

  &.active {
    @include theme.useTheme {
      background-color: theme.getVar('activityBarHover');
      color: theme.getVar('textColor');
    }

    font-weight: 500;
  }
}

// ---- 右侧内容区 ----
.settings-main {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.settings-content {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  padding: 16px 20px;
  box-sizing: border-box;
  scrollbar-width: none;
  -ms-overflow-style: none;

  &::-webkit-scrollbar {
    width: 0;
    height: 0;
    background: transparent;
  }
}

// ---- 分组区块（扁平化，无阴影） ----
.settings-section {
  border-radius: 4px;
  padding: 16px;
  margin-bottom: 12px;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderColor');
  }
}

.section-title {
  font-size: 14px;
  margin: 0 0 12px 0;
  padding-bottom: 8px;
  font-weight: 600;

  @include theme.useTheme {
    color: theme.getVar('textColor');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

// ---- 设置项 ----
.settings-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 0;

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }

  &:last-child {
    border-bottom: none;
  }
}

.setting-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.setting-label {
  display: block;
  font-size: 13px;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.setting-description {
  display: block;
  font-size: 12px;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.update-info {
  display: block;
  font-size: 12px;
  margin-top: 4px;
  font-weight: 500;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.setting-control {
  flex-shrink: 0;
  margin-left: 16px;
}

.read-lines-control {
  display: flex;
  align-items: center;
  gap: 8px;
}

// ---- 按钮：桌面风格，无浮动效果 ----
.el-button {
  transition: background-color 0.15s;

  &:hover {
    transform: none;
    box-shadow: none;
  }
}

// ---- 窗口预设 ----
.section-desc {
  font-size: 12px;
  margin: -8px 0 12px 0;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.window-preset-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}

.window-preset-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-radius: 4px;
  transition: background-color 0.15s;

  @include theme.useTheme {
    border: 1px solid theme.getVar('borderColor');
    background-color: theme.getVar('appMainBgColor');
  }

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('activityBarHover');
    }
  }
}

.preset-info {
  flex: 1;
  min-width: 0;
}

.preset-name-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 3px;
}

.preset-name {
  font-size: 13px;
  font-weight: 500;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.preset-meta {
  font-size: 12px;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.preset-sep {
  margin: 0 6px;
  opacity: 0.4;
}

.preset-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
  margin-left: 12px;
}

.window-preset-empty {
  text-align: center;
  padding: 24px;
  font-size: 13px;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

// ---- 编辑弹窗表单 ----
.window-edit-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.form-item {
  display: flex;
  flex-direction: column;
  gap: 4px;

  &.form-item-row {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
  }
}

.form-label {
  font-size: 13px;
  font-weight: 500;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.form-label-group {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.form-hint {
  font-size: 12px;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.plugin-checkbox-list {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.custom-size-inputs {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 6px;

  .el-input-number {
    flex: 1;
  }
}

.size-sep {
  font-size: 13px;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}
</style>
