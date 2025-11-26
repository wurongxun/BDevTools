<template>
  <el-breadcrumb class="breadcrumb-container" separator=">" data-tauri-drag-region>
    <el-breadcrumb-item v-for="item in list" :key="item.path">
      {{ getMetaName(item) }}
    </el-breadcrumb-item>
  </el-breadcrumb>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from "vue-i18n";

const { t, locale } = useI18n()
const router = useRouter()
const list = computed(() => {
  const { matched } = router.currentRoute.value
  return matched.filter((item) => item.meta.title && !item.meta.noBreadCrumbs)
})

function getMetaName(item: string | any) {
  if (item.meta.i18n) {
    return t(item.meta.i18n)
  }
  return item.meta.title;
}

</script>

<style lang="scss" scoped>
.breadcrumb-container {
  display: flex;
  align-items: center;
  user-select: none; // 添加这一行

  :deep(.el-breadcrumb__inner, .el-breadcrumb__item) {
    display: inline-flex;
    align-items: center;
    margin: 1px;
  }

  .menu-icon {
    padding-right: 5px;
  }
}
</style>
