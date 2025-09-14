<template>
  <el-dropdown @command="handleCommand">
    <!-- <el-badge :is-dot="errorStore.isNewError"> -->
    <svg-icon class-name="svg-icon" style="height: 1rem;width: 1rem;" icon-class="setting" />
    <!-- </el-badge> -->
    <template #dropdown>
      <el-dropdown-menu>
        <div class="userinfo-name">{{ userInfo.nickName }}</div>
        <el-dropdown-item command="setting">设置</el-dropdown-item>
        <el-dropdown-item command="about">关于</el-dropdown-item>
        <el-dropdown-item command="help" divided>帮助</el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>
<script setup lang="ts">
import useStore from '@/store/index'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'

const { user } = useStore()
const router = useRouter()
const { userInfo } = storeToRefs(user)

const onStting = () => {
  router.push({
    path: '/App/AppSetting'
  })
}

const onAbout = () => {
  router.push({
     path: '/App/AppAbout'
  })
}

const onHelp = () => {
  router.push({
     path: '/App/AppHelp'
  })
}

const handleCommand = (command: string | number | object) => {
  switch (command) {
    case 'setting':
      onStting()
      break
    case 'about':
      onAbout()
      break
    case 'help':
      onHelp()
      break

    default:
      break
  }
}
</script>

<style scoped lang="scss">
.svg-icon {
  width: 20px;
  height: 20px;
  margin-left: 12px;
  margin-right: 12px;
  // margin-bottom: 2px;
  cursor: pointer;
}
</style>
