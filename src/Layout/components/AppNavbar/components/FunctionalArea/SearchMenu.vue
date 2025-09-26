<template>
  <div class="container">
    <el-icon class="icon-box" @click="onSearchRoutes">
      <Search />
    </el-icon>
    <div class="auto-input-box" :class="{ show: isSearch }">
      <el-autocomplete ref="autoInputRef" v-model="keyword" :fetch-suggestions="querySearch" clearable
        class="inline-input w-50" placeholder="Please Input File Name" @blur="isSearch = false"
        @select="handleSelect" />
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import useStore from '@/store'
import type { ComputedRef, Ref } from 'vue'
import type { RouteRecordRaw } from 'vue-router'
import { TreeNode } from '@/Types/entitys'

interface IRoutes extends Omit<RouteRecordRaw, 'redirect'> {
  redirect?: string
}

const router = useRouter()
const { routeStore, routerNoteStore } = useStore()

let autoInputRef: Ref<any> = ref(null)
let keyword: Ref<string> = ref('')
let isSearch: Ref<boolean> = ref(false)
let { flattenRouters } = storeToRefs(routeStore)
const search_data: ComputedRef<TreeNode[]> = computed(() => routerNoteStore.search_data)


let timeout: ReturnType<typeof setTimeout>
const links = ref<TreeNode[]>([{
  id: '0', label: '哈哈哈',
  value: '哈哈哈', // el-autocomplete 需要 value 属性 children: [],
  isFolder: false,
  routerPath: '',
  folder: '',
  filePath: '',
  isfile: false
}, {
  id: '',
  label: '你好',
  value: '你好', // el-autocomplete 需要 value 属性
  isFolder: false,
  routerPath: '',
  folder: '',
  filePath: '',
  isfile: false
}])
const querySearch = (queryString: string, cb: (arg: any) => void) => {
  console.log('queryString', queryString)
  console.log('search_data', search_data.value)

  // 清除之前的定时器
  // clearTimeout(timeout)
  const results = queryString
    ? search_data.value.filter(createFilter(queryString))
    : search_data.value;
  // cb(results)
  clearTimeout(timeout)
  timeout = setTimeout(() => {
    cb(results)
  }, 3000 * Math.random())
}
const createFilter = (queryString: string) => {
  // return (file: TreeNode) => {
  //   console.log('createFilter file',file)
  //   console.log('createFilter file',file.label)
  //   return (
  //     file.label.toLowerCase().indexOf(queryString.toLowerCase()) === 0
  //   )
  // }
  return (file: TreeNode) => {
    try {
      const regexp = new RegExp(queryString, 'i')
      return regexp.test(file.label)
    } catch (e) {
      // 如果正则表达式无效，回退到普通匹配
      return file.label.toLowerCase().includes(queryString.toLowerCase())
    }
  }
}
const handleSelect = (route: TreeNode) => {
  console.log('handleSelect route', route)
  let routerPath = route.routerPath;
  if (routerPath) {
    router.push({
      path: routerPath
    })
  }

}

const onSearchRoutes = () => {
  isSearch.value = true
  // wait animation running time
  setTimeout(() => {
    autoInputRef.value.focus()
  }, 500)
}


</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as variables;

:deep(.el-input__wrapper) {
  box-shadow: none !important;
}

.container {
  margin-right: 12px;

  .auto-input-box {
    display: block;
    overflow: hidden;
    width: 0px;
    transition: width 0.5s;
    border-bottom: 1px solid #ccc;

    &.show {
      width: 200px;
      margin-left: 12px;
    }
  }

  .icon-box {
    cursor: pointer;
    margin-right: 0;

    &:hover {
      color: variables.$theme-color;
    }
  }
}
</style>
