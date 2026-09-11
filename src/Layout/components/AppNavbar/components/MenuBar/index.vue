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
                  <span class="option-label">暂无记录</span>
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
                  <span class="option-label">清除最近文件夹</span>
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
import { load, Store } from '@/core/runtime'
import { invoke } from '@/api/rpc'
import { tauri_request } from '@/utils/tauri_request'
import { ResponseData } from '@/Types/entitys'
import NotificationUtil from '@/utils/NotificationUtil'
import { changeWorkspace } from '@/utils/index'
import { useWorkspaceStore } from '@/store/modules/workspace'
import WorkspaceManageDialog from '@/components/WorkspaceManageDialog.vue'
import GlobalPromptDialog from '@/components/GlobalPromptDialog.vue'

import { get_file_Api } from '@/api/file'
import useStore from '@/store'
import { useDocumentManagerStore } from '@/plugins/files/store/documents'
import { open, subscribe } from '@/core/runtime'
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
    fileItems.push({ label: '打开文件', shortcut: 'Ctrl+O', command: 'openFile' })
    fileItems.push({ label: '打开文件夹', shortcut: 'Ctrl+Shift+O', command: 'openWorkingDirectory' })

    // 最近文件夹子菜单（来源于当前工作区 folders）
    const submenuItems: MenuItem[] = recentProjectDirs.value.map(p => ({
      label: getDirName(p),
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
// ===== 命名输入弹窗（替代原生 prompt） =====
const showNamePrompt = ref(false)
const namePromptTitle = ref('')
const namePromptMessage = ref('')
const pendingNameAction = ref<'file' | 'folder'>('file')

/** 文件名/文件夹名校验：拦截空值与 Windows 非法字符 */
function validateName(value: string): string | true {
  if (!value) return '名称不能为空'
  if (/[\\/:*?"<>|]/.test(value)) return '名称不能包含 \\ / : * ? " < > | 字符'
  return true
}

async function handleNameConfirm(name: string) {
  const dir = await getWorkingDirectory()
  if (!dir) {
    NotificationUtil.warn('请先选择工作目录')
    return
  }
  if (pendingNameAction.value === 'file') {
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
  } else {
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

  switch (item.command) {
    case 'newFile': {
      const dir = await getWorkingDirectory()
      if (!dir) {
        NotificationUtil.warn('请先选择工作目录')
        return
      }
      // 用项目内输入弹窗替代浏览器原生 prompt()：可主题化、可校验
      pendingNameAction.value = 'file'
      namePromptTitle.value = '新建文件'
      namePromptMessage.value = `将在 ${dir} 下创建文件`
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
      NotificationUtil.success('已清除最近文件夹')
      break
    }

    case 'newFolder': {
      const dir = await getWorkingDirectory()
      if (!dir) {
        NotificationUtil.warn('请先选择工作目录')
        return
      }
      pendingNameAction.value = 'folder'
      namePromptTitle.value = '新建文件夹'
      namePromptMessage.value = `将在 ${dir} 下创建文件夹`
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
    if (res.code === 200 && res.data) {
      // 优先按 preset_id 匹配，后备按窗口标题（即预设名称）匹配
      const matched = res.data.find(w => w.preset_id === presetId)
        || res.data.find(w => w.title === preset.name)
      if (matched) {
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
  // 监听窗口预设变更（设置窗口新增/修改/删除预设后广播），实时刷新窗口菜单
  unlistenPresetsChanged = await subscribe('window-presets-changed', () => {
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
