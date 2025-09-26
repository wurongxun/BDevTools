<template>
  <div class="logo-box" :class="{ 'is-collapse': isCollapse }" @click="handleToHome" data-tauri-drag-region>
    <el-image class="logo" fit="contain" :src="logo" />
    <h1 class="title" :class="{ 'shrink-title': isCollapse }" data-tauri-drag-region>{{ title }}</h1>
  </div>
</template>

<script setup lang="ts">
import useStore from '@/store'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import { title } from '@/setting'
import logo from '@/assets/logo/64x64.png'

const { system } = useStore()
const { isCollapse } = storeToRefs(system)
const router = useRouter()

function handleToHome() {
  router.push({
    path: '/'
  })
}
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as variables;

.logo-box {
  user-select: none;
  width: variables.$menu-width;
  overflow: hidden;
  height: 51px;
  display: flex;
  align-items: center;
  cursor: pointer;
  margin-bottom: 1px;

  @include useTheme {
    border-bottom: 1px solid getVar('borderColor');
  }

  &.is-collapse {
    width: variables.$collapse-width;
  }

  .icon {
    margin: 15px;
    flex-shrink: 0;
  }

  .title {
    font-size: 16px;
    white-space: nowrap;

    &.shrink-title {
      display: none;
    }
  }
}

.logo {
  width: 50px;
  height: 50px;
  padding: 4px 4px;
}
</style>
