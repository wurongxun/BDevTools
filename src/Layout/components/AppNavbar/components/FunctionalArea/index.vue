<template>
  <div class="functional-area">
    <!-- 搜索 -->
    <!-- <div class="fa-item search-item" :class="{ 'is-expanded': isSearch }">
      <el-icon class="fa-icon" @click="onSearchRoutes">
        <Search />
      </el-icon>
      <div class="search-input-wrap">
        <el-autocomplete
          ref="autoInputRef"
          v-model="keyword"
          :fetch-suggestions="querySearch"
          clearable
          class="inline-input w-50"
          placeholder="Please Input File Name"
          @blur="isSearch = false"
          @select="handleSelect"
        />
      </div>
    </div> -->




    <!-- 消息 -->
<!--    <el-popover-->
<!--      placement="bottom-end"-->
<!--      :width="350"-->
<!--      trigger="click"-->
<!--      popper-class="message-popover"-->
<!--      :offset="12"-->
<!--      :show-arrow="false"-->
<!--    >-->
<!--      <template #reference>-->
<!--        <el-badge :is-dot="is_dot" :max="99">-->
<!--          <el-icon class="fa-icon">-->
<!--            <Message />-->
<!--          </el-icon>-->
<!--        </el-badge>-->
<!--      </template>-->
<!--      <div class="message-container">-->
<!--        <div class="message-header">-->
<!--          <h3>消息中心</h3>-->
<!--          <el-button link type="primary" size="small" @click="markAllAsRead">全部已读</el-button>-->
<!--        </div>-->
<!--        <el-tabs v-model="activeName" class="message-tabs">-->
<!--          <el-tab-pane :label="`未读 (${unreadCount})`" name="wait">-->
<!--            <div v-if="messageList.length === 0" class="empty-state">-->
<!--              <el-empty description="暂无未读消息" :image-size="80" />-->
<!--            </div>-->
<!--            <el-scrollbar v-else max-height="300px">-->
<!--              <div-->
<!--                v-for="(item, index) in messageList"-->
<!--                :key="index"-->
<!--                class="message-item"-->
<!--                @click="readMessage(index)"-->
<!--              >-->
<!--                <div class="message-title">{{ item.title }}</div>-->
<!--                <div class="message-text">{{ item.content }}</div>-->
<!--                <div class="message-time">{{ item.time || '刚刚' }}</div>-->
<!--              </div>-->
<!--            </el-scrollbar>-->
<!--          </el-tab-pane>-->
<!--          <el-tab-pane :label="`已读 (${readCount})`" name="done">-->
<!--            <div v-if="readMessageList.length === 0" class="empty-state">-->
<!--              <el-empty description="暂无已读消息" :image-size="80" />-->
<!--            </div>-->
<!--            <el-scrollbar v-else max-height="300px">-->
<!--              <div-->
<!--                v-for="(item, index) in readMessageList"-->
<!--                :key="index"-->
<!--                class="message-item read"-->
<!--              >-->
<!--                <div class="message-title">{{ item.title }}</div>-->
<!--                <div class="message-text">{{ item.content }}</div>-->
<!--                <div class="message-time">{{ item.time }}</div>-->
<!--              </div>-->
<!--            </el-scrollbar>-->
<!--          </el-tab-pane>-->
<!--        </el-tabs>-->
<!--      </div>-->
<!--    </el-popover>-->

    <!-- 主题切换 -->
    <el-icon class="fa-icon" @click="dark_change" v-show="!isDark">
      <Sunny />
    </el-icon>
    <el-icon class="fa-icon" @click="dark_change" v-show="isDark">
      <Moon />
    </el-icon>
    <!-- 切换侧边栏 -->
    <div
      class="fa-icon"
      :class="{ 'is-active': !isCollapse }"
      @click="toggleSidebar"
      :title="isCollapse ? '显示侧边栏' : '隐藏侧边栏'"
    >
      <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
        <rect x="3" y="3" width="18" height="18" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/>
        <rect x="3" y="3" width="6" height="18" rx="1" :fill="!isCollapse ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="1.2" opacity="0.9"/>
      </svg>
    </div>
    <!-- 窗口控制 -->
    <WindowControls />
  </div>
</template>

<script lang="ts" setup name="FunctionalArea">import { ref, computed, onMounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
import { Search, Sunny, Moon, Message } from '@element-plus/icons-vue'
import { load, Store } from '@tauri-apps/plugin-store'
import { isDark, toggleDark } from '@/composables/useDark'
import useStore from '@/store'
import { TreeNode } from '@/Types/entitys'
import WindowControls from '@/components/WindowControls.vue'
import { openAppWindow } from '@/utils/WindowUtils'
import { useDocumentManagerStore } from '@/plugins/files/store/documents'
const store = useStore()
const docStore = useDocumentManagerStore()
const { t } = useI18n()

const { isCollapse } = storeToRefs(store.system)
const { search_data } = storeToRefs(store.fileTreeStore)

// ========== 侧边栏切换 ==========
function toggleSidebar(){
console.log(isCollapse.value)
  if (isCollapse.value) {
    store.system.expandCollapse()
  } else {
    store.system.packCollapse()
  }
}

// ========== 主题切换 ==========

let piniaStore: Store | undefined
async function dark_change() {
  if (piniaStore) {
    toggleDark()
    await piniaStore.set('dark', { value: isDark.value })
    await piniaStore.save()
    store.system.toggleDark(isDark.value)
  }
}
onMounted(async () => {
  piniaStore = await load('settings.json', { autoSave: false, defaults: {} })
  const dark_val = await piniaStore.get<{ value: boolean }>('dark')
  if (dark_val !== undefined && dark_val.value !== isDark.value) {
    toggleDark()
    store.system.toggleDark(isDark.value)
  }
})

// ========== 搜索 ==========

let autoInputRef = ref<any>(null)
let keyword = ref('')
let isSearch = ref(false)

let timeout: ReturnType<typeof setTimeout>
const querySearch = (queryString: string, cb: (arg: any) => void) => {
  const results = queryString ? search_data.value.filter(createFilter(queryString)) : search_data.value
  clearTimeout(timeout)
  timeout = setTimeout(() => cb(results), 3000 * Math.random())
}
const createFilter = (queryString: string) => {
  return (file: TreeNode) => {
    try {
      return new RegExp(queryString, 'i').test(file.label)
    } catch {
      return file.label.toLowerCase().includes(queryString.toLowerCase())
    }
  }
}
const handleSelect = (route: TreeNode) => {
  if (route.routerPath) docStore.openDocument(route.routerPath)
}
const onSearchRoutes = () => {
  isSearch.value = true
  setTimeout(() => autoInputRef.value?.focus(), 500)
}

// ========== 消息 ==========
let is_dot = ref(true)
let activeName = ref('wait')

interface MessageItem {
  title: string
  content: string
  time?: string
  read?: boolean
}

const messageList = ref<MessageItem[]>([
  { title: 'BackendDevelopmentTools 0.2.0 版发布', content: 'BackendDevelopmentTools 0.2.0 版发布啦，新增了多项功能和优化，请升级到最新版体验！', time: '2024-01-15 10:30' },
  { title: '系统通知', content: '欢迎使用 BackendDevelopmentTools，这是一个基于 Tauri 的跨平台桌面工具。', time: '2024-01-14 09:00' }
])
const readMessageList = ref<MessageItem[]>([])
const unreadCount = computed(() => messageList.value.length)
const readCount = computed(() => readMessageList.value.length)

const readMessage = (index: number) => {
  const message = messageList.value.splice(index, 1)[0]
  message.read = true
  message.time = new Date().toLocaleString('zh-CN')
  readMessageList.value.unshift(message)
  if (messageList.value.length === 0) is_dot.value = false
}
const markAllAsRead = () => {
  readMessageList.value.unshift(...messageList.value.map(msg => ({ ...msg, read: true, time: new Date().toLocaleString('zh-CN') })))
  messageList.value = []
  is_dot.value = false
}

// ========== 设置 ==========
const handleSettingCommand = (command: string) => {
  const pages: Record<string, string> = { setting: '/App/setting', about: '/App/about', help: '/App/help' }
  if (pages[command]) openAppWindow(pages[command])
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.functional-area {
  display: flex;
  align-items: center;
  height: 100%;
  gap: 2px;
}

.fa-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  cursor: pointer;
  border-radius: 4px;
  transition: background-color 0.15s;
  flex-shrink: 0;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &:hover {
    @include theme.useTheme {
      color: theme.getVar('textColor');
      background-color: theme.getVar('activityBarHover');
    }
  }

  &.is-active {
    @include theme.useTheme {
      color: theme.getVar('activityBarActive');
    }
  }

  svg {
    width: 14px;
    height: 14px;
  }
}
</style>
