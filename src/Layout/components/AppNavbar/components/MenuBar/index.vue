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
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { load, Store } from '@tauri-apps/plugin-store'
import { tauri_request } from '@/utils/tauri_request'
import { ResponseData } from '@/Types/entitys'
import NotificationUtil from '@/utils/NotificationUtil'
import { get_notebooks_Api } from '@/api/file'
import useStore from '@/store'
import { open } from '@tauri-apps/plugin-dialog'
import { openAppWindow } from '@/utils/WindowUtils'
const {useDocumentManagerStore,routerNoteStore } = useStore()
const router = useRouter()
const activeMenu = ref<string | null>(null)
let store: Store | undefined

interface MenuItem {
  label: string
  shortcut?: string
  divider?: boolean
  disabled?: boolean
  command: string
}

interface Menu {
  label: string
  items: MenuItem[]
  action?: string
}
// items: [
//   { label: '新建文件', shortcut: 'Ctrl+N', command: 'newFile' },
//   { label: '打开文件', shortcut: 'Ctrl+O', command: 'openFile' },
//   { label: '新建文件夹', command: 'newFolder' },
//   { divider: true, label: '', command: '' },
//   { label: '保存', shortcut: 'Ctrl+S', command: 'save' },
//   { label: '另存为', shortcut: 'Ctrl+Shift+S', command: 'saveAs' },
//   { divider: true, label: '', command: '' },
//   { label: '设置', command: 'settings' },
// ]

// ,
// {
//   label: '编码(N)',
//     items: [
//   { label: 'UTF-8', command: 'encodingUtf8' },
//   { label: 'GBK', command: 'encodingGbk' },
//   { label: 'UTF-16', command: 'encodingUtf16' },
//   { label: 'ASCII', command: 'encodingAscii' },
// ]
// }
const menus: Menu[] = [
  {
    label: '文件(F)',
    items: [
      { label: '打开文件', shortcut: 'Ctrl+O', command: 'openFile' },
      { label: '更换工作区', shortcut: 'Ctrl+O', command: 'openWorkingDirectory' },
      { divider: true, label: '', command: '' },
      { label: '设置', command: 'settings' },
    ]
  },
  {
    label: '终端(T)',
    items: [],
    action: 'newTerminal'
  },
  {
    label: '帮助(H)',
    items: [
      { label: '帮助', command: 'help' },
      { label: '关于', command: 'about' }
    ]
  }
]

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
          await get_notebooks_Api()
        } else {
          NotificationUtil.error(res?.msg || '创建失败')
        }
      }
      break
    }
    case 'openFile': {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Markdown', extensions: ['*'] }]
      })
      if (selected) {
        console.log('[MenuBar] Open file:', selected)
        useDocumentManagerStore.openDocument(selected);
      }
      break
    }
    case 'openWorkingDirectory': {
      let file_directory: string | null = await open({
        multiple: false,
        directory: true
      })
      if (file_directory == null){
        return
      }
      if (!store){
        store = await load('settings.json', {
          autoSave: false,
          defaults: {}
        })
      }
      if (store) {
        await store.set('md_working_directory', { value: file_directory })
        await store.save();
        await get_notebooks_Api();
      }

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
          await get_notebooks_Api()
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
      openAppWindow('/AppSetting')
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
      openAppWindow('/AppHelp')
      break
    }
    case 'about': {
      openAppWindow('/AppAbout')
      break
    }
    case 'newTerminal': {
      openAppWindow('/AppTerminal')
      break
    }
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

onMounted(() => {
  document.addEventListener('click', handleGlobalClick)
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('click', handleGlobalClick)
  document.removeEventListener('keydown', handleKeydown)
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
    padding: 0 10px;
    height: 100%;
    display: flex;
    align-items: center;
    cursor: pointer;
    font-size: 13px;
    white-space: nowrap;
    transition: background-color 0.15s;

    @include theme.useTheme {
      color: theme.getVar('textColor');
    }

    &:hover,
    &.is-active {
      @include theme.useTheme {
        background-color: rgba(128, 128, 128, 0.12);
      }
    }

    .menu-dropdown {
      position: absolute;
      top: 100%;
      left: 0;
      min-width: 200px;
      padding: 4px 0;
      z-index: 99999;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
      border-radius: 6px;

      @include theme.useTheme {
        background-color: theme.getVar('bgColor');
        border: 1px solid theme.getVar('borderColor');
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
        padding: 6px 16px;
        font-size: 13px;
        cursor: pointer;
        transition: background-color 0.15s;

        @include theme.useTheme {
          color: theme.getVar('textColor');
        }

        &:hover {
          @include theme.useTheme {
            background-color: $theme-color;
            color: #fff;
          }

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
        }

        .option-shortcut {
          font-size: 12px;
          opacity: 0.5;
          margin-left: 24px;
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
