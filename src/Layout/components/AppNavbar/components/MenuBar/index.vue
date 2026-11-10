<template>
  <!-- 应用主菜单：menubar 语义 + 方向键导航（WCAG 2.1.1 / 4.1.2） -->
  <div class="menu-bar" role="menubar" aria-label="应用主菜单">
    <div
      v-for="(menu, menuIndex) in menus"
      :key="menu.label"
      class="menu-item"
      :class="{ 'is-active': activeMenu === menu.label }"
    >
      <button
        :ref="(el) => setTopRef(el, menuIndex)"
        class="menu-btn"
        type="button"
        role="menuitem"
        aria-haspopup="true"
        :aria-expanded="activeMenu === menu.label"
        @click="handleMenuClick(menu)"
        @mouseenter="handleMouseEnter(menu.label)"
        @keydown="onTopLevelKeydown($event, menu, menuIndex)"
      >
        <span>{{ menu.label }}</span>
      </button>
      <transition name="menu-dropdown">
        <div
          v-if="activeMenu === menu.label"
          :ref="(el) => setDropdownRef(el)"
          class="menu-dropdown"
          role="menu"
          :aria-label="menu.label"
          @mouseleave="closeMenu"
          @keydown="onDropdownKeydown($event, menuIndex)"
        >
          <template v-for="(item, idx) in menu.items" :key="idx">
            <div
              v-if="item.divider"
              class="menu-divider"
              role="separator"
            ></div>
            <!-- 子菜单项（如"最近的项目"） -->
            <div
              v-else-if="item.submenu"
              class="menu-option has-submenu"
              role="menuitem"
              tabindex="-1"
              :class="{ 'is-disabled': item.disabled }"
              :aria-disabled="item.disabled || undefined"
              @mouseenter="activeSubmenu = item.command"
              @mouseleave="activeSubmenu = null"
              @focus="activeSubmenu = item.command"
            >
              <span class="option-label">{{ item.label }}</span>
              <span class="submenu-arrow" aria-hidden="true">▶</span>
              <!-- 子菜单弹出层 -->
              <div
                v-if="activeSubmenu === item.command"
                class="submenu-popup"
                role="menu"
                :aria-label="item.label"
                @mouseenter="activeSubmenu = item.command"
                @mouseleave="activeSubmenu = null"
              >
                <div
                  v-if="item.submenu.length === 0"
                  class="menu-option is-disabled"
                  role="menuitem"
                  tabindex="-1"
                  aria-disabled="true"
                >
                  <span class="option-label">{{ t('menu.noRecent') }}</span>
                </div>
                <div
                  v-for="(subItem, subIdx) in item.submenu"
                  :key="subIdx"
                  class="menu-option submenu-item"
                  role="menuitem"
                  tabindex="-1"
                  :class="{ 'is-disabled': subItem.disabled }"
                  :aria-disabled="subItem.disabled || undefined"
                  @click.stop="handleMenuCommand(subItem)"
                >
                  <span class="option-label" :title="subItem.label">{{ subItem.label }}</span>
                </div>
                <div v-if="item.submenu.length > 0" class="menu-divider" role="separator"></div>
                <div
                  v-if="item.submenu.length > 0"
                  class="menu-option"
                  role="menuitem"
                  tabindex="-1"
                  @click.stop="handleMenuCommand({ label: '清除最近文件夹', command: 'clearRecentProjects' })"
                >
                  <span class="option-label">{{ t('menu.clearRecent') }}</span>
                </div>
              </div>
            </div>
            <!-- 普通菜单项 -->
            <div
              v-else
              class="menu-option"
              role="menuitem"
              tabindex="-1"
              :class="{ 'is-disabled': item.disabled }"
              :aria-disabled="item.disabled || undefined"
              @click.stop="handleMenuCommand(item)"
            >
              <span class="option-label">{{ item.label }}</span>
              <span v-if="item.shortcut" class="option-shortcut">{{ item.shortcut }}</span>
              <span v-if="item.checked" class="option-check" aria-hidden="true">✓</span>
            </div>
          </template>
        </div>
      </transition>
    </div>

    <!-- 工作区管理对话框（基于 GlobalDialog 的共享组件） -->
    <WorkspaceManageDialog v-model="showWorkspaceDialog" />

    <!-- 命名输入弹窗：替代浏览器原生 prompt()，与桌面应用观感一致且可主题化 -->
    <GlobalPromptDialog
      v-model="showNamePrompt"
      :title="namePromptTitle"
      :message="namePromptMessage"
      :validator="validateName"
      @confirm="handleNameConfirm"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { load, Store, checkUpdate } from '@/core/runtime'
import { settingsRequestedTab } from '@/core/settingsNavigation'
import { invoke } from '@/api/rpc'
import { tauri_request } from '@/utils/tauri_request'
import { ResponseData } from '@/Types/entitys'
import NotificationUtil from '@/utils/NotificationUtil'
import { changeWorkspace } from '@/utils/index'
import { useWorkspaceStore } from '@/store/modules/workspace'
import { useLanguageSwitch, LANGUAGE_OPTIONS } from '@/composables/useLanguageSwitch'
import type { AppLocale } from '@/locales'
import WorkspaceManageDialog from '@/components/WorkspaceManageDialog.vue'
import GlobalPromptDialog from '@/components/GlobalPromptDialog.vue'

import { get_file_Api } from '@/api/file'
import useStore from '@/store'
import { useDocumentManagerStore } from '@/store/modules/documents'
import { subscribe } from '@/core/runtime'
import { WINDOW_PRESETS_CHANGED_EVENT } from '@/core/windowPresets'
import { openAppWindow, createEditorWindow, createPresetWindow } from '@/utils/WindowUtils'
import { pluginManager } from '@/core/pluginManager'
import { openCommandPalette } from '@/composables/useCommandPalette'
import type { WindowPresetConfig } from '@/utils/WindowUtils'
import type { WindowMeta } from '@/Types/workspace'
const {fileTreeStore,useLayoutStore } = useStore()
const workspaceStore = useWorkspaceStore()
const docStore = useDocumentManagerStore()
const { currentLanguage, switchLanguage } = useLanguageSwitch()
const { t } = useI18n()
const activeMenu = ref<string | null>(null)
const activeSubmenu = ref<string | null>(null)
const showWorkspaceDialog = ref(false)
let store: Store | undefined

interface MenuItem {
  label: string
  shortcut?: string
  divider?: boolean
  disabled?: boolean
  command: string
  submenu?: MenuItem[]
  /** 语言菜单用：当前选中的语言项打勾 */
  checked?: boolean
}

interface Menu {
  label: string
  items: MenuItem[]
  action?: string
}

// ===== 窗口预设状态 =====
const windowPresets = ref<WindowPresetConfig[]>([])
const mainWindowPreset = ref<WindowPresetConfig>({
  id: 'main',
  name: t('menu.mainWindow'),
  enabledPlugins: [],
  width: 960,
  height: 640,
  openOnStartup: true,
  focusMode: false
})
const allWindowPresets = computed(() => [mainWindowPreset.value, ...windowPresets.value])
/** 预设变更事件监听取消函数 */
let unlistenPresetsChanged: (() => void) | null = null

/** 判断当前窗口是否启用了 files 插件 */
const hasFilesPlugin = computed(() => {
  const enabled = useLayoutStore.enabledPlugins
  // 空数组表示显示全部插件
  return enabled.length === 0 || enabled.includes('files')
})


/** 结果落在 files 插件的菜单命令：执行前先切过去，否则打开的文件 / 目录变化看不见 */
const FILE_MENU_COMMANDS = new Set(['openFile', 'openWorkingDirectory', 'openRecentProject'])

/**
 * 跳到 files 插件：打开的文件由它的主面板（编辑器）承载，选中的目录由它的侧栏（文件树）呈现。
 * 侧栏收起时顺带展开 —— 与活动栏点击插件的行为一致，否则「跳转」只剩图标高亮、文件树仍不可见。
 * 本窗口未启用或当前构建未注册 files 插件时保持原状，避免把别的插件面板误展开。
 */
function switchToFilesPlugin() {
  if (!hasFilesPlugin.value || !pluginManager.hasPlugin('files')) return
  pluginManager.switchTool('files')
  if (useLayoutStore.sidebarPanelCollapsed) {
    useLayoutStore.setSidebarPanelCollapsed(false)
  }
}

// ===== 最近文件夹 =====
// 统一从当前工作区的 folders 获取（打开文件夹时由 workspace store 置顶写入），不再单独维护
const recentProjectDirs = computed(() => workspaceStore.currentFolders ?? [])

/** 获取目录名用于显示 */
function getDirName(dirPath: string): string {
  const parts = dirPath.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1] || dirPath
}

// ===== 动态菜单 =====
const menus = computed<Menu[]>(() => {
  const fileItems: MenuItem[] = []

  if (hasFilesPlugin.value) {
    fileItems.push({ label: t('menu.openFile'), shortcut: 'Ctrl+O', command: 'openFile' })
    fileItems.push({ label: t('menu.openFolder'), shortcut: 'Ctrl+Shift+O', command: 'openWorkingDirectory' })

    // 最近文件夹子菜单（来源于当前工作区 folders）
    const submenuItems: MenuItem[] = recentProjectDirs.value.map(p => ({
      label: getDirName(p),
      command: 'openRecentProject',
      _dirPath: p
    } as MenuItem & { _dirPath: string }))

    fileItems.push({
      label: t('menu.recentFolders'),
      command: 'recentProjects',
      submenu: submenuItems
    })

    //fileItems.push({ label: '新建窗口', shortcut: 'Ctrl+Shift+N', command: 'newWindow' })
    fileItems.push({ divider: true, label: '', command: '' })
  } else {
    //fileItems.push({ label: '新建窗口', shortcut: 'Ctrl+Shift+N', command: 'newWindow' })
    fileItems.push({ divider: true, label: '', command: '' })
  }

  fileItems.push({ divider: true, label: '', command: '' })

  // 工作区菜单项
  fileItems.push({ label: t('menu.openWorkspace'), command: 'openWorkspace' })
  fileItems.push({ label: t('menu.changeWorkFolder'), shortcut: 'Ctrl+Shift+O', command: 'openWorkingDirectory' })

  // 命令面板：统一唤起插件贡献的命令（插件间不互相 import，只通过命令表交互）
  fileItems.push({ label: t('menu.commandPalette'), shortcut: 'Ctrl+Shift+P', command: 'commandPalette' })
  fileItems.push({ label: t('menu.settings'), command: 'settings' })

  // 窗口菜单项：列出所有配置的窗口预设
  const windowItems: MenuItem[] = allWindowPresets.value.map(preset => ({
    label: preset.name,
    command: `window_preset_${preset.id}`
  }))
  windowItems.push({ divider: true, label: '', command: '' })
  windowItems.push({ label: t('menu.newWindow'), shortcut: 'Ctrl+Shift+N', command: 'newWindow' })

  // 语言切换菜单：与设置页共用 useLanguageSwitch 同一套切换逻辑
  const languageItems: MenuItem[] = LANGUAGE_OPTIONS.map(opt => ({
    label: t(opt.labelKey),
    command: `lang_${opt.value}`,
    checked: opt.value === currentLanguage.value
  }))

  return [
    {
      label: `${t('menu.file')}(F)`,
      items: fileItems
    },
    {
      label: `${t('locale.language')}(L)`,
      items: languageItems
    },
    {
      label: `${t('menu.window')}(W)`,
      items: windowItems
    },
    {
      label: `${t('menu.aboutMenu')}(A)`,
      items: [
        { label: t('menu.aboutApp'), command: 'aboutApp' },
        { divider: true, label: '', command: '' },
        { label: t('menu.checkUpdate'), command: 'checkUpdate' },
        { divider: true, label: '', command: '' },
        { label: t('menu.quitApp'), command: 'quitApp' }
      ]
    }
  ]
})

const getWorkingDirectory = async (): Promise<string> => {
  if (!store) {
    store = await load('settings.json', { autoSave: false, defaults: {} })
  }
  const val = await store.get<{ value: string }>('md_working_directory')
  return val?.value || ''
}
// ===== 命名输入弹窗（替代原生 prompt） =====
const showNamePrompt = ref(false)
const namePromptTitle = ref('')
const namePromptMessage = ref('')
const pendingNameAction = ref<'file' | 'folder'>('file')

/** 文件名/文件夹名校验：拦截空值与 Windows 非法字符 */
function validateName(value: string): string | true {
  if (!value) return t('menu.nameEmpty')
  if (/[\\/:*?"<>|]/.test(value)) return t('menu.nameInvalid')
  return true
}

async function handleNameConfirm(name: string) {
  const dir = await getWorkingDirectory()
  if (!dir) {
    NotificationUtil.warn(t('menu.warnSelectDir'))
    return
  }
  if (pendingNameAction.value === 'file') {
    const res = await tauri_request<ResponseData<string>>('create_note_file', {
      father_path: dir,
      new_note_md_file_name: name
    })
    if (res?.code === 200) {
      NotificationUtil.success(t('menu.fileCreated'))
      await get_file_Api()
    } else {
      NotificationUtil.error(res?.msg || t('menu.createFailed'))
    }
  } else {
    const res = await tauri_request<ResponseData<string>>('create_folder', {
      father_path: dir,
      new_folder_name: name
    })
    if (res?.code === 200) {
      NotificationUtil.success(t('menu.folderCreated'))
      await get_file_Api()
    } else {
      NotificationUtil.error(res?.msg || t('menu.createFailed'))
    }
  }
}

/**
 * 菜单「检查更新」：与设置页共用 `checkUpdate`（同一事实源），这里只做即时反馈；
 * 发现新版本时切到设置插件的「应用」分区，下载安装仍由那边的更新卡片负责。
 */
async function checkForAppUpdate() {
  try {
    const result = await checkUpdate()
    if (result) {
      NotificationUtil.success(t('settingsPlugin.general.updateFoundToast', { version: result.version || '' }))
      settingsRequestedTab.value = 'app'
      pluginManager.switchTool('settings')
    } else {
      NotificationUtil.info(t('settingsPlugin.general.upToDate'))
    }
  } catch (e) {
    NotificationUtil.error(t('settingsPlugin.general.checkUpdateFailed', {
      msg: e instanceof Error ? e.message : String(e)
    }))
  }
}

const handleMenuClick = (menu: Menu) => {
  if (menu.action) {
    handleMenuCommand({ label: '', command: menu.action })
    return
  }
  toggleMenu(menu.label)
}
const handleMenuCommand = async (item: MenuItem) => {
  // 记录当前打开的顶级菜单，关闭后把焦点归还给它（WCAG 2.4.3）
  const openedMenuIndex = menus.value.findIndex((m) => m.label === activeMenu.value)
  closeMenu()
  if (openedMenuIndex >= 0) nextTick(() => focusTopMenu(openedMenuIndex))
  if (item.disabled) return

  // 语言切换：lang_{locale} 命令统一走 useLanguageSwitch
  if (item.command.startsWith('lang_')) {
    await switchLanguage(item.command.slice(5) as AppLocale)
    return
  }

  // 打开文件 / 打开文件夹 / 最近文件夹统一先落到 files 插件，再弹选择框（对话框期间视图已就位）
  if (FILE_MENU_COMMANDS.has(item.command)) switchToFilesPlugin()

  switch (item.command) {
    case 'newFile': {
      const dir = await getWorkingDirectory()
      if (!dir) {
        NotificationUtil.warn(t('menu.warnSelectDir'))
        return
      }
      // 用项目内输入弹窗替代浏览器原生 prompt()：可主题化、可校验
      pendingNameAction.value = 'file'
      namePromptTitle.value = t('menu.newFileTitle')
      namePromptMessage.value = t('menu.newFileMessage', { dir })
      showNamePrompt.value = true
      break
    }
    case 'openFile': {
      const { open } = await import('@/core/runtime')
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Markdown', extensions: ['*'] }]
      })
      if (selected) {
        docStore.openDocument(selected);
      }
      break
    }
    case 'openWorkingDirectory': {
      // changeWorkspace 内 setWorkingDirectory 会触发 workspace store 的 watch，自动置顶写入 folders
      await changeWorkspace();
      break
    }
    case 'openWorkspace': {
      showWorkspaceDialog.value = true
      break
    }
    case 'openRecentProject': {
      const dirPath = (item as MenuItem & { _dirPath: string })._dirPath
      if (dirPath) {
        fileTreeStore.setWorkingDirectory(dirPath)
        // 持久化到 settings.json
        let s: Store | undefined
        if (!s) {
          s = await load('settings.json', { autoSave: false, defaults: {} })
        }
        if (s) {
          await s.set('md_working_directory', { value: dirPath })
          await s.save()
        }
        await get_file_Api()
        // setWorkingDirectory 已触发 workspace watch 将该目录置顶到 folders
      }
      break
    }
    case 'clearRecentProjects': {
      // 清空当前工作区的 folders（最近文件夹列表随工作区 folders 统一管理）
      await workspaceStore.updateCurrentWorkspace({ folders: [] })
      await workspaceStore.loadAllWorkspaces()
      NotificationUtil.success(t('menu.recentCleared'))
      break
    }

    case 'newFolder': {
      const dir = await getWorkingDirectory()
      if (!dir) {
        NotificationUtil.warn(t('menu.warnSelectDir'))
        return
      }
      pendingNameAction.value = 'folder'
      namePromptTitle.value = t('menu.newFolderTitle')
      namePromptMessage.value = t('menu.newFolderMessage', { dir })
      showNamePrompt.value = true
      break
    }
    case 'save': {
      window.dispatchEvent(new CustomEvent('editor-save'))
      break
    }
    case 'saveAs': {
      window.dispatchEvent(new CustomEvent('editor-save-as'))
      break
    }
    case 'settings': {
      // 设置已剥离为内置插件：切到设置插件（入口固定在活动栏底部），不再新开独立窗口
      pluginManager.switchTool('settings')
      break
    }
    case 'commandPalette': {
      openCommandPalette()
      break
    }
    case 'encodingUtf8':
    case 'encodingGbk':
    case 'encodingUtf16':
    case 'encodingAscii': {
      const encoding = item.label
      window.dispatchEvent(new CustomEvent('encoding-change', { detail: { encoding } }))
      NotificationUtil.success(t('menu.encodingChanged', { encoding }))
      break
    }
    case 'aboutApp': {
      // 关于：直达设置插件的「关于」分区，不再另开独立窗口
      settingsRequestedTab.value = 'about'
      pluginManager.switchTool('settings')
      break
    }
    case 'checkUpdate': {
      await checkForAppUpdate()
      break
    }
    case 'quitApp': {
      // 与托盘「退出」同一入口：app.exit(0) 走 ExitRequested，绕过主窗口 close-to-tray
      try {
        await invoke('quit_app')
      } catch (e) {
        console.warn('退出应用失败:', e)
        NotificationUtil.error(t('menu.quitAppFailed'))
      }
      break
    }
    case 'help': {
      openAppWindow('/App/help')
      break
    }
    case 'about': {
      openAppWindow('/App/about')
      break
    }
    case 'newWindow': {
      // 不传工作区 ID，新窗口启动时弹窗让用户选择工作区或新建临时工作区
      const meta = await createEditorWindow()
      if (meta) {
        NotificationUtil.success(t('menu.windowCreated', { label: meta.label }))
      } else {
        NotificationUtil.error(t('menu.createEditorWindowFailed'))
      }
      break
    }
    case 'newTerminal': {
      openAppWindow('/App/terminal')
      break
    }
    default: {
      // 处理窗口预设命令：window_preset_{id}
      if (item.command.startsWith('window_preset_')) {
        const presetId = item.command.replace('window_preset_', '')
        await handleWindowPresetClick(presetId)
      }
      break
    }
  }
}

/** 加载窗口预设配置 */
async function loadWindowPresets() {
  if (!store) {
    store = await load('settings.json', { autoSave: false, defaults: {} })
  }
  const val = await store.get<WindowPresetConfig[]>('window_presets')
  if (val && Array.isArray(val)) {
    windowPresets.value = val
  }
  const mainVal = await store.get<WindowPresetConfig>('main_window_preset')
  if (mainVal) {
    mainWindowPreset.value = { ...mainVal, id: 'main', name: mainVal.name || t('menu.mainWindow') }
  }
}

/** 窗口预设菜单项点击：已打开则聚焦，未打开则创建 */
async function handleWindowPresetClick(presetId: string) {
  const preset = allWindowPresets.value.find(p => p.id === presetId)
  if (!preset) return

  // 先查一次已打开的窗口（命中就聚焦，省掉一次创建往返）。
  //
  // 查不到结果时**必须中止**，不能往下落到创建分支：那两个窗口就会共用同一个预设，而它们的
  // 插件清单各改各的、还会互相顶掉（后端也有同一判据，见 create_editor_window 的 409 —— 这里
  // 只是省往返，不是唯一防线）。
  let windows: WindowMeta[]
  try {
    const res = await invoke<ResponseData<WindowMeta[]>>('get_window_list')
    if (res.code !== 200 || !Array.isArray(res.data)) {
      console.warn('查询窗口列表返回异常:', res.code, res.msg)
      NotificationUtil.error(t('menu.readWindowListFailed'))
      return
    }
    windows = res.data
  } catch (e) {
    console.warn('查询窗口列表失败:', e)
    NotificationUtil.error(t('menu.readWindowListFailed'))
    return
  }

  // 优先按 preset_id 匹配，后备按窗口标题（即预设名称）匹配
  const matched = windows.find(w => w.preset_id === presetId)
    || windows.find(w => w.title === preset.name)
  if (matched) {
    await invoke('focus_window', { label: matched.label })
    return
  }

  // 窗口未打开，根据预设创建新窗口
  const result = await createPresetWindow(preset)
  if (result.ok) return
  switch (result.reason) {
    case 'duplicate':
      // 后端说这个预设的窗口已经开着（前端这份列表略旧）：已聚焦过去，说明一下而不是静默
      NotificationUtil.info(t('menu.presetWindowOpened'))
      break
    case 'workspace-locked':
      NotificationUtil.warn(t('menu.presetWorkspaceLocked'))
      break
    default:
      NotificationUtil.error(t('menu.createWindowFailed'))
  }
}

const toggleMenu = (label: string) => {
  activeMenu.value = activeMenu.value === label ? null : label
}

const handleMouseEnter = (label: string) => {
  if (activeMenu.value) {
    activeMenu.value = label
  }
}

const closeMenu = () => {
  activeMenu.value = null
  activeSubmenu.value = null
}

// ===== 键盘导航（桌面菜单栏标准交互） =====
const topMenuRefs = ref<HTMLElement[]>([])
let dropdownEl: HTMLElement | null = null

function setTopRef(el: unknown, index: number) {
  if (el) topMenuRefs.value[index] = el as HTMLElement
}

function setDropdownRef(el: unknown) {
  dropdownEl = (el as HTMLElement) ?? null
}

/** 当前下拉面板中可操作的菜单项（排除分隔线与禁用项） */
function getMenuOptions(): HTMLElement[] {
  if (!dropdownEl) return []
  return Array.from(dropdownEl.querySelectorAll<HTMLElement>('.menu-option:not(.is-disabled)'))
}

function focusMenuOption(index: number) {
  const options = getMenuOptions()
  if (options.length === 0) return
  const target = (index + options.length) % options.length
  options[target].focus()
}

function focusTopMenu(index: number) {
  const count = menus.value.length
  if (count === 0) return
  topMenuRefs.value[(index + count) % count]?.focus()
}

/** 打开指定序号的顶级菜单并聚焦首项（← → 切换用） */
function openMenuByIndex(index: number) {
  const count = menus.value.length
  if (count === 0) return
  const target = (index + count) % count
  activeMenu.value = menus.value[target].label
  nextTick(() => {
    topMenuRefs.value[target]?.focus()
    focusMenuOption(0)
  })
}

/** 顶级菜单按钮：↓/Enter/空格 展开，← → 切换相邻菜单 */
function onTopLevelKeydown(e: KeyboardEvent, menu: Menu, index: number) {
  switch (e.key) {
    case 'ArrowDown':
    case 'Enter':
    case ' ':
      e.preventDefault()
      activeMenu.value = menu.label
      nextTick(() => focusMenuOption(0))
      break
    case 'ArrowRight':
      e.preventDefault()
      openMenuByIndex(index + 1)
      break
    case 'ArrowLeft':
      e.preventDefault()
      openMenuByIndex(index - 1)
      break
    case 'Escape':
      closeMenu()
      break
  }
}

/** 下拉面板：↑↓/Home/End 移动，Enter 激活，Esc 关闭并归还焦点，← → 切换顶级菜单 */
function onDropdownKeydown(e: KeyboardEvent, menuIndex: number) {
  const options = getMenuOptions()
  const current = options.indexOf(document.activeElement as HTMLElement)

  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault()
      focusMenuOption(current + 1)
      break
    case 'ArrowUp':
      e.preventDefault()
      focusMenuOption(current <= 0 ? options.length - 1 : current - 1)
      break
    case 'Home':
      e.preventDefault()
      focusMenuOption(0)
      break
    case 'End':
      e.preventDefault()
      focusMenuOption(options.length - 1)
      break
    case 'Enter':
    case ' ':
      e.preventDefault()
      if (document.activeElement instanceof HTMLElement) document.activeElement.click()
      break
    case 'Escape':
      e.preventDefault()
      closeMenu()
      nextTick(() => focusTopMenu(menuIndex))
      break
    case 'ArrowRight':
      e.preventDefault()
      openMenuByIndex(menuIndex + 1)
      break
    case 'ArrowLeft':
      e.preventDefault()
      openMenuByIndex(menuIndex - 1)
      break
    case 'Tab':
      // 菜单栏内部用 Tab 退出，避免焦点陷入（WCAG 2.1.2）
      e.preventDefault()
      closeMenu()
      nextTick(() => focusTopMenu(menuIndex))
      break
  }
}

/**
 * 按 Alt+字母 匹配菜单：从 label 尾部的 (X) 中解析快捷键，
 * 例如「文件(F)」→ f。原先硬编码 { f: '文件' }，与实际 label 不匹配导致快捷键恒失效。
 */
function resolveAltMenuLabel(key: string): string | undefined {
  const lowered = key.toLowerCase()
  return menus.value.find((m) => {
    const matched = m.label.match(/\(([A-Za-z])\)\s*$/)
    return matched?.[1]?.toLowerCase() === lowered
  })?.label
}

const handleGlobalClick = (e: MouseEvent) => {
  const target = e.target as HTMLElement
  if (!target.closest('.menu-bar')) {
    closeMenu()
  }
}

const handleKeydown = (e: KeyboardEvent) => {
  // Alt 组合键：仅在命中菜单快捷键时拦截，避免吞掉其它 Alt 组合
  if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
    const label = resolveAltMenuLabel(e.key)
    if (label) {
      e.preventDefault()
      e.stopPropagation()
      if (activeMenu.value === label) {
        closeMenu()
        return
      }
      activeMenu.value = label
      nextTick(() => {
        const index = menus.value.findIndex((m) => m.label === label)
        if (index >= 0) {
          topMenuRefs.value[index]?.focus()
          focusMenuOption(0)
        }
      })
      return
    }
  }
  if (e.key === 'Escape') {
    closeMenu()
  }
}

onMounted(async () => {
  document.addEventListener('click', handleGlobalClick)
  document.addEventListener('keydown', handleKeydown)
  await loadWindowPresets()
  // 加载当前工作区（新窗口等待用户选择期间不自动分配）
  if (!workspaceStore.pendingWorkspaceSelection) {
    await workspaceStore.loadCurrentWorkspace(null)
  }
  // 监听窗口预设变更（设置页新增/修改/删除预设后广播），实时刷新窗口菜单
  unlistenPresetsChanged = await subscribe(WINDOW_PRESETS_CHANGED_EVENT, () => {
    loadWindowPresets()
  })
})

onUnmounted(() => {
  document.removeEventListener('click', handleGlobalClick)
  document.removeEventListener('keydown', handleKeydown)
  unlistenPresetsChanged?.()
  unlistenPresetsChanged = null
})
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss';

.menu-bar {
  display: flex;
  align-items: center;
  height: 100%;
  user-select: none;

  .menu-item {
    position: relative;
    height: 100%;
    display: flex;
    align-items: center;
    white-space: nowrap;

    .menu-btn {
      height: 100%;
      padding: 0 8px;
      display: flex;
      align-items: center;
      cursor: pointer;
      font-size: var(--font-size-base);
      font-family: inherit;
      transition: background-color 0.15s;
      // 语义化 <button> 需要抹平浏览器默认外观
      appearance: none;
      border: none;
      background: transparent;

      @include theme.useTheme {
        color: theme.getVar('textColor');
      }

      &:hover {
        @include theme.useTheme {
          background-color: theme.getVar('activityBarHover');
        }
      }
    }

    &.is-active .menu-btn {
      @include theme.useTheme {
        background-color: theme.getVar('activityBarHover');
      }
    }

    .menu-dropdown {
      position: absolute;
      top: 100%;
      left: 0;
      min-width: 200px;
      padding: 4px 0;
      z-index: 99999;
      border-radius: var(--radius-md);

      @include theme.useTheme {
        background-color: theme.getVar('bgColor');
        border: 1px solid theme.getVar('borderColor');
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
      }

      .menu-divider {
        height: 1px;
        margin: 4px 0;

        @include theme.useTheme {
          background-color: theme.getVar('borderColor');
        }
      }

      .menu-option {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 5px 14px;
        font-size: var(--font-size-base);
        cursor: pointer;
        transition: background-color 0.15s;
        position: relative;

        @include theme.useTheme {
          color: theme.getVar('textColor');
        }

        &:hover,
        &:focus-visible {
          @include theme.useTheme {
            background-color: theme.getVar('hoverColor');
            // 文字保持主文本色：原先写死 #fff，在亮色 hover 底色 #ecf5ff 上仅 1.10:1，
            // 文字等同消失（WCAG 1.4.3 需 4.5:1）
            color: theme.getVar('textColor');
          }

          .option-shortcut {
            opacity: 1;
          }
        }

        // 暗色主题 hover 底色较深（IDEA_MENU_HOVER #4b6eaf），主文本色 #bbbbbb 仅 2.64:1，
        // 改用纯白达到 5.06:1
        html.dark &:hover,
        html.dark &:focus-visible {
          color: #fff;
        }

        &.is-disabled {
          cursor: not-allowed;
          // 用专用禁用色令牌替代 opacity: 0.4（后者实测仅 2.43:1 / 2.16:1）
          @include theme.useTheme {
            color: theme.getVar('textDisabled');
          }

          &:hover,
          &:focus-visible {
            background-color: transparent;

            @include theme.useTheme {
              color: theme.getVar('textDisabled');
            }
          }
        }

        .option-label {
          flex: 1;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        .option-shortcut {
          font-size: var(--font-size-sm);
          opacity: 0.5;
          margin-left: 24px;
        }

        .option-check {
          font-size: var(--font-size-sm);
          margin-left: 24px;
          flex-shrink: 0;

          @include theme.useTheme {
            color: theme.getVar('focusRing');
          }
        }

        .submenu-arrow {
          font-size: var(--font-size-xs);
          opacity: 0.5;
          margin-left: 12px;
        }

        // 子菜单弹出层
        &.has-submenu {
          .submenu-popup {
            position: absolute;
            left: 100%;
            top: 0;
            min-width: 280px;
            max-height: 360px;
            overflow-y: auto;
            padding: 4px 0;
            z-index: 100000;
            border-radius: var(--radius-md);

            @include theme.useTheme {
              background-color: theme.getVar('bgColor');
              border: 1px solid theme.getVar('borderColor');
              box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
            }

            .submenu-item .option-label {
              font-size: var(--font-size-base);
            }
          }
        }
      }
    }
  }
}

.menu-dropdown-enter-active {
  transition: all 0.15s ease;
}

.menu-dropdown-leave-active {
  transition: all 0.1s ease;
}

.menu-dropdown-enter-from {
  opacity: 0;
  transform: translateY(-4px);
}

.menu-dropdown-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
