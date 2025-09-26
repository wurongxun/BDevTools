<template>
  <el-popover placement="bottom" :width="300" trigger="click">
    <template #reference>
      <el-badge :is-dot="is_dot">
        <el-icon class="icon-box message-icon">
          <Message />
        </el-icon>
      </el-badge>
    </template>
    <div class="message-box">
      <el-tabs v-model="activeName" class="demo-tabs" stretch>
        <el-tab-pane label="未读" name="wait">
          <el-collapse accordion>
            <el-collapse-item
              v-for="(item, index) in messageList"
              :key="index"
              :title="item.title"
              :name="index"
            >
              <div class="message-content">
                {{ item.content }}
              </div>
            </el-collapse-item>
          </el-collapse>
        </el-tab-pane>
        <el-tab-pane label="已读" name="done">
          <el-empty description="暂无已读消息" />
        </el-tab-pane>
      </el-tabs>
    </div>
  </el-popover>
</template>

<script lang="ts" setup>
import { ref } from 'vue'
let is_dot = ref(false)
let activeName = ref('wait')
const messageList = ref([
  { title: 'MdViewer 0.2.0 版发布', content: 'MdViewer 0.2.0 版发布啦，请升级到最新版' }
])
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as variables;

.icon-box {
  cursor: pointer;

  &:hover {
    color: variables.$theme-color;
  }

  &.message-icon {
    margin-top: 6px;
  }
}
</style>
