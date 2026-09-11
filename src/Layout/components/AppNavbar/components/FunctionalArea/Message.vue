<template>
  <el-popover
    placement="bottom-end"
    :width="350"
    trigger="click"
    popper-class="message-popover"
    :virtual-ref="undefined"
    :offset="12"
    :show-arrow="false"
  >
    <template #reference>
      <el-badge :is-dot="is_dot" :max="99">
        <el-icon class="icon-box message-icon">
          <Message />
        </el-icon>
      </el-badge>
    </template>
    <div class="message-container">
      <div class="message-header">
        <h3>消息中心</h3>
        <app-button link type="primary" size="small" @click="markAllAsRead">
          全部已读
        </app-button>
      </div>
      <el-tabs v-model="activeName" class="message-tabs">
        <el-tab-pane :label="`未读 (${unreadCount})`" name="wait">
          <div v-if="messageList.length === 0" class="empty-state">
            <el-empty description="暂无未读消息" :image-size="80" />
          </div>
          <el-scrollbar v-else max-height="300px">
            <div
              v-for="(item, index) in messageList"
              :key="index"
              class="message-item"
              @click="readMessage(index)"
            >
              <div class="message-title">{{ item.title }}</div>
              <div class="message-text">{{ item.content }}</div>
              <div class="message-time">{{ item.time || '刚刚' }}</div>
            </div>
          </el-scrollbar>
        </el-tab-pane>
        <el-tab-pane :label="`已读 (${readCount})`" name="done">
          <div v-if="readMessageList.length === 0" class="empty-state">
            <el-empty description="暂无已读消息" :image-size="80" />
          </div>
          <el-scrollbar v-else max-height="300px">
            <div
              v-for="(item, index) in readMessageList"
              :key="index"
              class="message-item read"
            >
              <div class="message-title">{{ item.title }}</div>
              <div class="message-text">{{ item.content }}</div>
              <div class="message-time">{{ item.time }}</div>
            </div>
          </el-scrollbar>
        </el-tab-pane>
      </el-tabs>
    </div>
  </el-popover>
</template>

<script lang="ts" setup>
import { ref, computed } from 'vue'

let is_dot = ref(true)
let activeName = ref('wait')

interface MessageItem {
  title: string
  content: string
  time?: string
  read?: boolean
}

const messageList = ref<MessageItem[]>([
  {
    title: 'BackendDevTools 0.2.0 版发布',
    content: 'BackendDevTools 0.2.0 版发布啦，新增了多项功能和优化，请升级到最新版体验！',
    time: '2024-01-15 10:30'
  },
  {
    title: '系统通知',
    content: '欢迎使用 BackendDevTools，这是一个基于 Tauri 的跨平台桌面工具。',
    time: '2024-01-14 09:00'
  }
])

const readMessageList = ref<MessageItem[]>([])

const unreadCount = computed(() => messageList.value.length)
const readCount = computed(() => readMessageList.value.length)

const readMessage = (index: number) => {
  const message = messageList.value.splice(index, 1)[0]
  message.read = true
  message.time = new Date().toLocaleString('zh-CN')
  readMessageList.value.unshift(message)

  if (messageList.value.length === 0) {
    is_dot.value = false
  }
}

const markAllAsRead = () => {
  readMessageList.value.unshift(...messageList.value.map(msg => ({
    ...msg,
    read: true,
    time: new Date().toLocaleString('zh-CN')
  })))
  messageList.value = []
  is_dot.value = false
}
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as variables;

.icon-box {
  font-size: var(--font-size-2xl);
  cursor: pointer;
  transition: color 0.2s ease;
  vertical-align: middle;

  &:hover {
    color: variables.$theme-color;
  }

  &.message-icon {
    margin-top: 0;
  }
}

.message-container {
  .message-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--el-border-color-lighter);

    h3 {
      margin: 0;
      font-size: var(--font-size-xl);
      font-weight: 600;
      color: var(--el-text-color-primary);
    }
  }

  .message-tabs {
    :deep(.el-tabs__header) {
      margin: 0;
      padding: 0 16px;
    }

    :deep(.el-tabs__nav-wrap::after) {
      height: 1px;
    }
  }

  .empty-state {
    padding: 40px 20px;
    text-align: center;
  }

  .message-item {
    padding: 12px 16px;
    border-bottom: 1px solid var(--el-border-color-lighter);
    cursor: pointer;
    transition: all 0.2s ease;

    &:hover {
      background-color: var(--el-fill-color-light);
    }

    &.read {
      opacity: 0.7;
    }

    .message-title {
      font-size: var(--font-size-lg);
      font-weight: 500;
      color: var(--el-text-color-primary);
      margin-bottom: 6px;
    }

    .message-text {
      font-size: var(--font-size-md);
      color: var(--el-text-color-regular);
      line-height: 1.5;
      margin-bottom: 6px;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
    }

    .message-time {
      font-size: var(--font-size-base);
      color: var(--el-text-color-secondary);
    }
  }
}
</style>

<style lang="scss">
// 全局样式，用于提升 popover 层级并确保显示在最顶层
.message-popover {
  z-index: 99999 !important;

  &.el-popper {
    z-index: 99999 !important;
  }

  .el-popover__inner {
    padding: 0;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15) !important;
    border-radius: var(--radius-xl);
    overflow: hidden;
  }

  // 确保箭头也显示在顶层
  .el-popper__arrow {
    z-index: 99999 !important;

    &::before {
      background: white;
      box-shadow: -2px -2px 4px rgba(0, 0, 0, 0.05);
    }
  }
}

// 针对深色模式的适配
html.dark .message-popover {
  .el-popper__arrow::before {
    background: var(--el-bg-color);
    box-shadow: -2px -2px 4px rgba(0, 0, 0, 0.3);
  }
}
</style>
