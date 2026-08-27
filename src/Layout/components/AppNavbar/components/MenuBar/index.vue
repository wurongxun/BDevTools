<template>
  <div class="menu-bar" >
    <div
      v-for="menu in menus"
      :key="menu.label"
      class="menu-item"
      :class="{ 'is-active': activeMenu === menu.label }"
      @click="handleMenuClick(menu)"
      @mouseenter="handleMouseEnter(menu.label)"
    >
      <span>{{ menu.label }}</span>
      <transition name="menu-dropdown">
        <div
          v-if="activeMenu === menu.label"
          class="menu-dropdown"
          @mouseleave="closeMenu"
        >
          <template v-for="(item, idx) in menu.items" :key="idx">
            <div
              v-if="item.divider"
              class="menu-divider"
            ></div>
            <!-- 子菜单项（如"最近的项目"） -->
            <div
              v-else-if="item.submenu"
              class="menu-option has-submenu"
              :class="{ 'is-disabled': item.disabled }"
              @mouseenter="activeSubmenu = item.command"
              @mouseleave="activeSubmenu = null"
            >
              <span class="option-label">{{ item.label }}</span>
              <span class="submenu-arrow">▶</span>
              <!-- 子菜单弹出层 -->
              <div
                v-if="activeSubmenu === item.command"
                class="submenu-popup"
                @mouseenter="activeSubmenu = item.command"
                @mouseleave="activeSubmenu = null"
              >
                <div
                  v-if="item.submenu.length === 0"
                  class="menu-option is-disabled"
                >
                  <span class="option-label">暂无记录</span>
                </div>
                <div
                  v-for="(subItem, subIdx) in item.submenu"
                  :key="subIdx"
                  class="menu-option submenu-item"
                  :class="{ 'is-disabled': subItem.disabled }"
                  @click.stop="handleMenuCommand(subItem)"
                >
                  <span class="option-label" :title="subItem.label">{{ subItem.label }}</span>
                </div>
                <div v-if="item.submenu.length > 0" class="menu-divider"></div>
                <div
                  v-if="item.submenu.length > 0"
                  class="menu-option"
                  @click.stop="handleMenuCommand({ label: '清除最近项目', command: 'clearRecentProjects' })"
                >
                  <span class="option-label">清除最近项目</span>
                </div>
              </div>
            </div>
            <!-- 普通菜单项 -->
            <div
              v-else
              class="menu-option"
              :class="{ 'is-disabled': item.disabled }"
              @click.stop="handleMenuCommand(item)"
            >
              <span class="option-label">{{ item.label }}</span>
              <span v-if="item.shortcut" class="option-shortcut">{{ item.shortcut }}</span>
            </div>
          </template>
        </div>
      </transition>
    </div>

    <!-- 工作区管理对话框（基于 GlobalDialog 的共享组件） -->
    <WorkspaceManageDialog v-model="showWorkspaceDialog" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { load, Store } from '@/core/runtime'
import { invoke } from '@/api/rpc'
import { tauri_request } from '@/utils/tauri_request'
import { ResponseData } from '@/Types/entitys'
import NotificationUtil from '@/utils/NotificationUtil'
import { changeWorkspace } from '@/utils/index'
import { useWorkspaceStore } from '@/store/modules/workspace'
import WorkspaceManageDialog from '@/components/WorkspaceManageDialog.vue'

import { get_file_Api } from '@/api/file'
import useStore from '@/store'
import { useDocumentManagerStore } from '@/plugins/files/store/documents'
import { open, listen } from '@/core/runtime'
import { openAppWindow, createEditorWindow, createPresetWindow } from '@/utils/WindowUtils'
import type { WindowPresetConfig } from '@/utils/WindowUtils'
import type { WindowMeta } from '@/Types/workspace'
import type { WorkspaceConfig } from '@/Types/workspace'
const {fileTreeStore,useLayoutStore } = useStore()
const workspaceStore = useWorkspaceStore()
const docStore = useDocumentManagerStore()
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
}

interface Menu {
  label: string
  items: MenuItem[]
  action?: string
}

const MAX_RECENT_PROJECTS = 20

// ===== 最近项目状态 =====
const recentProjects = ref<string[]>([])

// ===== 窗口预设状态 =====
const windowPresets = ref<WindowPresetConfig[]>([])
const mainWindowPreset = ref<WindowPresetConfig>({
  id: 'main',
  name: '主窗口',
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


// ===== 最近项目持久化 =====
async function loadRecentProjects() {
  if (!store) {
    store = await load('settings.json', { autoSave: false, defaults: {} })
  }
  const val = await store.get<string[]>('recent_projects')
  recentProjects.value = val || []
}

async function saveRecentProjects() {
  if (!store) {
    store = await load('settings.json', { autoSave: false, defaults: {} })
  }
  await store.set('recent_projects', recentProjects.value)
  await store.save()
}

function addRecentProject(dirPath: string) {
  // 移除重复项，添加到最前
  recentProjects.value = recentProjects.value.filter(p => p !== dirPath)
  recentProjects.value.unshift(dirPath)
  // 限制数量
  if (recentProjects.value.length > MAX_RECENT_PROJECTS) {
    recentProjects.value = recentProjects.value.slice(0, MAX_RECENT_PROJECTS)
  }
  saveRecentProjects()
}

function removeRecentProject(dirPath: string) {
  recentProjects.value = recentProjects.value.filter(p => p !== dirPath)
  saveRecentProjects()
}

function clearRecentProjects() {
  recentProjects.value = []
  saveRecentProjects()
}

/** 获取目录名用于显示 */
function getDirName(dirPath: string): string {
  const parts = dirPath.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1] || dirPath
}

// ===== 动态菜单 =====
const menus = computed<Menu[]>(() => {
  const fileItems: MenuItem[] = []

  if (hasFilesPlugin.value) {
    fileItems.push({ label: '打开文件', shortcut: 'Ctrl+O', command: 'openFile' })
    fileItems.push({ label: '打开文件夹', shortcut: 'Ctrl+Shift+O', command: 'openWorkingDirectory' })

    // 最近项目子菜单
    const submenuItems: MenuItem[] = recentProjects.value.map(p => ({
      label: p,
      command: 'openRecentProject',
      _dirPath: p
    } as MenuItem & { _dirPath: string }))

    fileItems.push({
      label: '最近的文件夹',
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
  fileItems.push({ label: '打开工作区', command: 'openWorkspace' })
  fileItems.push({ label: '更换工作文件夹', shortcut: 'Ctrl+Shift+O', command: 'openWorkingDirectory' })

  fileItems.push({ label: '设置', command: 'settings' })

  // 窗口菜单项：列出所有配置的窗口预设
  const windowItems: MenuItem[] = allWindowPresets.value.map(preset => ({
    label: preset.name,
    command: `window_preset_${preset.id}`
  }))
  windowItems.push({ divider: true, label: '', command: '' })
  windowItems.push({ label: '新建窗口', shortcut: 'Ctrl+Shift+N', command: 'newWindow' })

  return [
    {
      label: '文件(F)',
      items: fileItems
    },
    {
      label: '窗口(W)',
      items: windowItems
    },
    {
      label: '帮助(H)',
      items: [
        { label: '帮助', command: 'help' },
        { label: '关于', command: 'about' }
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
const handleMenuClick = (menu: Menu) => {
  if (menu.action) {
    handleMenuCommand({ label: '', command: menu.action })
    return
  }
  toggleMenu(menu.label)
}
const handleMenuCommand = async (item: MenuItem) => {
  closeMenu()
  if (item.disabled) return

  switch (item.command) {
    case 'newFile': {
      const dir = await getWorkingDirectory()
      if (!dir) {
        NotificationUtil.warn('请先选择工作目录')
        return
      }
      const name = prompt('请输入文件名：')
      if (name) {
        const res = await tauri_request<ResponseData<string>>('create_note_file', {
          father_path: dir,
          new_note_md_file_name: name
        })
        if (res?.code === 200) {
          NotificationUtil.success('文件创建成功')
          await get_file_Api()
        } else {
          NotificationUtil.error(res?.msg || '创建失败')
        }
      }
      break
    }
    case 'openFile': {
      const { open } = await import('@/core/runtime')
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Markdown', extensions: ['*'] }]
      })
      if (selected) {
        console.log('[MenuBar] Open file:', selected)
        docStore.openDocument(selected);
      }
      break
    }
    case 'openWorkingDirectory': {
      await changeWorkspace();
      // 记录到最近项目
      if (fileTreeStore.workingDirectory) {
        addRecentProject(fileTreeStore.workingDirectory)
      }
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
        // 移到最近列表最前
        addRecentProject(dirPath)
      }
      break
    }
    case 'clearRecentProjects': {
      clearRecentProjects()
      NotificationUtil.success('已清除最近项目')
      break
    }

    case 'newFolder': {
      const dir = await getWorkingDirectory()
      if (!dir) {
        NotificationUtil.warn('请先选择工作目录')
        return
      }
      const name = prompt('请输入文件夹名：')
      if (name) {
        const res = await tauri_request<ResponseData<string>>('create_folder', {
          father_path: dir,
          new_folder_name: name
        })
        if (res?.code === 200) {
          NotificationUtil.success('文件夹创建成功')
          await get_file_Api()
        } else {
          NotificationUtil.error(res?.msg || '创建失败')
        }
      }
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
      openAppWindow('/App/setting')
      break
    }
    case 'encodingUtf8':
    case 'encodingGbk':
    case 'encodingUtf16':
    case 'encodingAscii': {
      const encoding = item.label
      window.dispatchEvent(new CustomEvent('encoding-change', { detail: { encoding } }))
      NotificationUtil.success(`编码已切换为 ${encoding}`)
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
        NotificationUtil.success(`编辑器窗口已创建: ${meta.label}`)
      } else {
        NotificationUtil.error('创建编辑器窗口失败，请查看控制台日志')
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
    mainWindowPreset.value = { ...mainVal, id: 'main', name: mainVal.name || '主窗口' }
  }
}

/** 窗口预设菜单项点击：已打开则聚焦，未打开则创建 */
async function handleWindowPresetClick(presetId: string) {
  const preset = allWindowPresets.value.find(p => p.id === presetId)
  if (!preset) return

  try {
    // 查询当前已打开的窗口列表
    const res = await invoke<ResponseData<WindowMeta[]>>('get_window_list')
    console.log('[MenuBar] 窗口列表:', JSON.stringify(res.data))
    if (res.code === 200 && res.data) {
      // 优先按 preset_id 匹配，后备按窗口标题（即预设名称）匹配
      const matched = res.data.find(w => w.preset_id === presetId)
        || res.data.find(w => w.title === preset.name)
      if (matched) {
        console.log('[MenuBar] 找到已打开窗口:', matched.label, 'preset_id:', matched.preset_id, 'title:', matched.title)
        await invoke('focus_window', { label: matched.label })
        return
      }
    }
  } catch (e) {
    console.warn('查询窗口列表失败:', e)
  }

  // 窗口未打开，根据预设创建新窗口
  const meta = await createPresetWindow(preset)
  if (!meta) {
    NotificationUtil.error('创建窗口失败，请查看控制台日志')
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

const handleGlobalClick = (e: MouseEvent) => {
  const target = e.target as HTMLElement
  if (!target.closest('.menu-bar')) {
    closeMenu()
  }
}

const handleKeydown = (e: KeyboardEvent) => {
  if (e.altKey) {
    e.preventDefault()
    const key = e.key.toLowerCase()
    const menuMap: Record<string, string> = {
      'f': '文件',
      'e': '编码'
    }
    if (menuMap[key]) {
      activeMenu.value = activeMenu.value === menuMap[key] ? null : menuMap[key]
    }
  }
  if (e.key === 'Escape') {
    closeMenu()
  }
}

onMounted(async () => {
  document.addEventListener('click', handleGlobalClick)
  document.addEventListener('keydown', handleKeydown)
  await loadRecentProjects()
  await loadWindowPresets()
  // 加载当前工作区（新窗口等待用户选择期间不自动分配）
  if (!workspaceStore.pendingWorkspaceSelection) {
    await workspaceStore.loadCurrentWorkspace(null)
  }
  // 监听窗口预设变更（设置窗口新增/修改/删除预设后广播），实时刷新窗口菜单
  unlistenPresetsChanged = await listen('window-presets-changed', () => {
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
    padding: 0 8px;
    height: 100%;
    display: flex;
    align-items: center;
    cursor: pointer;
    font-size: var(--font-size-base);
    white-space: nowrap;
    transition: background-color 0.15s;

    @include theme.useTheme {
      color: theme.getVar('textColor');
    }

    &:hover,
    &.is-active {
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

        &:hover {
          @include theme.useTheme {
            background-color: theme.getVar('hoverColor');
          }
          color: #fff;

          .option-shortcut {
            opacity: 1;
          }
        }

        &.is-disabled {
          opacity: 0.4;
          cursor: not-allowed;

          &:hover {
            background-color: transparent;
            color: inherit;
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
