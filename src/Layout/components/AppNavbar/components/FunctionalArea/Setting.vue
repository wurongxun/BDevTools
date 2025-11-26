<template>
  <el-dropdown @command="handleCommand">
    <header class="avatar-box">
      <svg-icon class-name="svg-icon" style="height: 1rem;width: 1rem;" icon-class="setting" />
    </header>
    <template #dropdown>
      <el-dropdown-menu>
        <!-- <div class="userinfo-name">{{ userInfo.nickName }}</div> -->
        <el-dropdown-item command="setting">{{ $t('settings.name') }}</el-dropdown-item>
        <el-dropdown-item command="about">{{ $t('about.name') }}</el-dropdown-item>
        <el-dropdown-item command="help" divided>{{ $t('help.name') }}</el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>
<script setup lang="ts">
import useStore from '@/store/index'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useI18n } from "vue-i18n";
const { t, locale } = useI18n()

const { user } = useStore()
const router = useRouter()
const { userInfo } = storeToRefs(user)

const onStting = () => {
  router.push({
    path: '/AppSetting'
  })
}

const onAbout = () => {
  router.push({
    path: '/AppAbout'
  })
}

const onHelp = () => {
  router.push({
    path: '/AppHelp'
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

:deep(.el-dropdown:focus) {
  outline: none !important;
}

:deep(.el-dropdown > header:focus-visible) {
  outline: none !important;
}

.avatar-box {
  // padding: 0 12px;
  display: flex;
  align-items: center;
  // margin-right: 12px;

  &:focus-visible {
    outline: none !important;
  }
}
</style>